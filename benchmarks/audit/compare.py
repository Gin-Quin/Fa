#!/usr/bin/env python3
"""Reproducible native Fa/Goose comparison, with optional dynamic instruction counts.

Builds both languages, retains optimized LLVM IR/C/assembly, checks against Rust,
then alternates timed runs. Outputs live under the ignored audit/build directory.
Use --directory to retain a baseline before rebuilding the compiler.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import resource
import shutil
import statistics
import subprocess
import time


ROOT = Path(__file__).resolve().parents[2]
DEFAULTS = {
    "binary-trees": 17, "blur": 3000, "calc": 400000, "fannkuch-redux": 10,
    "fib": 42, "graph": 100000, "interp": 21, "lru": 2000000,
    "mandelbrot": 4000, "nbody": 15000000, "particles": 300000,
    "push": 32000000, "records": 4000000, "respond": 400000, "scene": 12,
    "sexp": 20000, "sieve": 100000000, "sort-objects": 1000000,
    "spectral-norm": 4000, "string-building": 3000000, "strlist": 2000000,
    "sum": 32000000, "tree": 21, "word-count": 5000000, "words": 500000,
}
AUDIT = {name: max(1, size // 10) for name, size in DEFAULTS.items()}
AUDIT.update({"binary-trees": 14, "fib": 35, "interp": 17, "scene": 9,
              "tree": 17, "fannkuch-redux": 9, "words": 100000,
              "blur": 1000, "mandelbrot": 1000, "spectral-norm": 1000})


def execute(command, log=None, **kwargs):
    result = subprocess.run([str(part) for part in command], capture_output=True, **kwargs)
    if log:
        log.write_bytes(result.stdout + result.stderr)
    if result.returncode:
        raise RuntimeError(f"{command[0]} exited {result.returncode}: "
                           + result.stderr.decode(errors="replace")[-3000:])
    return result.stdout


def timed(command):
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    start = time.perf_counter()
    execute(command)
    wall = time.perf_counter() - start
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime
    return {"wall": wall, "cpu": cpu}


def profile(command, directory):
    # VALGRIND should name the real executable, not a wrapper tied to $HOME.
    execute([os.environ.get("VALGRIND", "valgrind"), "--tool=callgrind",
             "--dump-instr=yes", f"--callgrind-out-file={directory / 'callgrind.out'}",
             *command], directory / "callgrind.log")
    match = re.search(r"^summary:\s+(\d+)",
                      (directory / "callgrind.out").read_text(), re.M)
    return int(match[1])


def build(name, size, directory):
    source = ROOT / "benchmarks" / name
    fa_dir, goose_dir = directory / "fa", directory / "goose"
    fa_dir.mkdir(parents=True, exist_ok=True)
    goose_dir.mkdir(parents=True, exist_ok=True)
    execute([os.environ.get("FA", ROOT / "target/release/fa"), "build", source / "main.fa",
             "--target=native", "--emit-ir", f"--output={fa_dir}"], fa_dir / "build.log")
    fa = fa_dir / "native/main"
    goose_source = (source / "main.goose").read_text()
    goose_source, replaced = re.subn(
        r"^(\s*let\s+\w+\s*=\s*)\d+(\s*;.*BENCH_N.*)$",
        lambda match: f"{match[1]}{size}{match[2]}", goose_source, flags=re.M)
    (goose_dir / "main.goose").write_text(goose_source)
    for module in (ROOT / "benchmarks/goose").glob("*.goose"):
        shutil.copy2(module, goose_dir)
    goose = goose_dir / "main"
    execute([os.environ.get("GOOSE", Path.home() / ".local/goose/goose"),
                      "build", "-O2", "--standalone", "-v", "-o", goose,
                      goose_dir / "main.goose"], goose_dir / "build.log")
    # Goose logs the generated C path before invoking its C compiler.
    log = (goose_dir / "build.log").read_text()
    generated = re.findall(r"[^\s\"']+/main\.c", log)
    if generated and Path(generated[-1]).exists():
        shutil.copy2(generated[-1], goose_dir / "main.c")
        compiler = os.environ.get("GOOSE_CC", "cc")
        if "clang" in Path(compiler).name:
            execute([compiler, "-O2", "-S", "-emit-llvm", goose_dir / "main.c",
                     "-o", goose_dir / "main.ll"], goose_dir / "ir-build.log")
    for exe in [fa, goose]:
        exe.with_suffix(".s").write_bytes(execute(["objdump", "-d", "--no-show-raw-insn", exe]))
    execute(["rustc", "--edition", "2024", "-C", "opt-level=3", source / "main.rs",
             "-o", directory / "rust"], directory / "rust-build.log")
    expected = execute([directory / "rust", str(size)])
    (directory / "expected.txt").write_bytes(expected)
    commands = {"fa": [str(fa), str(size)],
                "goose": [str(goose)] + ([] if replaced else [str(size)])}
    for side, command in commands.items():
        actual = execute(command)
        (directory / side / "stdout.txt").write_bytes(actual)
        if actual != expected:
            raise RuntimeError(f"{name}: {side} output differs from Rust")
    return commands


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("names", nargs="*", choices=sorted(DEFAULTS))
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--runs", type=int, default=7)
    parser.add_argument("--default-sizes", action="store_true")
    parser.add_argument("--profile", action="store_true")
    parser.add_argument("--reuse", action="store_true", help="Time existing builds")
    parser.add_argument("--rebuild-fa", action="store_true", help="With --reuse, rebuild only Fa")
    parser.add_argument("--report", default="results.json", help="JSON filename inside --directory")
    args = parser.parse_args()
    if args.runs < 1:
        parser.error("--runs must be positive")
    if args.rebuild_fa and not args.reuse:
        parser.error("--rebuild-fa requires --reuse")
    args.directory = args.directory.resolve()
    args.directory.mkdir(parents=True, exist_ok=True)
    cpu = next((line.split(":", 1)[1].strip() for line in Path("/proc/cpuinfo").read_text().splitlines()
                if line.startswith("model name")), platform.processor())
    results = {"machine": platform.platform(), "cpu": cpu,
               "date": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
               "revision": execute(["git", "-c", f"safe.directory={ROOT}", "rev-parse", "HEAD"])
               .decode().strip(), "runs": args.runs, "benchmarks": {}}
    for name in args.names or sorted(DEFAULTS):
        size = (DEFAULTS if args.default_sizes else AUDIT)[name]
        directory = args.directory / name
        try:
            if args.reuse:
                if args.rebuild_fa:
                    fa_dir = directory / "fa"
                    execute([os.environ.get("FA", ROOT / "target/release/fa"), "build",
                             ROOT / "benchmarks" / name / "main.fa", "--target=native", "--emit-ir",
                             f"--output={fa_dir}"], fa_dir / "build.log")
                    exe = fa_dir / "native/main"
                    exe.with_suffix(".s").write_bytes(execute(["objdump", "-d", "--no-show-raw-insn", exe]))
                goose_source = (directory / "goose/main.goose").read_text()
                commands = {"fa": [str(directory / "fa/native/main"), str(size)],
                            "goose": [str(directory / "goose/main")]
                            + ([] if "BENCH_N" in goose_source else [str(size)])}
                expected = (directory / "expected.txt").read_bytes()
                for command in commands.values():
                    if execute(command) != expected:
                        raise RuntimeError("output differs from the saved Rust output")
            else:
                commands = build(name, size, directory)
            result = {"size": size, "output_sha256": hashlib.sha256(expected if args.reuse
                      else (directory / "expected.txt").read_bytes()).hexdigest()}
            samples = {"fa": [], "goose": []}
            for run in range(args.runs):
                for side in (["fa", "goose"] if run % 2 == 0 else ["goose", "fa"]):
                    samples[side].append(timed(commands[side]))
            for side in samples:
                result[side] = {"samples": samples[side], **{
                    key: statistics.median(sample[key] for sample in samples[side])
                    for key in ("wall", "cpu")}}
                result[side]["binary_sha256"] = hashlib.sha256(Path(commands[side][0]).read_bytes()).hexdigest()
                if args.profile:
                    result[side]["instructions"] = profile(commands[side], directory / side)
            results["benchmarks"][name] = result
            print(f"{name:16} n={size:9} CPU Fa/Goose "
                  f"{result['fa']['cpu']:.6f}/{result['goose']['cpu']:.6f} "
                  f"({result['fa']['cpu'] / result['goose']['cpu']:.2f}x)", flush=True)
        except (RuntimeError, OSError) as error:
            results["benchmarks"][name] = {"error": str(error), "size": size}
            print(f"{name}: {error}", flush=True)
        (args.directory / args.report).write_text(json.dumps(results, indent=2) + "\n")
    if any("error" in result for result in results["benchmarks"].values()):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
