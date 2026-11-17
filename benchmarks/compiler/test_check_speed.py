import unittest
from check_speed import category, comparison, distribution, markdown


class StatisticsTests(unittest.TestCase):
    def test_threshold_is_inclusive_and_noise_control_is_strict(self):
        a = [{"wall": 1.2}] * 11
        b = [{"wall": 1.0}] * 11
        control = {"ratio_of_medians": 1., "median": 1.}
        self.assertEqual(comparison(a, b, "wall", control)["verdict"], "met")
        control["median"] = 1.03
        self.assertEqual(comparison(a, b, "wall", control)["verdict"], "unresolved")
        self.assertEqual(comparison([{"wall": 2.}] * 11, b, "wall", control)["verdict"], "behind")

    def test_quartiles_retain_all_samples(self):
        self.assertEqual(distribution(list(range(11))), {"q1": 2.5, "median": 5, "q3": 7.5})

    def test_instruction_categories_do_not_overlap(self):
        self.assertEqual(category("fa_analyzer::check::comptime::evaluate"), "compile_time")
        self.assertEqual(category("fa_analyzer::check::moves::finish_moves"), "references_moves_relations")
        self.assertEqual(category("fa_analyzer::types::store::kind"), "types_checking")
        self.assertEqual(category("malloc"), "runtime_other")

    def test_render_is_derived_from_json(self):
        self.assertIn("wall-time goal", markdown({"series": []}))


if __name__ == "__main__":
    unittest.main()
