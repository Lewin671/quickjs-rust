"""Job status, failure durability and replay bindings for T017 review fixes."""
import contextlib
import json
import os
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from tools.benchmark import screen
from tools.benchmark.case_health import CaseHealthFailure, mark_readiness
from tools.benchmark.layout_identity import digest
from tools.benchmark.pair_recording import measure
from tools.benchmark.process import ProcessResult
from tools.benchmark.screen_replay import replay_case
from tools.benchmark.tests.test_case_health import Monitor, cases, recorded


FAILURE = ProcessResult('now', 1, 2, 1, -9, True, 'partial', 'timeout', False, False)


def development_case(root):
    from tools.benchmark.development import load_development_manifest, MANIFEST_PATH
    manifest = load_development_manifest(MANIFEST_PATH)
    suite = manifest.suites[0]
    case = suite.cases[0]
    work = root/'work.js'
    work.write_text('original')
    return screen.CaseSpec(f'development/{suite.id}/{case.id}', 'whole', work,
        expected_stdout=f'CHECKSUM {case.expected_checksum}\n' + screen._EXPECTED_STDOUT_BY_ROLE['candidate'],
        timeout_seconds=manifest.measurement.timeout_for(suite.id, case.id))


def crash(*args):
    raise screen.ProcessFailure(FAILURE, 'timeout')


def health(root):
    path = root/'failed.json'
    return recorded(cases()[0], Path('/cand'), Path('/base'), 5, None, 1, path, Monitor()).health


def calibration(root, failed):
    """Run the artifact/CLI owner with only host/process collection replaced."""
    # Load the continuation owner before replacing its load verifier; otherwise
    # its module-level import would retain the mock after this test finishes.
    from tools.benchmark import layout_calibration, layout_continuation
    binary = root/'source'
    binary.write_bytes(b'binary')
    state = health(root) if failed else None
    row = {'id': 'broad/slow', 'ratios': {} if failed else {m: [1.]*30 for m in ('wall_ns', 'instructions')}}
    if failed: row['health'] = state
    def collect(cases, variants, binaries, pairs, tool, timeout, output, monitor, workers):
        (output/'variant0').mkdir()
        return [mark_readiness({'pairs': 30, 'cases': [row]})], {'collection_seconds': 1}
    out = root/'out'
    argv = ['calibration', '--binary', str(binary), '--sha256', digest(binary), '--lane', 'portfolio', '--output', str(out)]
    with contextlib.ExitStack() as stack:
        for name, kwargs in {
            'tools.benchmark.pair_disturbance.quieting_preflight': {'return_value': {}},
            'tools.benchmark.pair_disturbance.Monitor.from_queue': {'return_value': SimpleNamespace(snapshot=lambda: {})},
            'tools.benchmark.pair_disturbance.verify_run': {'return_value': {}},
            'tools.benchmark.layout_calibration.host_identity': {'return_value': {}},
            'tools.benchmark.case_schedule.collect': {'side_effect': collect},
            'tools.benchmark.screen.counter_tool': {'return_value': None},
            'tools.benchmark.screen.measurement_lock': {'side_effect': lambda *_: contextlib.nullcontext()},
            'tools.benchmark.screen.wait_for_builds': {'return_value': []},
            'tools.benchmark.screen.check_load': {'return_value': None},
            'tools.benchmark.screen.launch_preflight': {'return_value': []},
            'tools.benchmark.screen.resolve_cases': {'return_value': cases()},
            'tools.benchmark.layout_identity.command': {'return_value': 'a'*40},
        }.items(): stack.enter_context(patch(name, **kwargs))
        stack.enter_context(patch('sys.argv', argv))
        status = layout_calibration.main()
    return status, out


class ReviewTests(unittest.TestCase):
    def test_calibration_job_status_and_marker_preserve_healthy_shape(self):
        for failed in (False, True):
            with self.subTest(failed=failed), tempfile.TemporaryDirectory() as tmp:
                status, out = calibration(Path(tmp), failed)
                result = json.loads((out/'screen.json').read_text())
                self.assertEqual(status, 4 if failed else 0)
                if failed:
                    self.assertEqual(result['readiness'], 'incomplete')
                    self.assertTrue(result['inventory_complete'])
                    self.assertFalse((out/'power.json').exists())
                else:
                    self.assertNotIn('readiness', result)
                    self.assertNotIn('inventory_complete', result)
                    self.assertTrue((out/'power.json').exists())

    def test_panel_and_a2_collectors_write_top_level_readiness(self):
        import platform
        from tools.benchmark import layout_panel, layout_development_a2
        for owner in ('panel', 'a2'):
            with self.subTest(owner=owner), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                state = health(root)
                packets = [mark_readiness({'pairs': 5, 'cases': [{'id': 'broad/slow', 'ratios': {}, 'health': state}]})]
                source = root/'source'
                source.write_bytes(b'binary')
                matrix = {'host': {}, 'host_node': platform.node(), 'revision': 'a'*40,
                          'variants': [{'path': str(source), 'sha256': digest(source)} for _ in range(6)]}
                identities = {r: {**matrix, 'variants': [{'path': f'/tmp/{r}/v{k}/qjs'} for k in range(6)]}
                              for r in ('base000', 'cand000')}
                scheduling = {'collection_seconds': 1}
                out = root/'out'
                with contextlib.ExitStack() as stack:
                    for name, kwargs in {
                        'tools.benchmark.pair_disturbance.quieting_preflight': {'return_value': {}},
                        'tools.benchmark.pair_disturbance.Monitor.from_queue': {'return_value': SimpleNamespace(snapshot=lambda: {})},
                        'tools.benchmark.pair_disturbance.verify_run': {'return_value': {}},
                        'tools.benchmark.screen.counter_tool': {'return_value': None},
                        'tools.benchmark.screen.measurement_lock': {'side_effect': lambda *_: contextlib.nullcontext()},
                        'tools.benchmark.screen.wait_for_builds': {'return_value': []},
                        'tools.benchmark.screen.check_load': {'return_value': None},
                        'tools.benchmark.screen.launch_preflight': {'return_value': []},
                        'tools.benchmark.case_schedule.collect': {'return_value': (packets, scheduling)},
                        'tools.benchmark.screen.resolve_cases': {'return_value': cases()},
                    }.items(): stack.enter_context(patch(name, **kwargs))
                    if owner == 'panel':
                        stack.enter_context(patch('tools.benchmark.layout_panel.host_identity', return_value={}))
                        stack.enter_context(patch('tools.benchmark.layout_panel.matrix', side_effect=lambda p: identities[str(p)]))
                        value = layout_panel.measure_panel(Path('base000'), Path('cand000'), out, natural=True)
                        path = out/'panel.json'
                    else:
                        stack.enter_context(patch('tools.benchmark.layout_development_a2.quieting_preflight', return_value={}))
                        stack.enter_context(patch('tools.benchmark.layout_development_a2.verify_run', return_value={}))
                        stack.enter_context(patch('tools.benchmark.layout_development_a2.host_identity', return_value={}))
                        stack.enter_context(patch('tools.benchmark.layout_development_a2.matrix', return_value=matrix))
                        stack.enter_context(patch('tools.benchmark.layout_development_a2.lane_cases', return_value=cases()))
                        path = layout_development_a2.collect(Path('matrix'), out, digest(source), 'portfolio')
                        value = json.loads(path.read_text())
                self.assertEqual(value['readiness'], 'incomplete')
                self.assertTrue(value['inventory_complete'])
                self.assertEqual(json.loads(path.read_text())['readiness'], 'incomplete')

    def test_panel_and_a2_cli_return_distinct_incomplete_status(self):
        from tools.benchmark import layout_panel, layout_development_a2
        for failed in (False, True):
            with self.subTest(failed=failed), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                value = mark_readiness({'panels': [{'cases': [{'health': health(root)}]}]}) if failed else {'panels': []}
                path = root/'a2.json'
                path.write_text(json.dumps(value))
                with patch('sys.argv', ['panel', '--base-matrix', 'base', '--candidate-matrix', 'cand', '--output', str(root)]), \
                        patch('tools.benchmark.layout_panel.measure_panel', return_value=value):
                    self.assertEqual(layout_panel.main(), 4 if failed else 0)
                with patch('sys.argv', ['a2', '--matrix', 'matrix', '--sha256', 'a'*64, '--output', str(root)]), \
                        patch('tools.benchmark.layout_development_a2.collect', return_value=path):
                    self.assertEqual(layout_development_a2.main(), 4 if failed else 0)

    def test_schedule_estimator_pilot_a2_and_speed_preserve_failure_at_job_level(self):
        from tools.benchmark import layout_schedule_calibration as schedule
        for stage in ('estimator', 'pilot', 'a2', 'speed'):
            with self.subTest(stage=stage), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                plan = root/'plan.json'
                plan.write_text(json.dumps({'measurement_schedule': {'kind': 'case-shards-v1', 'workers': 4},
                    'npm_image': 'a'*64, 'portfolio_image': 'b'*64, 'max_load_replacements': 3}))
                seed = root/'seed.json'
                seed.write_text(json.dumps({'calibration_plan': {'path': str(plan), 'sha256': digest(plan)}}))
                failed = mark_readiness({'panels': [{'cases': [{'health': health(root)}]}], 'collection_seconds': 3})
                def collect(*args):
                    # A2: source,target,image,lane,workers; pilot: binary,image,lane,target,workers.
                    target = args[1] if stage == 'a2' else args[3]
                    target.mkdir(parents=True)
                    path = target/'screen.json'
                    path.write_text(json.dumps(failed))
                    return path
                def panel(*args, **kwargs):
                    (root/'panel').mkdir(exist_ok=True)
                    (root/'panel/panel.json').write_text(json.dumps(failed))
                    return failed
                argv = ['schedule', '--plan', str(plan), '--stage', stage, '--inputs', str(root),
                        '--lane', 'portfolio', '--seed', str(seed)]
                with patch.dict(os.environ, BENCH_OUT=str(root), BENCH_QUEUE=str(root)), \
                        patch('sys.argv', argv), \
                        patch('tools.benchmark.layout_calibration.collect', side_effect=collect) as pilot, \
                        patch('tools.benchmark.layout_development_a2.collect', side_effect=collect) as a2, \
                        patch('tools.benchmark.layout_panel.measure_panel', side_effect=panel) as panels:
                    self.assertEqual(schedule.main(), 4)
                payload = json.loads((root/('speed.json' if stage == 'speed' else 'result.json')).read_text())
                self.assertEqual(payload['readiness'], 'incomplete')
                self.assertTrue(payload['inventory_complete'])
                if stage == 'speed':
                    self.assertEqual(panels.call_count, 1)
                    self.assertIsNone(payload['panel_seconds'])
                else:
                    self.assertEqual((a2 if stage == 'a2' else pilot).call_count, 1)  # no health retries

    def test_secondary_failures_keep_raw_packet_before_checks_and_propagate(self):
        for phase in ('monitor', 'replay', 'workload'):
            with self.subTest(phase=phase), tempfile.TemporaryDirectory() as tmp:
                root = Path(tmp)
                path = root/'raw.json'
                spec = development_case(root)
                def take(*args):
                    if phase == 'workload': spec.workload.write_text('changed')
                    crash()
                class CheckedMonitor:
                    def snapshot(self):
                        self.assert_packet()
                        if phase == 'monitor': raise OSError('monitor failed')
                        return Monitor().snapshot()
                    def assert_packet(self):
                        assert path.is_file()
                        assert json.loads(path.read_text())['attempts'][0]['samples'][0]['process_failure']['stdout'] == 'partial'
                with patch('tools.benchmark.screen.sample', side_effect=take), contextlib.ExitStack() as stack:
                    if phase == 'replay':
                        stack.enter_context(patch('tools.benchmark.case_health_replay.process_health', side_effect=ValueError('replay failed')))
                    with self.assertRaisesRegex((ValueError, OSError), 'failed|changed'):
                        measure(spec, Path('/cand'), Path('/base'), 5, None, 1, path, CheckedMonitor())
                raw = json.loads(path.read_text())
                self.assertIn('error', raw)
                self.assertEqual(raw['attempts'][0]['samples'][0]['process_failure']['stdout'], 'partial')
                self.assertEqual(len(path.with_suffix('.jsonl').read_text().splitlines()), 1)
                with self.assertRaises(ValueError): replay_case(raw, 5, {'base': '/base', 'candidate': '/cand'}, allow_health=True)

    def test_failed_development_replay_uses_frozen_stdout_and_identity(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            spec = development_case(root)
            path = root/'raw.json'
            with patch('tools.benchmark.screen.sample', side_effect=crash):
                measure(spec, Path('/cand'), Path('/base'), 5, None, 1, path, Monitor())
            raw = json.loads(path.read_text())
            replay_case(raw, 5, {'base': '/base', 'candidate': '/cand'}, allow_health=True)
            for field, value in (('expected_stdout', 'forged'), ('development_manifest_sha256', '0'*64),
                                 ('timeout_seconds', 1), ('workload_sha256', 'bad')):
                bad = {**raw, field: value}
                with self.subTest(field=field), self.assertRaises(ValueError):
                    replay_case(bad, 5, {'base': '/base', 'candidate': '/cand'}, allow_health=True)

    def test_statistical_consumers_raise_typed_health_instead_of_keyerror(self):
        from tools.benchmark.layout_schedule_bands import estimate
        from tools.benchmark.layout_qualification import replay_calibration
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            failed = {'id': 'broad/slow', 'health': health(root)}
            with self.assertRaises(CaseHealthFailure):
                estimate({(k, 'broad/slow'): failed for k in range(6)}, {}, {'broad/slow'}, set())
            for name in ('base0', 'cand0'):
                (root/name).mkdir()
                (root/name/'qjs').write_bytes(b'binary')
            (root/'identity.json').write_text(json.dumps({'sha256': digest(root/'base0/qjs'),
                'binaries': ['/base', '/cand']}))
            (root/'cases').mkdir()
            (root/'cases/broad--slow.json').write_text(json.dumps(failed))
            path = root/'screen.json'
            path.write_text(json.dumps({'cases': [{**failed, 'ratios': {}}]}))
            with self.assertRaises(CaseHealthFailure): replay_calibration(path, digest(root/'base0/qjs'))

    def test_formal_health_keeps_prior_missing_and_optional_evidence_bindings(self):
        from tools.benchmark.layout_formal import evaluate_formal
        from tools.benchmark.tests.test_layout_formal import FormalTests
        fixture = FormalTests()
        fixture.setUp()
        fixture.unit['layout_gate'].update(panel_pairs=6, held_out_pairs=31, panel_required=True)
        verdict = {'case_health': {'failed': {}}, 'missing': ['case failed'], 'failures': [], 'passed': False}
        with patch('tools.benchmark.layout_formal.load_policy', return_value=fixture.policy), \
                patch('tools.benchmark.layout_formal.with_held_out_successor', return_value=(fixture.policy, 'b'*64)), \
                patch('tools.benchmark.layout_formal.replay', side_effect=[fixture.panels, fixture.held]), \
                patch('tools.benchmark.layout_formal.evaluate', return_value=verdict), \
                patch('tools.benchmark.layout_formal_raw.extract', return_value={'case': [1.]*30}), \
                patch('tools.benchmark.layout_formal.continued_cases', return_value=({}, {'case': 'c'*64})) as continued, \
                patch('tools.benchmark.layout_formal.digest', return_value='a'*64):
            result, missing, _, hashes = evaluate_formal(fixture.unit, {'engines': {}}, {}, {}, {}, 'promotion',
                Path('panel'), Path('held'), Path('bundle'), formal_continuations=[Path('chain')])
        self.assertIn('layout panel differs from frozen pair budget', missing)
        self.assertIn('held-out evidence differs from frozen pair budget', missing)
        self.assertIn('frozen plan requires K=6', missing)
        self.assertEqual(result['missing'], missing)
        self.assertEqual(hashes['held_out_policy_sha256'], 'b'*64)
        self.assertEqual(hashes['formal_continuation_sha256'], {'case': 'c'*64})
        continued.assert_called_once()

    def test_iteration_does_not_auto_complete_mandatory_panel_after_health_failure(self):
        from tools.benchmark import layout_iteration
        from tools.benchmark.tests.test_layout_rule import policy, packet
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            p = policy()
            data = packet(p)
            data['cases'][0].update(health=health(root), ratios={})
            name = data['cases'][0]['id']
            unit = {'base_sha': 'a'*40, 'layout_gate': {'policy_path': 'fake', 'policy_sha256': 'b'*64,
                    'mechanism_class': 'instruction-blind', 'panel_required': True, 'memory_wall_targets': [name]},
                    'fast_gate': {'target_ids': [name], 'target_max_candidate_over_base': .97}}
            with patch('sys.argv', ['iteration', '--plan', 'fake', '--output', str(root/'attempt')]), \
                    patch('tools.benchmark.layout_iteration.load_unit', return_value=(unit, 'c'*64)), \
                    patch('tools.benchmark.layout_iteration.load_policy', return_value=p), \
                    patch('tools.benchmark.layout_identity.command', return_value=''), \
                    patch('tools.benchmark.layout_iteration.build'), \
                    patch('tools.benchmark.layout_iteration.measure_panel', return_value={'panels': [data]}) as measured, \
                    patch('builtins.print'):
                self.assertEqual(layout_iteration.main(), 4)
            measured.assert_called_once()
            result = json.loads((root/'attempt/verdict.json').read_text())
            self.assertEqual(result['label'], 'incomplete')
            self.assertEqual(result['readiness'], 'incomplete')
            self.assertFalse(result['passed'])


if __name__ == '__main__':
    unittest.main()
