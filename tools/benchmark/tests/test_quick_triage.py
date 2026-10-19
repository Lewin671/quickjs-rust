"""Triage must stay selection-only and conservative, with no acceptance artifacts."""
import json
import os
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from tools.benchmark import quick_triage as triage
from tools.benchmark.layout_identity import ROOT
from tools.benchmark.layout_policy import load_policy


class TriageTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy = load_policy(ROOT / triage.POLICY, triage.POLICY_SHA, live=False)

    def summary(self, wall, instructions, control_wall=1.0):
        def row(name, w, i):
            return {'id': name, 'ratios': {'wall_ns': [w]*5, 'instructions': [i]*5}}
        target, control = 'development/papaparse/csv', 'sentinel/string_key_map_churn'
        return triage.summarize([{'cases': [row(target, wall, instructions),
                                           row(control, control_wall, 1.0)]}], [target], self.policy)

    def test_fixed_inventory_includes_all_sentinels_and_no_held_out(self):
        ids = triage.inventory(['development/papaparse/csv'], self.policy)
        self.assertEqual(len(ids), 20)
        self.assertEqual({n for n in ids if n.startswith('sentinel/')},
                         {n for n in self.policy['cases'] if n.startswith('sentinel/')})
        for targets in ([], ['held-out/x/y'], ['broad'], ['file:/tmp/probe'],
                        ['development/papaparse/csv']*2):
            with self.assertRaises(ValueError):
                triage.inventory(targets, self.policy)
        self.assertEqual(triage.inventory([triage.CONTROLS[0]], self.policy), list(triage.CONTROLS))

    def test_big_gain_cannot_hide_obvious_or_uncertain_control_cost(self):
        self.assertEqual(self.summary(.2, .12)['verdict'], 'likely win')
        self.assertEqual(self.summary(.2, .12, 2.0)['verdict'], 'likely loss')
        self.assertEqual(self.summary(1.0, 1.0)['verdict'], 'unclear')
        self.assertEqual(self.summary(.2, 1.1)['verdict'], 'likely loss')
        # A single high sample is uncertainty, not proof of neutrality or loss.
        packets = [{'cases': [{'id':'development/papaparse/csv',
            'ratios': {'wall_ns':[.2]*5, 'instructions':[.12]*5}},
            {'id':'sentinel/string_key_map_churn',
             'ratios': {'wall_ns':[1,1,1,1,2], 'instructions':[1]*5}}]}]
        self.assertEqual(triage.summarize(packets, ['development/papaparse/csv'], self.policy)['verdict'], 'unclear')

    def test_request_is_sealed_before_build_failure_and_failure_is_retained(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            out = root / 'out'
            out.mkdir()
            (root / 'status').write_text('running 2026-10-19T00:00:00Z')
            target = out / 'triage'
            args = SimpleNamespace(base='base', candidate='cand',
                                   target=['development/papaparse/csv'], output=target)
            def fail(*_):
                request = json.loads((target / 'request.json').read_text())
                self.assertFalse(request['decision_evidence'])
                self.assertEqual(request['attempts_consumed'], 0)
                self.assertEqual(request['measurement_schedule']['workers'], 4)
                raise ValueError('build failed')
            with patch.dict(os.environ, BENCH_OUT=str(out)), \
                    patch.object(triage, 'load_policy', return_value=self.policy), \
                    patch.object(triage, 'command', return_value='f'*40), \
                    patch.object(triage, 'quieting_preflight', return_value={}), \
                    patch.object(triage.screen, 'wait_for_builds', return_value=[]), \
                    patch.object(triage, 'build', side_effect=fail):
                self.assertEqual(triage.run(args), 2)
                with self.assertRaises(FileExistsError):
                    triage.run(args)
            result = json.loads((target / 'triage.json').read_text())
            self.assertEqual(result['verdict'], 'unclear')
            self.assertFalse(result['decision_evidence'])
            self.assertIn('build failed', (target / 'triage.md').read_text())
            self.assertFalse((target / 'panel.json').exists())
            self.assertFalse((target / 'verdict.json').exists())

    def test_refuses_non_timing_queue_job(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'status').write_text('running now no-timing')
            with patch.dict(os.environ, BENCH_OUT=str(root / 'out')):
                with self.assertRaisesRegex(ValueError, 'timing job required'):
                    triage.run(None)


if __name__ == '__main__':
    unittest.main()
