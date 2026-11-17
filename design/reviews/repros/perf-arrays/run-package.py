#!/usr/bin/env python3
"""Reuse the existing review package runner against the current warm libraries.

Run after the LLVM-enabled workspace build, under taskset -c 0-4.
No second Cargo command or duplicate Rust driver is needed.
"""
from pathlib import Path
import os
import subprocess

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[3]
OUT = Path("/tmp/review-perf-arrays/package")
OUT.mkdir(parents=True, exist_ok=True)
deps = ROOT / "target/debug/deps"
llvm = Path.home() / ".local/llvm18/usr/lib/llvm-18/lib"
command = ["rustc", "--edition=2024", str(HERE.parent / "collections-maps/run_package.rs"),
           "-L", f"dependency={deps}", "-L", str(llvm), "-C", f"link-arg=-Wl,-rpath,{llvm}"]
for crate in ["fa_analyzer", "fa_codegen_llvm"]:
    library = max(deps.glob(f"lib{crate}-*.rlib"), key=lambda path: path.stat().st_mtime)
    command += ["--extern", f"{crate}={library}"]
for directory in (ROOT / "target/debug/build").glob("*/out"):
    command += ["-L", str(directory)]
command += ["-o", str(OUT / "runner")]
subprocess.run(command, check=True)
os.environ["PATH"] = f'{Path.home()}/.bun/bin:{Path.home()}/.local/bin:' + os.environ["PATH"]
subprocess.run([str(OUT / "runner"), str(HERE), str(OUT / "output")], check=True)
