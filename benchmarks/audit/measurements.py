"""Statistics and collection shared by the single compare.py driver (stdlib only)."""

import math
import os
from pathlib import Path
import statistics
import subprocess
import tempfile
import time


def quantile(values, probability):
    """Linear interpolation at (n - 1) * p (Hyndman–Fan type 7)."""
    values = sorted(values)
    if not values or not 0 <= probability <= 1:
        raise ValueError("quantile needs samples and a probability in [0, 1]")
    position = (len(values) - 1) * probability
    lower = math.floor(position)
    upper = math.ceil(position)
    return values[lower] + (values[upper] - values[lower]) * (position - lower)


def paired(numerator, denominator):
    if len(numerator) != len(denominator) or not numerator:
        raise ValueError("paired series must have the same nonzero length")
    if any(value <= 0 or not math.isfinite(value) for value in [*numerator, *denominator]):
        raise ValueError("CPU samples must be finite and positive")
    ratios = [a / b for a, b in zip(numerator, denominator)]
    return {"ratio_of_medians": statistics.median(numerator) / statistics.median(denominator),
            "q1": quantile(ratios, .25), "median": statistics.median(ratios),
            "q3": quantile(ratios, .75), "ratios": ratios}


def control_passes(control):
    return all(abs(control[key] - 1) < .03 - 1e-12
               for key in ("ratio_of_medians", "median"))


def verdict(comparison, threshold, control):
    estimates = [comparison[key] for key in ("ratio_of_medians", "median")]
    passes = [value <= threshold if threshold == 1.10 else value < threshold for value in estimates]
    if (passes[0] != passes[1]
            or (any(abs(value - threshold) <= .05 + 1e-12 for value in estimates)
                and not control_passes(control))):
        return "within noise"
    return "goal met" if all(passes) else "behind"


def order_for_round(labels, index):
    offset = (index // 2) % len(labels)
    order = labels[offset:] + labels[:offset]
    return order if index % 2 == 0 else order[::-1]


def snapshot():
    return {"utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "local": time.strftime("%Y-%m-%dT%H:%M:%S%z"),
            "load_average": list(os.getloadavg())}


def sample(command, expected, timeout=180):
    """wait4 measures one child's CPU/RSS, independent of earlier children.

    Output goes to files so timeout handling cannot deadlock on a full pipe.
    CPU retains microsecond resolution; a separate GNU-time run measures RSS too.
    """
    with tempfile.TemporaryDirectory() as temporary:
        directory = Path(temporary)
        with (directory / "stdout").open("wb") as stdout, (directory / "stderr").open("wb") as stderr:
            start = time.perf_counter()
            process = subprocess.Popen(command, stdout=stdout, stderr=stderr)
            # Poll with wait4 WNOHANG: collect this child only, preserve full CPU precision.
            while True:
                pid, status, usage = os.wait4(process.pid, os.WNOHANG)
                if pid:
                    process.returncode = os.waitstatus_to_exitcode(status)
                    break
                if time.perf_counter() - start > timeout:
                    process.kill()
                    process.wait()
                    raise RuntimeError(f"{command[0]}: timeout after {timeout}s")
                time.sleep(.002)
        wall = time.perf_counter() - start
        if process.returncode:
            raise RuntimeError(f"{command[0]} exited {process.returncode}: "
                               + (directory / "stderr").read_text(errors="replace")[-1000:])
        if (directory / "stdout").read_bytes() != expected:
            raise RuntimeError(f"{command[0]}: measured output differs from Rust")
        return {"cpu": usage.ru_utime + usage.ru_stime, "user": usage.ru_utime,
                "system": usage.ru_stime, "wall": wall, "peak_rss_kib": usage.ru_maxrss}


def gnu_rss(command, expected, directory):
    output = directory / "rss.txt"
    result = subprocess.run(["/usr/bin/time", "-f", "%M", "-o", str(output), *command],
                            capture_output=True, timeout=180, check=True)
    if result.stdout != expected:
        raise RuntimeError(f"{command[0]}: RSS output differs from Rust")
    return int(output.read_text().strip())
