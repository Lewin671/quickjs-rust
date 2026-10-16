"""Formal/held-out promotion gates remain independent of diagnostic labels."""
import copy
import statistics
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.benchmark.layout_formal import evaluate_formal
from tools.benchmark.tests.test_layout_rule import policy, packet, change


class FormalTests(unittest.TestCase):
    def setUp(self):
        self.policy = policy()
        self.policy['host_node'] = 'test-host'
        self.data = packet(self.policy)
        self.target = 'broad/top_level_function_call'
        self.data = change(self.data, self.target, wall=.9)
        self.values = {c['id']: [.9 if c['id'] == self.target else 1.]*30 for c in self.data['cases']}
        self.panels = {'pairs': 5, 'panels': [self.data]}
        self.held = {'pairs': 30, 'panels': [{'pairs': 30, 'cases': [{'id': 'held-out/lib/program',
                       'ratios': {'wall_ns': [1.]*30, 'instructions': [1.]*30}}]}]}
        self.unit = {'layout_gate': {'policy_path': 'test', 'policy_sha256': 'a'*64,
                     'panel_required': False, 'mechanism_class': 'cpu-work', 'memory_wall_targets': [],
                     'panel_pairs': 5, 'held_out_pairs': 30, 'formal_blocks': 30},
                     'fast_gate': {'target_ids': [self.target], 'target_max_candidate_over_base': .97}}

    def analyze(self, values):
        import math
        ratio = math.exp(statistics.fmean(math.log(statistics.median(v)) for v in values.values()))
        return {'ratio': ratio, 'confidence_interval': {'lower': ratio*.999, 'upper': ratio*1.001},
                'precise': True}

    def decide(self, held=True, analyzer=None):
        with patch('tools.benchmark.layout_formal.load_policy', return_value=self.policy), \
             patch('tools.benchmark.layout_formal.replay', side_effect=lambda p, *a, **k:
                   self.held if k.get('held') else self.panels), \
             patch('tools.benchmark.layout_formal_raw.extract', return_value=self.values), \
             patch('tools.benchmark.layout_formal.analyze', side_effect=analyzer or self.analyze), \
             patch('tools.benchmark.layout_formal.digest', return_value='a'*64):
            return evaluate_formal(self.unit, {'engines': {}}, {}, {}, {}, 'promotion',
                                   Path('panel'), Path('held') if held else None, Path('bundle'))

    def test_pass_has_formal_statistics_and_hashes(self):
        verdict, missing, failed, hashes = self.decide()
        self.assertTrue(verdict['passed'])
        self.assertFalse(missing or failed)
        self.assertIn('portfolio', verdict['formal_statistics'])
        self.assertEqual(hashes['held_out_panel_sha256'], 'a'*64)

    def test_missing_held_out_blocks_promotion(self):
        verdict, missing, failed, _ = self.decide(held=False)
        self.assertEqual(verdict['label'], 'incomplete')
        self.assertTrue(any('held-out' in s for s in missing))
        self.assertFalse(failed)

    def test_target_formal_failure_rejects_passing_screen(self):
        self.values[self.target] = [1.]*30
        verdict, _, failed, _ = self.decide()
        self.assertEqual(verdict['label'], 'reject')
        self.assertTrue(any('target payoff' in s for s in failed))

    def test_imprecise_formal_input_is_not_waived_by_panel(self):
        self.panels['panels'] *= 6
        verdict, missing, _, _ = self.decide(analyzer=lambda values: {**self.analyze(values), 'precise': False})
        self.assertEqual(verdict['label'], 'incomplete')
        self.assertTrue(any('imprecise' in s for s in missing))

    def test_formal_wall_event_needs_panel_and_mixed_panel_resolves(self):
        self.values['broad/method_call'] = [1.04]*30
        verdict, missing, _, _ = self.decide()
        self.assertEqual(verdict['label'], 'incomplete')
        self.assertTrue(any('requires K=6' in s for s in missing))
        self.panels['panels'] *= 6
        self.assertTrue(self.decide()[0]['passed'])

    def test_formal_lane_upper_interval_cannot_hide_in_portfolio(self):
        for name in self.values:
            if name.startswith('sentinel/'):
                self.values[name] = [1.016]*30
        verdict, missing, failed, _ = self.decide()
        self.assertFalse(verdict['passed'])
        self.assertTrue(any('sentinel' in s for s in missing+failed))

    def test_heldout_program_upper_interval_must_fit_band(self):
        self.held['panels'][0]['cases'][0]['ratios']['wall_ns'] = [1.0095]*30
        verdict, missing, failed, _ = self.decide()
        self.assertEqual(verdict['label'], 'incomplete')
        self.assertTrue(any('held-out lane' in s for s in missing))
        self.assertFalse(failed)

    def test_memory_changes_require_resource_evidence(self):
        self.unit['layout_gate'].update(mechanism_class='instruction-blind',
                                       memory_wall_targets=[self.target], panel_required=True)
        self.panels['panels'] *= 6
        verdict, missing, failed, _ = self.decide()
        self.assertEqual(verdict['label'], 'incomplete')
        self.assertTrue(any('resource' in s for s in missing))
        self.assertFalse(failed)

    def test_intermediate_stage_has_no_payoff_but_keeps_its_budget(self):
        from tools.benchmark.layout_rule import evaluate
        data = packet(self.policy)
        verdict = evaluate(self.policy, data, [self.target], .8, stage=True, stage_limit=1.01)
        self.assertTrue(verdict['passed'])
        data = change(data, self.target, wall=1.02)
        verdict = evaluate(self.policy, data, [self.target], .8, stage=True, stage_limit=1.01,
                           panel=[data]*6)
        self.assertEqual(verdict['label'], 'reject')

class PlanBindingTests(unittest.TestCase):
    def test_schema_three_roundtrip_and_preimplementation_policy_host_binding(self):
        import json
        import tempfile
        from tools.benchmark.performance_decision import load_unit, validate_unit_against_queue
        from tools.benchmark.layout_identity import ROOT
        from tools.benchmark.held_out import opportunity_ids
        original = json.loads((ROOT/'tasks/performance-units/ordinary-own-slot-hot-read-fat-lto.json').read_text())
        original.update(schema_version=3, unit_kind='leaf', layout_gate={
            'policy_path': 'benchmarks/policies/test.json', 'policy_sha256': 'a'*64,
            'mechanism_class': 'cpu-work', 'memory_wall_targets': [], 'panel_required': False,
            'panel_pairs': 5, 'held_out_pairs': 30, 'formal_blocks': 30})
        original['fast_gate'].pop('control_ids')
        original['fast_gate'].pop('control_max_candidate_over_base')
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp)/'unit.json'
            path.write_text(json.dumps(original))
            unit, sha = load_unit(path)
        p = policy()
        p['host_node'] = 'test-host'
        p['held_out'] = {name: {'wall_band': 1.01} for name in opportunity_ids()}
        queue = {'schema_version': 3, 'candidate_sha': unit['base_sha'], 'host': {'node': 'test-host'},
                 'external': [{'id': unit['priority']['opportunity_ids'][0], 'rank': 1}],
                 'broad': [], 'sentinel': []}
        with patch('tools.benchmark.layout_policy.load_policy', return_value=p):
            result = validate_unit_against_queue(unit, sha, queue, unit['queue']['sha256'])
            self.assertEqual(result['status'], 'valid')
            legacy = copy.deepcopy(unit)
            legacy['schema_version'] = 2
            with self.assertRaisesRegex(ValueError, 'schema-3 layout plan'):
                validate_unit_against_queue(legacy, sha, queue, unit['queue']['sha256'])
            queue['host']['node'] = 'other-host'
            with self.assertRaisesRegex(ValueError, 'hosts differ'):
                validate_unit_against_queue(unit, sha, queue, unit['queue']['sha256'])


if __name__ == '__main__':
    unittest.main()
