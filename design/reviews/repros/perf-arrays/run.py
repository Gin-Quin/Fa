#!/usr/bin/env python3
"""Compare review programs with Bun at O0/O3 on all LLVM targets.

Run under taskset -c 0-4. Retain stdout/stderr, commands, and RSS in /tmp.
The runner enforces a 512 MiB process-group RSS limit and 60-second timeout.
"""
import json
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path(__file__).resolve().parents[4]
OUT = Path(os.environ.get("REVIEW_OUTPUT", "/tmp/review-perf-arrays/matrix"))
OUT.mkdir(parents=True, exist_ok=True)
os.environ["PATH"] = f'{Path.home()}/.bun/bin:{Path.home()}/.local/bin:' + os.environ["PATH"]
records = []

def run(label, command, expected=None):
    command = list(map(str, command))
    record = {"label": label, "command": command}
    try:
        with (OUT / f"{label}.stdout").open("wb") as stdout, (OUT / f"{label}.stderr").open("wb") as stderr:
            process = subprocess.Popen(command, stdout=stdout, stderr=stderr, start_new_session=True)
            started = time.monotonic()
            peak = 0
            while process.poll() is None:
                rss = 0
                for path in Path("/proc").glob("[0-9]*/stat"):
                    try:
                        fields = path.read_text().rsplit(")", 1)[1].split()
                        if int(fields[2]) == process.pid:
                            rss += int(fields[21]) * os.sysconf("SC_PAGE_SIZE") // 1024
                    except (OSError, ValueError, IndexError):
                        pass
                peak = max(peak, rss)
                if peak > 512*1024 or time.monotonic() - started > 60:
                    os.killpg(process.pid, 9)
                    process.wait()
                    raise RuntimeError("RSS limit or timeout exceeded")
                time.sleep(.01)
        record.update({"exit": process.returncode, "peak_rss_kib": peak})
        actual = (OUT / f"{label}.stdout").read_bytes()
        if process.returncode:
            raise RuntimeError((OUT / f"{label}.stderr").read_text(errors="replace"))
        if expected is None:
            expected = actual
        if actual != expected:
            raise RuntimeError(f"expected {expected!r}, observed {actual!r}")
        record["pass"] = True
    except Exception as error:
        record.update({"pass": False, "error": str(error)})
    records.append(record)
    (OUT / "results.json").write_text(json.dumps(records, indent=2) + "\n")
    print(label, "PASS" if record["pass"] else "FAIL", flush=True)
    return expected

for argument in sys.argv[1:]:
    source = Path(argument).resolve()
    name = source.stem
    expected = run(f"{name}-bun", [ROOT / "target/release/fa", "run", "--target=bun", source])
    for target in ["native", "native-freestanding", "wasm"]:
        for opt in [0, 3]:
            label = f"{name}-{target}-O{opt}"
            run(label, [ROOT / "target/debug/examples/build", source,
                f"--target={target}", f"-O{opt}", "--leak-check", "--emit-ir",
                f'--asynchronous={os.environ.get("REVIEW_ASYNC", "false")}',
                f'--overflow-checks={os.environ.get("REVIEW_OVERFLOW", "true")}',
                f"--output={OUT / label}", "--run"], expected)

sys.exit(1 if any(not record["pass"] for record in records) else 0)
