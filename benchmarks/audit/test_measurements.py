import copy
import sys
import unittest

from measurements import control_passes, order_for_round, paired, quantile, sample, verdict
import tables


class Statistics(unittest.TestCase):
    def test_type_seven_quartiles_odd_even_singleton(self):
        self.assertEqual([quantile([4, 1, 3, 2], p) for p in (.25, .5, .75)], [1.75, 2.5, 3.25])
        self.assertEqual(quantile([3, 1, 2], .25), 1.5)
        self.assertEqual(quantile([7], .75), 7)
        with self.assertRaises(ValueError):
            quantile([], .5)

    def test_pair_before_summarizing_and_preserve_outliers(self):
        result = paired([2, 20, 100], [1, 10, 1000])
        self.assertEqual(result['ratios'], [2, 2, .1])
        self.assertEqual(result['median'], 2)
        self.assertEqual(result['ratio_of_medians'], 2)
        self.assertAlmostEqual(result['q1'], 1.05)
        self.assertEqual(result['q3'], 2)
        different = paired([1, 10, 20], [1, 100, 2])
        self.assertEqual(different['ratio_of_medians'], 5)
        self.assertEqual(different['median'], 1)

    def test_reject_mismatched_zero_negative_nonfinite_samples(self):
        for numerator, denominator in [([], []), ([1], [1, 2]), ([0], [1]),
                                       ([1], [-1]), ([float('inf')], [1])]:
            with self.assertRaises(ValueError):
                paired(numerator, denominator)

    def test_control_needs_both_statistics_strictly_within_three_percent(self):
        for a, b, expected in [(1.02, .98, True), (1, 1.03, False), (.97, 1, False),
                               (1.04, 1, False), (1, .96, False)]:
            self.assertEqual(control_passes({'ratio_of_medians': a, 'median': b}), expected)

    def test_cpu_gate_needs_control_at_both_margin_boundaries(self):
        bad = {'ratio_of_medians': 1.04, 'median': 1}
        good = {'ratio_of_medians': 1, 'median': 1}
        for ratio in (1.05, 1.15):
            stats = {'ratio_of_medians': ratio, 'median': ratio}
            self.assertEqual(verdict(stats, 1.10, bad), 'within noise')
        self.assertEqual(verdict({'ratio_of_medians': 1.08, 'median': 1.09}, 1.1, good), 'goal met')
        self.assertEqual(verdict({'ratio_of_medians': 1.12, 'median': 1.12}, 1.1, good), 'behind')
        self.assertEqual(verdict({'ratio_of_medians': 1.08, 'median': 1.12}, 1.1, good), 'within noise')
        self.assertEqual(verdict({'ratio_of_medians': .9, 'median': .9}, 1.1, bad), 'goal met')
        self.assertEqual(verdict({'ratio_of_medians': 1.1, 'median': 1.1}, 1.1, good), 'goal met')
        self.assertEqual(verdict({'ratio_of_medians': 1., 'median': 1.}, 1., good), 'behind')

    def test_balanced_interleaving_reverses_and_rotates(self):
        labels = ['fa', 'goose', 'rust', 'fa_control']
        orders = [order_for_round(labels, i) for i in range(8)]
        for i in range(0, 8, 2):
            self.assertEqual(orders[i], orders[i + 1][::-1])
        for label in labels:
            self.assertEqual(sorted(order.index(label) for order in orders), [0, 0, 1, 1, 2, 2, 3, 3])

    def test_every_sample_checks_the_output_and_exit(self):
        result = sample([sys.executable, '-c', 'print(42)'], b'42\n')
        self.assertGreater(result['cpu'], 0)
        self.assertGreater(result['peak_rss_kib'], 0)
        with self.assertRaisesRegex(RuntimeError, 'differs from Rust'):
            sample([sys.executable, '-c', 'print(41)'], b'42\n')
        with self.assertRaisesRegex(RuntimeError, 'exited 2'):
            sample([sys.executable, '-c', 'raise SystemExit(2)'], b'')
        with self.assertRaisesRegex(RuntimeError, 'timeout'):
            sample([sys.executable, '-c', 'import time; time.sleep(1)'], b'', timeout=.01)

    def test_secondary_memory_ceiling_is_an_explicit_failure(self):
        with self.assertRaisesRegex(RuntimeError, "memory limit 1 KiB exceeded"):
            sample([sys.executable, '-c', 'import time; time.sleep(1)'], b'', max_rss_kib=1)

    def test_markdown_renders_errors_and_separate_override_rows(self):
        phase = {'mode': 'cpu', 'size_set': 'default', 'start': {'utc': '2026-11-16'},
                 'benchmarks': {'example': {'size': 10, 'error': 'build failed'}}}
        other = copy.deepcopy(phase)
        other['size_set'] = 'override'
        other['benchmarks']['example']['size'] = 16
        output = tables.markdown({'series': [phase, other]})
        self.assertIn('`example`', output)
        self.assertIn('`example@16`', output)
        self.assertIn('build failed', output)


if __name__ == '__main__':
    unittest.main()
