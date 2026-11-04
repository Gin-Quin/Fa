import unittest

from compare import goose_source_at_size


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


if __name__ == "__main__":
    unittest.main()
