#!/usr/bin/env python3
"""Evaluate the branch's firing assertions with each optimization absent.

Uses archived fa/fa-native pairs under FA_REVIEW_ARCHIVE (default
/tmp/perf-collections): base, probes, headers, positive. Rebuild those at
cbe3c5a6, 30728961, c2a1876d, 574bcf21 if the archives are gone.
The current compiler supplies the positive control. Run under taskset -c 0-4.
"""
import os
from pathlib import Path
import subprocess

root = Path(__file__).resolve().parents[4]
out = Path(os.environ.get("FA_REVIEW_OUT", "/tmp/review-collections-maps/ir-controls"))
archive = Path(os.environ.get("FA_REVIEW_ARCHIVE", "/tmp/perf-collections"))
out.mkdir(parents=True, exist_ok=True)
sources = {
    "negative": "function append = (items: Array<UInt32>, value: UInt32) mutates items => { items.push(value) }\nmutable values: Map<UInt32, UInt32> = []\nmutable items: Array<UInt32> = []\nlet key: UInt32 = 3\nif values.get(key) is none {\nappend(items, key)\nvalues[key] = 7\n}\nconsole.log(values.get(key))\n",
    "header": "mutable map = Map<UInt32, UInt32>.from([(0, 1)], capacity = 128)\nmap[2] = 3\nconsole.log(map.get(2), map.size)\n",
    "positive": "function run = (input: UInt32) => {\nlet key = input\nmutable map = Map<UInt32, UInt32>.from([(key, 7)], capacity = 16)\nconsole.log(map.get(key))\nmap[key] = 8\nconsole.log(map.take(key))\n}\nrun(3)\n",
    "scalar64": "function run = (key: Int64) => {\nmutable map = Map<Int64, Int64>.from([], capacity = 16)\nmap[key] = 7\nconsole.log(map.get(key))\n}\nrun(3)\n",
}
stages = {"negative": "base", "header": "probes", "positive": "headers", "scalar64": "positive"}


def body(ir, name):
    return next(s.split("\n}", 1)[0] for s in ir.split("define ") if "@fa." + name + "(" in s)


def compile_ir(name, source, compiler, target, state):
    path = out / (name + ".fa")
    path.write_text(source)
    directory = out / f"{name}-{state}-{target}"
    result = subprocess.run([str(compiler), "build", str(path), "--target=" + target,
                             "-O0", "--emit-ir", "--output=" + str(directory)],
                            cwd=root, capture_output=True, text=True, timeout=120)
    assert result.returncode == 0, (name, state, result.stdout, result.stderr)
    return (directory / target / ("main.wasm.ll" if target == "wasm" else "main.ll")).read_text()


for name, source in sources.items():
    for target in ("native", "wasm"):
        for state, compiler in (("off", archive / stages[name] / "fa"),
                                ("on", root / "target/debug/fa")):
            ir = compile_ir(name, source, compiler, target, state)
            if name == "negative":
                fires = "probe_cached" in ir
            elif name == "header":
                fires = "@fa_table_new_local(" in ir
            elif name == "positive":
                function = body(ir, "run")
                fires = ("probe_cached" in function and "@fa_table_remove(" not in function
                         and "store i1 true, ptr %probe_valid" in function)
            else:
                fires = "i64 10995116277760)" in body(ir, "run")
            print(name, target, state, "firing assertions", fires, flush=True)
            assert fires == (state == "on"), (name, state)

for case, maps, recursive, fixed, expected in (("ordinary", 48, False, False, 42),
                                               ("recursive", 8, True, False, 5),
                                               ("literal", 48, False, True, 41)):
    source = "function many = (depth: Integer, key: UInt32): Integer => {\n"
    if fixed:
        source += "let fixed: Map<UInt32, UInt32> = [(key, 1)]\n"
    source += "".join(f"mutable map{i} = Map<UInt32, UInt32>.from([(key, 0)], capacity = 32)\n" for i in range(maps))
    source += "if depth > 0 { return many(depth - 1, key) }\n" if recursive else "assert depth == 0\n"
    sizes = [f"map{i}.size" for i in range(maps)] + (["fixed.size"] if fixed else [])
    source += "console.log(" + ", ".join(sizes) + ")\n0\n}\nconsole.log(many(0, 7))\n"
    for target in ("native", "wasm"):
        for state, compiler in (("off", archive / "probes/fa"), ("on", root / "target/debug/fa")):
            ir = compile_ir("budget-" + case, source, compiler, target, state)
            count = body(ir, "many").count("@fa_table_new_local(")
            print(case, target, state, "headers", count, "required", expected, flush=True)
            assert count == (expected if state == "on" else 0)
