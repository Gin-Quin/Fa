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

ROOT = Path(__file__).resolve().parents[2]
RUNNER_SHA256 = hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
sys.path.insert(0, str(ROOT / "benchmarks/audit"))
from measurements import control_passes, order_for_round, paired, quantile, snapshot
from resources import sample


def run(command, **kwargs):
    return subprocess.run(list(map(str, command)), check=True, capture_output=True, **kwargs)


def digest(path):
    with Path(path).open("rb") as source:
        return hashlib.file_digest(source, "sha256").hexdigest()


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
            or (any(abs(v - 1.2) <= .05 + 1e-12 for v in estimates) and not control_passes(control))):
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
        fixture["configuration"] = {name: digest(directory / name) for name in (
            "fa/package.fa", "ts/tsconfig.json", "ts/package.json")}
        # Explicit independent validation against the pinned TS7 binary as well as Bun.
        run(["taskset", "-c", "0-4", args.bun, ROOT / "packages/benchmark-typescript/node_modules/@typescript/native/bin/tsc",
             "--noEmit", "-p", fixture["tsInput"]], timeout=180)
        fixture["validation"] = {
            "fa": sample(["taskset", "-c", "0-4", "env", "-u", "RAYON_NUM_THREADS", str(args.fa), "check", fixture["faInput"]], b"", timeout=600, max_rss_kib=512 * 1024),
            "bun": sample(["taskset", "-c", "0-4", str(args.bun), "check", "--no-script", "-p", fixture["tsInput"]], b"", timeout=600),
        }
        fixture["validation_purpose"] = "clean-input correctness and memory; not an acceptance timing series"
        fixtures.append(fixture)
        print(f"prepared {count}: Fa {fixture['fa']['lines']} / TS {fixture['ts']['lines']} lines", flush=True)
    data["fixtures"] = fixtures


def measure(args, data):
    data.setdefault("series", [])
    complete = {(row["files"], row["mode"]) for row in data["series"]}
    for fixture in data["fixtures"]:
        for mode in args.modes:
            if (fixture["files"], mode) in complete:
                continue
            if not args.allow_busy and os.getloadavg()[0] >= args.max_load:
                raise RuntimeError("timing requires one-minute load <6; resume in a quiet window, or use --allow-busy once for a provisional series")
            row = {"files": fixture["files"], "mode": mode, "start": snapshot(), "samples": {}, "orders": [], "runner_sha256": RUNNER_SHA256}
            cmds = commands(args, fixture, mode)
            active = data.get("active_series")
            if active and (active["files"], active["mode"]) == (fixture["files"], mode):
                if active["commands"] != cmds or active["planned_runs"] != args.runs:
                    raise RuntimeError("unfinished series command/run count changed")
                row = active
                row.setdefault("resumptions", []).append({"snapshot": snapshot(), "runner_sha256": RUNNER_SHA256})
            row["commands"] = cmds
            row["planned_runs"] = args.runs
            row["resource_method"] = "GNU time child RSS; wait4 CPU includes the identical launcher"
            # A whole successful run warms filesystem pages; no checker cache is persisted.
            for label in ("fa", "bun"):
                sample(cmds[label], b"", timeout=300, max_rss_kib=512 * 1024 if label == "fa" else None)
            for index in range(len(row["orders"]), args.runs):
                if not args.allow_busy and os.getloadavg()[0] >= args.max_load:
                    raise RuntimeError("host load reached 6; completed rounds are checkpointed for quiet-window resume")
                order = order_for_round(list(cmds), index)
                row["orders"].append(order)
                for label in order:
                    value = sample(cmds[label], b"", timeout=300,
                                   max_rss_kib=512 * 1024 if label.startswith("fa") else None)
                    row["samples"].setdefault(label, []).append(value)
                print(f"{fixture['files']} {mode}: round {index + 1}/{args.runs}", flush=True)
                data["active_series"] = row
                save(args, data)
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
            row["provisional"] = row_is_provisional(row)
            data["series"].append(row)
            data.pop("active_series", None)
            save(args, data)


# Exclusive instruction attribution: disjoint categories, not overlapping inclusive sums.
# Optimizer inlining moves helper costs into their caller. Keep unclassified runtime visible.
def walker_method_phases():
    # Optimized Rust names omit the source module for inherent Walker methods.
    # Rustfmt puts each impl at column zero and its methods at one hard tab.
    groups = {
        "scanning_resolving": ("modules", "interface"),
        "flow_effects": ("control", "narrowing", "proofs", "effects", "tasks", "callbacks", "reactivity", "concurrency", "errors"),
        "references_moves_relations": ("references", "reference_sensitivity", "provenance", "moves", "relations"),
        "compile_time": ("comptime",),
    }
    methods, sources = {}, {}
    for group, modules in groups.items():
        for module in modules:
            path = ROOT / f"packages/analyzer/src/check/{module}.rs"
            if not path.exists():
                continue
            sources[str(path.relative_to(ROOT))] = digest(path)
            inside = False
            for line in path.read_text().splitlines():
                if re.match(r"^impl(?:<[^>]*>)?\s+Walker(?:<.*>)?\s*\{", line):
                    inside = True
                elif line == "}":
                    inside = False
                match = re.match(r"^\t(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(\w+)", line)
                if inside and match:
                    name = match[1]
                    if name in methods and methods[name] != group:
                        raise RuntimeError(f"ambiguous Walker method phase: {name}")
                    methods[name] = group
    return methods, sources


METHOD_PHASES, METHOD_PHASE_SOURCES = walker_method_phases()


def category(name, methods=None):
    match = re.search(r"(?:<fa_analyzer::check::Walker>|fa_analyzer::check::Walker)::(\w+)", name)
    phases = METHOD_PHASES if methods is None else methods
    if match and match[1] in phases:
        return phases[match[1]]
    rules = [
        ("runtime_other", ("fa_interpreter::memory",)),
        ("compile_time", ("fa_interpreter", "fa_analyzer::comptime", "::check::comptime")),
        ("references_moves_relations", ("::check::references", "::check::reference_sensitivity", "::check::provenance", "::check::moves", "::check::relations", "::flow::values")),
        ("flow_effects", ("::flow::", "::check::flow", "::check::effects", "::check::control", "::check::conditions", "::check::tasks")),
        ("scanning_resolving", ("fa_codebase_parser::", "fa_analyzer::scan", "fa_analyzer::graph", "::check::modules", "::check::interface", "::symbols::", "::resolve")),
        ("rendering", ("fa::report", "::diagnostics::", "fa::color")),
        ("syntax_decode", ("fa_analyzer::syntax", "fa_parser::nodes::Node", "fa_parser::typed_syntax_tree::TypedSyntaxTree>::name")),
        ("types_checking", ("fa_analyzer::types", "fa_analyzer::check::")),
        ("reading_parsing", ("fa_parser::", "::parse_module", "std::fs::", "std::io::")),
        ("pipeline_session", ("fa_analyzer::", "fa::")),
    ]
    return next((group for group, patterns in rules if any(p in name for p in patterns)), "runtime_other")


def parse_annotation(annotation, methods=None):
    functions = []
    for line in annotation.splitlines():
        match = re.match(r"\s*([\d,]+)\s+\([^)]*\)\s+(.*)", line)
        if match and "PROGRAM TOTALS" not in match[2]:
            functions.append({"instructions": int(match[1].replace(",", "")),
                              "function": match[2], "category": category(match[2], methods)})
    total = re.search(r"([\d,]+)\s+\([^)]*\)\s+PROGRAM TOTALS", annotation)
    if not total or not functions:
        raise ValueError("Callgrind annotation has no totals/functions")
    return int(total[1].replace(",", "")), functions


def annotate_profile(args, data):
    result = data["profile"]
    output = Path(result["callgrind"]["path"])
    if not output.exists() and result["callgrind"].get("archive"):
        import gzip
        output = args.directory / "profile/restored.callgrind.out"
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_bytes(gzip.decompress((ROOT / result["callgrind"]["archive"]).read_bytes()))
    if digest(output) != result["callgrind"]["sha256"]:
        raise RuntimeError("Callgrind artifact changed")
    annotate = shutil.which("callgrind_annotate") or str(Path.home() / ".local/valgrind/usr/bin/callgrind_annotate")
    result.setdefault("walker_method_phases", METHOD_PHASES)
    result.setdefault("phase_registry_sources", METHOD_PHASE_SOURCES)
    for inclusive in (False, True):
        annotation = run([annotate, f"--inclusive={'yes' if inclusive else 'no'}",
                          "--auto=no", "--threshold=100", output]).stdout.decode()
        (output.parent / ("functions-inclusive.txt" if inclusive else "functions.txt")).write_text(annotation)
        total, functions = parse_annotation(annotation, result["walker_method_phases"])
        result["instructions"] = total
        result["inclusive_functions" if inclusive else "functions"] = functions
    groups = {}
    for function in result["functions"]:
        group = function["category"]
        groups[group] = groups.get(group, 0) + function["instructions"]
    result["exclusive_categories"] = groups
    result["exclusive_coverage"] = sum(groups.values()) / result["instructions"]
    if not .99 <= result["exclusive_coverage"] <= 1.000001:
        raise RuntimeError("exclusive function totals do not cover whole-process instructions")
    save(args, data)


def profile(args, data):
    fixture = data["fixtures"][-1]
    directory = args.directory / "profile"
    directory.mkdir(parents=True, exist_ok=True)
    result = {"files": fixture["files"], "timings": {}, "runner_sha256": RUNNER_SHA256,
              "walker_method_phases": METHOD_PHASES, "phase_registry_sources": METHOD_PHASE_SOURCES}
    for mode in ("single", "default"):
        command = commands(args, fixture, mode)["fa"] + ["--timings"]
        path = directory / f"timings-{mode}.txt"
        before = snapshot()
        measured = corpus_sample(command, output_path=path, timeout=300)
        result.setdefault("timing_load", {})[mode] = {"start": before, "end": snapshot()}
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
    result["callgrind"] = {"path": str(output), "sha256": digest(output)}
    data["profile"] = result
    annotate_profile(args, data)


def legacy_probe(args, data):
    directory = args.directory / "legacy-targets"
    run(["taskset", "-c", "0-4", args.bun, ROOT / "benchmarks/compiler/generate.ts",
         "--files", data["fixtures"][-1]["files"], "--lines", data["lines"], "--output", directory])
    paths = sorted(directory.rglob("*.fa"))
    result = {"description": "Original generator defaults, including 5% target readers and both bun/native targets; one correctness/memory probe, not an acceptance series.",
              "files": len(paths), "lines": sum(path.read_bytes().count(b"\n") for path in paths),
              "target_readers": sum("@if target" in path.read_text() for path in paths),
              "sources": {str(path.relative_to(directory)): digest(path) for path in paths}}
    output = directory / "timings.txt"
    command = ["taskset", "-c", "0-4", "env", "-u", "RAYON_NUM_THREADS", str(args.fa), "check", str(directory), "--timings"]
    result["resources"] = sample(command, b"", timeout=600, max_rss_kib=512 * 1024, output_path=output)
    result["timings"] = output.read_text()
    data["legacy_target_probe"] = result
    save(args, data)


def corpus_sample(command, output_path=None, timeout=60):
    return sample(command, None, timeout=timeout, max_rss_kib=512 * 1024,
                  allowed_exits=(0, 1), check=False, output_path=output_path)


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


def rss_supplement(args, data):
    for row in data.get("series", []):
        if row["files"] > 100 or "launcher_peak_rss_kib" in row["samples"]["fa"][0]:
            continue
        row["rss_note"] = "Independent GNU-time RSS supplement: original wait4 RSS is a launcher-contaminated upper bound; original CPU/wall preserved."
        row["rss_samples"] = {"fa": [], "bun": []}
        for index in range(args.runs):
            for label in order_for_round(["fa", "bun"], index):
                value = sample(row["commands"][label], b"", max_rss_kib=512 * 1024 if label == "fa" else None)
                row["rss_samples"][label].append(value)
        row["rss_summary"] = {label: distribution([s["peak_rss_kib"] for s in samples])
                              for label, samples in row["rss_samples"].items()}
        save(args, data)


def row_loads(row):
    values = [row[key]["load_average"][0] for key in ("start", "end") if key in row]
    stored = row.get("samples", {})
    groups = stored.values() if isinstance(stored, dict) else [stored]
    for samples in groups:
        for value in samples:
            values.extend(value[key] for key in ("load_1m_start", "load_1m_end") if key in value)
    return values


def row_is_provisional(row):
    values = row_loads(row)
    return not values or max(values) >= 6


def load_cell(row):
    values = row_loads(row)
    return f"{min(values):.2f}–{max(values):.2f}" if values else "not captured"


def markdown(data):
    lines = ["# Checker comparison", "", "Generated from the accompanying JSON; seconds and MiB. Default parallelism on CPUs 0–4 is the wall-time goal; single worker on CPU 5 measures CPU cost. Shared machine, warm filesystem cache, fresh checker processes.", "",
             "| Files | Mode | Observed load L1 | Fa wall Q1 / med / Q3 | Bun wall Q1 / med / Q3 | Fa/Bun wall medians / paired [Q1,Q3] | Wall goal | CPU ratio medians / paired | Control wall / CPU | Fa / Bun peak MiB (max) |", "|---:|---|---:|---:|---:|---:|---|---:|---|---:|"]
    def dist(d):
        return " / ".join(f"{d[k]:.4f}" for k in ("q1", "median", "q3"))
    for row in data.get("series", []):
        a, b = row["summary"]["fa"], row["summary"]["bun"]
        w, c = row["comparisons"]["wall"], row["comparisons"]["cpu"]
        ctrl = " / ".join(f"{row['controls'][m]['ratio_of_medians']:.3f},{row['controls'][m]['median']:.3f} {'pass' if row['controls'][m]['passes'] else 'fail'}" for m in ("wall", "cpu"))
        rss = [max(s["peak_rss_kib"] for s in row.get("rss_samples", row["samples"])[label]) / 1024 for label in ("fa", "bun")]
        goal = w['verdict'] if row['mode'] == 'default' else 'reference only'
        if row_is_provisional(row):
            goal = "provisional: " + goal
        lines.append(f"| {row['files']} | {row['mode']} | {load_cell(row)} | {dist(a['wall'])} | {dist(b['wall'])} | {w['ratio_of_medians']:.3f} / {w['median']:.3f} [{w['q1']:.3f},{w['q3']:.3f}] | {goal} | {c['ratio_of_medians']:.3f} / {c['median']:.3f} | {ctrl} | {rss[0]:.1f} / {rss[1]:.1f} |")
    lines += ["", "| Files | Mode | Tool | Observed load L1 | User Q1 / med / Q3 | System Q1 / med / Q3 | CPU Q1 / med / Q3 | CPU/wall (median ratio) | RSS MiB Q1 / med / Q3 |", "|---:|---|---|---:|---:|---:|---:|---:|---:|"]
    for row in data.get("series", []):
        for label in ("fa", "bun"):
            s = row["summary"][label]
            rss = {k: v / 1024 for k, v in row.get("rss_summary", {}).get(label, s["peak_rss_kib"]).items()}
            lines.append(f"| {row['files']} | {row['mode']} | {label} | {load_cell(row)} | {dist(s['user'])} | {dist(s['system'])} | {dist(s['cpu'])} | {s['cpu']['median'] / s['wall']['median']:.2f} | {dist(rss)} |")
    lines += ["", "All timing figures with observed one-minute load >=6 are provisional. Load columns show min/max of captured observations; old rows have endpoint readings, newer samples also retain start/end load. No historical per-sample load is invented."]
    if any("rss_samples" in row for row in data.get("series", [])):
        lines += ["", "Small initial CPU-5 rows use separately labeled eleven-run GNU-time RSS supplements; their primary CPU/wall samples remain unchanged."]
    if data.get("fixtures"):
        lines += ["", "| Library files | Fa source files / lines / bytes | TS source files / lines / bytes |", "|---:|---:|---:|"]
        for fixture in data["fixtures"]:
            cells = [" / ".join(str(fixture[language][key]) for key in ("files", "lines", "bytes")) for language in ("fa", "ts")]
            lines.append(f"| {fixture['files']} | {cells[0]} | {cells[1]} |")
    profile = data.get("profile", {})
    if profile.get("instructions"):
        total = profile["instructions"]
        lines += ["", "## Largest-size profile", "", "Exclusive user-space instruction attribution; inlining charges helpers to their surviving caller. These are not CPU/wall milliseconds; kernel time is outside Callgrind.", "", "| Phase/helper category | Instructions | % of whole process |", "|---|---:|---:|"]
        for name, count in sorted(profile["exclusive_categories"].items(), key=lambda item: -item[1]):
            lines.append(f"| {name} | {count:,} | {count / total * 100:.2f} |")
        lines += ["", "| Function (exclusive) | Instructions | % of process |", "|---|---:|---:|"]
        for function in profile["functions"][:20]:
            lines.append(f"| `{function['function']}` | {function['instructions']:,} | {function['instructions']/total*100:.2f} |")
        for mode, text in profile["timings"].items():
            lines += ["", f"### Diagnostic wall phases: {mode} (load {load_cell(profile.get('timing_load', {}).get(mode, {}))}; provisional when busy or not captured)", "", "```text", text.rstrip(), "```"]
    if "legacy_target_probe" in data:
        probe = data["legacy_target_probe"]
        r = probe["resources"]
        lines += ["", "## Original target-dependent generator", "", f"One correctness/memory probe, {probe['target_readers']} target readers, {probe['files']} files, {probe['lines']} lines: {r['peak_rss_kib']/1024:.2f} MiB peak. Per-probe load was not captured; archived phase times are provisional. This is not an acceptance timing series.", "", "```text", probe["timings"].rstrip(), "```"]
    if data.get("corpus", {}).get("totals"):
        corpus = data["corpus"]
        t = corpus["totals"]
        lines += ["", "## Repository corpus", "", f"Observed load L1 {load_cell(corpus)}; timings provisional when >=6. {corpus['files']} tracked .fa files, fresh process per file in its repository context: {t['wall']:.3f} s aggregate wall, {t['user']:.3f} s user + {t['system']:.3f} s system = {t['cpu']:.3f} s CPU; peak {corpus['max_peak_rss_kib']/1024:.2f} MiB. Exit counts: {corpus['exit_counts']}; abnormal/timeout/memory failures: {len(corpus['failures'])}. Expected diagnostic fixtures keep exit 1."]
    return "\n".join(lines) + "\n"


def save(args, data):
    args.report.parent.mkdir(parents=True, exist_ok=True)
    temporary = args.report.with_suffix(".tmp")
    temporary.write_text(json.dumps(data, indent=2) + "\n")
    temporary.replace(args.report)
    args.report.with_suffix(".md").write_text(markdown(data))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--mode", choices=("all", "prepare", "measure", "profile", "corpus", "legacy", "rss", "annotate", "render"), default="all")
    parser.add_argument("--files", type=lambda s: list(map(int, s.split(','))), default=[1, 100, 1000, 2000])
    parser.add_argument("--allow-busy", action="store_true", help="record one provisional timing series on an overloaded host")
    parser.add_argument("--max-load", type=float, default=6, help="timing requires one-minute load below this (default 6)")
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
        native_path = run(["taskset", "-c", "0-4", args.bun, "-e",
                           'import getExePath from "./packages/benchmark-typescript/node_modules/@typescript/native/lib/getExePath.js"; console.log(getExePath());']).stdout.decode().strip()
        versions = json.loads(run(["taskset", "-c", "0-4", args.bun, "-p", "JSON.stringify(process.versions)"]).stdout)
        pinned = json.loads((ROOT / "packages/benchmark-typescript/node_modules/@typescript/native/package.json").read_text())["version"]
        if versions["typescript"] != pinned:
            raise RuntimeError(f"Bun semantics {versions['typescript']} differ from pinned TypeScript {pinned}; use matching Bun before comparing")
        data = {"schema": 1, "revision": run(["git", "rev-parse", "HEAD"]).stdout.decode().strip(),
                "start": snapshot(), "machine": platform.uname()._asdict(), "runs": args.runs,
                "fa_binary": identity(args.fa), "bun_binary": identity(args.bun),
                "versions": versions, "pinned_typescript": pinned,
                "native_typescript_binary": identity(native_path),
                "runner_sha256": RUNNER_SHA256, "generator_sha256": digest(ROOT / "benchmarks/compiler/paired.ts"),
                "sources": {str(p): digest(ROOT / p) for p in (
                    "benchmarks/compiler/check_speed.py", "benchmarks/compiler/paired.ts",
                    "benchmarks/compiler/generate.ts", "benchmarks/compiler/resources.py", "packages/benchmark-typescript/bun.lock")},
                "environment": {key: os.environ[key] for key in (
                    "FA_PIPELINE", "FA_TREE_BUDGET", "FA_SCHEDULE_SEED", "RAYON_NUM_THREADS", "BUN_JSC_useJIT") if key in os.environ},
                "seed": 1, "lines": args.lines, "affinity": {"single": "5", "default": "0-4"},
                "goal": "wall time with default parallelism <= 1.20 * bun check; shared machine",
                "resource_method": "GNU time child RSS; wait4 microsecond CPU including identical GNU-time launcher overhead",
                "logical_cpus": os.cpu_count(), "runner_affinity": sorted(os.sched_getaffinity(0)),
                "cpu_model": next((line.split(":", 1)[1].strip() for line in Path("/proc/cpuinfo").read_text().splitlines() if line.startswith("model name")), None),
                "cache": "Only OS file pages survive: no watch server, incremental Program or .tsbuildinfo"}
        prepare(args, data)
        save(args, data)
    else:
        data = json.loads(args.report.read_text())
        if args.mode != "render" and (identity(args.fa) != data["fa_binary"] or identity(args.bun) != data["bun_binary"]):
            raise RuntimeError("checker binary changed since prepare")
    if args.mode != "render":
        for fixture in data["fixtures"]:
            for relative, expected in fixture.get("configuration", {}).items():
                if digest(args.directory / str(fixture["files"]) / relative) != expected:
                    raise RuntimeError(f"configuration changed: {relative}")
            for language in ("fa", "ts"):
                for relative, expected in fixture[language]["sources"].items():
                    if digest(args.directory / str(fixture["files"]) / relative) != expected:
                        raise RuntimeError(f"fixture changed: {relative}")
    if args.mode in ("all", "profile"):
        profile(args, data)
    if args.mode == "annotate":
        annotate_profile(args, data)
    if args.mode in ("all", "legacy"):
        legacy_probe(args, data)
    if args.mode in ("all", "corpus"):
        corpus(args, data)
    if args.mode == "rss":
        rss_supplement(args, data)
    if args.mode in ("all", "measure"):
        measure(args, data)
    save(args, data)


if __name__ == "__main__":
    main()
