"""Every decision path, including immutable T053/T054 historical screens."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.benchmark.layout_rule import evaluate
from tools.benchmark.layout_iteration import evaluate_attempt
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


class MigrationIterationTests(unittest.TestCase):
    def setUp(self):
        self.policy = policy()
        self.data = packet(self.policy)
        self.target = 'external/jetstream3-js-subset/hash-map'
        self.unit = {
            'unit_kind': 'migration',
            'migration': {'current_stage': 1, 'stages': 3,
                          'cumulative_target_ids': [self.target],
                          'stage_max_candidate_over_base': 1.01},
            'fast_gate': {'target_ids': [self.target],
                          'target_max_candidate_over_base': .97},
            'layout_gate': {'mechanism_class': 'cpu-work', 'memory_wall_targets': [],
                            'panel_required': True},
        }

    def verdict(self, data=None):
        return evaluate_attempt(self.unit, self.policy, [data or self.data]*6)

    def test_neutral_intermediate_stage_does_not_need_final_payoff(self):
        self.assertTrue(self.verdict()['passed'])

    def test_intermediate_stage_keeps_its_cumulative_regression_budget(self):
        result = self.verdict(change(self.data, self.target, wall=1.02))
        self.assertEqual(result['label'], 'reject')
        self.assertIn(self.target + ': persistent six-layout wall cost', result['failures'])

    def test_final_stage_requires_payoff_and_accepts_improvement(self):
        self.unit['migration']['current_stage'] = 3
        self.assertEqual(self.verdict()['label'], 'reject')
        self.assertTrue(self.verdict(change(self.data, self.target, wall=.96))['passed'])

    def test_leaf_still_requires_payoff(self):
        self.unit['unit_kind'] = 'leaf'
        self.unit.pop('migration')
        self.assertEqual(self.verdict()['label'], 'reject')


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
        self.assertEqual(self.run_rule(change(self.data, self.name,
                            instructions=[1.02, 1.04, 1.04, 1.04, 1.04]))['label'], 'reject')
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

    def test_target_aggregate_is_additional_to_the_frozen_natural_gate(self):
        data = change(self.data, self.name, wall=.98)
        other = change(self.data, self.name, wall=.90)
        panels = [data] + [other]*5
        optional = evaluate(self.policy, data, [self.name], .97, panel=panels)
        self.assertEqual(optional['label'], 'reject')
        declared = evaluate(self.policy, data, [self.name], .97, panel=panels, panel_required=True)
        self.assertEqual(declared['label'], 'reject')
        self.assertLess(declared['cases'][self.name]['target_aggregate'], .97)

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
    def test_live_policy_accepts_dns_suffix_change_but_requires_same_machine_and_recipe(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'policy.json'
            data = {**policy(), 'host_node': 'iMac.local'}
            path.write_text(json.dumps(data))
            sha = digest(path)
            with patch('tools.benchmark.layout_policy.host_identity', return_value=data['host']), \
                 patch('tools.benchmark.layout_policy.platform.node', return_value='IMAC.lan'):
                self.assertEqual(load_policy(path, sha), data)
                self.assertEqual(digest(path), sha)
            with patch('tools.benchmark.layout_policy.host_identity', return_value=data['host']), \
                 patch('tools.benchmark.layout_policy.platform.node', return_value='mini.lan'):
                with self.assertRaisesRegex(ValueError, 'recalibration'):
                    load_policy(path, sha)
            with patch('tools.benchmark.layout_policy.host_identity', return_value={'test': 'other'}), \
                 patch('tools.benchmark.layout_policy.platform.node', return_value='iMac.lan'):
                with self.assertRaisesRegex(ValueError, 'recalibration'):
                    load_policy(path, sha)

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

    def test_formal_self_comparison_binds_its_own_revision_and_binary(self):
        from tools.benchmark.layout_qualification import verify_formal_identity
        summary = {'engines': {r: {'source_revision': 'b'*40, 'binary_sha256': 'c'*64}
                               for r in ('base', 'candidate')}}
        verify_formal_identity(summary, 'b'*40)
        with self.assertRaisesRegex(ValueError, 'frozen revision'):
            verify_formal_identity(summary, 'a'*40)
        summary['engines']['candidate']['binary_sha256'] = 'd'*64
        with self.assertRaisesRegex(ValueError, 'byte-identical'):
            verify_formal_identity(summary, 'b'*40)

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
                'bodies': {'first': ['ret '], 'last': ['ret ', 'nop ']},
                'sections': [(1000, 200, '__TEXT/__text', 0)]}
        new = {'addresses': {'_qjs_layout_noise_pad': 1000, 'first': 1016, 'last': 1116},
               'bodies': {'first': ['ret '], 'last': ['ret ', 'nop ', 'int3 ']},
               'sections': [(1000, 216, '__TEXT/__text', 0)], 'padding_bytes': b'\x90'*16}
        self.assertEqual(audit(base, new, 16)['functions'], 2)
        for mutate in (lambda d: d['addresses'].__setitem__('last', 1132),
                       lambda d: d['bodies']['first'].append('nop '),
                       lambda d: d['addresses'].__setitem__('new_function', 2000)):
            value = copy.deepcopy(new)
            mutate(value)
            with self.assertRaises(ValueError):
                audit(base, value, 16)


    def test_section_page_relocation_preserves_declared_padding_and_function_bodies(self):
        base = {'addresses': {'first': 0x2000, 'last': 0x2100},
                'sections': [(0x2000, 512, '__TEXT/__text', 0)],
                'bodies': {'first': ['ret '], 'last': ['ret ']}}
        new = {'addresses': {'_qjs_layout_noise_pad': 0x1000, 'first': 0x13f0, 'last': 0x14f0},
               'sections': [(0x1000, 1520, '__TEXT/__text', 0)],
               'bodies': {'first': ['ret '], 'last': ['ret ']}, 'padding_bytes': b'\x90'*1008}
        result = audit(base, new, 1008)
        self.assertEqual(result['actual_uniform_shift'], -3088)
        self.assertEqual(result['text_section_shift'], -4096)
        self.assertEqual(result['section_relative_shift'], 1008)
        self.assertTrue(result['declared_nops_verified'])
        new['padding_bytes'] = b'\x90'*1007 + b'\0'
        with self.assertRaisesRegex(ValueError, 'NOP atom'):
            audit(base, new, 1008)


if __name__ == '__main__':
    unittest.main()


class PrecisionFallbackTests(unittest.TestCase):
    def test_cap_band_is_diagnostic_but_panel_and_other_gates_stay_required(self):
        p=policy();name=next(iter(p['cases']))
        p['precision']={'cases':{name:{'status':'imprecise on this host'}}}
        p['lanes']={k:2. for k in p['lanes']}
        data=change(packet(p),name,wall=1.2)
        result=evaluate(p,data,[],.97)
        self.assertTrue(result['panel_required'])
        self.assertFalse(result['complete'])
        result=evaluate(p,data,[],.97,panel=[data]*6)
        self.assertTrue(result['passed'])
        self.assertTrue(result['cases'][name]['diagnostic_only'])
        p['lanes']['portfolio']=1.0
        self.assertIn('wall lane ceiling exceeded',' '.join(evaluate(p,data,[],.97,panel=[data]*6)['failures']))
        p['lanes']['portfolio']=2.
        data=change(data,name,instructions=1.04)
        self.assertIn('hard cap',' '.join(evaluate(p,data,[],.97,panel=[data]*6)['failures']))
