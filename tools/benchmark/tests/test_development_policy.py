"""Development is selectable evidence; held-out remains independent acceptance."""
import copy
import dataclasses
import json
import tempfile
import unittest
from argparse import Namespace
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from tools.benchmark import bundle, compare, formal_case_context
from tools.benchmark.development import opportunity_ids, MANIFEST_PATH, load_development_manifest
from tools.benchmark.held_out import opportunity_ids as held_ids
from tools.benchmark.layout_identity import digest
from tools.benchmark.layout_policy import load_policy
from tools.benchmark.layout_rule import evaluate
from tools.benchmark.tests.test_layout_rule import policy, packet, change
from tools.benchmark.tests.decision_fixtures import summary, internal, external, development


def extended_policy():
    p = policy()
    p['development_manifest_sha256'] = digest(MANIFEST_PATH)
    p['lanes']['development'] = 1.01
    p['cases'].update({n: {'wall_band': 1.02, 'instruction_band': 1.002} for n in opportunity_ids()})
    return p


class DevelopmentRuleTests(unittest.TestCase):
    def setUp(self):
        self.policy = extended_policy()
        self.data = packet(self.policy)
        self.target = 'development/papaparse/csv'
        self.control = 'development/diff/compare'

    def test_development_target_requires_payoff_all_pairs_and_panel_aggregate(self):
        data = change(self.data, self.target, wall=.90)
        self.assertTrue(evaluate(self.policy, data, [self.target], .97, panel=[data]*6, panel_required=True)['passed'])
        for values in ([.99]*5, [.90]*4 + [1.]):
            bad = change(data, self.target, wall=values)
            self.assertFalse(evaluate(self.policy, bad, [self.target], .97)['passed'])
        with self.assertRaisesRegex(ValueError, 'unknown layout target'):
            evaluate(policy(), packet(), [self.target], .97)

    def test_non_target_development_instruction_floor_and_hard_cap(self):
        for value, label in ((1.01, 'pass'), (1.02, 'control event'), (1.04, 'reject')):
            data = change(self.data, self.control, instructions=value)
            result = evaluate(self.policy, data, [], .97)
            self.assertEqual(result['cases'][self.control]['label'], label)
        self.policy['cases'][self.control]['instruction_band'] = 1.025
        data = change(self.data, self.control, instructions=1.02)
        self.assertTrue(evaluate(self.policy, data, [], .97)['passed'])

    def test_persistent_development_wall_cost_needs_all_six(self):
        data = change(self.data, self.control, wall=1.03)
        self.assertEqual(evaluate(self.policy, data, [], .97)['label'], 'wall event')
        self.assertTrue(evaluate(self.policy, data, [], .97, panel=[data]*5 + [self.data])['passed'])
        self.assertTrue(any('persistent' in f for f in evaluate(self.policy, data, [], .97, panel=[data]*6)['failures']))

    def test_development_lane_cannot_hide_in_a_faster_portfolio(self):
        data = copy.deepcopy(self.data)
        for row in data['cases']:
            row['ratios']['wall_ns'] = [1.015 if row['id'].startswith('development/') else .90]*5
        result = evaluate(self.policy, data, [], .97)
        self.assertLess(result['lanes']['0/portfolio']['ratio'], 1.)
        self.assertTrue(any('development: wall lane' in f for f in result['failures']))

    def test_held_out_never_becomes_a_selection_target(self):
        self.assertFalse(opportunity_ids() & held_ids())
        for name in held_ids():
            with self.assertRaisesRegex(ValueError, 'unknown layout target'):
                evaluate(self.policy, self.data, [name], .97)

    def test_v2_policy_bytes_and_inventory_remain_loadable(self):
        path = Path('benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v2.json')
        old = load_policy(path, digest(path), live=False)
        self.assertNotIn('development', old['lanes'])
        self.assertEqual(len(old['cases']), 76)
        self.assertEqual(digest(path), '22a1907dae06eb963dcb25592a05973e3bd69153e3c23a70ddec08ef64f07ca6')

    def test_v3_policy_binds_manifest_and_exact_inventory(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'policy.json'
            path.write_text(json.dumps(self.policy))
            load_policy(path, digest(path), live=False)
            for key in ('development_manifest_sha256', 'cases'):
                bad = copy.deepcopy(self.policy)
                if key == 'cases':
                    bad[key].pop(self.target)
                else:
                    bad[key] = 'a'*64
                path.write_text(json.dumps(bad))
                with self.assertRaisesRegex(ValueError, 'development'):
                    load_policy(path, digest(path), live=False)


class DevelopmentBundleTests(unittest.TestCase):
    def test_new_bundle_seals_and_replays_all_development_artifacts(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'summary.json').write_text('{}')
            for name in (*bundle.ARTIFACTS, *bundle.DEVELOPMENT_ARTIFACTS):
                (root / name).write_text('{}')
            value = bundle.seal(root)
            self.assertEqual(value['bundle']['schema_version'], 2)
            bundle.verify(root/'summary.json', root/'report.json', root/'external-report.json', root/'sentinel-report.json')
            self.assertEqual(bundle.development_report_path(root/'summary.json'), root/'development-report.json')
            with self.assertRaisesRegex(ValueError, 'sealed comparison'):
                bundle.development_report_path(root/'summary.json', root/'other.json')
            (root/'development-raw.jsonl').write_text('changed')
            with self.assertRaisesRegex(ValueError, 'artifact changed'):
                bundle.verify(root/'summary.json', root/'report.json', root/'external-report.json', root/'sentinel-report.json')

    def test_four_lanes_run_in_both_formal_schedules(self):
        engines = summary('c'*40, 'a'*40)['engines']
        dev = development(engines)
        for suite in dev['suites']:
            suite.update(complete_comparison=True, complete_base_comparison=True)
        reports = {'broad': internal(engines), 'sentinel': internal(engines, lane='sentinel'),
                   'external': external(engines), 'development': dev}
        names = {'broad':'report.json', 'sentinel':'sentinel-report.json', 'external':'external-report.json', 'development':'development-report.json'}
        outputs = []
        for concurrent in (False, True):
            with tempfile.TemporaryDirectory() as tmp:
                args = Namespace(output_dir=Path(tmp), development=True, concurrent_lanes=concurrent)
                def lane(name, args, *unused):
                    (args.output_dir/names[name]).write_text(json.dumps(reports[name]))
                    return reports[name]
                with patch.object(compare, '_measure_lane', side_effect=lane), patch.object(compare, 'markdown_for', return_value=''), \
                     patch.object(compare, 'render_markdown', return_value=''):
                    outputs.append(compare._measure(args, None, None, None, None))
        self.assertEqual([p['decision_readiness'] for p in outputs], ['ready_for_unit_gates']*2)
        self.assertTrue(all('development-report.json' in p['bundle']['artifacts'] for p in outputs))

    def test_npm_formal_continuation_checks_pinned_checksum(self):
        from tools.benchmark.tests.test_formal_continuation import FormalContinuationTests
        helper = FormalContinuationTests()
        ctx = helper.context()
        ctx.update(development=True, case=SimpleNamespace(expected_checksum='12345678'))
        result = dataclasses.replace(helper.result(), stdout='CHECKSUM 12345678\n__QJS_EXTERNAL_OK__\n__QJS_EXTERNAL_OK__\n')
        self.assertEqual(formal_case_context.validate_result(ctx, 'base', result), 50)
        for output in ('CHECKSUM abcdef00\n__QJS_EXTERNAL_OK__\n__QJS_EXTERNAL_OK__\n', '__QJS_EXTERNAL_OK__\n__QJS_EXTERNAL_OK__\n'):
            with self.assertRaisesRegex(ValueError, 'formal continuation sample'):
                formal_case_context.validate_result(ctx, 'base', dataclasses.replace(result, stdout=output))

    def test_workers_collect_both_whole_process_lanes_in_the_same_pool(self):
        engines = summary('c'*40, 'a'*40)['engines']
        reports = {'broad':internal(engines), 'sentinel':internal(engines, lane='sentinel'),
                   'external':external(engines), 'development':development(engines)}
        class Internal:
            def __init__(self, *args): pass
            def measure_shard(self, *args): pass
            def merge_shards(self, rows): return True
            def close(self): pass
        class Whole:
            def __init__(self, *args, lane=None, **kwargs):
                self.lane = 'development' if lane is not None else 'external'
                self.cases = [(None, SimpleNamespace(id=self.lane), None, None)]
            def capability(self, i):
                return [{'duration_ns':1, 'phase':'capability'}]*3
            def measurement(self, i, capability):
                assert len(capability) == 3
                return [{'duration_ns':1, 'phase':'measurement'}]*3
            def finish(self, records, output):
                assert len(records) == 6
                assert [r['phase'] for r in records] == ['capability']*3 + ['measurement']*3
                return reports[self.lane]
            def close(self): pass
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            path = root/'source.json'
            path.write_text('{}')
            manifest = SimpleNamespace(path=path, cases=[])
            args = Namespace(output_dir=root, workers=4, blocks=30, seed=0,
                             external_cache=root, candidate=root, base=root, quickjs_ng=root, development=True)
            with patch.object(compare, 'ShardedRun', Internal), patch.object(compare, 'PreviewRun', Whole), \
                 patch.object(compare, '_internal_report', side_effect=lambda lane,*args:reports[lane]):
                result = compare._measure_with_workers(args, {'broad':manifest, 'sentinel':manifest},
                                                       {'broad':[], 'sentinel':[]})
            self.assertEqual(set(result), {'broad', 'sentinel', 'external', 'development'})


class DevelopmentQualificationTests(unittest.TestCase):
    def test_qualification_keeps_parent_and_held_out_budgets_and_requires_lane(self):
        from tools.benchmark import layout_development_qualification as qualification
        from tools.benchmark.layout_policy import geomean
        import statistics
        parent = policy()
        parent['precision'] = {'cases': {}, 'held_out': {}}
        seed = extended_policy()
        pilot = {'host': parent['host'], 'pairs': 30, 'cases': [
            {'id': n, 'ratios': {'wall_ns': [1.]*30, 'instructions': [1.]*30}} for n in opportunity_ids()]}
        formal = {n: [1.]*30 for n in seed['cases']}
        def fast(values):
            ratio = geomean([statistics.median(v) for v in values.values()])
            return {'ratio': ratio, 'confidence_interval': {'lower': ratio*.999, 'upper': ratio*1.001},
                    'relative_half_width': .0011, 'precise': True, 'blocks': 30}
        with patch.object(qualification, 'analyze', side_effect=fast):
            good = qualification.qualify(parent, seed, pilot, formal)
            self.assertTrue(good['qualified'])
            self.assertEqual(good['held_out'], parent['held_out'])
            self.assertEqual(good['evidence'] if 'evidence' in good else {}, parent.get('evidence', {}))
            self.assertEqual(good['precision']['held_out'], parent['precision']['held_out'])
            bad_formal = {n: [1.02]*30 if n.startswith('development/') else [.9]*30 for n in formal}
            bad = qualification.qualify(parent, seed, pilot, bad_formal)
            self.assertFalse(bad['qualified'])
            self.assertTrue(any('development: formal' in f for f in bad['failures']))
            self.assertEqual(bad['lanes'], seed['lanes'])
            changed = copy.deepcopy(seed)
            changed['cases'][next(iter(parent['cases']))]['wall_band'] *= 2
            with self.assertRaisesRegex(ValueError, 'changes parent'):
                qualification.qualify(parent, changed, pilot, formal)

    def test_development_timeout_manifest_uses_reference_rule_and_refuses_override(self):
        from tools.benchmark.external_timeouts import derive
        original = json.loads(MANIFEST_PATH.read_text())
        rows = [{'phase':'measurement', 'role':'quickjs-ng', 'status':'ok',
                 'binary_sha256':'a'*64, 'suite_id':s['id'], 'case_id':c['id'],
                 'block':b, 'duration_ns':20_000_000_000}
                for s in original['suites'] for c in s['cases'] for b in range(30)]
        provenance = {'job':'reference', 'raw_sha256':'b'*64, 'summary_sha256':'c'*64,
                      'reference_revision':'d'*40, 'reference_binary_sha256':'a'*64,
                      'blocks':30, 'selection':'latest-valid-canonical-NG'}
        new = copy.deepcopy(original)
        new.update(schema_version=2, preview_id='quickjs-development-preview-v2')
        new['measurement']['timeout_policy'] = derive(rows, provenance=provenance)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'manifest.json'
            path.write_text(json.dumps(new))
            manifest = load_development_manifest(path)
            self.assertEqual(set(manifest.measurement.case_timeouts.values()), {80.})
            case = next(iter(new['measurement']['timeout_policy']['cases']))
            new['measurement']['timeout_policy']['cases'][case]['timeout_seconds'] = 81
            path.write_text(json.dumps(new))
            with self.assertRaisesRegex(ValueError, 'reference derivation'):
                load_development_manifest(path)


if __name__ == '__main__':
    unittest.main()
