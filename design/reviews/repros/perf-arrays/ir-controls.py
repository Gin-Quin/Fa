#!/usr/bin/env python3
"""Show that the firing checks fail on the archived pre-optimization compiler.

The archive paths are review-machine evidence; supply replacements via the dict
below when reproducing on another machine. Run under taskset -c 0-4.
"""
import os
from pathlib import Path
import re
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
OUT = Path("/tmp/review-perf-arrays/ir-controls")
OUT.mkdir(parents=True, exist_ok=True)
archives = {
    "remainder": Path("/tmp/perf-arrays/base/fa"),
    "fill": Path("/tmp/perf-arrays/remainder/fa"),
    "growing": Path("/tmp/perf-arrays/bool/compiler/fa"),
    "extents": Path("/tmp/perf-arrays/reverse-reverted/compiler/fa"),
}
checks = {
    "remainder": lambda ir: "invariant_divisor:" in ir,
    "fill": lambda ir: "call void @fa_array_fill(" in ir,
    "growing": lambda ir: "push_cursor" in ir,
    "extents": lambda ir: "ret i64 5" in re.search(r"define[^\n]*@fa.extent\(.*?\n}", ir, re.S)[0],
}
for name, before in archives.items():
    for phase, compiler in [("before", before), ("after", ROOT / "target/release/fa")]:
        directory = OUT / f"{name}-{phase}"
        memory = OUT / f"{name}-{phase}.rss"
        command = ["/usr/bin/time", "-f", "%M", "-o", str(memory), str(compiler), "build", str(HERE / "ir" / f"{name}.fa"),
                   "--target=native", "--asynchronous=false", "--emit-ir", "-O0", f"--output={directory}"]
        result = subprocess.run(command, capture_output=True, timeout=60)
        (OUT / f"{name}-{phase}.log").write_bytes(result.stdout + result.stderr)
        assert result.returncode == 0, result.stderr
        assert int(memory.read_text()) < 512 * 1024
        ir = (directory / "native/main.ll").read_text()
        assert checks[name](ir) == (phase == "after"), (name, phase)
        print(name, phase, "firing assertion", checks[name](ir))
