"""Case containment, replay and aggregate refusal without real measurements."""
import copy
import dataclasses
import itertools
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.benchmark import screen
from tools.benchmark.case_health import CaseHealthFailure
from tools.benchmark.case_schedule import collect, verify
from tools.benchmark.layout_evidence import replay
from tools.benchmark.layout_identity import digest
from tools.benchmark.layout_rule import evaluate
from tools.benchmark.pair_disturbance import verify_run
from tools.benchmark.pair_recording import measure
from tools.benchmark.screen_replay import replay_case
from tools.benchmark.tests.test_pair_disturbance import snapshot


class Monitor:
    def snapshot(self):
        return snapshot(end=1e20)


def output(n):
    return 'QJS_BENCH_RESULT ' + json.dumps({'operations': n, 'checksum': n})


def recorded(case, candidate, base, pairs, tool, timeout, path, monitor, **kwargs):
    ticks = itertools.count(10)
    slow = False

    def take(tool, argv, timeout):
        nonlocal slow
        n = int(argv[-1]) if case.kind == 'internal' else 1
        wall = n * 100_000_000
        if case.id == 'broad/slow' and argv[0] == str(candidate) and n == 1 and not slow:
            wall = 300_000_000
            slow = True
        return screen.Sample(n*10, 0, wall, wall, output(n) if case.kind == 'internal' else 'ok')

    with patch('tools.benchmark.screen.calibrate', return_value=1), \
            patch('tools.benchmark.pair_recording.time.time', side_effect=lambda: next(ticks)), \
            patch('tools.benchmark.screen.sample', side_effect=take):
        return measure(case, candidate, base, pairs, tool, timeout, path, monitor, **kwargs)


def cases():
    return [screen.CaseSpec('broad/slow', 'internal', Path('/work'), 'slow'),
            screen.CaseSpec('broad/good', 'internal', Path('/work'), 'good'),
            screen.CaseSpec('external/s/good', 'whole', Path('/work'))]


def panel(root, workers=4):
    from tools.benchmark.tests.test_layout_evidence import fixture
    seed = root / 'seed'
    seed.mkdir()
    e, policy, _, _ = fixture(seed)
    target = root / 'collection'
    target.mkdir()
    binaries = {k: {r: Path(e[r]['variants'][k]['path']) for r in ('base', 'candidate')} for k in range(6)}
    with patch('tools.benchmark.screen.counter_tool'):
        packets, scheduling = collect(cases(), list(range(6)), binaries, 5, None, 1,
                                      target, Monitor(), workers, measure=recorded)
    e.update(panels=packets, **scheduling)
    policy.update(cases={c.id: {} for c in cases()}, measurement_schedule=scheduling['measurement_schedule'])
    files = sorted(target.glob('variant*/*.json'))
    e['case_files'] = {str(p.relative_to(target)): digest(p) for p in files}
    e['disturbance_runs'] = [{'variants': list(range(6)), 'load': Monitor().snapshot(),
                             'counts': verify_run([json.loads(p.read_text()) for p in files], Monitor().snapshot())}]
    path = target / 'panel.json'
    path.write_text(json.dumps(e))
    return e, policy, path


class HealthTests(unittest.TestCase):
    def test_slow_correct_n_retains_signed_increment_and_all_raw_samples(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'raw.json'
            result = recorded(cases()[0], Path('/cand'), Path('/base'), 5, None, 1, path, Monitor())
            raw = json.loads(path.read_text())
            self.assertEqual(len(raw['attempts']), 5)
            self.assertEqual(len(path.with_suffix('.jsonl').read_text().splitlines()), 20)
            self.assertEqual(raw['valid_attempt_indices'], list(range(5)))
            self.assertEqual(raw['rounds'][0]['attempt_count'], 5)
            self.assertTrue(raw['complete'])  # inventory is complete, analysis is not
            self.assertNotIn('ratios', raw)
            self.assertNotIn('costs', raw)
            self.assertEqual(result.ratios, {})
            inc = [r for r in raw['health']['increments'] if r['role'] == 'candidate'][0]
            self.assertEqual((inc['wall_ns'], inc['user_ns'], inc['instructions']), (-100_000_000, -100_000_000, 10))
            self.assertTrue(inc['load_valid'])
            self.assertEqual(raw['health']['process_distribution']['candidate/1/user_ns']['max'], 300_000_000)
            with self.assertRaises(CaseHealthFailure):
                replay_case(raw, 5, {'base': '/base', 'candidate': '/cand'})
            self.assertIsNone(replay_case(raw, 5, {'base': '/base', 'candidate': '/cand'}, allow_health=True))
            for change in ('increment', 'selection', 'ratio', 'remove', 'complete'):
                bad = copy.deepcopy(raw)
                if change == 'increment': bad['health']['increments'][0]['wall_ns'] += 1
                if change == 'selection': bad['valid_attempt_indices'].pop()
                if change == 'ratio': bad['ratios'] = {'wall_ns': [1.]*5}
                if change == 'remove': bad.pop('health')
                if change == 'complete': bad['complete'] = False
                with self.assertRaises(ValueError, msg=change):
                    replay_case(bad, 5, {'base': '/base', 'candidate': '/cand'}, allow_health=True)

    def test_all_cases_and_layouts_finish_serial_and_forked_no_aggregate(self):
        for workers in (1, 4):
            with self.subTest(workers=workers), tempfile.TemporaryDirectory() as tmp:
                e, policy, path = panel(Path(tmp), workers)
                raws = [json.loads(p.read_text()) for p in sorted(path.parent.glob('variant*/*.json'))]
                self.assertEqual(len(raws), 18)
                self.assertEqual(sum(len(r['attempts']) for r in raws), 90)
                verify(e, [c.id for c in cases()], raws)
                replay(path, policy)
                with patch('tools.benchmark.layout_rule.geomean', side_effect=AssertionError('aggregate')):
                    result = evaluate(policy, e['panels'][0], ['broad/good'], .97, panel=e['panels'])
                self.assertEqual(result['label'], 'incomplete')
                self.assertFalse(result['passed'])
                self.assertFalse(result['complete'])
                self.assertTrue(result['inventory_complete'])
                self.assertEqual(result['lanes'], {})
                self.assertEqual(result['cases']['0/broad/good'], {'collection': 'complete', 'analysis': 'complete'})
                self.assertEqual(result['cases']['0/broad/slow'], {'collection': 'complete', 'analysis': 'failed'})
                self.assertEqual(len(result['case_health']), 7)  # natural plus its six-layout panel
                text = screen.render([dataclasses.make_dataclass('Row', ['id', 'iterations', 'ratios', 'health'])(
                    'broad/slow', 1, {}, next(r['health'] for r in raws if 'health' in r))], 5, False)
                self.assertNotIn('geomean', text)

    def test_decision_verifier_stops_before_formal_statistics(self):
        from tools.benchmark.layout_formal import evaluate_formal
        with tempfile.TemporaryDirectory() as tmp:
            e, policy, path = panel(Path(tmp))
            unit = {'layout_gate': {'policy_path': 'unused', 'policy_sha256': 'a'*64,
                    'panel_pairs': 5, 'held_out_pairs': 30, 'mechanism_class': 'ordinary',
                    'memory_wall_targets': [], 'panel_required': False},
                    'fast_gate': {'target_ids': ['broad/good'], 'target_max_candidate_over_base': .97}}
            engines = {r: {'source_revision': e[r]['revision'], 'binary_sha256': e[r]['variants'][0]['sha256']}
                       for r in ('base', 'candidate')}
            with patch('tools.benchmark.layout_formal.load_policy', return_value=policy), \
                    patch('tools.benchmark.layout_formal.with_held_out_successor', return_value=(policy, None)), \
                    patch('tools.benchmark.layout_formal.analyze', side_effect=AssertionError('aggregate')):
                verdict, missing, failed, hashes = evaluate_formal(unit, {'engines': engines}, {}, {}, {},
                    'promotion', path, None)
            self.assertEqual(verdict['label'], 'incomplete')
            self.assertTrue(missing)
            self.assertFalse(failed)
            self.assertEqual(hashes['layout_panel_sha256'], digest(path))
            self.assertNotIn('formal_statistics', verdict)

    def test_interrupted_inventory_and_changed_health_are_refused(self):
        for change in ('case', 'file', 'layout', 'health', 'hash'):
            with self.subTest(change=change), tempfile.TemporaryDirectory() as tmp:
                e, policy, path = panel(Path(tmp))
                if change == 'case': e['panels'][2]['cases'].pop()
                if change == 'file': e['case_files'].pop(next(iter(e['case_files'])))
                if change == 'layout': e['panels'].pop()
                if change == 'health': e['panels'][0]['cases'][0]['health']['increments'][0]['wall_ns'] += 1
                if change == 'hash': e['case_files'][next(iter(e['case_files']))] = '0'*64
                path.write_text(json.dumps(e))
                with self.assertRaises(ValueError): replay(path, policy)

    def test_timeout_exit_checksum_and_calibration_failures_are_terminal(self):
        from tools.benchmark.process import ProcessResult
        for kind in ('timeout', 'exit', 'checksum', 'calibration'):
            with self.subTest(kind=kind), tempfile.TemporaryDirectory() as tmp:
                path = Path(tmp) / 'raw.json'
                spec = cases()[0] if kind == 'calibration' else screen.CaseSpec('external/s/c', 'whole', Path('/work'), expected_stdout='ok')
                result = ProcessResult('now', 1, 2, 1, -9 if kind == 'timeout' else 2,
                                       kind == 'timeout', 'partial', 'error', False, False)
                def take(*args):
                    if kind == 'checksum': return screen.Sample(10, 0, 10, 100, 'wrong')
                    raise screen.ProcessFailure(result, 'failed')
                with patch('tools.benchmark.screen.sample', side_effect=take):
                    value = measure(spec, Path('/cand'), Path('/base'), 5, None, 1, path, Monitor())
                raw = json.loads(path.read_text())
                self.assertFalse(raw['complete'])
                self.assertEqual(value.health['collection'], 'failed')
                self.assertEqual(len(path.with_suffix('.jsonl').read_text().splitlines()), 1)
                with self.assertRaises(CaseHealthFailure):
                    replay_case(raw, 5, {'base': '/base', 'candidate': '/cand'})
                replay_case(raw, 5, {'base': '/base', 'candidate': '/cand'}, allow_health=True)
                if kind == 'timeout':
                    self.assertEqual(raw['attempts'][0]['samples'][0]['process_failure']['stdout'], 'partial')
                self.assertNotIn('ratios', raw)

    def test_captured_t072_packet_stays_a_failure_without_rewriting_it(self):
        from tools.benchmark.screen_replay import replay_attempts
        path = Path(__file__).parent / 'fixtures/case-health/t072-attempt2-local-read.json'
        self.assertEqual(digest(path), 'a80ea014415d92c87b29ef8184aca85ab6f08880aabe3275324fba45f6a00497')
        raw = json.loads(path.read_text())
        paths = {role: raw['attempts'][0]['samples'][index]['argv'][0]
                 for role, index in (('base', 0), ('candidate', 2))}
        with self.assertRaisesRegex(ValueError, 'incomplete pair recovery'):
            replay_case(raw, 5, paths)
        # Analyze the sealed attempts prospectively in memory. The captured
        # historical packet and T072's standing remain unchanged.
        observed = {**raw, 'complete': True}
        with self.assertRaises(CaseHealthFailure) as failed:
            replay_attempts(observed, 5, paths)
        increments = failed.exception.health['increments']
        bad = next(row for row in increments if row['attempt_index'] == 0 and row['role'] == 'candidate')
        self.assertEqual(bad['wall_ns'], -49_952_756)
        self.assertEqual(bad['user_ns'], -50_000_000)
        self.assertEqual(bad['instructions'], 4_544_284_580)
        self.assertTrue(bad['load_valid'])
        observed.pop('error')
        observed['health'] = failed.exception.health
        replay_case(observed, 5, paths, allow_health=True)
        self.assertEqual(raw['complete'], False)

    def test_load_invalid_slow_pair_is_replaced_only_for_load(self):
        class Loaded:
            def snapshot(self): return snapshot('00:00:10 200 daemon\n', 1e20)
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / 'raw.json'
            result = recorded(cases()[0], Path('/cand'), Path('/base'), 5, None, 1, path, Loaded())
            raw = json.loads(path.read_text())
            self.assertNotIn('health', raw)
            self.assertEqual(len(raw['attempts']), 6)
            self.assertEqual(raw['valid_attempt_indices'], [1, 2, 3, 4, 5])
            self.assertEqual(len(path.with_suffix('.jsonl').read_text().splitlines()), 24)
            self.assertEqual(result.ratios['wall_ns'], [1.]*5)
            replay_case(raw, 5, {'base': '/base', 'candidate': '/cand'})

    def test_short_and_nonpositive_wall_or_instruction_increment_never_retries(self):
        for metric, large in (('wall_ns', 140_000_000), ('wall_ns', 100_000_000),
                              ('wall_ns', 90_000_000), ('instructions', 10), ('instructions', 5)):
            with self.subTest(metric=metric, large=large), tempfile.TemporaryDirectory() as tmp:
                path = Path(tmp) / 'raw.json'
                def take(tool, argv, timeout):
                    n = int(argv[-1])
                    values = {'instructions': n*10, 'cycles': 0, 'user_ns': n*100_000_000,
                              'wall_ns': n*100_000_000, 'stdout': output(n)}
                    if argv[0] == '/cand' and n == 2: values[metric] = large
                    return screen.Sample(**values)
                with patch('tools.benchmark.screen.sample', side_effect=take):
                    result = measure(cases()[0], Path('/cand'), Path('/base'), 5, None, 1,
                                     path, Monitor(), iterations=1)
                self.assertEqual(result.health['analysis'], 'failed')
                raw = json.loads(path.read_text())
                self.assertEqual(len(raw['attempts']), 5)
                self.assertEqual(len(raw['rounds']), 1)
                replay_case(raw, 5, {'base': '/base', 'candidate': '/cand'}, allow_health=True)

    def test_process_failures_continue_other_cases_and_requested_layouts(self):
        from tools.benchmark.process import ProcessResult
        for workers in (1, 4):
            for reason in ('timeout', 'checksum'):
                with self.subTest(workers=workers, reason=reason), tempfile.TemporaryDirectory() as tmp:
                    inventory = [screen.CaseSpec('external/s/' + name, 'whole', Path('/work'), expected_stdout='ok')
                                 for name in ('bad', 'good', 'last')]
                    result = ProcessResult('now', 1, 2, 1, -9, True, 'partial', 'timeout', False, False)
                    def take(case, candidate, base, pairs, tool, timeout, path, monitor, **kwargs):
                        if case.id != 'external/s/bad':
                            return recorded(case, candidate, base, pairs, tool, timeout, path, monitor, **kwargs)
                        def sample(*args):
                            if reason == 'checksum': return screen.Sample(10, 0, 10, 100, 'wrong')
                            raise screen.ProcessFailure(result, 'timeout')
                        with patch('tools.benchmark.screen.sample', side_effect=sample):
                            return measure(case, candidate, base, pairs, tool, timeout, path, monitor, **kwargs)
                    binaries = {k: {'base': Path('/base'), 'candidate': Path('/cand')} for k in range(6)}
                    with patch('tools.benchmark.screen.counter_tool'):
                        packets, scheduling = collect(inventory, list(range(6)), binaries, 5, None, 1,
                                                      Path(tmp), Monitor(), workers, measure=take)
                    raws = [json.loads(p.read_text()) for p in Path(tmp).glob('variant*/*.json')]
                    verify(scheduling, [c.id for c in inventory], raws)
                    self.assertEqual(len(raws), 18)
                    self.assertEqual(sum('health' in raw for raw in raws), 6)
                    for raw in raws:
                        replay_case(raw, 5, {'base': '/base', 'candidate': '/cand'}, allow_health=True)
                    for packet in packets:
                        self.assertEqual(packet['readiness'], 'incomplete')
                        self.assertTrue(packet['inventory_complete'])
                        self.assertEqual(packet['cases'][2]['ratios']['wall_ns'], [1.]*5)

    def test_counter_wrapper_failure_is_not_an_engine_health_failure(self):
        from tools.benchmark.process import ProcessResult
        tool = screen.counter_tool('Darwin', trust_cycles=False)
        result = ProcessResult('now', 1, 2, 1, 1, False, '', 'counter tool denied', False, False)
        with self.assertRaises(screen.CountersUnavailable):
            screen.sample(tool, ['/cand', '--raw', '/work'], 1, runner=lambda *_: result)
        executed = dataclasses.replace(result, stderr='123 instructions retired\n')
        with self.assertRaises(screen.ProcessFailure) as failed:
            screen.sample(tool, ['/cand', '--raw', '/work'], 1, runner=lambda *_: executed)
        self.assertEqual(failed.exception.result, executed)

    def test_diagnostic_layout_replay_reconstructs_health_from_hashed_raw(self):
        import contextlib
        import io
        from types import SimpleNamespace
        from tools.benchmark.layout_replay import run
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            e, policy, path = panel(root)
            bands = root / 'bands.json'
            bands.write_text(json.dumps(policy))
            args = SimpleNamespace(policy=None, diagnostic_bands=bands, screen=path,
                                   target=[], payoff=.97, output=root/'replay.json')
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(run(args), 0)
            result = json.loads(args.output.read_text())
            self.assertEqual(result['label'], 'incomplete')
            self.assertFalse(result['claim_eligible'])
            self.assertFalse(result['historical_standing_changed'])
            self.assertEqual(result['lanes'], {})
            raw = path.parent/'variant0/broad--slow.json'
            broken = json.loads(raw.read_text())
            broken['health']['increments'][0]['wall_ns'] += 1
            raw.write_text(json.dumps(broken))
            e['case_files'][str(raw.relative_to(path.parent))] = digest(raw)
            path.write_text(json.dumps(e))
            with self.assertRaises(ValueError): run(args)

    def test_infrastructure_errors_propagate_instead_of_becoming_health(self):
        for failure in (OSError('disk'), screen.CountersUnavailable('no counters'), screen.ScreenError('spawn failed')):
            with self.subTest(failure=failure), tempfile.TemporaryDirectory() as tmp:
                path = Path(tmp) / 'raw.json'
                with patch('tools.benchmark.screen.sample', side_effect=failure):
                    with self.assertRaises(type(failure)):
                        measure(cases()[0], Path('/cand'), Path('/base'), 5, None, 1, path, Monitor())
                self.assertNotIn('health', json.loads(path.read_text()))


if __name__ == '__main__':
    unittest.main()
