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
    cases = [{'id': 'broad/a', 'ratios': ratios}]
    packet = {'pairs': 5, 'cases': cases}
    costs = {role: {'wall_ns': [100]*5, 'instructions': [10]*5} for role in binaries}
    samples = []
    for pair in range(5):
        for role in (('base', 'candidate') if pair % 2 == 0 else ('candidate', 'base')):
            samples.append({'argv': [binaries[role]['variants'][0]['path']],
                            'sample': {'wall_ns': 100, 'instructions': 10, 'stdout': 'OK\n'}})
    raw = {'id': 'broad/a', 'kind': 'whole', 'samples': samples, 'costs': costs, 'ratios': ratios}
    path = folder / 'variant0/broad--a.json'
    path.parent.mkdir()
    path.write_text(json.dumps(raw))
    evidence = {'schema_version': 1, 'artifact_type': 'quickjs-layout-panel',
                'host': {}, 'host_node': 'test-host', 'held_out': False, 'pairs': 5,
                **binaries, 'panels': [packet], 'case_files': {'variant0/broad--a.json': digest(path)}}
    (folder / 'panel.json').write_text(json.dumps(evidence))
    policy = {'host': {}, 'host_node': 'test-host', 'cases': {'broad/a': {}}}
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
                evidence['case_files']['variant0/broad--a.json'] = digest(path)
                (folder / 'panel.json').write_text(json.dumps(evidence))
                with self.assertRaises(ValueError):
                    replay(folder / 'panel.json', policy)

    def test_frozen_blind_plan_and_budget_are_enforced(self):
        gate = {'policy_path': 'benchmarks/policies/test.json', 'policy_sha256': 'a'*64,
                'mechanism_class': 'instruction-blind', 'memory_wall_targets': ['broad/a'],
                'panel_required': True, 'panel_pairs': 5, 'held_out_pairs': 30, 'formal_blocks': 30}
        unit = {'layout_gate': gate, 'fast_gate': {'target_ids': ['broad/a']}}
        validate(unit)
        for key, value in (('panel_required', False), ('memory_wall_targets', []),
                           ('policy_path', '../escape'), ('panel_pairs', 4), ('held_out_pairs', 5)):
            altered = copy.deepcopy(unit)
            altered['layout_gate'][key] = value
            with self.assertRaises(ValueError):
                validate(altered)

    def test_invalid_sample_is_retained_and_sampler_restored(self):
        with tempfile.TemporaryDirectory() as work:
            out = Path(work) / 'bad.json'
            sample = screen.Sample(10, 0, 0, 0, '')
            original = screen.sample
            def broken(*args):
                for _ in range(2):
                    screen.sample(None, ['/binary'], 10)
                return screen.CaseResult('case', 'whole', None,
                                         {'wall_ns': [], 'instructions': []})
            with patch('tools.benchmark.screen.sample', return_value=sample) as sampler, \
                 patch('tools.benchmark.screen.screen_case', side_effect=broken):
                with self.assertRaises(screen.ScreenError):
                    measure(screen.CaseSpec('case', 'whole', Path('/case.js')),
                            Path('/cand'), Path('/base'), 1, None, 10, out)
                self.assertIs(screen.sample, sampler)
            self.assertEqual(len(out.with_suffix('.jsonl').read_text().splitlines()), 2)
            self.assertIs(screen.sample, original)


if __name__ == '__main__':
    unittest.main()

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
                with self.assertRaises(SnapshotError):
                    store.snapshot_engine('base', binary, digest(binary))
                self.assertEqual(paths[1].read_bytes(), b'binary')
            finally:
                store.cleanup()
