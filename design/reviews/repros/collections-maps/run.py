#!/usr/bin/env python3
"""Compare review fixtures with Bun, O0/O3 x 3 LLVM targets, leak checking.

Run from the repository root under taskset -c 0-4. No build is implicit.
FA_REVIEW_OUT selects the scratch directory, FA_REVIEW_FILTER selects names.
"""
import json
import os
from pathlib import Path
import signal
import subprocess
import time

ROOT = Path(__file__).resolve().parents[4]
HERE = Path(__file__).resolve().parent
OUT = Path(os.environ.get("FA_REVIEW_OUT", "/tmp/review-collections-maps"))
OUT.mkdir(parents=True, exist_ok=True)
ENV = dict(os.environ)
ENV["PATH"] = str(Path.home() / ".bun/bin") + ":" + ENV.get("PATH", "")
ENV["CC"] = str(Path.home() / ".local/llvm18/usr/bin/clang-18")
ENV["LLVM_SYS_181_PREFIX"] = str(Path.home() / ".local/llvm18/usr/lib/llvm-18")
RESULTS = []


def run(command, label):
    """Limit each compiler/runtime process to 512 MiB RSS and 120 seconds."""
    log = OUT / (label + ".log")
    started = time.monotonic()
    peak = 0
    reason = None
    with log.open("w") as output:
        proc = subprocess.Popen(command, cwd=ROOT, env=ENV, stdout=output,
                                stderr=subprocess.STDOUT, start_new_session=True)
        while proc.poll() is None:
            try:
                status = Path(f"/proc/{proc.pid}/status").read_text()
                rss = next((int(line.split()[1]) for line in status.splitlines()
                            if line.startswith("VmRSS:")), 0)
                peak = max(peak, rss)
            except (FileNotFoundError, ProcessLookupError):
                pass
            if peak > 512 * 1024 or time.monotonic() - started > 120:
                reason = "RSS limit" if peak > 512 * 1024 else "timeout"
                os.killpg(proc.pid, signal.SIGKILL)
                break
            time.sleep(0.05)
        code = proc.wait()
    return code, log.read_text(), peak, reason


for source in sorted(HERE.glob("*.fa")):
    if source.name == "illegal-iteration.fa":
        continue
    if os.environ.get("FA_REVIEW_FILTER", "") not in source.name:
        continue
    base = source.stem
    expected = "\n".join(line.split("-- prints: ", 1)[1]
                         for line in source.read_text().splitlines() if "-- prints: " in line) + "\n"
    code, text, peak, reason = run([str(ROOT / "target/debug/fa"), "run", str(source)], base + "-bun")
    oracle = text
    ok = code == 0 and text == expected
    RESULTS.append(dict(case=base, target="bun", passed=ok, code=code, output=text, peak_rss_kib=peak))
    print(base, "bun", "PASS" if ok else "FAIL", flush=True)
    for target in ("native", "wasm", "native-freestanding"):
        for level in (0, 3):
            label = f"{base}-{target}-O{level}"
            command = [str(ROOT / "target/debug/examples/build"), str(source),
                       "--target=" + target, "-O" + str(level), "--leak-check", "--emit-ir",
                       "--output=" + str(OUT / label), "--run"]
            if base != "await":
                command.append("--asynchronous=false")
            code, text, peak, reason = run(command, label)
            passed = code == 0 and text == oracle
            RESULTS.append(dict(case=base, target=target, opt=level, passed=passed,
                                code=code, output=text, peak_rss_kib=peak, limit=reason))
            print(label, "PASS" if passed else "FAIL", flush=True)
    (OUT / "results.json").write_text(json.dumps(RESULTS, indent=2) + "\n")

raise SystemExit(0 if all(row["passed"] for row in RESULTS) else 1)
