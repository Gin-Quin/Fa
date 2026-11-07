import copy
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import artifacts
import compare


class ReuseSafety(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        source = self.root / "benchmarks/binary-trees"
        source.mkdir(parents=True)
        (source / "main.fa").write_text("let n = 14 -- BENCH_N\nconsole.log(n)\n")
        (source / "main.goose").write_text("let N = 14; // BENCH_N\nfn main() { print(N); }\n")
        (source / "main.rs").write_text("fn main() {}\n")
        (self.root / "benchmarks/goose").mkdir()
        self.root_patch = patch.object(compare, "ROOT", self.root)
        self.root_patch.start()
        self.addCleanup(self.root_patch.stop)
        self.compilers = {
            side: {"path": f"/tools/{side}", "version": "1", "sha256": side}
            for side in ("fa", "goose", "rust")}
        self.compilers["goose"].update({
            "c_compiler": {"path": "/tools/clang-18", "version": "18", "sha256": "clang"},
            "stdlib": {"path": "/tools/stdlib", "files": {}}})
        self.directory = self.root / "saved"
        self.directory.mkdir()
        self.saved_plan = compare.plan("binary-trees", 14, self.compilers)
        # All three saved programs still agree at the old constant size. Output
        # equality alone must never authorize reuse at a newly requested size.
        self.output = b"14\n"
        for side, paths in compare.ARTIFACTS.items():
            manifest, binary, stdout, _ = paths
            compare.prepare_sources(self.directory, side, self.saved_plan)
            executable = self.directory / binary
            executable.parent.mkdir(parents=True, exist_ok=True)
            executable.write_text("#!/bin/sh\nprintf '14\\n'\n")
            executable.chmod(0o755)
            artifacts.save(self.directory, manifest, self.saved_plan["requests"][side], binary,
                           self.saved_plan["arguments"][side], stdout, self.output,
                           self.output if side != "rust" else None)

    def test_unchanged_artifacts_can_be_reused(self):
        commands, expected, manifests = compare.prepare(self.directory, self.saved_plan, reuse=True)
        self.assertEqual(expected, self.output)
        self.assertEqual(set(manifests), {"fa", "goose", "rust"})
        self.assertEqual(commands["fa"][-1], "14")

    def test_default_size_cannot_reuse_old_constant_binaries_and_output(self):
        current = compare.plan("binary-trees", compare.DEFAULTS["binary-trees"], self.compilers)
        with self.assertRaisesRegex(RuntimeError, "size"):
            compare.prepare(self.directory, current, reuse=True)

    def test_changed_sources_are_rejected_even_when_output_is_unchanged(self):
        (self.root / "benchmarks/binary-trees/main.fa").write_text(
            "let n = 14 -- BENCH_N\nconsole.log(n + 0)\n")
        current = compare.plan("binary-trees", 14, self.compilers)
        with self.assertRaisesRegex(RuntimeError, "source"):
            compare.prepare(self.directory, current, reuse=True)

    def test_shared_goose_module_changes_invalidate_reuse(self):
        (self.root / "benchmarks/goose/rng.goose").write_text("fn next() {}\n")
        current = compare.plan("binary-trees", 14, self.compilers)
        with self.assertRaisesRegex(RuntimeError, "source"):
            compare.prepare(self.directory, current, reuse=True)

    def test_compiler_flags_and_modes_are_part_of_the_contract(self):
        for key, value in (("compiler", {"path": "/tools/gcc-13", "version": "13"}),
                           ("flags", ["build", "-O0"]),
                           ("modes", {"asynchronous": True})):
            with self.subTest(field=key):
                current = copy.deepcopy(self.saved_plan)
                current["requests"]["goose"][key] = value
                with self.assertRaisesRegex(RuntimeError, key):
                    compare.prepare(self.directory, current, reuse=True)

    def test_changing_only_the_native_backend_hash_is_rejected(self):
        current = copy.deepcopy(self.saved_plan)
        current["requests"]["fa"]["compiler"]["native_backend"] = {"sha256": "new"}
        with self.assertRaisesRegex(RuntimeError, "compiler"):
            compare.prepare(self.directory, current, reuse=True)

    def test_tampered_binary_or_output_is_rejected(self):
        for path, message in (("fa/native/main", "executable hash"),
                              ("fa/stdout.txt", "saved output"),
                              ("expected.txt", "saved output"),
                              ("fa/main.fa", "saved source")):
            with self.subTest(path=path):
                artifact = self.directory / path
                original = artifact.read_bytes()
                artifact.write_bytes(original + b"changed")
                try:
                    with self.assertRaisesRegex(RuntimeError, message):
                        compare.prepare(self.directory, self.saved_plan, reuse=True)
                finally:
                    artifact.write_bytes(original)

    def test_legacy_builds_without_manifests_must_be_rebuilt(self):
        (self.directory / "goose/manifest.json").unlink()
        with self.assertRaisesRegex(RuntimeError, "missing or invalid manifest"):
            compare.prepare(self.directory, self.saved_plan, reuse=True)

    def test_rebuild_fa_does_not_hide_a_goose_size_mismatch(self):
        current = compare.plan("binary-trees", 17, self.compilers)
        with patch.object(compare, "build_side") as build:
            with self.assertRaisesRegex(RuntimeError, "size"):
                compare.prepare(self.directory, current, reuse=True, rebuild_fa=True)
            build.assert_not_called()

    def test_rebuild_fa_allows_a_new_fa_compiler_after_checking_goose(self):
        current = copy.deepcopy(self.saved_plan)
        current["requests"]["fa"]["compiler"]["sha256"] = "new-backend"
        with patch.object(compare, "build_side", return_value=({}, self.output)) as build:
            compare.prepare(self.directory, current, reuse=True, rebuild_fa=True)
            self.assertEqual(build.call_count, 1)
            self.assertEqual(build.call_args.args[1], "fa")

    def test_constant_and_runtime_goose_builds_are_distinct(self):
        current = compare.plan("binary-trees", 14, self.compilers, runtime_size=True)
        with self.assertRaisesRegex(RuntimeError, "modes"):
            compare.prepare(self.directory, current, reuse=True)


class ExecutableIdentity(unittest.TestCase):
    def test_symlink_invocation_keeps_its_tool_name(self):
        with tempfile.TemporaryDirectory() as temporary:
            directory = Path(temporary)
            proxy = directory / "proxy"
            proxy.write_text('#!/bin/sh\nbasename "$0"\n')
            proxy.chmod(0o755)
            compiler = directory / "rustc"
            compiler.symlink_to(proxy)
            identity = artifacts.executable_identity(compiler)
            self.assertEqual(identity["version"], "rustc")
            self.assertEqual(identity["path"], str(compiler))
            self.assertEqual(identity["real_path"], str(proxy))


class SuiteSelection(unittest.TestCase):
    def test_original_sixteen_and_own_ports_are_separate(self):
        self.assertEqual(set(compare.GOOSE_SUITE), {
            "binary-trees", "blur", "calc", "graph", "interp", "lru", "particles", "push",
            "records", "respond", "scene", "sexp", "strlist", "sum", "tree", "words"})
        self.assertEqual(len(compare.SECONDARY), 9)
        self.assertFalse(set(compare.GOOSE_SUITE) & set(compare.SECONDARY))
        self.assertEqual(set(compare.GOOSE_SUITE) | set(compare.SECONDARY), set(compare.DEFAULTS))


if __name__ == "__main__":
    unittest.main()
