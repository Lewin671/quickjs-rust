"""Every decision path, including immutable T053/T054 historical screens."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.benchmark.layout_rule import evaluate
from tools.benchmark.layout_policy import analyze, load_policy, rows
from tools.benchmark.layout_identity import digest, host_identity, OFFSETS
from tools.benchmark.layout_variant_audit import audit
from tools.benchmark.layout_qualification import qualify

FIXTURES = Path(__file__).parent / 'fixtures/layout-policy'


def policy():
    seed = json.loads((FIXTURES / 'a2-bands.json').read_text())
    return {**seed, 'schema_version': 1, 'artifact_type': 'quickjs-layout-policy',
            'qualified': True, 'offsets': list(OFFSETS), 'host': {'test': 'host'},
            'held_out': {'held-out/lib/program': {'wall_band': 1.02}},
            'lanes': {**seed['lanes'], 'held-out': 1.01}}


def packet(p=None):
    p = p or policy()
    return {'pairs': 5, 'cases': [{'id': n, 'ratios': {'wall_ns': [1.0]*5,
                  'instructions': [1.0]*5}} for n in p['cases']]}


def change(data, name, wall=None, instructions=None):
    result = copy.deepcopy(data)
    row = next(c for c in result['cases'] if c['id'] == name)
    if wall is not None:
        row['ratios']['wall_ns'] = [wall]*5 if isinstance(wall, float) else wall
    if instructions is not None:
        row['ratios']['instructions'] = [instructions]*5 if isinstance(instructions, float) else instructions
    return result


class VerdictTests(unittest.TestCase):
    def setUp(self):
        self.policy = policy()
        self.data = packet(self.policy)
        self.name = 'broad/top_level_function_call'

    def run_rule(self, data=None, **kw):
        return evaluate(self.policy, data or self.data, [], .97, **kw)

    def test_pass_with_fractional_instruction_growth_and_boundary_floor(self):
        for instructions in (1.0021, 1.01):
            verdict = self.run_rule(change(self.data, self.name, instructions=instructions))
            self.assertEqual(verdict['label'], 'pass')

    def test_control_event_needs_panel_and_faster_wall_is_not_rejection(self):
        data = change(self.data, self.name, wall=.98, instructions=1.012)
        self.assertEqual(self.run_rule(data)['label'], 'control event')
        result = self.run_rule(data, panel=[data]*6)
        self.assertTrue(result['passed'])
        self.assertEqual(result['event_resolution'], 'nonpersistent')

    def test_case_specific_instruction_ceiling_overrides_one_percent(self):
        data = change(self.data, 'external/sunspider-1.0/regexp-dna', instructions=1.0143)
        self.assertEqual(self.run_rule(data)['label'], 'pass')

    def test_inconsistent_instruction_alert_is_incomplete(self):
        data = change(self.data, self.name, instructions=[1., 1.02, 1.02, 1.02, 1.02])
        self.assertEqual(self.run_rule(data)['label'], 'incomplete')

    def test_instruction_hard_cap_and_all_pairs_requirement(self):
        self.assertEqual(self.run_rule(change(self.data, self.name, instructions=1.031))['label'], 'reject')
        self.assertNotEqual(self.run_rule(change(self.data, self.name,
                            instructions=[1., 1.04, 1.04, 1.04, 1.04]))['label'], 'reject')

    def test_wall_event_and_mixed_panel_resolve_nonpersistent(self):
        data = change(self.data, self.name, wall=1.04)
        self.assertEqual(self.run_rule(data)['label'], 'wall event')
        verdict = self.run_rule(data, panel=[data]*5 + [self.data])
        self.assertTrue(verdict['passed'])
        self.assertEqual(verdict['cases'][self.name]['label'], 'pass')

    def test_six_persistent_walls_reject_even_with_flat_instructions(self):
        data = change(self.data, self.name, wall=1.04)
        result = self.run_rule(data, panel=[data]*6)
        self.assertEqual(result['label'], 'reject')
        self.assertTrue(any('persistent' in f for f in result['failures']))

    def test_hard_cap_in_non_natural_layout_is_not_hidden(self):
        data = change(self.data, self.name, instructions=1.04)
        self.assertEqual(self.run_rule(panel=[self.data]*5 + [data])['label'], 'reject')

    def test_lanes_and_portfolio_do_not_average_away_failed_lane(self):
        data = copy.deepcopy(self.data)
        for row in data['cases']:
            row['ratios']['wall_ns'] = [1.03 if row['id'].startswith('sentinel/') else .95]*5
        result = self.run_rule(data)
        self.assertEqual(result['label'], 'reject')
        self.assertTrue(any('sentinel' in f for f in result['failures']))
        self.assertLess(result['lanes']['0/portfolio']['ratio'], 1)

    def test_target_requires_payoff_noise_band_and_every_pair(self):
        for values, passes in (([.96]*5, True), ([.98]*5, False), ([.96]*4+[1.], False)):
            data = change(self.data, self.name, wall=values)
            self.assertEqual(evaluate(self.policy, data, [self.name], .97)['passed'], passes)
        data = change(self.data, 'broad/global_read', wall=.95)
        self.assertFalse(evaluate(self.policy, data, ['broad/global_read'], .97)['passed'])

    def test_blind_requires_frozen_memory_targets_and_full_panel(self):
        with self.assertRaises(ValueError):
            self.run_rule(instruction_blind=True)
        data = change(self.data, self.name, wall=.9)
        kw = dict(instruction_blind=True, memory_targets=[self.name])
        self.assertFalse(evaluate(self.policy, data, [self.name], .97, **kw)['passed'])
        self.assertTrue(evaluate(self.policy, data, [self.name], .97, panel=[data]*6, **kw)['passed'])

    def test_heldout_missing_program_cost_lane_and_precision_count(self):
        self.assertEqual(self.run_rule(promotion=True)['label'], 'incomplete')
        held = {'pairs': 30, 'cases': [{'id': 'held-out/lib/program',
                   'ratios': {'wall_ns': [1.]*30, 'instructions': [1.]*30}}]}
        self.assertTrue(self.run_rule(promotion=True, held_out=held)['passed'])
        held['cases'][0]['ratios']['wall_ns'] = [1.03]*30
        self.assertEqual(self.run_rule(promotion=True, held_out=held)['label'], 'reject')
        held['cases'][0]['ratios']['wall_ns'] = [1.015]*30
        self.assertEqual(self.run_rule(promotion=True, held_out=held)['label'], 'reject')
        held['pairs'] = 5
        with self.assertRaises(ValueError):
            self.run_rule(promotion=True, held_out=held)

    def test_missing_duplicate_bad_metric_partial_panel_refused(self):
        for mutate in (lambda d: d['cases'].pop(), lambda d: d['cases'].append(d['cases'][0]),
                       lambda d: d['cases'][0]['ratios']['wall_ns'].pop(),
                       lambda d: d['cases'][0]['ratios']['instructions'].__setitem__(0, float('nan'))):
            data = copy.deepcopy(self.data)
            mutate(data)
            with self.assertRaises(ValueError):
                self.run_rule(data)
        with self.assertRaises(ValueError):
            self.run_rule(panel=[self.data]*5)
        with self.assertRaises(ValueError):
            self.run_rule(panel=[change(self.data, self.name, wall=.99)]*6)

    def test_recorded_t053_t054_replays(self):
        for name, label, count in (('T053', 'control event', 5), ('T054', 'wall event', 2)):
            data = json.loads((FIXTURES / (name+'.json')).read_text())
            targets = ['external/jetstream3-js-subset/hash-map'] if name == 'T053' else []
            result = evaluate(self.policy, data, targets, .97)
            self.assertEqual(result['label'], label)
            self.assertEqual(len(result['events']), count)
            self.assertFalse(result['failures'])
            self.assertFalse(result['passed'])


class QualificationTests(unittest.TestCase):
    def test_exact_statistic_and_precision_deterministic(self):
        result = analyze({'a': [1.]*30, 'b': [1.21]*30}, draws=100)
        self.assertAlmostEqual(result['ratio'], 1.1)
        self.assertTrue(result['precise'])
        self.assertEqual(result, analyze({'a': [1.]*30, 'b': [1.21]*30}, draws=100))
        self.assertFalse(analyze({'a': [1.]*5}, draws=10)['precise'])

    def test_hash_unqualified_host_and_invalid_policy_refused(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'policy.json'
            data = policy()
            path.write_text(json.dumps(data))
            self.assertEqual(load_policy(path, digest(path), live=False)['cases'], data['cases'])
            with self.assertRaises(ValueError):
                load_policy(path, '0'*64, live=False)
            with patch('tools.benchmark.layout_policy.host_identity', return_value={}):
                with self.assertRaisesRegex(ValueError, 'recalibration'):
                    load_policy(path, digest(path))
            data['qualified'] = False
            path.write_text(json.dumps(data))
            with self.assertRaises(ValueError):
                load_policy(path, digest(path), live=False)

    def test_non_mac_x86_refuses_before_tools(self):
        with patch('platform.system', return_value='Linux'):
            with self.assertRaisesRegex(ValueError, 'x86_64 macOS'):
                host_identity()

    def test_qualification_does_not_widen_failed_lane(self):
        p = policy()
        data = packet(p)
        data['pairs'] = 30
        data['host'] = p['host']
        for c in data['cases']:
            c['ratios'] = {m: [1.]*30 for m in ('wall_ns', 'instructions')}
        held = {'pairs': 30, 'host': p['host'], 'cases': [{'id': 'held-out/lib/program',
                'ratios': {'wall_ns': [1.]*30, 'instructions': [1.]*30}}]}
        def precise(values):
            return {'ratio': 1., 'confidence_interval': {'lower': .999, 'upper': 1.001}, 'precise': True}
        with patch('tools.benchmark.layout_qualification.analyze', side_effect=precise), \
             patch('tools.benchmark.held_out.opportunity_ids', return_value={'held-out/lib/program'}):
            self.assertTrue(qualify(p, data, held)['qualified'])
            p['lanes']['broad'] = 1.0001
            result = qualify(p, data, held)
            self.assertFalse(result['qualified'])
            self.assertEqual(result['lanes']['broad'], 1.0001)


class StaticAuditTests(unittest.TestCase):
    def test_uniform_shift_body_tail_and_drift(self):
        base = {'addresses': {'first': 1000, 'last': 1100},
                'bodies': {'first': ['ret '], 'last': ['ret ', 'nop ']}}
        new = {'addresses': {'_qjs_layout_noise_pad': 1000, 'first': 1016, 'last': 1116},
               'bodies': {'first': ['ret '], 'last': ['ret ', 'nop ', 'int3 ']}}
        self.assertEqual(audit(base, new, 16)['functions'], 2)
        for mutate in (lambda d: d['addresses'].__setitem__('last', 1132),
                       lambda d: d['bodies']['first'].append('nop '),
                       lambda d: d['addresses'].__setitem__('new_function', 2000)):
            value = copy.deepcopy(new)
            mutate(value)
            with self.assertRaises(ValueError):
                audit(base, value, 16)


if __name__ == '__main__':
    unittest.main()
