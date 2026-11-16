#!/usr/bin/env python3
"""Recompute the engineer's instruction tables from the retained driver JSON."""
import json
from pathlib import Path

root = Path(__file__).resolve().parents[4]
profiles = {}
for phase in ("before", "probes", "headers", "positive", "scalar64"):
    data = json.loads((root / f"benchmarks/results/2026-11-16-collections-{phase}.json").read_text())
    profiles[phase] = next(s for s in data.get("series", [data]) if s["mode"] == "profile")["benchmarks"]

for phase, previous in zip(("probes", "headers", "positive", "scalar64"),
                           ("before", "probes", "headers", "positive")):
    print(phase)
    regressions = []
    for name, after in profiles[phase].items():
        before = profiles[previous][name]
        assert before["size"] == after["size"]
        assert before["output_sha256"] == after["output_sha256"]
        for language in ("fa", "goose", "rust"):
            assert (before["artifacts"][language]["request"]["source_sha256"] ==
                    after["artifacts"][language]["request"]["source_sha256"])
        a, b = before["fa"]["events"]["Ir"], after["fa"]["events"]["Ir"]
        delta = 100 * (b / a - 1)
        if delta > 1:
            regressions.append((name, delta))
        if name in ("lru", "words", "word-count"):
            print(name, after["size"], a, b, f"{delta:+.8f}%",
                  after["goose"]["events"]["Ir"], after["rust"]["events"]["Ir"])
    assert regressions == ([] if phase != "headers" else [("lru", regressions[0][1])])
    print("above +1%:", regressions)
