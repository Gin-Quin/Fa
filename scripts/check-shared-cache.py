#!/usr/bin/env python3
"""Compare two concurrent, private dependency builds with/without shared sccache.

Run with no other Cargo command active. This is the explicit concurrency probe;
ordinary development should use one Cargo command and CARGO_BUILD_JOBS=2.
Outputs and scratch projects are retained in the required --output directory.
"""
import argparse
import concurrent.futures
import json
import os
from pathlib import Path
import subprocess
import time
import tomllib


def toml(value):
    if isinstance(value, bool):
        return str(value).lower()
    if isinstance(value, dict):
        return "{ " + ", ".join(f"{json.dumps(k)} = {toml(v)}" for k, v in value.items()) + " }"
    if isinstance(value, list):
        return "[" + ", ".join(toml(v) for v in value) + "]"
    return json.dumps(value)


def dependencies(root):
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    result = {}
    for member in manifest["workspace"]["members"]:
        package = tomllib.loads((root / member / "Cargo.toml").read_text())
        for section in ["dependencies", "dev-dependencies", "build-dependencies"]:
            for name, specification in package.get(section, {}).items():
                if isinstance(specification, dict) and "path" in specification:
                    continue
                spec = {"version": specification} if isinstance(specification, str) else specification.copy()
                previous = result.get(name)
                if previous is not None:
                    if spec.get("version") != previous.get("version"):
                        raise ValueError(f"different versions of {name}; reconcile the probe graph")
                    spec["features"] = sorted(set(spec.get("features", []) + previous.get("features", [])))
                    spec["default-features"] = spec.get("default-features", True) or previous.get("default-features", True)
                result[name] = spec
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, required=True)
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    specs = dependencies(root)
    manifest = '[package]\nname = "fa-cache-proof"\nversion = "0.0.0"\nedition = "2024"\n[workspace]\n[profile.dev]\ndebug = "line-tables-only"\n[dependencies]\n'
    manifest += "\n".join(f"{json.dumps(name)} = {toml(spec)}" for name, spec in sorted(specs.items()))
    environment = {**os.environ, "CARGO_BUILD_JOBS": "1", "RUSTC_WRAPPER": str(root / "scripts/rustc-cache.sh")}
    cache = environment.get("FA_SCCACHE", str(Path.home() / ".local/bin/sccache"))
    records = []

    def build(name, affinity, enabled):
        directory = output / name
        directory.mkdir()
        (directory / "src").mkdir()
        (directory / "src/lib.rs").write_text("pub fn cache_probe() -> u32 { 42 }\n")
        (directory / "Cargo.toml").write_text(manifest)
        (directory / "Cargo.lock").write_bytes((root / "Cargo.lock").read_bytes())
        started = time.monotonic()
        command = ["taskset", "-c", affinity, "cargo", "build", "--offline", "--manifest-path", str(directory / "Cargo.toml")]
        with (output / f"{name}.log").open("w") as log:
            result = subprocess.run(command, env={**environment, "FA_RUST_CACHE": str(int(enabled)), "CARGO_TARGET_DIR": str(directory / "target")}, stdout=log, stderr=log, check=False)
        record = {"name": name, "start": started, "seconds": time.monotonic() - started, "exit": result.returncode}
        if result.returncode:
            raise RuntimeError(f"{name} failed: see {output / (name + '.log')}")
        if "file lock on build directory" in (output / f"{name}.log").read_text():
            raise RuntimeError("independent target directories must not serialize on their build lock")
        print(json.dumps(record), flush=True)
        return record

    def pair(label, enabled):
        with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
            a = executor.submit(build, label + "-a", "0-1", enabled)
            b = executor.submit(build, label + "-b", "2-3", enabled)
            results = [a.result(), b.result()]
        assert max(r["start"] for r in results) < min(r["start"] + r["seconds"] for r in results), "builds must overlap"
        records.extend(results)

    pair("uncached", False)
    records.append(build("seed", "0-4", True))
    before = subprocess.check_output([cache, "--show-stats", "--stats-format=json"], env=environment)
    (output / "stats-before.json").write_bytes(before)
    pair("cached", True)
    after = subprocess.check_output([cache, "--show-stats", "--stats-format=json"], env=environment)
    (output / "stats-after.json").write_bytes(after)
    (output / "measurements.json").write_text(json.dumps(records, indent=2) + "\n")


if __name__ == "__main__":
    main()
