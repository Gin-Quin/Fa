import unittest
import sys
import json
import tempfile
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch
import check_speed
from resources import sample
from check_speed import category, comparison, distribution, markdown


class StatisticsTests(unittest.TestCase):
    def test_threshold_is_inclusive_and_noise_control_is_strict(self):
        a = [{"wall": 1.2}] * 11
        b = [{"wall": 1.0}] * 11
        control = {"ratio_of_medians": 1., "median": 1.}
        self.assertEqual(comparison(a, b, "wall", control)["verdict"], "met")
        control["median"] = 1.03
        self.assertEqual(comparison(a, b, "wall", control)["verdict"], "unresolved")
        for margin in (1.15, 1.25):
            self.assertEqual(comparison([{"wall": margin}] * 11, b, "wall", control)["verdict"], "unresolved")
        self.assertEqual(comparison([{"wall": 2.}] * 11, b, "wall", control)["verdict"], "behind")

    def test_quartiles_retain_all_samples(self):
        self.assertEqual(distribution(list(range(11))), {"q1": 2.5, "median": 5, "q3": 7.5})

    def test_instruction_categories_do_not_overlap(self):
        self.assertEqual(category("fa_analyzer::check::comptime::evaluate"), "compile_time")
        self.assertEqual(category("fa_analyzer::check::moves::finish_moves"), "references_moves_relations")
        self.assertEqual(category("fa_analyzer::types::store::kind"), "types_checking")
        self.assertEqual(category("malloc"), "runtime_other")
        self.assertEqual(category("fa_interpreter::memory::CountingAllocator::alloc"), "runtime_other")

    def test_optimized_walker_names_use_their_source_phase(self):
        self.assertEqual(category("???:<fa_analyzer::check::Walker>::record_value_origins"), "references_moves_relations")
        self.assertEqual(category("???:<fa_analyzer::check::Walker>::publish_interface"), "scanning_resolving")
        self.assertEqual(category("???:<fa_analyzer::check::Walker>::finish_tasks"), "flow_effects")
        self.assertEqual(category("fa_analyzer::syntax::view"), "syntax_decode")

    def test_child_rss_excludes_parent_high_water(self):
        allocation = bytearray(64 << 20)
        allocation[0] = 1
        del allocation
        value = sample([sys.executable, "-c", "pass"], b"")
        self.assertLess(value["peak_rss_kib"], 32 * 1024)
        self.assertGreater(value["launcher_peak_rss_kib"], 64 * 1024)

    def test_limit_applies_to_tool_memory(self):
        with self.assertRaisesRegex(RuntimeError, "memory|KiB"):
            sample([sys.executable, "-c", "import time; value=bytearray(32<<20); time.sleep(.1)"],
                   b"", max_rss_kib=24 * 1024)

    def test_expected_negative_corpus_exit_retains_diagnostics(self):
        value = sample([sys.executable, "-c", "print(42); raise SystemExit(1)"], None,
                       allowed_exits=(0, 1), check=False)
        self.assertEqual(value["exit"], 1)
        self.assertIsNone(value["failure"])
        self.assertEqual(value["diagnostic_bytes"], 3)

    def test_resume_keeps_completed_round_and_does_not_repeat_completed_series(self):
        value = {"wall": .02, "user": .01, "system": .001, "cpu": .011, "peak_rss_kib": 10000}
        with tempfile.TemporaryDirectory() as directory:
            args = SimpleNamespace(fa=Path("fa"), bun=Path("bun"), modes=["single"],
                                   runs=2, report=Path(directory)/"result.json")
            data = {"fixtures": [{"files": 1, "faInput": "input.fa", "tsInput": "ts",
                                  "fa": {"files": 1, "lines": 500, "bytes": 10000},
                                  "ts": {"files": 1, "lines": 500, "bytes": 10000}}]}
            calls = 0
            def interrupted(*_args, **_kwargs):
                nonlocal calls
                calls += 1
                if calls == 6:
                    raise RuntimeError("interrupted after first checkpoint")
                return value.copy()
            with patch.object(check_speed, "sample", side_effect=interrupted):
                with self.assertRaisesRegex(RuntimeError, "interrupted"):
                    check_speed.measure(args, data)
            saved = json.loads(args.report.read_text())
            self.assertEqual(len(saved["active_series"]["orders"]), 1)
            with patch.object(check_speed, "sample", return_value=value.copy()) as resumed:
                check_speed.measure(args, saved)
                self.assertEqual(resumed.call_count, 5)  # Two warmups and one round.
            with patch.object(check_speed, "sample") as completed:
                check_speed.measure(args, saved)
                completed.assert_not_called()
            self.assertEqual(len(saved["series"]), 1)
            self.assertEqual(len(saved["series"][0]["samples"]["fa"]), 2)

    def test_callgrind_parser_keeps_totals_outside_function_sums(self):
        total, functions = check_speed.parse_annotation("100 (100.0%) PROGRAM TOTALS\n80 (80.0%) ???:fa_analyzer::types::kind\n20 (20.0%) ???:malloc\n")
        self.assertEqual(total, 100)
        self.assertEqual(sum(row["instructions"] for row in functions), total)
        self.assertEqual(functions[0]["category"], "types_checking")

    def test_render_is_derived_from_json(self):
        self.assertIn("wall-time goal", markdown({"series": []}))


if __name__ == "__main__":
    unittest.main()
