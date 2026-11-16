import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

import tables


class PublishedTables(unittest.TestCase):
    def test_strict_control_changes_only_the_published_verdict_and_survives_render(self):
        stats = dict(ratio_of_medians=.9, median=.91, q1=.89, q3=.92, verdict='goal met')
        cpu = dict(size=10, control=dict(ratio_of_medians=1.04, median=1, passes=False),
                   comparisons=dict(goose=stats, rust=stats))
        phase = dict(schema=2, mode='cpu', size_set='default', start=dict(utc='2026-11-16'),
                     benchmarks=dict(example=cpu))
        self.assertIn('goal met / goal met', tables.markdown(dict(series=[phase])))
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source, result, rendered = (root / name for name in ('phase.json', 'result.json', 'result.md'))
            source.write_text(json.dumps(phase))
            tables.render([source], result, rendered, require_control=True)
            first = rendered.read_text()
            self.assertIn('not resolved / not resolved', first)
            self.assertNotIn('goal met / goal met', first)
            self.assertEqual(json.loads(result.read_text())['series'], [phase])
            tables.render([result], result, rendered)
            self.assertEqual(rendered.read_text(), first)
        cpu['control'] = dict(ratio_of_medians=1, median=1, passes=True)
        output = tables.markdown(dict(series=[phase], require_control=True))
        self.assertIn('goal met / goal met', output)

    def test_generated_block_is_repeatable_and_preserves_all_other_content(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            readme = root / 'README.md'
            original = 'Before\n' + tables.BEGIN + '\nStale\n' + tables.END + '\nAfter\n'
            readme.write_text(original)
            phase = root / 'phase.json'
            phase.write_text(json.dumps({'schema': 2, 'mode': 'cpu', 'size_set': 'default',
                'start': {'utc': '2026-11-16'},
                'benchmarks': {'example': {'size': 10, 'error': 'build failed'}}}))
            destination = root / 'results/baseline.json'
            markdown = root / 'results/baseline.md'
            with patch.object(tables, 'README', readme):
                tables.render([phase], destination, markdown, True)
                first = readme.read_text()
                tables.render([phase], destination, markdown, True)
                self.assertEqual(readme.read_text(), first)
            self.assertTrue(first.startswith('Before\n' + tables.BEGIN))
            self.assertTrue(first.endswith(tables.END + '\nAfter\n'))
            self.assertNotIn('Stale', first)
            self.assertIn('(results/baseline.json)', first)
            self.assertIn('(baseline.json)', markdown.read_text())
            self.assertEqual(json.loads(destination.read_text())['series'][0]['benchmarks'],
                             json.loads(phase.read_text())['benchmarks'])

    def test_missing_markers_fail_before_writing_results(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            readme = root / 'README.md'
            readme.write_text('No markers')
            destination = root / 'result.json'
            with patch.object(tables, 'README', readme):
                with self.assertRaisesRegex(ValueError, 'markers'):
                    tables.render([], destination, None, True)
            self.assertFalse(destination.exists())


if __name__ == '__main__':
    unittest.main()
