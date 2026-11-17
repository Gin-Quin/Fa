#!/usr/bin/env python3
"""Behavior tests for the build wrappers, independent of Cargo and the network."""

import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

SCRIPTS = Path(__file__).resolve().parent


class DevTools(unittest.TestCase):
    def test_only_registry_compilation_uses_the_shared_cache(self):
        with tempfile.TemporaryDirectory(prefix="fa dev tools ") as scratch:
            root = Path(scratch)
            compiler = root / "compiler"
            cache = root / "cache"
            log = root / "calls.jsonl"
            compiler.write_text(
                "#!/usr/bin/env python3\n"
                "import json,os,sys\n"
                "with open(os.environ['FA_WRAPPER_LOG'], 'a') as f:\n"
                " f.write(json.dumps(['compiler', *sys.argv[1:]]) + '\\n')\n"
                "sys.exit(7)\n"
            )
            cache.write_text(
                "#!/usr/bin/env python3\n"
                "import json,os,sys\n"
                "with open(os.environ['FA_WRAPPER_LOG'], 'a') as f:\n"
                " f.write(json.dumps(['cache']) + '\\n')\n"
                "os.execv(sys.argv[1], sys.argv[1:])\n"
            )
            compiler.chmod(0o755)
            cache.chmod(0o755)
            for manifest, enabled, expected in [
                ("/user/.cargo/registry/src/index/package", "1", True),
                ("/work/Fa/packages/parser", "1", False),
                ("/user/.cargo/registry/src/index/package", "0", False),
                ("", "1", False),
            ]:
                with self.subTest(manifest=manifest, enabled=enabled):
                    log.unlink(missing_ok=True)
                    result = subprocess.run(
                        [str(SCRIPTS / "rustc-cache.sh"), str(compiler), "argument with spaces", "$literal"],
                        env={**os.environ, "CARGO_MANIFEST_DIR": manifest,
                             "FA_RUST_CACHE": enabled, "FA_SCCACHE": str(cache),
                             "FA_WRAPPER_LOG": str(log)},
                        check=False,
                    )
                    calls = [json.loads(line) for line in log.read_text().splitlines()]
                    self.assertEqual(result.returncode, 7)
                    self.assertEqual(calls[-1], ["compiler", "argument with spaces", "$literal"])
                    self.assertEqual(calls[0] == ["cache"], expected)

    def test_timing_runner_preserves_failure_and_records_it(self):
        with tempfile.TemporaryDirectory() as scratch:
            log = Path(scratch) / "timings.jsonl"
            result = subprocess.run(
                [str(SCRIPTS / "test-timings.py"), "python3", "-c", "raise SystemExit(9)"],
                env={**os.environ, "FA_SUITE_TIMINGS": str(log)},
                capture_output=True, check=False,
            )
            self.assertEqual(result.returncode, 9)
            row = json.loads(log.read_text())
            self.assertEqual(row["exit"], 9)
            self.assertEqual(row["binary"], "python3")
            self.assertGreater(row["seconds"], 0)


if __name__ == "__main__":
    unittest.main()
