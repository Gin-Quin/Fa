#!/usr/bin/env python3
"""Check every retained array instruction gate against its driver records."""
import json
from pathlib import Path
import sys

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "benchmarks/audit"))
import tables
import measurements

results = ROOT / "benchmarks/results"
total = 0
for gate in sorted(results.glob("*arrays*-gate.json")):
    prefix = gate.name.removesuffix("-gate.json")
    before_prefix = prefix.removesuffix("-restored")
    def profiles(path):
        data = json.loads(path.read_text())
        return {(name, row["size"]): row for phase in data["series"]
                if phase["mode"] == "profile" for name, row in phase["benchmarks"].items()}
    before = profiles(results / f"{before_prefix}-before.json")
    after = profiles(results / f"{prefix}.json")
    data = json.loads(gate.read_text())
    assert data["passes"] and len(data["rows"]) == 40
    for row in data["rows"]:
        key = (row["name"], row["size"])
        b, a = before[key], after[key]
        assert b["output_sha256"] == a["output_sha256"]
        for side in ["fa", "goose", "rust"]:
            for phase, record in [("before", b), ("after", a)]:
                field = f"{phase}_ir" if side == "fa" else f"{phase}_{side}"
                assert row[field] == record[side]["events"]["Ir"], (gate, key, field)
            for field in ["source_sha256", "flags"]:
                assert a["artifacts"][side]["request"][field] == b["artifacts"][side]["request"][field]
        assert row["after_ir"] <= row["before_ir"] * 1.01
        assert abs(row["delta_percent"] - (row["after_ir"] / row["before_ir"] - 1) * 100) < 1e-10
    total += len(data["rows"])
    print(gate.name, "40/40 matched")

for path in sorted(results.glob("*arrays*.md")):
    data = json.loads(path.with_suffix(".json").read_text())
    title, rest = tables.markdown(data).split("\n", 1)
    rendered = title + f"\n\n[Measurements and provenance]({path.stem}.json).\n\n" + rest.lstrip("\n")
    assert rendered == path.read_text(), path
    print(path.name, "exact driver rendering")
print(total, "instruction comparisons verified")

cpu_rows = 0
for path in sorted(results.glob("*arrays*.json")):
    data = json.loads(path.read_text())
    for phase in data.get("series", []):
        if phase["mode"] != "cpu":
            continue
        assert phase["cpu_affinity"] == [5]
        for name, row in phase["benchmarks"].items():
            assert len(row["orders"]) == 21
            samples = {key: [sample["cpu"] for sample in value["samples"]]
                       for key, value in row.items() if isinstance(value, dict) and "samples" in value}
            assert all(len(values) == 21 for values in samples.values())
            for key, side in [("control", "fa_control"), ("goose", "goose"), ("rust", "rust")]:
                actual = measurements.paired(samples["fa"], samples[side])
                recorded = row["control"] if key == "control" else row["comparisons"][key]
                for field in actual:
                    assert actual[field] == recorded[field], (path, name, key, field)
            assert measurements.control_passes(row["control"]) == row["control"]["passes"]
            for label, pair in row.get("additional_pairs", {}).items():
                assert measurements.paired(samples["fa"], samples[label]) == pair
            for order in row["orders"]:
                assert sorted(order) == sorted(samples)
            cpu_rows += 1
print(cpu_rows, "retained CPU rows independently recomputed, including archived duplicates")
