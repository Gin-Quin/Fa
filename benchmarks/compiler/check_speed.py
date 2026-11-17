#!/usr/bin/env python3
"""Paired fresh-process checker audit. See README.md; Python stdlib only."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import statistics
import subprocess
import sys
import time
import tempfile

ROOT = Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "benchmarks/audit"))
from measurements import control_passes, order_for_round, paired, quantile, sample, snapshot


def run(command, **kwargs):
    return subprocess.run(list(map(str, command)), check=True, capture_output=True, **kwargs)


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def identity(path):
    path = Path(path).resolve()
    return {"path": str(path), "sha256": digest(path), "bytes": path.stat().st_size}


def distribution(values):
    return {"q1": quantile(values, .25), "median": statistics.median(values),
            "q3": quantile(values, .75)}


def comparison(a, b, metric, control):
    result = paired([s[metric] for s in a], [s[metric] for s in b])
    estimates = [result["ratio_of_medians"], result["median"]]
    result["threshold"] = 1.2
    if ((estimates[0] <= 1.2) != (estimates[1] <= 1.2)
            or (any(abs(v - 1.2) <= .05 for v in estimates) and not control_passes(control))):
        result["verdict"] = "unresolved"
    else:
        result["verdict"] = "met" if all(v <= 1.2 for v in estimates) else "behind"
    return result


def commands(args, fixture, mode):
    fa = ["taskset", "-c", "5" if mode == "single" else "0-4", "env", "-u", "RAYON_NUM_THREADS"]
    if mode == "single":
        fa += ["RAYON_NUM_THREADS=1"]
    fa += [str(args.fa), "check", fixture["faInput"]]
    bun = ["taskset", "-c", "5" if mode == "single" else "0-4", str(args.bun), "check", "--no-script", "-p", fixture["tsInput"]]
    if mode == "single":
        bun += ["--threads=1"]
    return {"fa": fa, "bun": bun, "fa_control": fa}


def prepare(args, data):
    fixtures = []
    for count in args.files:
        directory = args.directory / str(count)
        shutil.rmtree(directory, ignore_errors=True)
        generated = run(["taskset", "-c", "0-4", args.bun, ROOT / "benchmarks/compiler/paired.ts", directory, count, args.lines])
        fixture = json.loads(generated.stdout)
        # The single-file input is a copy; package entry/configuration are unused for it.
        for language in ("fa", "ts"):
            paths = ([Path(fixture["faInput"])] if language == "fa" and count == 1 else
                     [directory / "ts/library/m0001.ts"] if language == "ts" and count == 1 else
                     sorted((directory / language).rglob(f"*.{language}")))
            fixture[language] = {"files": len(paths), "lines": sum(p.read_bytes().count(b"\n") for p in paths),
                                 "bytes": sum(p.stat().st_size for p in paths),
                                 "sources": {str(p.relative_to(directory)): digest(p) for p in paths}}
        # Explicit independent validation against the pinned TS7 binary as well as Bun.
        run(["taskset", "-c", "0-4", args.bun, ROOT / "packages/benchmark-typescript/node_modules/@typescript/native/bin/tsc",
             "--noEmit", "-p", fixture["tsInput"]], timeout=180)
        fixtures.append(fixture)
        print(f"prepared {count}: Fa {fixture['fa']['lines']} / TS {fixture['ts']['lines']} lines", flush=True)
    data["fixtures"] = fixtures


def measure(args, data):
    data.setdefault("series", [])
    for fixture in data["fixtures"]:
        for mode in args.modes:
            row = {"files": fixture["files"], "mode": mode, "start": snapshot(), "samples": {}, "orders": [], "runner_sha256": digest(__file__)}
            cmds = commands(args, fixture, mode)
            row["commands"] = cmds
            # A whole successful run warms filesystem pages; no checker cache is persisted.
            for label in ("fa", "bun"):
                sample(cmds[label], b"", timeout=300, max_rss_kib=512 * 1024 if label == "fa" else None)
            for index in range(args.runs):
                order = order_for_round(list(cmds), index)
                row["orders"].append(order)
                for label in order:
                    value = sample(cmds[label], b"", timeout=300,
                                   max_rss_kib=512 * 1024 if label.startswith("fa") else None)
                    row["samples"].setdefault(label, []).append(value)
                print(f"{fixture['files']} {mode}: round {index + 1}/{args.runs}", flush=True)
            row["summary"] = {label: {metric: distribution([s[metric] for s in samples])
                                              for metric in ("wall", "cpu", "user", "system", "peak_rss_kib")}
                              for label, samples in row["samples"].items()}
            row["controls"] = {}
            row["comparisons"] = {}
            for metric in ("wall", "cpu"):
                control = paired([s[metric] for s in row["samples"]["fa_control"]], [s[metric] for s in row["samples"]["fa"]])
                control["passes"] = control_passes(control)
                row["controls"][metric] = control
                row["comparisons"][metric] = comparison(row["samples"]["fa"], row["samples"]["bun"], metric, control)
                if args.runs < 11:
                    row["comparisons"][metric]["verdict"] = "smoke only"
            row["end"] = snapshot()
            data["series"].append(row)
            save(args, data)


# Exclusive instruction attribution: disjoint categories, not overlapping inclusive sums.
# Optimizer inlining moves helper costs into their caller. Keep unclassified runtime visible.
def category(name):
    rules = [
        ("compile_time", ("fa_interpreter", "fa_analyzer::comptime", "::check::comptime")),
        ("references_moves_relations", ("::check::references", "::check::moves", "::check::relations", "::flow::values")),
        ("flow_effects", ("::flow::", "::check::flow", "::check::effects", "::check::control", "::check::conditions", "::check::tasks")),
        ("reading_parsing", ("fa_parser::", "::parse_module", "std::fs::", "std::io::")),
        ("scanning_resolving", ("fa_codebase_parser::", "fa_analyzer::scan", "fa_analyzer::graph", "::check::modules", "::symbols::", "::resolve")),
        ("rendering", ("fa::report", "::diagnostics::", "fa::color")),
        ("types_checking", ("fa_analyzer::types", "fa_analyzer::check::", "fa_analyzer::syntax")),
        ("pipeline_session", ("fa_analyzer::", "fa::")),
    ]
    return next((group for group, patterns in rules if any(p in name for p in patterns)), "runtime_other")


def profile(args, data):
    fixture = data["fixtures"][-1]
    directory = args.directory / "profile"
    directory.mkdir(parents=True, exist_ok=True)
    result = {"files": fixture["files"], "timings": {}, "runner_sha256": digest(__file__)}
    for mode in ("single", "default"):
        command = commands(args, fixture, mode)["fa"] + ["--timings"]
        path = directory / f"timings-{mode}.txt"
        measured = corpus_sample(command, output_path=path, timeout=300)
        if measured["failure"] or measured["exit"]:
            raise RuntimeError(f"phase profile failed: {measured}")
        result["timings"][mode] = path.read_text()
        result.setdefault("timing_resources", {})[mode] = measured
    executable = shutil.which("valgrind") or str(Path.home() / ".local/bin/valgrind")
    output = directory / "callgrind.out"
    command = ["taskset", "-c", "0-4", "env", "RAYON_NUM_THREADS=1", executable,
               "--tool=callgrind", "--compress-strings=no", "--compress-pos=no",
               f"--callgrind-out-file={output}", str(args.fa), "check", fixture["faInput"]]
    result["callgrind_command"] = command
    data["profile"] = result
    save(args, data)
    # Profilers do not run on reserved measurement CPU 5. Their RSS is host overhead.
    with (directory / "callgrind.log").open("wb") as log:
        subprocess.run(command, check=True, stdout=log, stderr=log, timeout=7200)
    annotate = shutil.which("callgrind_annotate") or str(Path.home() / ".local/valgrind/usr/bin/callgrind_annotate")
    annotation = run([annotate, "--inclusive=no", "--threshold=100", output]).stdout.decode()
    (directory / "functions.txt").write_text(annotation)
    functions = []
    for line in annotation.splitlines():
        match = re.match(r"\s*([\d,]+)\s+\([^)]*\)\s+(.*)", line)
        if match and "PROGRAM TOTALS" not in match[2]:
            name = match[2]
            functions.append({"instructions": int(match[1].replace(",", "")), "function": name, "category": category(name)})
    total = re.search(r"([\d,]+)\s+\([^)]*\)\s+PROGRAM TOTALS", annotation)
    result["instructions"] = int(total[1].replace(",", "")) if total else None
    result["functions"] = functions
    groups = {}
    for function in functions:
        group = function["category"]
        groups[group] = groups.get(group, 0) + function["instructions"]
    result["exclusive_categories"] = groups
    result["callgrind"] = {"path": str(output), "sha256": digest(output)}
    data["profile"] = result
    save(args, data)


def corpus_sample(command, output_path=None, timeout=60):
    with tempfile.TemporaryFile() as output:
        start = time.perf_counter()
        child = subprocess.Popen(command, stdout=output, stderr=output)
        failure = None
        while True:
            pid, status, usage = os.wait4(child.pid, os.WNOHANG)
            if pid:
                child.returncode = os.waitstatus_to_exitcode(status)
                break
            try:
                proc = Path(f"/proc/{child.pid}/status").read_text()
                rss = next((int(line.split()[1]) for line in proc.splitlines() if line.startswith("VmRSS:")), 0)
            except FileNotFoundError:
                rss = 0
            if failure is None and (rss > 512 * 1024 or time.perf_counter() - start > timeout):
                failure = "memory limit" if rss > 512 * 1024 else "timeout"
                child.kill()
            time.sleep(.002)
        output.seek(0)
        diagnostics = output.read()
        if output_path is not None:
            output_path.write_bytes(diagnostics)
        if usage.ru_maxrss > 512 * 1024:
            failure = "memory limit"
        return {"wall": time.perf_counter() - start, "user": usage.ru_utime,
                "system": usage.ru_stime, "cpu": usage.ru_utime + usage.ru_stime,
                "peak_rss_kib": usage.ru_maxrss, "exit": child.returncode,
                "diagnostic_bytes": len(diagnostics), "diagnostic_sha256": hashlib.sha256(diagnostics).hexdigest(),
                "failure": failure if failure else (None if child.returncode in (0, 1) else "abnormal exit")}


def corpus(args, data):
    paths = run(["git", "ls-files", "*.fa"], cwd=ROOT).stdout.decode().splitlines()
    result = {"files": len(paths), "start": snapshot(), "samples": [],
              "description": "Each tracked .fa checked in its repository context, fresh process, sequential. Intentional negative fixtures retain exit 1."}
    # Measure Fa itself, not a Python wrapper: RSS is enforced during every check.
    for index, path in enumerate(paths):
        command = ["taskset", "-c", "5", "env", "RAYON_NUM_THREADS=1", str(args.fa), "check", str(ROOT / path)]
        value = corpus_sample(command)
        value["path"] = path
        result["samples"].append(value)
        if index % 100 == 0:
            print(f"corpus {index + 1}/{len(paths)}", flush=True)
            data["corpus"] = result
            save(args, data)
    result["end"] = snapshot()
    result["exit_counts"] = {str(code): sum(s["exit"] == code for s in result["samples"])
                             for code in sorted({s["exit"] for s in result["samples"]})}
    result["failures"] = [s for s in result["samples"] if s["failure"]]
    result["totals"] = {key: sum(s[key] for s in result["samples"]) for key in ("wall", "user", "system", "cpu")}
    result["max_peak_rss_kib"] = max(s["peak_rss_kib"] for s in result["samples"])
    data["corpus"] = result
    save(args, data)


def markdown(data):
    lines = ["# Checker comparison", "", "Generated from the accompanying JSON; seconds and MiB. Default parallelism on CPUs 0–4 is the wall-time goal; single worker on CPU 5 measures CPU cost. Shared machine, warm filesystem cache, fresh checker processes.", "",
             "| Files | Mode | Fa wall Q1 / med / Q3 | Bun wall Q1 / med / Q3 | Fa/Bun wall medians / paired [Q1,Q3] | Wall goal | CPU ratio medians / paired | Control wall / CPU | Fa / Bun peak MiB (max) |", "|---:|---|---:|---:|---:|---|---:|---|---:|"]
    def dist(d):
        return " / ".join(f"{d[k]:.4f}" for k in ("q1", "median", "q3"))
    for row in data.get("series", []):
        a, b = row["summary"]["fa"], row["summary"]["bun"]
        w, c = row["comparisons"]["wall"], row["comparisons"]["cpu"]
        ctrl = " / ".join(f"{row['controls'][m]['ratio_of_medians']:.3f},{row['controls'][m]['median']:.3f} {'pass' if row['controls'][m]['passes'] else 'fail'}" for m in ("wall", "cpu"))
        rss = [max(s["peak_rss_kib"] for s in row["samples"][label]) / 1024 for label in ("fa", "bun")]
        lines.append(f"| {row['files']} | {row['mode']} | {dist(a['wall'])} | {dist(b['wall'])} | {w['ratio_of_medians']:.3f} / {w['median']:.3f} [{w['q1']:.3f},{w['q3']:.3f}] | {w['verdict']} | {c['ratio_of_medians']:.3f} / {c['median']:.3f} | {ctrl} | {rss[0]:.1f} / {rss[1]:.1f} |")
    lines += ["", "| Files | Mode | Tool | User Q1 / med / Q3 | System Q1 / med / Q3 | CPU Q1 / med / Q3 | CPU/wall (median ratio) | RSS MiB Q1 / med / Q3 |", "|---:|---|---|---:|---:|---:|---:|---:|"]
    for row in data.get("series", []):
        for label in ("fa", "bun"):
            s = row["summary"][label]
            rss = {k: v / 1024 for k, v in s["peak_rss_kib"].items()}
            lines.append(f"| {row['files']} | {row['mode']} | {label} | {dist(s['user'])} | {dist(s['system'])} | {dist(s['cpu'])} | {s['cpu']['median'] / s['wall']['median']:.2f} | {dist(rss)} |")
    return "\n".join(lines) + "\n"


def save(args, data):
    args.report.parent.mkdir(parents=True, exist_ok=True)
    temporary = args.report.with_suffix(".tmp")
    temporary.write_text(json.dumps(data, indent=2) + "\n")
    temporary.replace(args.report)
    args.report.with_suffix(".md").write_text(markdown(data))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("all", "prepare", "measure", "profile", "corpus", "render"), default="all")
    parser.add_argument("--files", type=lambda s: list(map(int, s.split(','))), default=[1, 100, 1000, 2000])
    parser.add_argument("--no-build", action="store_true", help="use an already-built release checker")
    parser.add_argument("--lines", type=int, default=500)
    parser.add_argument("--runs", type=int, default=11)
    parser.add_argument("--modes", type=lambda s: s.split(','), default=["single", "default"])
    parser.add_argument("--directory", type=Path, default=ROOT / "benchmarks/build/check-speed")
    parser.add_argument("--report", type=Path, default=ROOT / "benchmarks/results/check-speed.json")
    parser.add_argument("--fa", type=Path, default=ROOT / "target/release/fa")
    parser.add_argument("--bun", type=Path, default=Path.home() / ".bun/bin/bun")
    args = parser.parse_args()
    if any(mode not in ("single", "default") for mode in args.modes):
        parser.error("modes must be single and/or default")
    if args.runs < 1 or any(count < 1 for count in args.files) or args.lines < 50:
        parser.error("runs/files must be positive and lines >= 50")
    args.directory = args.directory.resolve()
    args.fa, args.bun = args.fa.resolve(), args.bun.resolve()
    os.chdir(ROOT)
    if args.mode in ("all", "prepare"):
        if not args.no_build:
            run(["taskset", "-c", "0-4", "env", "CARGO_BUILD_JOBS=2", "cargo", "build", "--release", "-p", "fa", "--bin", "fa"])
        versions = json.loads(run(["taskset", "-c", "0-4", args.bun, "-p", "JSON.stringify(process.versions)"]).stdout)
        pinned = json.loads((ROOT / "packages/benchmark-typescript/node_modules/@typescript/native/package.json").read_text())["version"]
        if versions["typescript"] != pinned:
            raise RuntimeError(f"Bun semantics {versions['typescript']} differ from pinned TypeScript {pinned}; use matching Bun before comparing")
        data = {"schema": 1, "revision": run(["git", "rev-parse", "HEAD"]).stdout.decode().strip(),
                "start": snapshot(), "machine": platform.uname()._asdict(), "runs": args.runs,
                "fa_binary": identity(args.fa), "bun_binary": identity(args.bun),
                "versions": versions, "pinned_typescript": pinned,
                "runner_sha256": digest(__file__), "generator_sha256": digest(ROOT / "benchmarks/compiler/paired.ts"),
                "sources": {str(p): digest(ROOT / p) for p in (
                    "benchmarks/compiler/check_speed.py", "benchmarks/compiler/paired.ts",
                    "benchmarks/compiler/generate.ts", "packages/benchmark-typescript/bun.lock")},
                "environment": {key: os.environ[key] for key in (
                    "FA_PIPELINE", "FA_TREE_BUDGET", "FA_SCHEDULE_SEED", "RAYON_NUM_THREADS", "BUN_JSC_useJIT") if key in os.environ},
                "seed": 1, "lines": args.lines, "affinity": {"single": "5", "default": "0-4"},
                "goal": "wall time with default parallelism <= 1.20 * bun check; shared machine",
                "cache": "Only OS file pages survive: no watch server, incremental Program or .tsbuildinfo"}
        prepare(args, data)
        save(args, data)
    else:
        data = json.loads(args.report.read_text())
        if args.mode != "render" and (identity(args.fa) != data["fa_binary"] or identity(args.bun) != data["bun_binary"]):
            raise RuntimeError("checker binary changed since prepare")
    if args.mode != "render":
        for fixture in data["fixtures"]:
            for language in ("fa", "ts"):
                for relative, expected in fixture[language]["sources"].items():
                    if digest(args.directory / str(fixture["files"]) / relative) != expected:
                        raise RuntimeError(f"fixture changed: {relative}")
    if args.mode in ("all", "measure"):
        measure(args, data)
    if args.mode in ("all", "profile"):
        profile(args, data)
    if args.mode in ("all", "corpus"):
        corpus(args, data)
    save(args, data)


if __name__ == "__main__":
    main()
