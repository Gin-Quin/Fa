#!/usr/bin/env python3
"""Checked Fa/Goose/Rust build, paired CPU/control, profile and secondary-target driver.

See audit/README.md for the four phases and schema-2 JSON/Markdown generation.
"""

import argparse
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

import artifacts
import measurements
import tables


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
            "growing-index-array": ("growing-index", "array.fa"),
            "push-array": ("push", "array.fa"),
            "sum-reserved": ("sum", "reserved.fa"),
            "particles-reserved": ("particles", "reserved.fa"),
            "blur-reserved": ("blur", "reserved.fa"),
            "respond-interpolation": ("respond", "interpolation.fa"),
            "lru-open": ("lru", "open.fa"),
            "words-wide": ("words", "wide.fa")}
# A structural variant can have its own matching Rust row. The selected entry
# is copied as main.rs; alternate entries never pollute another row's manifest.
RUST_VARIANTS = {"lru-open": "open.rs", "words-wide": "wide.rs"}
DEFAULTS.update({name: DEFAULTS[directory] for name, (directory, _) in VARIANTS.items()})
AUDIT = {name: int(size) for name, size in
         (line.split() for line in Path(__file__).with_name("sizes.txt").read_text().splitlines())}
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


def profile(command, directory, expected):
    directory.mkdir(parents=True, exist_ok=True)
    path = directory / "callgrind.out"
    output = execute([os.environ.get("VALGRIND", str(Path.home() / ".local/bin/valgrind")),
             "--tool=callgrind", "--dump-instr=yes", "--cache-sim=yes", "--branch-sim=yes",
             "--I1=32768,8,64", "--D1=32768,8,64", "--LL=8388608,16,64",
             f"--callgrind-out-file={path}", *command], directory / "callgrind.log")
    if output != expected:
        raise RuntimeError(f"{command[0]}: profiled output differs from Rust")
    text = path.read_text()
    events = re.search(r"^events: (.*)$", text, re.M)[1].split()
    totals = [int(value) for value in re.search(r"^summary: (.*)$", text, re.M)[1].split()]
    assembly = Path(command[0]).with_suffix(".s")
    hot = execute([sys.executable, Path(__file__).with_name("hot.py"), path, assembly,
                   "--calls", "--top", "20"])
    if hot.startswith(b"total self cost 0 "):
        raise RuntimeError(f"{command[0]}: no executable instruction addresses matched the profile")
    (directory / "hot.txt").write_bytes(hot)
    cache = {}
    for event in ("D1mr", "D1mw", "DLmr", "DLmw"):
        annotation = execute([sys.executable, Path(__file__).with_name("hot.py"), path,
                              assembly, "--event", event, "--top", "8"])
        (directory / f"hot-{event}.txt").write_bytes(annotation)
        cache[event] = annotation.decode()
    annotations = {}
    annotate = Path.home() / ".local/valgrind/usr/bin/callgrind_annotate"
    for inclusive in (False, True):
        report = execute([annotate, f"--inclusive={'yes' if inclusive else 'no'}", path])
        (directory / ("inclusive.txt" if inclusive else "self.txt")).write_bytes(report)
        annotations["inclusive" if inclusive else "exclusive"] = report.decode()
    return {"events": dict(zip(events, totals)), "callgrind": str(path),
            "profile_sha256": artifacts.file_hash(path), "hot_assembly": hot.decode(),
            "hot_cache_assembly": cache, "function_profiles": annotations,
            "annotation": str(directory / "self.txt")}


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


def toolchain(fa_program=None):
    """Hash the actual compilers, including Fa's delegated native backend."""
    fa = artifacts.executable_identity(fa_program or os.environ.get("FA", ROOT / "target/release/fa"))
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
    alternate_rust = {entry for variant, entry in RUST_VARIANTS.items()
                      if VARIANTS[variant][0] == source.name}
    rust = {str(path.relative_to(source)): path.read_bytes() for path in source.rglob("*.rs")
            if path.name not in alternate_rust}
    if name in RUST_VARIANTS:
        rust["main.rs"] = (source / RUST_VARIANTS[name]).read_bytes()
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
    return {side: command(directory, side, build_plan) for side in ("fa", "goose", "rust")}, expected, manifests


def secondary(directory, name, size, compilers, expected, runs, reuse, memory_mib):
    build_plan = plan(name, size, compilers)
    results = {}
    for target in ("wasm", "bun"):
        destination = directory / target
        destination.mkdir(parents=True, exist_ok=True)
        flags = [flag.replace("--target=native", f"--target={target}")
                 for flag in FA_FLAGS if flag != "--emit-ir"]
        requested = artifacts.request(name, target, size, build_plan["inputs"]["fa"],
            compilers["fa"], flags, {"target": target, "asynchronous": False,
            "overflow_checks": False}, build_plan["requests"]["fa"]["environment"])
        manifest_path = "manifest.json"
        try:
            if not reuse:
                for file, data in build_plan["inputs"]["fa"].items():
                    path = destination / file
                    path.parent.mkdir(parents=True, exist_ok=True)
                    path.write_bytes(data)
                execute([compilers["fa"]["path"], flags[0], destination / "main.fa", *flags[1:],
                         f"--output={destination}"], destination / "build.log")
            loaders = sorted((destination / target).glob("*.js")) + sorted((destination / target).glob("*.mjs"))
            if len(loaders) != 1:
                raise RuntimeError(f"{target}: expected one JavaScript loader, found {loaders}")
            relative = str(loaders[0].relative_to(destination))
            command = [os.environ.get("BUN", str(Path.home() / ".bun/bin/bun")),
                       str(loaders[0]), str(size)]
            if reuse:
                manifest, _ = artifacts.load(destination, manifest_path, requested, relative,
                    [str(size)], "stdout.txt", expected)
                for file, data in build_plan["inputs"]["fa"].items():
                    if (destination / file).read_bytes() != data:
                        raise RuntimeError("secondary saved source differs")
                if target == "wasm" and artifacts.file_hash(destination / manifest["wasm"]) != manifest["wasm_sha256"]:
                    raise RuntimeError("wasm artifact hash differs")
            else:
                # Validate before publishing any reuse manifest.
                measurements.sample(command, expected, max_rss_kib=memory_mib * 1024)
                manifest = artifacts.save(destination, manifest_path, requested, relative,
                    [str(size)], "stdout.txt", expected, expected)
                if target == "wasm":
                    wasm = list((destination / target).glob("*.wasm"))
                    if len(wasm) != 1:
                        raise RuntimeError("expected one wasm artifact")
                    manifest.update(wasm=str(wasm[0].relative_to(destination)),
                                    wasm_sha256=artifacts.file_hash(wasm[0]))
                    (destination / manifest_path).write_text(json.dumps(manifest, indent=2) + "\n")
            samples = [measurements.sample(command, expected, max_rss_kib=memory_mib * 1024)
                       for _ in range(runs)]
            results[target] = {"samples": samples, "cpu": statistics.median(s["cpu"] for s in samples),
                               "wall": statistics.median(s["wall"] for s in samples), "artifact": manifest}
        except (RuntimeError, OSError, subprocess.SubprocessError) as error:
            results[target] = {"error": str(error)}
    return results


def additional_variant(name, directory, size, runtime_size, expected):
    """Revalidate the saved compiler live, then the current benchmark's full contract."""
    try:
        saved = json.loads((directory / "fa/manifest.json").read_text())
        compiler_path = saved["request"]["compiler"]["path"]
    except (OSError, ValueError, KeyError, TypeError) as error:
        raise RuntimeError(f"{directory}: missing or invalid additional Fa manifest") from error
    compilers = toolchain(compiler_path)
    build_plan = plan(name, size, compilers, runtime_size)
    commands, reference, manifests = prepare(directory, build_plan, reuse=True)
    if reference != expected:
        raise RuntimeError(f"{name}: additional variant has a different Rust reference")
    return commands["fa"], manifests


def save_report(path, results):
    results["end"] = measurements.snapshot()
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_suffix(".tmp")
    temporary.write_text(json.dumps(results, indent=2) + "\n")
    temporary.replace(path)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("names", nargs="*", choices=sorted(DEFAULTS))
    parser.add_argument("--suite", choices=("goose", "secondary", "all"), default="goose")
    parser.add_argument("--directory", type=Path)
    parser.add_argument("--mode", choices=("prepare", "cpu", "profile", "secondary"), default="cpu")
    parser.add_argument("--runs", type=int, default=21)
    parser.add_argument("--default-sizes", action="store_true")
    parser.add_argument("--size", type=int)
    parser.add_argument("--profile", action="store_true", help="Alias for --mode profile")
    parser.add_argument("--goose-runtime-size", action="store_true")
    parser.add_argument("--cpu", type=int, default=1)
    parser.add_argument("--reuse", action="store_true")
    parser.add_argument("--rebuild-fa", action="store_true")
    parser.add_argument("--report", type=Path, help="JSON output, default benchmarks/results/<mode>.json")
    parser.add_argument("--prepare-only", action="store_true", help="Alias for --mode prepare")
    parser.add_argument("--render", type=Path, nargs="+", help="Merge phase JSON and generate Markdown")
    parser.add_argument("--markdown", type=Path)
    parser.add_argument("--update-readme", action="store_true",
                        help="With --render, replace only the generated README result block")
    parser.add_argument("--require-control", action="store_true",
                        help="With --render, require a passing control for every CPU verdict")
    parser.add_argument("--supplemental", action="store_true",
                        help="Retain this diagnostic series in JSON without replacing headline table rows")
    parser.add_argument("--secondary-memory-mib", type=int, default=512)
    parser.add_argument("--additional", action="append", default=[], metavar="NAME=DIRECTORY",
                        help="Interleave an already prepared Fa variant (one selected benchmark, CPU mode)")
    args = parser.parse_args()
    if args.render:
        tables.render(args.render, args.report, args.markdown, args.update_readme,
                      require_control=args.require_control)
        return
    if args.require_control:
        parser.error("--require-control requires --render")
    if args.update_readme:
        parser.error("--update-readme requires --render")
    if not args.directory:
        parser.error("--directory is required for measurements")
    if args.profile:
        args.mode = "profile"
    if args.prepare_only:
        args.mode = "prepare"
    if args.size is not None and args.size < 1 or args.runs < 1 or args.secondary_memory_mib < 1:
        parser.error("size, runs and memory limit must be positive")
    if args.additional and (args.mode != "cpu" or len(args.names) != 1):
        parser.error("--additional requires CPU mode and exactly one selected benchmark")
    if args.rebuild_fa and not args.reuse:
        parser.error("--rebuild-fa requires --reuse")
    try:
        os.sched_setaffinity(0, {args.cpu})
    except (AttributeError, OSError) as error:
        parser.error(f"cannot select CPU {args.cpu}: {error}")
    args.directory = args.directory.resolve()
    args.directory.mkdir(parents=True, exist_ok=True)
    compilers = toolchain()
    cpu = next((line.split(":", 1)[1].strip() for line in Path("/proc/cpuinfo").read_text().splitlines()
                if line.startswith("model name")), platform.processor())
    results = {"schema": 2, "mode": args.mode, "start": measurements.snapshot(),
               "machine": platform.platform(), "cpu": cpu, "cpu_affinity": sorted(os.sched_getaffinity(0)),
               "revision": execute(["git", "-C", ROOT, "rev-parse", "HEAD"]).decode().strip(),
               "working_tree_dirty": bool(execute(["git", "-C", ROOT, "status", "--porcelain"])),
               "runs": args.runs, "size_set": "override" if args.size is not None
               else "default" if args.default_sizes else "audit", "benchmarks": {},
               "command_line": sys.argv, "compilers": compilers,
               "runner_source_sha256": artifacts.source_hash({
                   path.name: path.read_bytes() for path in (Path(__file__), Path(artifacts.__file__),
                   Path(measurements.__file__), Path(tables.__file__),
                   Path(__file__).with_name("sizes.txt"), Path(__file__).with_name("hot.py"))})}
    results["compiler_source_revision"] = os.environ.get("FA_SOURCE_REVISION")
    results["supplemental"] = args.supplemental
    if args.mode == "secondary":
        results["secondary_memory_limit_mib"] = args.secondary_memory_mib
        results["runtime"] = artifacts.executable_identity(os.environ.get("BUN", str(Path.home() / ".bun/bin/bun")))
    names = args.names or {"goose": GOOSE_SUITE, "secondary": SECONDARY, "all": sorted(DEFAULTS)}[args.suite]
    results["selection"] = list(names)
    report = args.report or ROOT / "benchmarks/results" / f"{args.mode}.json"
    if not report.is_absolute():
        report = ROOT / report
    for name in names:
        size = args.size if args.size is not None else (DEFAULTS if args.default_sizes else AUDIT)[name]
        directory = args.directory / name
        start = measurements.snapshot()
        try:
            build_plan = plan(name, size, compilers, args.goose_runtime_size)
            commands, expected, manifests = prepare(directory, build_plan, args.reuse, args.rebuild_fa)
            result = {"size": size, "suite": "goose" if name in GOOSE_SUITE else "secondary",
                      "output_sha256": artifacts.sha256(expected),
                      "artifacts": manifests, "start": start}
            if args.mode == "cpu":
                commands["fa_control"] = commands["fa"]
                additional = {}
                for specification in args.additional:
                    label, separator, path = specification.partition("=")
                    if not separator or label not in DEFAULTS or label in commands:
                        raise ValueError(f"invalid or duplicate additional variant: {specification}")
                    commands[label], additional[label] = additional_variant(
                        label, Path(path).resolve(), size, args.goose_runtime_size, expected)
                result["additional_artifacts"] = additional
                result["commands"] = commands
                samples = {side: [] for side in commands}
                orders = []
                for run in range(args.runs):
                    order = measurements.order_for_round(list(commands), run)
                    orders.append(order)
                    for side in order:
                        samples[side].append(measurements.sample(commands[side], expected))
                for side, series in samples.items():
                    result[side] = {"samples": series, **{
                        key: statistics.median(sample[key] for sample in series)
                        for key in ("cpu", "wall", "peak_rss_kib")}}
                cpu_series = {side: [s["cpu"] for s in series] for side, series in samples.items()}
                result["control"] = measurements.paired(cpu_series["fa"], cpu_series["fa_control"])
                result["control"]["passes"] = measurements.control_passes(result["control"])
                result["control"]["binary_sha256"] = manifests["fa"]["binary_sha256"]
                result["comparisons"] = {}
                result["acceptance_eligible"] = args.runs >= 21
                for side, threshold in (("goose", 1.10), ("rust", 1.0)):
                    comparison = measurements.paired(cpu_series["fa"], cpu_series[side])
                    comparison["threshold"] = threshold
                    comparison["verdict"] = (measurements.verdict(comparison, threshold, result["control"])
                                             if result["acceptance_eligible"] else "within noise")
                    result["comparisons"][side] = comparison
                result["additional_pairs"] = {
                    label: measurements.paired(cpu_series["fa"], cpu_series[label])
                    for label in additional}
                result["orders"] = orders
                for side in ("fa", "goose", "rust"):
                    location = directory / ("rust-profile" if side == "rust" else side)
                    location.mkdir(parents=True, exist_ok=True)
                    result[side]["gnu_peak_rss_kib"] = measurements.gnu_rss(commands[side], expected, location)
                    binary = Path(commands[side][0])
                    stripped = binary.with_suffix(".stripped")
                    shutil.copy2(binary, stripped)
                    execute(["strip", stripped])
                    result[side]["executable_bytes"] = binary.stat().st_size
                    result[side]["stripped_bytes"] = stripped.stat().st_size
            elif args.mode == "profile":
                for side in commands:
                    location = directory / ("rust-profile" if side == "rust" else side)
                    repetitions = 3 if side == "rust" and name in ("lru", "words", "words-wide", "word-count") else 1
                    profiles = [profile(commands[side], location / f"run-{index}", expected)
                                for index in range(repetitions)]
                    result[side] = sorted(profiles, key=lambda item: item["events"]["Ir"])[repetitions // 2]
                    result[side]["repeated_events"] = [item["events"] for item in profiles]
                    result[side]["randomized_hasher"] = repetitions > 1
            elif args.mode == "secondary":
                result["targets"] = secondary(directory, name, size, compilers, expected, args.runs, args.reuse,
                                               args.secondary_memory_mib)
            result["end"] = measurements.snapshot()
            results["benchmarks"][name] = result
            detail = (" ".join(f"Fa/{side}={value['ratio_of_medians']:.3f} ({value['verdict']})"
                      for side, value in result["comparisons"].items()) if "comparisons" in result else args.mode)
            print(f"{name:24} n={size:9} {detail}", flush=True)
        except (RuntimeError, OSError, subprocess.SubprocessError, ValueError) as error:
            results["benchmarks"][name] = {"error": str(error), "size": size, "start": start,
                                           "end": measurements.snapshot()}
            print(f"{name}: {error}", flush=True)
        save_report(report, results)
    if any("error" in result or any("error" in target for target in result.get("targets", {}).values())
           for result in results["benchmarks"].values()):
        raise SystemExit(1)


if __name__ == "__main__":
    main()
