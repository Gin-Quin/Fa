import unittest

from compare import AUDIT, DEFAULTS, ROOT, VARIANTS, fa_source, goose_source_at_size, plan


class RuntimeSizes(unittest.TestCase):
    def test_constant_mode_keeps_the_existing_protocol(self):
        source = "import std;\nlet N = 100; // BENCH_N\nfn main() { print(N); }\n"
        self.assertEqual(goose_source_at_size(source, 7), source.replace("100", "7"))

    def test_runtime_mode_keeps_checks_and_initializes_dependencies_in_order(self):
        source = ("let N = 100; // BENCH_N\nlet CAP = N / 8;\nlet KEYS = CAP * 2;\n"
                  "let OTHER = 3;\ncheckfree fn probe() {}\nfn main() { print(KEYS); }\n")
        result = goose_source_at_size(source, 7, runtime=True)
        self.assertIn("import os;", result)
        self.assertIn("import std;", result)
        self.assertIn("var N: i64 = 0;", result)
        self.assertIn("var CAP: i64 = 0;", result)
        self.assertIn("var KEYS: i64 = 0;", result)
        self.assertIn("let OTHER = 3;", result)
        self.assertIn("checkfree fn probe() {}", result)
        self.assertLess(result.index("N = audit_size;"), result.index("CAP = N / 8;"))
        self.assertLess(result.index("CAP = N / 8;"), result.index("KEYS = CAP * 2;"))
        self.assertNotIn("100", result)
        self.assertEqual(result, goose_source_at_size(source, 23, runtime=True))

    def test_programs_already_reading_argv_are_unchanged(self):
        source = "import os;\nfn main() { print(args().len); }\n"
        self.assertEqual(goose_source_at_size(source, 7, runtime=True), source)


class Variants(unittest.TestCase):
    def test_structural_variants_select_their_matching_rust_without_extra_entries(self):
        compilers = {'fa': {'path': 'fa'}, 'rust': {'path': 'rustc'},
                     'goose': {'path': 'goose', 'c_compiler': {'path': 'clang'},
                               'stdlib': {'path': 'stdlib'}}}
        for base, variant_name, entry in (('lru', 'lru-open', 'open.rs'),
                                         ('words', 'words-wide', 'wide.rs')):
            with self.subTest(variant=variant_name):
                main, variant = (plan(name, 64, compilers) for name in (base, variant_name))
                directory = ROOT / 'benchmarks' / base
                self.assertEqual(main['inputs']['rust'], {'main.rs': (directory / 'main.rs').read_bytes()})
                self.assertEqual(variant['inputs']['rust'], {'main.rs': (directory / entry).read_bytes()})
                self.assertNotEqual(main['requests']['rust']['source_sha256'],
                                    variant['requests']['rust']['source_sha256'])
                self.assertEqual(set(main['inputs']['fa']), {'main.fa'})
                self.assertEqual(set(variant['inputs']['fa']), {'main.fa'})
                self.assertNotEqual(main['inputs']['fa']['main.fa'], variant['inputs']['fa']['main.fa'])

    def test_variants_use_the_programs_and_sizes_of_their_benchmark(self):
        for name, (directory, source) in VARIANTS.items():
            self.assertEqual(fa_source(name), (ROOT / "benchmarks" / directory, source))
            self.assertTrue((ROOT / "benchmarks" / directory / source).exists())
            self.assertEqual(DEFAULTS[name], DEFAULTS[directory])
            self.assertEqual(AUDIT[name], AUDIT[directory])
        self.assertEqual(fa_source("sum"), (ROOT / "benchmarks" / "sum", "main.fa"))


if __name__ == "__main__":
    unittest.main()
