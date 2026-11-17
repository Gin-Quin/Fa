#!/usr/bin/env python3
"""Fresh processes, alternating variants, GNU time peak RSS; CPU 5 only.

Run under taskset -c 0-4. Only the timed JS subprocesses use CPU 5.
"""
import json
from pathlib import Path
import statistics
import subprocess
import sys
import tempfile

ROOT = Path(__file__).resolve().parent
study = sys.argv[1] if len(sys.argv) > 1 else "rng64"
names_for_study = {"rng64": ["bigint", "words", "scalar"], "when": ["chain", "switched"]}
assert study in names_for_study
samples = []
engines = [Path.home() / ".bun/bin/bun", Path.home() / ".local/bin/node"]
for engine in engines:
    version = subprocess.check_output([engine, "--version"], text=True).strip()
    for repeat in range(5):
        names = names_for_study[study].copy()
        if repeat % 2:
            names.reverse()
        for name in names:
            with tempfile.NamedTemporaryFile() as memory:
                output = subprocess.check_output(
                    ["/usr/bin/time", "-f", "%M", "-o", memory.name,
                     "taskset", "-c", "5", str(engine), str(ROOT / (study + ".mjs")), name],
                    text=True, timeout=60,
                )
                sample = json.loads(output)
                sample.update(engine=engine.name, version=version, repeat=repeat,
                              peak_kib=int(Path(memory.name).read_text()))
                samples.append(sample)
    for name in names_for_study[study]:
        group = [s for s in samples if s["engine"] == engine.name and s["name"] == name]
        print(engine.name, version, name,
              round(statistics.median(s["milliseconds"] for s in group), 3), "ms",
              round(statistics.median(s["peak_kib"] for s in group)), "KiB", flush=True)
checksums = {(s.get("lo"), s.get("hi"), s["checksum"]) for s in samples}
assert len(checksums) == 1, checksums
(ROOT / (study + "-results.json")).write_text(json.dumps(samples, indent=2) + "\n")
