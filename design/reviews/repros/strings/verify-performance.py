#!/usr/bin/env python3
"""Check the published instruction table against JSON and retained raw profiles.

Run from the repository root. An optional argument checks another report table.
Missing /tmp raw profiles are reported as unavailable, never as verified.
"""

import hashlib
import json
from pathlib import Path
import sys


def main():
    root = Path("benchmarks/results")
    before = json.loads((root / "2026-11-16-strings-main-profiles.json").read_text())
    after = json.loads((root / "2026-11-16-strings-trunk-profiles.json").read_text())
    report = Path(sys.argv[1]) if len(sys.argv) > 1 else root / "2026-11-16-strings-trunk-instructions.md"
    checked = set()
    raw_checked = 0
    raw_missing = 0
    maximum = (-100.0, "")
    text = report.read_text()
    if "### Instruction comparison with trunk" in text:
        text = text.split("### Instruction comparison with trunk", 1)[1].split("### Controlled CPU series", 1)[0]
    for line in text.splitlines():
        cells = [cell.strip() for cell in line.strip("|").split("|")]
        if len(cells) != 6 or cells[0] not in after["benchmarks"]:
            continue
        name = cells[0]
        old = before["benchmarks"][name]
        new = after["benchmarks"][name]
        first = old["fa"]["events"]["Ir"]
        last = new["fa"]["events"]["Ir"]
        change = 100 * (last / first - 1)
        expected = [str(new["size"]), str(first), str(last), f"{change:+.3f}%", f'{last / new["goose"]["events"]["Ir"]:.3f}']
        actual = [cell.replace(",", "") for cell in cells[1:]]
        assert actual == expected, (name, actual, expected)
        assert old["size"] == new["size"]
        assert old["output_sha256"] == new["output_sha256"]
        assert change <= 1, (name, change)
        for language in ["fa", "goose", "rust"]:
            profile = new[language]
            path = Path(profile["callgrind"])
            if not path.exists():
                raw_missing += 1
                continue
            data = path.read_bytes()
            assert hashlib.sha256(data).hexdigest() == profile["profile_sha256"]
            lines = data.decode().splitlines()
            events = next(line[8:].split() for line in lines if line.startswith("events: "))
            totals = next(line[9:].split() for line in lines if line.startswith("summary: "))
            assert int(totals[events.index("Ir")]) == profile["events"]["Ir"]
            raw_checked += 1
        maximum = max(maximum, (change, name))
        checked.add(name)
    assert checked == set(after["benchmarks"]), sorted(set(after["benchmarks"]) - checked)
    print(json.dumps({"rows": len(checked), "raw_verified": raw_checked, "raw_unavailable": raw_missing, "largest_increase": maximum}, indent=2))


if __name__ == "__main__":
    main()
