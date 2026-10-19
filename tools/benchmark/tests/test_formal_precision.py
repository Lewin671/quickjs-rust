"""Coverage, prospective binding and formal threshold integration."""
import json
import math
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.benchmark import formal_precision as fp
from tools.benchmark.layout_policy import load_policy
from tools.benchmark.layout_identity import digest, ROOT
from tools.benchmark.tests import test_layout_formal as fixtures


class PrecisionTests(unittest.TestCase):
    def test_exact_two_look_coverage_and_order_statistics(self):
        for n in (30, 60, 240):
            low, high = fp.rank_bounds(n)
            tail = sum(math.comb(n, j) for j in range(low + 1)) / 2**n
            self.assertLessEqual(tail, .0125)
            self.assertGreater(tail + math.comb(n, low + 1) / 2**n, .0125)
            self.assertEqual(high, n - 1 - low)
        self.assertEqual(fp.next_count(30, 30), 240)
        with self.assertRaises(ValueError):
            fp.next_count(30, 44)
        with self.assertRaises(ValueError):
            fp.analyze_case([1.] * 44)
        with self.assertRaises(ValueError):
            fp.analyze_case([math.nan] * 30)

    def test_width_and_direction_are_separate(self):
        s = fp.analyze_case([.8] * 120 + [1.] * 120)
        self.assertFalse(s['precise'])
        self.assertTrue(fp.ready(s, 1.05))
        self.assertTrue(fp.ready(s, .7))
        self.assertFalse(fp.ready(s, .9))

    def test_version_is_explicit_and_host_policy_bands_unchanged(self):
        folder = ROOT / 'benchmarks/policies'
        old = json.loads((folder/'imac-x86_64-rust1.95-fat16-layout-v5.json').read_text())
        path = folder/'imac-x86_64-rust1.95-fat16-layout-v6.json'
        new = load_policy(path, digest(path), live=False)
        self.assertFalse(fp.enabled(old))
        self.assertTrue(fp.enabled(new))
        for key in ('cases', 'held_out', 'lanes', 'precision', 'measurement_schedule', 'qualification'):
            self.assertEqual(old[key], new[key])
        with self.assertRaisesRegex(ValueError, 'unsupported formal precision'):
            fp.enabled({'formal_precision_rule': 'unknown'})

    def test_continuation_cannot_transplant_rule(self):
        import tempfile
        from tools.benchmark.layout_formal_continuation import replay
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp)/'continuation.json'
            path.write_text(json.dumps({'artifact_type': 'quickjs-formal-case-continuation',
                                        'formal_precision_rule': fp.RULE}))
            with self.assertRaisesRegex(ValueError, 'precision rule mismatch'):
                replay(path, Path(tmp), 'case')

    def test_v2_raw_replay_keeps_all_240_blocks_and_rejects_an_extra_look(self):
        import dataclasses
        import hashlib
        import tempfile
        from tools.benchmark.tests.test_formal_continuation import FormalContinuationTests
        from tools.benchmark import layout_formal_continuation as continuation
        from tools.benchmark import formal_case_context as context
        from tools.benchmark.pair_disturbance import PROTOCOL
        fixture = FormalContinuationTests()
        ctx = fixture.context()
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            raw, path = root/'samples.jsonl', root/'continuation.json'
            records = [{'block': b, 'order': o, 'role': r, 'argv': ctx['templates'][r]['argv'],
                        'result': dataclasses.asdict(dataclasses.replace(fixture.result(i),
                            stdout='__QJS_EXTERNAL_OK__\n' if r == 'quickjs-ng' else fixture.result(i).stdout))}
                       for i, (b, o, r) in enumerate(context.schedule(ctx, 240)[90:])]
            raw.write_text(''.join(json.dumps(r)+'\n' for r in records))
            value = {'artifact_type': 'quickjs-formal-case-continuation', 'schema_version': 1,
                     'formal_precision_rule': fp.RULE, 'comparison': 'candidate-base',
                     'source_hashes': {}, 'case_id': 'external/s/c', 'bundle': str(root),
                     'host_node': 'host', 'host': {}, 'previous': None, 'previous_blocks': 30,
                     'required_blocks': 240, 'raw_file': raw.name, 'raw_sha256': digest(raw),
                     'ratios': [1.]*240, 'load': {'protocol': PROTOCOL, 'text': '',
                         'sha256': hashlib.sha256(b'').hexdigest(), 'anchor': 1, 'sealed_at': 20}}
            with patch.object(continuation, 'context', return_value=ctx), \
                 patch.object(continuation, 'source_hashes', return_value={}):
                path.write_text(json.dumps(value))
                ratios, _ = continuation.replay(path, root, 'external/s/c', pilot=[1.]*30,
                    comparison='candidate-base', precision_rule=fp.RULE)
                self.assertEqual(ratios, [1.]*240)
                value['required_blocks'] = 120
                path.write_text(json.dumps(value))
                with self.assertRaisesRegex(ValueError, 'power/count'):
                    continuation.replay(path, root, 'external/s/c', pilot=[1.]*30,
                        comparison='candidate-base', precision_rule=fp.RULE)


class FormalIntegrationTests(unittest.TestCase):
    setUp = fixtures.FormalTests.setUp
    analyze = fixtures.FormalTests.analyze
    decide = fixtures.FormalTests.decide
    def test_v2_wide_favorable_control_passes_but_crossing_stays_incomplete(self):
        self.policy['formal_precision_rule'] = fp.RULE
        name = 'broad/method_call'
        self.values[name] = [.8] * 15 + [1.] * 15
        self.assertTrue(self.decide()[0]['passed'])
        self.values[name] = [1.] * 15 + [1.1] * 15
        self.panels['panels'] *= 6
        self.assertEqual(self.decide()[0]['label'], 'incomplete')

    def test_v2_wide_target_failure_is_rejected(self):
        self.policy['formal_precision_rule'] = fp.RULE
        self.values[self.target] = [1.] * 15 + [1.2] * 15
        self.assertTrue(any('target payoff' in s for s in self.decide()[2]))

    def test_v2_cannot_silently_accept_old_predicted_count(self):
        self.policy['formal_precision_rule'] = fp.RULE
        with patch('tools.benchmark.layout_formal.continued_cases',
                   return_value=({self.target: [.9] * 44}, {})):
            with self.assertRaises(ValueError):
                self.decide()


if __name__ == '__main__':
    unittest.main()
