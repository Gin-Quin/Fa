#!/usr/bin/env python3
"""Reproducible native Fa/Goose comparison, with optional dynamic instruction counts.

Builds both languages, retains optimized LLVM IR/C/assembly, checks against Rust,
then alternates timed runs. Outputs live under the ignored audit/build directory.
Use --directory to retain a baseline before rebuilding the compiler. VARIANTS names other
Fa versions of a benchmark (`string-building-bytes`), compared with the same Goose and Rust.
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
import sys
import time

import artifacts


ROOT = Path(__file__).resolve().parents[2]
DEFAULTS = {
    "binary-trees": 17, "blur": 3000, "calc": 400000, "fannkuch-redux": 10,
    "fib": 42, "graph": 100000, "growing-index": 1000000, "interp": 21, "lru": 2000000,
    "mandelbrot": 4000, "nbody": 15000000, "particles": 300000,
    "push": 32000000, "records": 4000000, "respond": 400000, "scene": 12,
    "sexp": 20000, "sieve": 100000000, "sort-objects": 1000000,
    "spectral-norm": 4000, "string-building": 3000000, "strlist": 2000000,
    "sum": 32000000, "tree": 21, "word-count": 5000000, "words": 500000,
}
# Other Fa versions of a benchmark, measured against the same Goose and Rust programs:
# name -> (benchmark directory, Fa source). `string-building-bytes` builds bytes in an
# `Array<Byte>`, like the Goose port; `string-building-growing` in `GrowingArray<Byte>`, whose
# items never move, like Goose's `u8[>..]` arrays.
VARIANTS = {"string-building-bytes": ("string-building", "bytes.fa"),
            "string-building-growing": ("string-building", "growing.fa"),
            "growing-index-array": ("growing-index", "array.fa")}
DEFAULTS.update({name: DEFAULTS[directory] for name, (directory, _) in VARIANTS.items()})
AUDIT = {name: max(1, size // 10) for name, size in DEFAULTS.items()}
AUDIT.update({"binary-trees": 14, "fib": 35, "interp": 17, "scene": 9,
              "tree": 17, "fannkuch-redux": 9, "words": 100000,
              "blur": 1000, "mandelbrot": 1000, "spectral-norm": 1000,
              "growing-index": 1000000, "growing-index-array": 1000000})
GOOSE_SUITE = ("binary-trees", "blur", "calc", "graph", "interp", "lru", "particles",
               "push", "records", "respond", "scene", "sexp", "strlist", "sum", "tree", "words")
SECONDARY = tuple(sorted(DEFAULTS.keys() - set(GOOSE_SUITE)))
FA_FLAGS = ["build", "--target=native", "--emit-ir", "-O3",
            "--asynchronous=false", "--overflow-checks=false"]
GOOSE_FLAGS = ["build", "-O2", "--standalone", "-v"]
RUST_FLAGS = ["--edition", "2024", "-C", "opt-level=3", "-C", "overflow-checks=off"]
BUILD_ENVIRONMENT = ("FA_COMPRESSED_LINKS", "FA_PIPELINE", "FA_LTO", "FA_IMPORT_LIMIT",
                     "FA_LLVM_MEMORY", "FA_NATIVE_TRIPLE", "FA_TOP_LEVEL_PART",
                     "FA_SPLIT", "FA_DEFER_UNITS", "FA_SCHEDULE_SEED", "FA_DEBUG_PACKED")


def execute(command, log=None, **kwargs):
    result = subprocess.run([str(part) for part in command], capture_output=True, **kwargs)
    if log:
        log.write_bytes(result.stdout + result.stderr)
    if result.returncode:
        raise RuntimeError(f"{command[0]} exited {result.returncode}: "
                           + result.stderr.decode(errors="replace")[-3000:])
    return result.stdout


def timed(command, expected):
    before = resource.getrusage(resource.RUSAGE_CHILDREN)
    start = time.perf_counter()
    actual = execute(command)
    wall = time.perf_counter() - start
    after = resource.getrusage(resource.RUSAGE_CHILDREN)
    cpu = after.ru_utime + after.ru_stime - before.ru_utime - before.ru_stime
    if actual != expected:
        raise RuntimeError(f"{command[0]}: timed output differs from Rust")
    return {"wall": wall, "cpu": cpu}


def profile(command, directory, expected):
    # VALGRIND should name the real executable, not a wrapper tied to $HOME.
    output = execute([os.environ.get("VALGRIND", "valgrind"), "--tool=callgrind",
             "--dump-instr=yes", f"--callgrind-out-file={directory / 'callgrind.out'}",
             *command], directory / "callgrind.log")
    if output != expected:
        raise RuntimeError(f"{command[0]}: profiled output differs from Rust")
    match = re.search(r"^summary:\s+(\d+)",
                      (directory / "callgrind.out").read_text(), re.M)
    return int(match[1])


SIZE_LINE = re.compile(r"^(\s*let\s+)(\w+)(\s*=\s*)(\d+)(\s*;.*BENCH_N.*)$", re.M)


def goose_source_at_size(source, size, runtime=False):
    """Change only the generated copy; retain every checkfree annotation.

    Runtime inputs and derived global integers are initialized, in dependency
    order, at the start of main. No input value is embedded in the binary.
    """
    match = SIZE_LINE.search(source)
    if not match:
        return source
    if not runtime:
        return SIZE_LINE.sub(lambda m: f"{m[1]}{m[2]}{m[3]}{size}{m[5]}", source)
    name = match[2]
    assignments = [f"{name} = audit_size;"]
    dependent = {name}
    source = SIZE_LINE.sub(lambda m: f"var {m[2]}: i64 = 0; // BENCH_RUNTIME_N", source)

    def derived(match):
        variable, expression = match[1], match[2]
        if dependent.intersection(re.findall(r"\b\w+\b", expression)):
            dependent.add(variable)
            assignments.append(f"{variable} = {expression};")
            return f"var {variable}: i64 = 0;"
        return match[0]

    # Only top-level integer initializers depending on N are moved. A type
    # needing N as a constant or an unproven checkfree kernel must still fail
    # compilation rather than silently relaxing the Goose program's guarantees.
    source = re.sub(r"^let\s+(\w+)\s*=\s*([^;]+);", derived, source, flags=re.M)
    for module in ("std", "os"):
        if not re.search(rf"^import {module};", source, re.M):
            source = f"import {module};\n" + source
    initialization = "\n".join("    " + line for line in [
        "let audit_args = args();",
        "if audit_args.len < 2 { return; }",
        "let audit_size, audit_ok = parse_int(audit_args[1]);",
        "if !audit_ok { return; }",
        *assignments,
    ])
    source, count = re.subn(r"\bfn main\(\)\s*\{", lambda m: m[0] + "\n" + initialization,
                            source, count=1)
    if count != 1:
        raise ValueError("runtime size requires one fn main()")
    return source


FA_SIZE_LINE = re.compile(r"^(\s*let\s+\w+\s*=\s*)([\d_]+)(\s*--.*BENCH_N.*)$", re.M)


def toolchain():
    """Hash the actual compilers, including Fa's delegated native backend."""
    fa = artifacts.executable_identity(os.environ.get("FA", ROOT / "target/release/fa"))
    fa_path = Path(fa["path"])
    with fa_path.open("rb") as executable:
        if executable.read(2) == b"#!":
            raise RuntimeError("FA must name the real fa or fa-native executable, not a wrapper")
    if fa_path.name != "fa-native":
        native = Path(fa["real_path"]).with_name("fa-native")
        if not native.is_file():
            native = shutil.which("fa-native")
        if native is None:
            raise RuntimeError("cannot identify the fa-native backend used by FA")
        fa["native_backend"] = artifacts.executable_identity(native)
    fa["link_driver"] = artifacts.executable_identity(os.environ.get("CC", "cc"))
    goose = artifacts.executable_identity(
        os.environ.get("GOOSE", Path.home() / ".local/goose/goose"), ("version",))
    goose["c_compiler"] = artifacts.executable_identity(os.environ.get("GOOSE_CC", "cc"))
    stdlib = Path(os.environ.get("GOOSE_STDLIB", Path(goose["path"]).parent / "stdlib")).resolve()
    if not stdlib.is_dir():
        raise RuntimeError(f"Goose standard library not found: {stdlib}")
    goose["stdlib"] = {"path": str(stdlib), "files": {
        str(path.relative_to(stdlib)): artifacts.file_hash(path)
        for path in sorted(stdlib.rglob("*")) if path.is_file()}}
    rust = artifacts.executable_identity("rustc")
    sysroot = Path(execute([rust["path"], "--print", "sysroot"]).decode().strip())
    rust["toolchain_binary"] = artifacts.executable_identity(sysroot / "bin/rustc")
    return {"fa": fa, "goose": goose, "rust": rust}


def fa_source(name):
    """The benchmark directory and the Fa source of a benchmark or a variant."""
    directory, source = VARIANTS.get(name, (name, "main.fa"))
    return ROOT / "benchmarks" / directory, source


def plan(name, size, compilers, runtime_size=False):
    source, fa_file = fa_source(name)
    if name in VARIANTS:
        # a variant is one file, built as the benchmark's `main.fa`
        fa = {"main.fa": (source / fa_file).read_bytes()}
    else:
        fa = {str(path.relative_to(source)): path.read_bytes() for path in source.rglob("*.fa")
              if path.name not in {variant for directory, variant in VARIANTS.values()
                                   if directory == name}}
    text = fa["main.fa"].decode()
    fa_constant = FA_SIZE_LINE.search(text) is not None
    fa["main.fa"] = FA_SIZE_LINE.sub(lambda m: f"{m[1]}{size}{m[3]}", text, count=1).encode()
    goose = {path.name: path.read_bytes() for path in (ROOT / "benchmarks/goose").glob("*.goose")}
    goose["main.goose"] = goose_source_at_size((source / "main.goose").read_text(),
                                              size, runtime_size).encode()
    goose_constant = SIZE_LINE.search(goose["main.goose"].decode()) is not None
    rust = {str(path.relative_to(source)): path.read_bytes() for path in source.rglob("*.rs")}
    inputs = {"fa": fa, "goose": goose, "rust": rust}
    flags = {"fa": FA_FLAGS, "goose": [*GOOSE_FLAGS, "--cc",
             compilers["goose"]["c_compiler"]["path"], "--stdlib",
             compilers["goose"]["stdlib"]["path"]], "rust": RUST_FLAGS}
    constant = {"fa": fa_constant, "goose": goose_constant, "rust": False}
    requested = {}
    for side in inputs:
        modes = {"target": "native", "asynchronous": False, "overflow_checks": False,
                 "size_binding": "constant" if constant[side] else "runtime"}
        if side == "goose":
            modes["runtime_size_rewrite"] = runtime_size
        requested[side] = artifacts.request(name, side, size, inputs[side], compilers[side],
            flags[side], modes, {key: os.environ[key] for key in BUILD_ENVIRONMENT
                                if key in os.environ} if side == "fa" else None)
    arguments = {"fa": [str(size)], "goose": [] if goose_constant else [str(size)],
                 "rust": [str(size)]}
    return {"inputs": inputs, "requests": requested, "arguments": arguments}


ARTIFACTS = {
    "fa": ("fa/manifest.json", "fa/native/main", "fa/stdout.txt", "fa"),
    "goose": ("goose/manifest.json", "goose/main", "goose/stdout.txt", "goose"),
    "rust": ("rust.manifest.json", "rust", "expected.txt", "rust-source"),
}


def command(directory, side, build_plan):
    return [str(directory / ARTIFACTS[side][1]), *build_plan["arguments"][side]]


def prepare_sources(directory, side, build_plan):
    destination = directory / ARTIFACTS[side][3]
    if destination.is_dir():
        # Remove old source inputs, preserving the retained IR, logs and executable.
        for suffix in ("*.fa", "*.goose", "*.rs"):
            for path in destination.rglob(suffix):
                path.unlink()
    for name, data in build_plan["inputs"][side].items():
        path = destination / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)


def build_side(directory, side, build_plan, expected=None):
    manifest_path, binary, stdout_path, source_dir = ARTIFACTS[side]
    (directory / manifest_path).unlink(missing_ok=True)
    prepare_sources(directory, side, build_plan)
    requested = build_plan["requests"][side]
    compiler, flags = requested["compiler"]["path"], requested["flags"]
    if side == "fa":
        args = [compiler, flags[0], directory / source_dir / "main.fa", *flags[1:],
                f"--output={directory / source_dir}"]
    elif side == "goose":
        args = [compiler, *flags, "-o", directory / binary, directory / source_dir / "main.goose"]
    else:
        args = [compiler, *flags, directory / source_dir / "main.rs", "-o", directory / binary]
    log_path = directory / source_dir / "build.log"
    execute(args, log_path)
    if side == "goose":
        generated = re.findall(r"[^\s\"']+/main\.c", log_path.read_text())
        if generated and Path(generated[-1]).exists():
            shutil.copy2(generated[-1], directory / source_dir / "main.c")
            cc = requested["compiler"]["c_compiler"]["path"]
            if "clang" in requested["compiler"]["c_compiler"]["version"].lower():
                execute([cc, "-O2", "-S", "-emit-llvm", directory / source_dir / "main.c",
                         "-o", directory / source_dir / "main.ll"],
                        directory / source_dir / "ir-build.log")
    exe = directory / binary
    exe.with_suffix(".s").write_bytes(execute(["objdump", "-d", "--no-show-raw-insn", exe]))
    output = execute(command(directory, side, build_plan))
    manifest = artifacts.save(directory, manifest_path, requested, binary,
                              build_plan["arguments"][side], stdout_path, output, expected)
    return manifest, output


def reuse_side(directory, side, build_plan, expected=None):
    manifest_path, binary, stdout_path, source_dir = ARTIFACTS[side]
    manifest, output = artifacts.load(directory, manifest_path, build_plan["requests"][side],
        binary, build_plan["arguments"][side], stdout_path, expected)
    for name, data in build_plan["inputs"][side].items():
        if (directory / source_dir / name).read_bytes() != data:
            raise RuntimeError(f"{directory / source_dir / name}: saved source differs; rebuild")
    if execute(command(directory, side, build_plan)) != output:
        raise RuntimeError(f"{side}: current output differs from its manifest")
    return manifest, output


def prepare(directory, build_plan, reuse=False, rebuild_fa=False):
    directory.mkdir(parents=True, exist_ok=True)
    operation = reuse_side if reuse else build_side
    manifests = {}
    manifests["rust"], expected = operation(directory, "rust", build_plan)
    # Validate Goose before --rebuild-fa mutates anything.
    manifests["goose"], _ = operation(directory, "goose", build_plan, expected)
    manifests["fa"], _ = (build_side if rebuild_fa else operation)(
        directory, "fa", build_plan, expected)
    return {side: command(directory, side, build_plan) for side in ("fa", "goose")}, expected, manifests


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("names", nargs="*", choices=sorted(DEFAULTS))
    parser.add_argument("--suite", choices=("goose", "secondary", "all"), default="goose",
                        help="Default selection: the 16 original Goose benchmarks")
    parser.add_argument("--directory", type=Path, required=True)
    parser.add_argument("--runs", type=int, default=7)
    parser.add_argument("--default-sizes", action="store_true")
    parser.add_argument("--size", type=int, help="Override the selected sizes (growing-index: 16, 1000 or 1000000)")
    parser.add_argument("--profile", action="store_true")
    parser.add_argument("--goose-runtime-size", action="store_true",
                        help="Read BENCH_N from argv in generated Goose copies, keeping checkfree")
    parser.add_argument("--cpu", type=int, help="Pin this process and its children to one CPU")
    parser.add_argument("--reuse", action="store_true", help="Time existing builds")
    parser.add_argument("--rebuild-fa", action="store_true", help="With --reuse, rebuild only Fa")
    parser.add_argument("--report", default="results.json", help="JSON filename inside --directory")
    parser.add_argument("--prepare-only", action="store_true", help="Build and validate without timing")
    args = parser.parse_args()
    if args.size is not None and args.size < 1:
        parser.error("--size must be positive")
    if args.runs < 1:
        parser.error("--runs must be positive")
    if args.rebuild_fa and not args.reuse:
        parser.error("--rebuild-fa requires --reuse")
    if args.cpu is not None:
        try:
            os.sched_setaffinity(0, {args.cpu})
        except (AttributeError, OSError) as error:
            parser.error(f"cannot select CPU {args.cpu}: {error}")
    args.directory = args.directory.resolve()
    args.directory.mkdir(parents=True, exist_ok=True)
    try:
        compilers = toolchain()
    except (RuntimeError, OSError) as error:
        parser.error(str(error))
    cpu = next((line.split(":", 1)[1].strip() for line in Path("/proc/cpuinfo").read_text().splitlines()
                if line.startswith("model name")), platform.processor())
    results = {"machine": platform.platform(), "cpu": cpu,
               "cpu_affinity": sorted(os.sched_getaffinity(0)),
               "date": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
               "revision": execute(["git", "-c", f"safe.directory={ROOT}", "rev-parse", "HEAD"])
               .decode().strip(), "working_tree_dirty": bool(execute(
                   ["git", "-c", f"safe.directory={ROOT}", "status", "--porcelain"])),
               "runs": args.runs, "measurements": "preparation" if args.prepare_only else "timing",
               "size_set": "override" if args.size is not None
               else "default" if args.default_sizes else "audit", "benchmarks": {}}
    results["command_line"] = sys.argv
    results["runner_source_sha256"] = artifacts.source_hash({
        path.name: path.read_bytes() for path in (Path(__file__), Path(artifacts.__file__))})
    names = args.names or {"goose": GOOSE_SUITE, "secondary": SECONDARY,
                           "all": sorted(DEFAULTS)}[args.suite]
    results["selection"] = list(names)
    results["compilers"] = compilers
    for name in names:
        size = args.size if args.size is not None else (DEFAULTS if args.default_sizes else AUDIT)[name]
        directory = args.directory / name
        try:
            build_plan = plan(name, size, compilers, args.goose_runtime_size)
            commands, expected, manifests = prepare(directory, build_plan, args.reuse, args.rebuild_fa)
            result = {"size": size, "output_sha256": artifacts.sha256(expected), "artifacts": manifests}
            result["goose_size"] = ("constant" if len(commands["goose"]) == 1 else "runtime")
            if args.prepare_only:
                results["benchmarks"][name] = result
                print(f"{name:16} n={size:9} outputs and manifests verified", flush=True)
                (args.directory / args.report).write_text(json.dumps(results, indent=2) + "\n")
                continue
            samples = {"fa": [], "goose": []}
            for run in range(args.runs):
                for side in (["fa", "goose"] if run % 2 == 0 else ["goose", "fa"]):
                    samples[side].append(timed(commands[side], expected))
            for side in samples:
                result[side] = {"samples": samples[side], **{
                    key: statistics.median(sample[key] for sample in samples[side])
                    for key in ("wall", "cpu")}}
                result[side]["binary_sha256"] = hashlib.sha256(Path(commands[side][0]).read_bytes()).hexdigest()
                if args.profile:
                    result[side]["instructions"] = profile(commands[side], directory / side, expected)
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
