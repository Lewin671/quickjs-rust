"""Raw replay, frozen plans and collector invariants for layout decisions."""
import copy
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.benchmark.layout_evidence import replay
from tools.benchmark.layout_identity import OFFSETS, digest
from tools.benchmark.layout_plan import validate
from tools.benchmark.screen_recording import measure
from tools.benchmark import screen


def fixture(folder):
    binaries = {}
    for role in ('base', 'candidate'):
        name = 'base' if role == 'base' else 'cand'
        binaries[role] = {'host': {}, 'host_node': 'test-host', 'static_pass': True,
            'revision': role[0]*40, 'offsets': list(OFFSETS), 'variants': [
                {'index': k, 'pad_bytes': pad, 'path': f'/tmp/{name}/v{k}/qjs', 'sha256': role[0]*64,
                 'audit': {'body_drift': 0, 'address_drift': 0}} for k, pad in enumerate(OFFSETS)]}
    ratios = {'wall_ns': [1.]*5, 'instructions': [1.]*5}
    cases = [{'id': 'external/s/a', 'ratios': ratios}]
    packet = {'pairs': 5, 'cases': cases}
    costs = {role: {'wall_ns': [100]*5, 'instructions': [10]*5} for role in binaries}
    samples = []
    for pair in range(5):
        for role in (('base', 'candidate') if pair % 2 == 0 else ('candidate', 'base')):
            samples.append({'argv': [binaries[role]['variants'][0]['path'], '--raw', '/tmp/case.js'],
                            'sample': {'wall_ns': 100, 'instructions': 10, 'stdout': 'OK\n'}})
    raw = {'id': 'external/s/a', 'kind': 'whole', 'samples': samples, 'costs': costs, 'ratios': ratios}
    path = folder / 'variant0/external--s--a.json'
    path.parent.mkdir()
    path.write_text(json.dumps(raw))
    evidence = {'schema_version': 1, 'artifact_type': 'quickjs-layout-panel',
                'host': {}, 'host_node': 'test-host', 'held_out': False, 'pairs': 5,
                **binaries, 'panels': [packet], 'case_files': {'variant0/external--s--a.json': digest(path)}}
    (folder / 'panel.json').write_text(json.dumps(evidence))
    policy = {'host': {}, 'host_node': 'test-host', 'cases': {'external/s/a': {}}}
    return evidence, policy, raw, path


class EvidenceTests(unittest.TestCase):
    def test_raw_replay_and_engine_binding(self):
        with tempfile.TemporaryDirectory() as work:
            folder = Path(work)
            evidence, policy, raw, path = fixture(folder)
            self.assertEqual(replay(folder / 'panel.json', policy)['panels'], evidence['panels'])
            engines = {role: {'source_revision': role[0]*40, 'binary_sha256': role[0]*64}
                       for role in ('base', 'candidate')}
            replay(folder / 'panel.json', policy, engines)
            engines['base']['binary_sha256'] = '0'*64
            with self.assertRaisesRegex(ValueError, 'formal bundle'):
                replay(folder / 'panel.json', policy, engines)

    def test_tampered_samples_ratios_inventory_paths_host_and_audits_refused(self):
        mutations = [
            lambda e, r: r['samples'][0]['sample'].__setitem__('wall_ns', 99),
            lambda e, r: r['samples'][0]['argv'].__setitem__(0, '/tmp/other'),
            lambda e, r: r['samples'].pop(),
            lambda e, r: e['panels'][0]['cases'].append(e['panels'][0]['cases'][0]),
            lambda e, r: e.__setitem__('host_node', 'another-host'),
            lambda e, r: e['candidate']['variants'][0].__setitem__('path', '/tmp/base/v0/qjs'),
            lambda e, r: e['candidate']['variants'][1]['audit'].__setitem__('body_drift', 1),
        ]
        for mutate in mutations:
            with self.subTest(mutate=mutate), tempfile.TemporaryDirectory() as work:
                folder = Path(work)
                evidence, policy, raw, path = fixture(folder)
                mutate(evidence, raw)
                path.write_text(json.dumps(raw))
                evidence['case_files']['variant0/external--s--a.json'] = digest(path)
                (folder / 'panel.json').write_text(json.dumps(evidence))
                with self.assertRaises(ValueError):
                    replay(folder / 'panel.json', policy)

    def test_equal_character_counts_do_not_accept_different_argv_byte_lengths(self):
        with tempfile.TemporaryDirectory() as work:
            folder = Path(work)
            evidence, policy, raw, path = fixture(folder)
            for variant in evidence['candidate']['variants']:
                variant['path'] = variant['path'].replace('/cand/', '/候选ab/')
            for row in raw['samples']:
                row['argv'][0] = row['argv'][0].replace('/cand/', '/候选ab/')
            raw_path = folder/'variant0/external--s--a.json'
            raw_path.write_text(json.dumps(raw))
            evidence['case_files']['variant0/external--s--a.json'] = digest(raw_path)
            path = folder/'panel.json'
            path.write_text(json.dumps(evidence))
            with self.assertRaisesRegex(ValueError, 'equal length'):
                replay(path, policy)

    def test_frozen_blind_plan_and_budget_are_enforced(self):
        gate = {'policy_path': 'benchmarks/policies/test.json', 'policy_sha256': 'a'*64,
                'mechanism_class': 'instruction-blind', 'memory_wall_targets': ['external/s/a'],
                'panel_required': True, 'panel_pairs': 5, 'held_out_pairs': 30, 'formal_blocks': 30}
        unit = {'layout_gate': gate, 'fast_gate': {'target_ids': ['external/s/a']}}
        validate(unit)
        for key, value in (('panel_required', False), ('memory_wall_targets', []),
                           ('policy_path', '../escape'), ('panel_pairs', 4), ('held_out_pairs', 5)):
            altered = copy.deepcopy(unit)
            altered['layout_gate'][key] = value
            with self.assertRaises(ValueError):
                validate(altered)


class SnapshotTests(unittest.TestCase):
    def test_equal_paths_exclusive_files_and_hashes(self):
        from tools.benchmark.snapshots import SnapshotStore, SnapshotError
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            binary = folder / 'source'
            binary.write_bytes(b'binary')
            store = SnapshotStore(folder, 'test')
            try:
                paths = [store.snapshot_engine(role, binary, digest(binary))
                         for role in ('candidate', 'base', 'quickjs-ng')]
                self.assertEqual(len(set(paths)), 3)
                self.assertEqual(len({len(str(p)) for p in paths}), 1)
                for role, path in zip(('candidate', 'base', 'quickjs-ng'), paths):
                    self.assertTrue(path.name.startswith(role + '-'))
                with self.assertRaises(SnapshotError):
                    store.snapshot_engine('base', binary, digest(binary))
                self.assertEqual(paths[1].read_bytes(), b'binary')
            finally:
                store.cleanup()

class PanelCollectionTests(unittest.TestCase):
    def test_matching_layouts_and_completion_never_remeasure_natural(self):
        from contextlib import nullcontext
        from tools.benchmark.layout_panel import measure_panel
        import platform
        with tempfile.TemporaryDirectory() as work:
            folder = Path(work)
            identities = {}
            for role in ('base000', 'cand000'):
                identities[role] = {'host': {}, 'host_node': platform.node(),
                    'variants': [{'path': f'/tmp/{role}/v{k}/qjs'} for k in range(6)]}
            calls = []
            def take(case, candidate, base, pairs, tool, timeout, output, monitor):
                calls.append((str(candidate), str(base)))
                return screen.CaseResult('external/s/a', 'whole', None,
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
                 patch('tools.benchmark.screen.resolve_cases', return_value=[
                     screen.CaseSpec('external/s/a', 'whole', folder/'case.js')]), \
                 patch('tools.benchmark.layout_panel.measure', side_effect=take), \
                 patch('tools.benchmark.layout_evidence.replay'):
                monitor.return_value.snapshot.return_value = {}
                initial = measure_panel(Path('base000'), Path('cand000'), folder/'out', natural=True)
                before = digest(folder/'out/panel.json')
                complete = measure_panel(Path('base000'), Path('cand000'), folder/'out', complete=True)
            self.assertEqual(calls, [(f'/tmp/cand000/v{k}/qjs', f'/tmp/base000/v{k}/qjs') for k in range(6)])
            self.assertEqual(complete['panels'][0], initial['panels'][0])
            self.assertEqual(digest(folder/'out/panel.json'), before)
            self.assertTrue((folder/'out/complete.json').is_file())

class CalibrationReplayTests(unittest.TestCase):
    def test_calibration_replays_all_pairs_and_refuses_changed_costs_or_binary(self):
        from tools.benchmark.layout_qualification import replay_calibration
        with tempfile.TemporaryDirectory() as tmp:
            folder = Path(tmp)
            binaries = []
            for role in ('base0', 'cand0'):
                path = folder/role/'qjs'
                path.parent.mkdir()
                path.write_bytes(b'calibration binary')
                binaries.append(str(path))
            sha = digest(Path(binaries[0]))
            (folder/'identity.json').write_text(json.dumps({'sha256': sha, 'binaries': binaries, 'host': {}}))
            ratios = {'wall_ns': [1.]*30, 'instructions': [1.]*30}
            packet = {'pairs': 30, 'host': {}, 'cases': [{'id': 'external/s/a', 'ratios': ratios}]}
            screen_path = folder/'screen.json'
            screen_path.write_text(json.dumps(packet))
            costs = {r: {'wall_ns': [100]*30, 'instructions': [10]*30} for r in ('base', 'candidate')}
            samples = []
            for pair in range(30):
                for role in (('base', 'candidate') if pair % 2 == 0 else ('candidate', 'base')):
                    samples.append({'argv': [binaries[role == 'candidate'], '--raw', '/tmp/case.js'],
                        'sample': {'wall_ns': 100, 'instructions': 10, 'stdout': 'OK'}})
            (folder/'cases').mkdir()
            raw_path = folder/'cases/external--s--a.json'
            raw = {'id': 'external/s/a', 'kind': 'whole', 'ratios': ratios, 'costs': costs, 'samples': samples}
            raw_path.write_text(json.dumps(raw))
            self.assertEqual(replay_calibration(screen_path, sha), packet)
            raw['samples'][0]['sample']['wall_ns'] = 99
            raw_path.write_text(json.dumps(raw))
            with self.assertRaisesRegex(ValueError, 'costs differ'):
                replay_calibration(screen_path, sha)
            with self.assertRaisesRegex(ValueError, 'frozen A2'):
                replay_calibration(screen_path, '0'*64)


if __name__ == '__main__':
    unittest.main()
