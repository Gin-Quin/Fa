#!/usr/bin/env python3
"""Times several builds of the same benchmark against each other: alternating runs, the
process (and its children) pinned to one CPU, the median of user + system CPU time, then
the Callgrind instruction count of each at a smaller size. Every output is checked first.

Usage: variants.py --expected expected.txt --size N --profile-size M --runs R --cpu C
                   --report results.json name=executable [name=executable ...]

The executables are those built by compare.py (`<directory>/<benchmark>/fa/native/main`,
`.../goose/main`), which takes the size as its first argument. Use --profile-expected for a Rust oracle at --profile-size; otherwise the first executable
is compared with the others. For growing-index, run once with --size 16 --profile-size 16,
then with --size 1000000 --profile-size 1000000, using the Array, GrowingArray and Goose
executables built by compare.py growing-index growing-index-array. All sizes are runtime inputs.
"""

import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import statistics
import subprocess
import tempfile
import time


def instructions(command):
    with tempfile.TemporaryDirectory() as directory:
        out = Path(directory) / "callgrind.out"
        subprocess.run([os.environ.get("VALGRIND", "valgrind"), "--tool=callgrind",
                        f"--callgrind-out-file={out}", *command], check=True,
                       capture_output=True)
        return int(re.search(r"^summary:\s+(\d+)", out.read_text(), re.M)[1])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("programs", nargs="+", help="name=executable")
    parser.add_argument("--expected", type=Path, required=True)
    parser.add_argument("--size", type=int, required=True)
    parser.add_argument("--profile-expected", type=Path, help="Rust output at --profile-size")
    parser.add_argument("--profile-size", type=int, required=True)
    parser.add_argument("--runs", type=int, default=21)
    parser.add_argument("--cpu", type=int, default=1)
    parser.add_argument("--report", type=Path, required=True)
    args = parser.parse_args()
    programs = dict(program.split("=", 1) for program in args.programs)
    expected = args.expected.read_bytes()
    reference = args.profile_expected.read_bytes() if args.profile_expected else None
    for name, executable in programs.items():
        output = subprocess.run([executable, str(args.size)], capture_output=True, check=True).stdout
        if output != expected:
            raise SystemExit(f"{name}: the output differs from {args.expected}")
        small = subprocess.run([executable, str(args.profile_size)], capture_output=True,
                               check=True).stdout
        reference = reference or small
        if small != reference:
            raise SystemExit(f"{name}: the output at {args.profile_size} differs")
    samples = {name: [] for name in programs}
    names = list(programs)
    for index in range(args.runs):
        order = names if index % 2 == 0 else list(reversed(names))
        for name in order:
            process = subprocess.Popen(["taskset", "-c", str(args.cpu), programs[name],
                                        str(args.size)], stdout=subprocess.DEVNULL)
            start = time.perf_counter()
            _, status, usage = os.wait4(process.pid, 0)
            wall = time.perf_counter() - start
            if status:
                raise SystemExit(f"{name} exited with {status}")
            samples[name].append({"cpu": usage.ru_utime + usage.ru_stime, "wall": wall})
    report = {"date": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()), "cpu": args.cpu,
              "runs": args.runs, "size": args.size, "profile_size": args.profile_size,
              "output_sha256": hashlib.sha256(expected).hexdigest(),
              "load_average": os.getloadavg(), "programs": {}}
    for name, executable in programs.items():
        cpu = statistics.median(sample["cpu"] for sample in samples[name])
        report["programs"][name] = {
            "executable": executable,
            "binary_sha256": hashlib.sha256(Path(executable).read_bytes()).hexdigest(),
            "cpu": cpu, "wall": statistics.median(sample["wall"] for sample in samples[name]),
            "cpu_min": min(sample["cpu"] for sample in samples[name]),
            "instructions": instructions([executable, str(args.profile_size)]),
            "samples": samples[name]}
        print(f"{name:12} median CPU {cpu:.4f} s, min {report['programs'][name]['cpu_min']:.4f} s,"
              f" {report['programs'][name]['instructions']:,} instructions", flush=True)
    args.report.write_text(json.dumps(report, indent=2) + "\n")


if __name__ == "__main__":
    main()
