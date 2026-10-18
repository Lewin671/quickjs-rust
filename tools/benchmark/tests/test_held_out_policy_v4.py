"""Policy v4: held-out six-layout persistence, its calibration and candidate continuation."""
import copy
import json
import platform
import statistics
import tempfile
import unittest
from contextlib import nullcontext
from pathlib import Path
from unittest.mock import patch

from tools.benchmark import layout_formal_continuation as continuation
from tools.benchmark import layout_held_out_policy as v4
from tools.benchmark import screen
from tools.benchmark.held_out import opportunity_ids as held_ids
from tools.benchmark.layout_identity import digest
from tools.benchmark.layout_policy import HELD_OUT_RULE, load_policy, with_held_out_successor
from tools.benchmark.layout_rule import evaluate
from tools.benchmark.tests import test_layout_formal as formal_fixture
from tools.benchmark.tests.test_layout_rule import packet, policy

PROGRAM = 'held-out/lib/program'
OTHER = 'held-out/lib/other'


def v4_policy():
    p = policy()
    p['held_out'] = {n: {'wall_band': 1.02, 'instruction_band': 1.002} for n in (PROGRAM, OTHER)}
    p['lanes']['held-out'] = 1.015
    p['held_out_rule'] = HELD_OUT_RULE
    return p


def held(wall=1., instructions=1., other=1., pairs=30):
    def ratios(value):
        return [value]*pairs if isinstance(value, float) else value
    return {'pairs': pairs, 'cases': [
        {'id': PROGRAM, 'ratios': {'wall_ns': ratios(wall), 'instructions': ratios(instructions)}},
        {'id': OTHER, 'ratios': {'wall_ns': ratios(other), 'instructions': [1.]*pairs}}]}


class HeldOutPersistenceRuleTests(unittest.TestCase):
    def setUp(self):
        self.policy = v4_policy()
        self.screen = packet(self.policy)

    def run_rule(self, natural, panel=None, p=None):
        return evaluate(p or self.policy, self.screen, [], .97, held_out=natural,
                        held_out_panel=panel, promotion=True)

    def test_flat_instruction_wall_excess_is_an_event_not_a_rejection(self):
        result = self.run_rule(held(wall=1.03))
        self.assertEqual(result['label'], 'wall event')
        self.assertFalse(result['failures'])
        self.assertIn('complete K=6 held-out panel required', result['missing'])
        self.assertEqual(result['held_out']['programs'][PROGRAM]['label'], 'held-out wall event')
        # An inconsistent instruction alert is still "no consistent regression".
        alert = held(wall=1.03, instructions=[1.]*15 + [1.02]*15)
        self.assertEqual(self.run_rule(alert)['label'], 'wall event')

    def test_mixed_panel_resolves_and_six_persistent_layouts_reject(self):
        natural = held(wall=1.03)
        mixed = self.run_rule(natural, [natural]*5 + [held()])
        self.assertTrue(mixed['passed'])
        self.assertEqual(mixed['held_out']['event_resolution'], 'nonpersistent')
        persistent = self.run_rule(natural, [natural]*6)
        self.assertEqual(persistent['label'], 'reject')
        self.assertIn('persistent six-layout held-out wall cost', ' '.join(persistent['failures']))

    def test_consistent_instruction_regression_with_wall_excess_rejects_without_panel(self):
        result = self.run_rule(held(wall=1.03, instructions=1.02))
        self.assertEqual(result['label'], 'reject')
        self.assertIn('consistent instruction regression', ' '.join(result['failures']))
        # Within its band, instruction growth alone keeps the v3 pass.
        self.assertTrue(self.run_rule(held(instructions=1.02))['passed'])
        # A panel layout cannot hide the same excess.
        natural = held(wall=1.03)
        result = self.run_rule(natural, [natural] + [held(wall=1.03, instructions=1.02)] + [held()]*4)
        self.assertEqual(result['label'], 'reject')

    def test_case_instruction_band_above_one_percent_is_the_ceiling(self):
        self.policy['held_out'][PROGRAM]['instruction_band'] = 1.03
        self.assertEqual(self.run_rule(held(wall=1.03, instructions=1.02))['label'], 'wall event')

    def test_lane_excess_needs_panel_and_rejects_only_in_every_layout(self):
        natural = held(wall=1.018, other=1.018)
        result = self.run_rule(natural)
        self.assertIn('held-out lane', result['held_out']['events'])
        self.assertFalse(result['complete'])
        self.assertTrue(self.run_rule(natural, [natural]*5 + [held()])['passed'])
        failed = self.run_rule(natural, [natural]*6)
        self.assertIn('held-out: six-layout lane geomean or every layout exceeds ceiling', failed['failures'])

    def test_cap_imprecise_program_needs_panel_and_cannot_reject_by_wall(self):
        self.policy['precision'] = {'held_out': {PROGRAM: {'status': 'imprecise on this host'}}}
        self.assertFalse(self.run_rule(held())['complete'])
        natural = held(wall=1.03)
        self.assertTrue(self.run_rule(natural, [natural]*6)['passed'])

    def test_panel_shape_and_legacy_policy_are_refused(self):
        natural = held(wall=1.03)
        with self.assertRaisesRegex(ValueError, 'exactly six'):
            self.run_rule(natural, [natural]*5)
        with self.assertRaisesRegex(ValueError, 'variant zero'):
            self.run_rule(natural, [held()]*6)
        with self.assertRaisesRegex(ValueError, 'pair counts'):
            self.run_rule(natural, [natural] + [held(wall=1.03, pairs=5)]*5)
        legacy = v4_policy()
        legacy.pop('held_out_rule')
        with self.assertRaisesRegex(ValueError, 'v4 policy'):
            self.run_rule(natural, [natural]*6, p=legacy)
        # v3 semantics are unchanged: one natural excess rejects.
        self.assertIn('held-out program band exceeded', ' '.join(self.run_rule(natural, p=legacy)['failures']))


class LaneEventRuleTests(unittest.TestCase):
    """v4: a natural lane excess is a lane event resolved by the K=6 panel."""

    def setUp(self):
        self.policy = v4_policy()
        self.base = packet(self.policy)

    def lane(self, value):
        data = copy.deepcopy(self.base)
        for row in data['cases']:
            if row['id'].startswith('sentinel/'):
                row['ratios']['wall_ns'] = [value]*5
        return data

    def test_natural_lane_excess_is_an_event_that_needs_the_panel(self):
        ceiling = self.policy['lanes']['sentinel']
        natural = self.lane(ceiling + .002)
        result = evaluate(self.policy, natural, [], .97)
        self.assertEqual(result['label'], 'wall event')
        self.assertEqual(result['events'], {'sentinel lane': 'lane event'})
        self.assertIn('complete K=6 matching-layout panel required', result['missing'])
        self.assertFalse(result['failures'])
        legacy = copy.deepcopy(self.policy)
        legacy.pop('held_out_rule')
        self.assertEqual(evaluate(legacy, natural, [], .97)['label'], 'reject')

    def test_panel_rejects_on_six_layout_geomean_or_every_layout(self):
        ceiling = self.policy['lanes']['sentinel']
        natural, low = self.lane(ceiling + .002), self.lane(1.)
        self.assertTrue(evaluate(self.policy, natural, [], .97, panel=[natural] + [low]*5)['passed'])
        heavy = self.lane(ceiling + .06)
        result = evaluate(self.policy, natural, [], .97, panel=[natural, heavy, heavy, low, low, low])
        self.assertIn('sentinel: six-layout lane geomean or every layout exceeds ceiling', result['failures'])
        self.assertEqual(evaluate(self.policy, natural, [], .97, panel=[natural]*6)['label'], 'reject')
        # A non-natural single excess under a neutral geomean is not a rejection.
        self.assertTrue(evaluate(self.policy, low, [], .97, panel=[low]*5 + [natural])['passed'])

    def test_instruction_hard_cap_still_rejects_immediately(self):
        data = copy.deepcopy(self.base)
        name = data['cases'][0]['id']
        data['cases'][0]['ratios']['instructions'] = [1.04]*5
        self.assertIn(name + ': consistent instruction excess exceeds median hard cap',
                      evaluate(self.policy, data, [], .97)['failures'])

    def test_t058_screen1_natural_packet_is_a_lane_event_needing_the_panel(self):
        root = Path(__file__).resolve().parents[3]
        path = root / 'benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v3.json'
        p = load_policy(path, digest(path), live=False)
        natural = json.loads((Path(__file__).parent / 'fixtures/layout-policy/t058-screen1-natural.json').read_text())
        target = 'development/papaparse/csv'
        legacy = evaluate(p, natural, [target], .7)
        self.assertEqual(legacy['failures'], ['layout 0 broad: wall lane ceiling exceeded'])
        p['held_out_rule'] = HELD_OUT_RULE
        result = evaluate(p, natural, [target], .7)
        self.assertFalse(result['failures'])
        self.assertIn('broad lane', result['events'])
        self.assertGreater(result['lanes']['0/broad']['ratio'], p['lanes']['broad'])
        self.assertTrue(result['panel_required'])
        self.assertIn('complete K=6 matching-layout panel required', result['missing'])
        self.assertEqual(result['cases'][target]['label'], 'pass')


class HeldOutPolicyLoadingTests(unittest.TestCase):
    def test_rule_requires_known_name_and_instruction_bands(self):
        p = v4_policy()
        p['host_node'] = 'n'
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'policy.json'
            path.write_text(json.dumps(p))
            self.assertEqual(load_policy(path, digest(path), live=False)['held_out_rule'], HELD_OUT_RULE)
            for change in ('rule', 'band'):
                bad = copy.deepcopy(p)
                if change == 'rule':
                    bad['held_out_rule'] = 'other'
                else:
                    bad['held_out'][PROGRAM].pop('instruction_band')
                path.write_text(json.dumps(bad))
                with self.subTest(change=change), self.assertRaisesRegex(ValueError, 'held-out'):
                    load_policy(path, digest(path), live=False)

    def test_successor_replaces_only_the_held_out_gate_of_frozen_parents(self):
        parent = policy()
        parent.update(host_node='n', precision={'cases': {}, 'held_out': {}})
        parent['held_out'] = {PROGRAM: {'wall_band': 1.003}, OTHER: {'wall_band': 1.003}}
        successor = {**v4_policy(), 'host_node': 'n', 'precision': {'cases': {'x': 1}, 'held_out': {'y': 2}}}
        meta = {'path': 'v4.json', 'sha256': 's'*64, 'parents': ('p'*64,)}
        with patch('tools.benchmark.layout_policy.load_policy', return_value=successor):
            merged, sha = with_held_out_successor(parent, 'p'*64, meta)
            self.assertEqual(sha, 's'*64)
            self.assertEqual(merged['held_out'], successor['held_out'])
            self.assertEqual(merged['lanes']['held-out'], 1.015)
            self.assertEqual(merged['lanes']['broad'], parent['lanes']['broad'])
            self.assertEqual(merged['cases'], parent['cases'])
            self.assertEqual(merged['precision'], {'cases': {}, 'held_out': {'y': 2}})
            self.assertEqual(with_held_out_successor(parent, 'q'*64, meta), (parent, None))
            successor['host_node'] = 'other'
            with self.assertRaisesRegex(ValueError, 'successor'):
                with_held_out_successor(parent, 'p'*64, meta)
        self.assertEqual(with_held_out_successor(parent, 'p'*64), (parent, None))

    def test_published_v4_is_qualified_and_succeeds_v2_v3_held_out_gates(self):
        from tools.benchmark.layout_policy import HELD_OUT_SUCCESSOR
        root = Path(__file__).resolve().parents[3] / 'benchmarks/policies'
        v4_path = root / 'imac-x86_64-rust1.95-fat16-layout-v4.json'
        self.assertEqual(digest(v4_path), HELD_OUT_SUCCESSOR['sha256'])
        self.assertIn(HELD_OUT_SUCCESSOR['sha256'], (root / 'imac-x86_64-rust1.95-fat16-layout-v4.sha256').read_text())
        published = load_policy(v4_path, digest(v4_path), live=False)
        self.assertEqual(published['held_out_rule'], HELD_OUT_RULE)
        for name in ('v2', 'v3'):
            path = root / f'imac-x86_64-rust1.95-fat16-layout-{name}.json'
            frozen = load_policy(path, digest(path), live=False)
            merged, sha = with_held_out_successor(frozen, digest(path))
            self.assertEqual(sha, HELD_OUT_SUCCESSOR['sha256'])
            self.assertEqual(merged['held_out'], published['held_out'])
            self.assertEqual(merged['lanes']['held-out'], published['lanes']['held-out'])
            self.assertEqual(merged['cases'], frozen['cases'])
            self.assertEqual({k: v for k, v in merged['lanes'].items() if k != 'held-out'},
                             {k: v for k, v in frozen['lanes'].items() if k != 'held-out'})
            self.assertNotIn('held_out_rule', frozen)
        self.assertEqual(with_held_out_successor(published, HELD_OUT_SUCCESSOR['sha256']), (published, None))
        v3 = load_policy(root / 'imac-x86_64-rust1.95-fat16-layout-v3.json', HELD_OUT_SUCCESSOR['parents'][1], live=False)
        self.assertEqual({k: v for k, v in published.items() if k not in (
            'held_out', 'held_out_rule', 'held_out_manifest_sha256', 'held_out_evidence', 'lanes',
            'precision', 'qualification')}, {k: v for k, v in v3.items() if k not in ('held_out', 'lanes', 'precision', 'qualification')})


class HeldOutFormalTests(formal_fixture.FormalTests):
    """Reuses the formal fixture with a v4 held-out policy."""

    def setUp(self):
        super().setUp()
        self.policy['held_out'] = {'held-out/lib/program': {'wall_band': 1.02, 'instruction_band': 1.002}}
        self.policy['held_out_rule'] = HELD_OUT_RULE

    def test_heldout_program_upper_interval_must_fit_band(self):
        pass  # The v3 interval rule is replaced below.

    def test_v4_upper_interval_above_band_needs_held_out_panel(self):
        self.policy['lanes']['held-out'] = 1.03
        natural = self.held['panels'][0]
        natural['cases'][0]['ratios']['wall_ns'] = [1.0195]*30
        verdict, missing, failed, _ = self.decide()
        self.assertFalse(failed)
        self.assertTrue(any('K=6 held-out panel required' in s for s in missing))
        self.held['panels'] = [natural]*6
        self.assertTrue(self.decide()[0]['passed'])
        persistent = copy.deepcopy(natural)
        persistent['cases'][0]['ratios']['wall_ns'] = [1.03]*30
        self.held['panels'] = [persistent]*6
        self.assertIn('persistent six-layout held-out wall cost', ' '.join(self.decide()[2]))

    def test_v4_lane_upper_interval_needs_panel(self):
        self.held['panels'][0]['cases'][0]['ratios']['wall_ns'] = [1.0095]*30
        verdict, missing, failed, _ = self.decide()
        self.assertFalse(failed)
        self.assertTrue(any('held-out lane upper interval' in s for s in missing))
        self.held['panels'] *= 6
        self.assertTrue(self.decide()[0]['passed'])


class CandidateFormalContinuationTests(formal_fixture.FormalTests):
    def setUp(self):
        super().setUp()
        self.other = 'broad/method_call'
        self.values[self.other] = [1.]*30

    def analyzer(self, values):
        stats = self.analyze(values)
        return {**stats, 'precise': list(values) != [self.other] or len(values[self.other]) > 30}

    def test_imprecise_control_completes_with_its_continuation(self):
        self.assertTrue(any('formal interval imprecise' in s
                            for s in self.decide(analyzer=self.analyzer)[1]))
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp)/'continuation.json'
            path.write_text(json.dumps({'case_id': self.other}))
            calls = []
            def replay(p, bundle, name, *, pilot, comparison):
                calls.append((name, comparison, pilot))
                return pilot + [1.]*191, self.policy['host']
            with patch('tools.benchmark.layout_formal_continuation.replay', side_effect=replay):
                verdict, missing, failed, hashes = self.decide_with([path], analyzer=self.analyzer)
                self.assertTrue(verdict['passed'], missing)
                self.assertEqual(calls, [(self.other, 'candidate-base', [1.]*30)])
                self.assertEqual(verdict['formal_continued_blocks'], {self.other: 221})
                self.assertIn(self.other, hashes['formal_continuation_sha256'])
                # Lanes keep the sealed joint 30 blocks.
                with self.assertRaisesRegex(ValueError, 'duplicate or unknown'):
                    self.decide_with([path, path], analyzer=self.analyzer)
            with patch('tools.benchmark.layout_formal_continuation.replay',
                       return_value=([1.]*221, {'other': 'host'})), \
                 self.assertRaisesRegex(ValueError, 'host/recipe'):
                self.decide_with([path], analyzer=self.analyzer)
            path.write_text(json.dumps({'case_id': 'broad/unknown'}))
            with self.assertRaisesRegex(ValueError, 'duplicate or unknown'):
                self.decide_with([path])

    def decide_with(self, paths, analyzer=None):
        from tools.benchmark.layout_formal import evaluate_formal
        seen = []
        def stats(values):
            seen.append({k: len(v) for k, v in values.items()})
            return (analyzer or self.analyze)(values)
        with patch('tools.benchmark.layout_formal.load_policy', return_value=self.policy), \
             patch('tools.benchmark.layout_formal.replay', side_effect=lambda p, *a, **k:
                   self.held if k.get('held') else self.panels), \
             patch('tools.benchmark.layout_formal_raw.extract', return_value=self.values), \
             patch('tools.benchmark.layout_formal.analyze', side_effect=stats), \
             patch('tools.benchmark.layout_formal.digest', return_value='a'*64):
            result = evaluate_formal(self.unit, {'engines': {}}, {}, {}, {}, 'promotion',
                                     Path('panel'), Path('held'), Path('bundle'), formal_continuations=paths)
        lanes = [s for s in seen if len(s) > 1]
        self.assertTrue(lanes and all(set(s.values()) == {30} for s in lanes))
        return result

    def test_replay_refuses_the_other_comparison_kind(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp)/'continuation.json'
            path.write_text(json.dumps({'artifact_type': 'quickjs-formal-case-continuation', 'case_id': 'c'}))
            with self.assertRaisesRegex(ValueError, 'comparison kind'):
                continuation.replay(path, Path(tmp), 'c', pilot=[1.]*30, comparison='candidate-base')
            path.write_text(json.dumps({'artifact_type': 'quickjs-formal-case-continuation',
                                        'case_id': 'c', 'comparison': 'candidate-base'}))
            with self.assertRaisesRegex(ValueError, 'comparison kind'):
                continuation.replay(path, Path(tmp), 'c', pilot=[1.]*30)

    def test_bundle_kind_follows_sealed_role_identities(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            engines = {r: {'binary_sha256': 'a', 'source_revision': 'r'} for r in ('base', 'candidate')}
            (root/'summary.json').write_text(json.dumps({'engines': engines, 'bundle': {'artifacts': []}}))
            self.assertEqual(continuation.comparison_kind(root), 'aa')
            with patch('tools.benchmark.bundle.verify'):
                self.assertEqual(set(continuation.source_hashes(root)), {'summary.json'})
                with self.assertRaisesRegex(ValueError, 'distinct sealed roles'):
                    continuation.source_hashes(root, 'candidate-base')
                engines['candidate'] = {'binary_sha256': 'b', 'source_revision': 's'}
                (root/'summary.json').write_text(json.dumps({'engines': engines, 'bundle': {'artifacts': []}}))
                self.assertEqual(continuation.comparison_kind(root), 'candidate-base')
                with self.assertRaisesRegex(ValueError, 'identical A/A roles'):
                    continuation.source_hashes(root)
                continuation.source_hashes(root, 'candidate-base')


def raw_case(name, wall, cost_ratio=1., instructions=1.):
    return {'id': name, 'ratios': {'wall_ns': [wall]*5, 'instructions': [instructions]*5},
            'costs': {'base': {'wall_ns': [100.]*5}, 'candidate': {'wall_ns': [100.*cost_ratio]*5}}}


class HeldOutCalibrationTests(unittest.TestCase):
    def setUp(self):
        self.ids = sorted(held_ids())
        self.aa = {'pairs': 30, 'host': {'h': 1}, 'cases': [
            {'id': n, 'ratios': {'wall_ns': [1.]*30, 'instructions': [1.00125]*30}} for n in self.ids]}
        self.parent = {'held_out': {n: {'wall_band': 1.0} for n in self.ids}, 'lanes': {'held-out': 1.0},
                       'host': {'h': 1}, 'precision': {'held_out': {}}, 'qualification': {'v2': 1},
                       'cases': {}, 'failures': []}
        self.original = {'cases': [{'id': 'broad/x', 'metrics': {
            'wall_ns': {'aa_median': .99}, 'instructions': {'aa_median': 1.0005}}}]}
        self.dev = {(0, 'development/a/b'): raw_case('development/a/b', 1.004),
                    (1, 'development/a/b'): raw_case('development/a/b', 1.5)}
        self.raw = {(k, n): raw_case(n, 1. + .01*k, cost_ratio=1. + .002*k) for k in range(6) for n in self.ids}

    def test_bands_take_the_max_of_layout_spreads_bias_and_parent_aa(self):
        seed = v4.bands(self.parent, self.original, self.dev, self.raw, self.aa)
        band = seed['held_out'][self.ids[0]]
        self.assertAlmostEqual(band['wall_band'], 1.05)  # paired-median spread dominates
        self.assertEqual(band['instruction_band'], 1.0013)  # A/A instruction extreme, rounded up
        self.assertAlmostEqual(seed['held_out_lane'], 1.05)
        quiet = {(k, n): raw_case(n, 1.) for k in range(6) for n in self.ids}
        seed = v4.bands(self.parent, self.original, self.dev, quiet, self.aa)
        self.assertAlmostEqual(seed['held_out'][self.ids[0]]['wall_band'], 1/.99)  # global bias floor
        self.assertEqual(seed['held_out_lane'], 1.0)  # parent ceiling floor
        bad = copy.deepcopy(self.parent)
        bad['held_out'][self.ids[0]]['wall_band'] = 1.01
        with self.assertRaisesRegex(ValueError, 'does not reproduce'):
            v4.bands(bad, self.original, self.dev, self.raw, self.aa)
        with self.assertRaisesRegex(ValueError, 'inventory'):
            v4.bands(self.parent, self.original, self.dev, {k: v for k, v in self.raw.items() if k[0]}, self.aa)

    def seed(self):
        return {'held_out': {n: {'wall_band': 1.05, 'instruction_band': 1.002} for n in self.ids},
                'held_out_lane': 1.01, 'held_out_manifest_sha256': 'm'}

    def test_qualification_inherits_parent_and_fails_outside_frozen_envelope(self):
        policy = v4.qualify(self.parent, self.seed(), self.aa)
        self.assertTrue(policy['qualified'], policy['failures'])
        self.assertEqual(policy['held_out_rule'], HELD_OUT_RULE)
        self.assertEqual(policy['lanes']['held-out'], 1.01)
        self.assertEqual(policy['qualification']['v2'], 1)
        self.assertEqual({r['status'] for r in policy['precision']['held_out'].values()}, {'precise'})
        shifted = copy.deepcopy(self.aa)
        for row in shifted['cases']:
            row['ratios']['wall_ns'] = [1.02]*30
        self.assertIn('held-out: A/A interval outside frozen lane envelope', v4.qualify(self.parent, self.seed(), shifted)['failures'])
        event = copy.deepcopy(self.aa)
        event['cases'][0]['ratios']['instructions'] = [1.02]*30
        self.assertTrue(any('instruction control event' in f for f in v4.qualify(self.parent, self.seed(), event)['failures']))
        noisy = copy.deepcopy(self.aa)
        noisy['cases'][0]['ratios']['wall_ns'] = [.8, 1.2]*15
        self.assertTrue(any('continuation incomplete' in f for f in v4.qualify(self.parent, self.seed(), noisy)['failures']))
        with self.assertRaisesRegex(ValueError, 'inventory/host'):
            v4.qualify(self.parent, self.seed(), {**self.aa, 'host': {'other': 1}})


class HeldOutSeedBindingTests(unittest.TestCase):
    def test_a2_lane_binds_held_out_manifest_and_inventory(self):
        from tools.benchmark.layout_development_a2 import lane_inputs
        from tools.benchmark.held_out import MANIFEST_PATH
        artifact, key, path, ids = lane_inputs('held-out')
        self.assertEqual((artifact, key, path, ids),
                         ('quickjs-held-out-layout-a2', 'held_out_manifest_sha256', MANIFEST_PATH, held_ids()))
        self.assertEqual(lane_inputs('development')[0], 'quickjs-development-layout-a2')

    def test_estimator_inputs_must_be_the_parent_policy_evidence(self):
        fixture = json.loads(v4.ORIGINAL_SEED.read_text())
        hashes = {'original': fixture['source_sha256'], 'dev': 'd', 'aa': 'h', 'a2': 'x'}
        parent = {'development_evidence': {'original-a2': {'sha256': fixture['source_sha256']},
                                           'a2': {'sha256': 'd'}},
                  'evidence': {'held-out': {'sha256': 'h'}}}
        paths = [Path(n) for n in ('a2', 'original', 'dev', 'aa')]
        with patch.object(v4, 'digest', side_effect=lambda p: hashes[str(p)]), \
             patch('tools.benchmark.layout_development_a2.replay', side_effect=ValueError('replayed')):
            with self.assertRaisesRegex(ValueError, 'replayed'):
                v4.inputs(parent, 'p', *paths, 'image')
            for key in ('original', 'dev', 'aa'):
                changed = dict(hashes, **{key: 'changed'})
                with self.subTest(key=key), patch.object(v4, 'digest', side_effect=lambda p: changed[str(p)]), \
                     self.assertRaisesRegex(ValueError, 'parent policy evidence'):
                    v4.inputs(parent, 'p', *paths, 'image')


class HeldOutPanelCompletionTests(unittest.TestCase):
    def test_held_out_completion_appends_five_layouts_at_the_natural_pair_count(self):
        from tools.benchmark.layout_panel import measure_panel
        with tempfile.TemporaryDirectory() as work:
            folder = Path(work)
            identities = {role: {'host': {}, 'host_node': platform.node(),
                                 'variants': [{'path': f'/tmp/{role}/v{k}/qjs'} for k in range(6)]}
                          for role in ('base000', 'cand000')}
            calls, replays = [], []
            def take(case, candidate, base, pairs, tool, timeout, output, monitor):
                calls.append((str(candidate), pairs))
                return screen.CaseResult(PROGRAM, 'whole', None,
                    {'instructions': [1.]*pairs, 'wall_ns': [1.]*pairs, 'cycles': []})
            with patch('tools.benchmark.pair_disturbance.quieting_preflight', return_value={}), \
                 patch('tools.benchmark.pair_disturbance.Monitor.from_queue') as monitor, \
                 patch('tools.benchmark.pair_disturbance.verify_run', return_value={}), \
                 patch('tools.benchmark.layout_panel.host_identity', return_value={}), \
                 patch('tools.benchmark.layout_panel.matrix', side_effect=lambda p: identities[str(p)]), \
                 patch('tools.benchmark.screen.counter_tool'), \
                 patch('tools.benchmark.screen.measurement_lock', side_effect=lambda *a: nullcontext()), \
                 patch('tools.benchmark.screen.wait_for_builds', return_value=[]), \
                 patch('tools.benchmark.screen.check_load'), \
                 patch('tools.benchmark.screen.launch_preflight', return_value=[]), \
                 patch('tools.benchmark.layout_panel.held_out_cases', return_value=[
                     screen.CaseSpec(PROGRAM, 'whole', folder/'case.js')]), \
                 patch('tools.benchmark.layout_panel.measure', side_effect=take), \
                 patch('tools.benchmark.layout_evidence.replay', side_effect=lambda *a, **k: replays.append(k)):
                monitor.return_value.snapshot.return_value = {}
                initial = measure_panel(Path('base000'), Path('cand000'), folder/'out', 30, held=True)
                with self.assertRaisesRegex(ValueError, 'exact natural screen'):
                    measure_panel(Path('base000'), Path('cand000'), folder/'out', 30, complete=True)
                complete = measure_panel(Path('base000'), Path('cand000'), folder/'out', 30, held=True, complete=True)
            self.assertEqual(calls, [(f'/tmp/cand000/v{k}/qjs', 30) for k in range(6)])
            self.assertEqual(replays, [{'held': True}])
            self.assertTrue(complete['held_out'])
            self.assertEqual(len(complete['panels']), 6)
            self.assertEqual(complete['panels'][0], initial['panels'][0])
            self.assertTrue((folder/'out/complete.json').is_file())


if __name__ == '__main__':
    unittest.main()
