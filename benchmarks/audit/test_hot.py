from pathlib import Path
import tempfile
import unittest

from hot import parse_callgrind


class ProfileEvents(unittest.TestCase):
    def test_compressed_positions_and_sparse_cache_events_exclude_call_edges(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'callgrind.out'
            path.write_text('positions: instr line\nevents: Ir Dr Dw D1mr\n'
                            'ob=(1) /tmp/main\nfn=(1) main\n'
                            '0x1000 0 10 2 1 3\n+4 0 20 2\n'
                            'cfn=(2) helper\ncalls=2 0x2000 0\n* 0 100 4 2 5\n'
                            '+4 0 5 1 1 1\n')
            costs, calls = parse_callgrind(path, '/tmp/main', 'Ir')
            self.assertEqual(dict(costs), {0x1000: 10, 0x1004: 20, 0x1008: 5})
            self.assertEqual(calls, [(0x1004, 'main', 'helper', 2, 100)])
            costs, calls = parse_callgrind(path, '/tmp/main', 'D1mr')
            self.assertEqual(dict(costs), {0x1000: 3, 0x1004: 0, 0x1008: 1})
            self.assertEqual(calls[0][-1], 5)

    def test_anonymous_pie_object_is_recovered_only_with_strong_address_overlap(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'callgrind.out'
            path.write_text('positions: instr line\nevents: Ir Dr Dw D1mr\n'
                            'ob=(4) ???\nfn=(1) 0x109000\n'
                            '0x109000 0 100 20 1 8\n+4 0 200 30 1 9\n'
                            'cfn=(2) 0x109100\ncalls=2 0x109100 0\n'
                            '* 0 10000 4 2 50\n'
                            'fn=(3) trampoline\n0x483d000 0 1 1 0 1\n')
            addresses = {0x1000, 0x1004, 0x1100}
            costs, calls = parse_callgrind(path, '/tmp/main', 'D1mr', addresses)
            self.assertEqual(dict(costs), {0x109000: 8, 0x109004: 9})
            self.assertEqual(len(calls), 1)
            costs, calls = parse_callgrind(path, '/tmp/main', 'Ir', {0x1000})
            self.assertEqual(dict(costs), {})
            self.assertEqual(calls, [])


if __name__ == '__main__':
    unittest.main()
