"""Load selection, append schedule and raw conservation without real timing."""
import copy
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.benchmark import screen
from tools.benchmark.pair_disturbance import (PROTOCOL, check_cap, eligibility,
    heavy_samples, invalid_window, verify_run)
from tools.benchmark.pair_recording import measure
from tools.benchmark.screen_replay import replay_case


def snapshot(text='', end=1000):
    return {'protocol': PROTOCOL, 'anchor': 1, 'text': text,
            'sha256': hashlib.sha256(text.encode()).hexdigest(), 'sealed_at': end}


class LoadTests(unittest.TestCase):
    def test_sum_threshold_guard_and_timestamp_bucket(self):
        heavy = heavy_samples('00:01:40 110 daemonA\n00:01:40 90 daemonB\n00:01:45 199 daemon\n', 1)
        self.assertEqual(heavy, [100])
        self.assertEqual(invalid_window(103, 104, heavy), [100])
        self.assertEqual(invalid_window(103.001, 104, heavy), [])
        self.assertEqual(invalid_window(97, 98, heavy), [100])
        self.assertEqual(invalid_window(96, 97.999, heavy), [])

    def test_midnight_and_malformed_monitor(self):
        self.assertEqual(heavy_samples('23:59:59 200 x\n00:00:04 200 y\n', 86390), [86399,86404])
        for text in ('bad row', '00:00:01 nan x', '00:00:01 20 x'):
            with self.assertRaises(ValueError):
                heavy_samples(text, 1)

    def test_cap_boundaries_and_cluster(self):
        check_cap([False]+[True]*4, [10,15,20,25,30,35,40,45,50,55,60,65])
        with self.assertRaisesRegex(ValueError, '20%'):
            check_cap([False]+[True]*3, [])
        with self.assertRaisesRegex(ValueError, '60 seconds'):
            check_cap([], list(range(10,71,5)))
        check_cap([], [10, 17, 70])

    def test_canonical_formal_keeps_whole_run_invalidation(self):
        from tools.benchmark.pair_disturbance import verify_formal_load
        verify_formal_load('',1)
        verify_formal_load('00:00:10 199 daemon\n',1)
        with self.assertRaisesRegex(ValueError, 'whole-run replacement'):
            verify_formal_load('00:00:10 100 a\n00:00:10 100 b\n',1)

    def test_snapshot_coverage_and_tampering(self):
        attempts = [{'started_at': 10, 'ended_at': 12}]
        self.assertEqual(eligibility(attempts, snapshot()), [True])
        with self.assertRaisesRegex(ValueError, 'completion'):
            eligibility(attempts, snapshot(end=19))
        altered = snapshot(); altered['text'] = '00:00:10 200 x\n'
        with self.assertRaisesRegex(ValueError, 'hash'):
            eligibility(attempts, altered)

    def test_snapshot_waits_for_wall_clock_despite_slew(self):
        # A slewed host clock can advance less than the monotonic sleep; the
        # seal must still cover the settle time in wall-clock terms.
        from tools.benchmark.pair_disturbance import Monitor
        with tempfile.TemporaryDirectory() as root:
            log = Path(root) / 'intruders.log'
            log.write_text('')
            clock = {'now': 1000.0}

            def slewed_sleep(seconds):
                clock['now'] += seconds * 0.99

            with patch('tools.benchmark.pair_disturbance.time.time', lambda: clock['now']), \
                    patch('tools.benchmark.pair_disturbance.time.sleep', slewed_sleep):
                monitor = Monitor(log)
                ended = clock['now']
                sealed = monitor.snapshot()
            self.assertGreaterEqual(sealed['sealed_at'], ended + PROTOCOL['settle_seconds'])
            eligibility([{'started_at': ended - 1, 'ended_at': ended}], sealed)

    def test_appended_recovery_conserves_all_attempts_and_order(self):
        class Monitor:
            anchor = 1
            def snapshot(self):
                return snapshot('00:00:10 200 daemon\n')
        # First pair overlaps heavy load, other four are valid. Append one
        # candidate-first pair (attempt 5), independent of observed values.
        ticks = iter([10,11,11,12, 20,21,21,22, 30,31,31,32,
                      40,41,41,42, 50,51,51,52, 60,61,61,62])
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp)/'case.json'
            with patch('tools.benchmark.pair_recording.time.time', side_effect=lambda: next(ticks)), \
                 patch('tools.benchmark.screen.sample', return_value=screen.Sample(100,0,0,100,'ok')):
                result = measure(screen.CaseSpec('external/s/c','whole',Path('/case')),
                    Path('/cand'),Path('/base'),5,None,1,output,Monitor())
            raw = json.loads(output.read_text())
            self.assertEqual(raw['valid_attempt_indices'], [1,2,3,4,5])
            self.assertEqual([r['attempt_count'] for r in raw['rounds']], [5,6])
            self.assertEqual(len(output.with_suffix('.jsonl').read_text().splitlines()),12)
            self.assertEqual(raw['attempts'][5]['samples'][0]['argv'][0], '/cand')
            self.assertEqual(replay_case(raw,5,{'base':'/base','candidate':'/cand'}),result.ratios)
            self.assertEqual(verify_run([raw],Monitor().snapshot())['invalid'],1)
            for mutate in ('selection', 'cost', 'count', 'timestamp', 'history'):
                bad = copy.deepcopy(raw)
                if mutate == 'selection': bad['valid_attempt_indices'][0] = 0
                if mutate == 'cost': bad['costs']['base']['wall_ns'][0] += 1
                if mutate == 'count': bad['rounds'][1]['attempt_count'] += 1
                if mutate == 'timestamp': bad['attempts'][0]['started_at'] += 1
                if mutate == 'history': bad['rounds'][1]['load'] = snapshot()
                with self.assertRaises(ValueError, msg=mutate):
                    replay_case(bad,5,{'base':'/base','candidate':'/cand'})

    def test_cap_failure_keeps_attempts_and_load_snapshot(self):
        class Monitor:
            def snapshot(self): return snapshot('00:00:10 200 daemon\n')
        ticks = iter([10,11,11,12])
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp)/'cap.json'
            with patch('tools.benchmark.pair_recording.time.time', side_effect=lambda: next(ticks)), \
                 patch('tools.benchmark.screen.sample', return_value=screen.Sample(10,0,0,100,'')):
                with self.assertRaisesRegex(ValueError, '20%'):
                    measure(screen.CaseSpec('external/s/c','whole',Path('/case')),
                        Path('/cand'),Path('/base'),1,None,1,output,Monitor())
            raw = json.loads(output.read_text())
            self.assertFalse(raw['complete'])
            self.assertEqual(len(raw['attempts']),1)
            self.assertIn('200 daemon',raw['rounds'][0]['load']['text'])

    def test_user_quieting_is_read_only_and_required(self):
        from tools.benchmark.pair_disturbance import quieting_preflight
        services = '\n'.join(f'"com.apple.{s}" => true' for s in ('mediaanalysisd','photoanalysisd'))
        with patch('subprocess.check_output', side_effect=['Indexing disabled.',services]):
            self.assertIn('indexing', quieting_preflight())
        with patch('subprocess.check_output', side_effect=['Indexing enabled.',services]):
            with self.assertRaisesRegex(ValueError, 'not disabled'):
                quieting_preflight()

    def test_source_host_binding_preserves_case_and_recipe_identity(self):
        from tools.benchmark.layout_qualification import verify_host_inputs
        from tools.benchmark.layout_identity import digest, ROOT
        name = 'benchmarks/manifest.json'
        old = json.loads((ROOT/name).read_text())
        old['protocol'] = {'id': 'previous-qualified-protocol'}
        text = json.dumps(old)
        recorded = {'cpu': 'same host', 'recipe': 'same recipe', 'manifests':
                    {name: hashlib.sha256((text+'\n').encode()).hexdigest(),
                     'benchmarks/external-preview.json': 'a'*64}}
        current = copy.deepcopy(recorded)
        current['manifests'][name] = digest(ROOT/name)
        with patch('tools.benchmark.layout_identity.command', return_value=text):
            verify_host_inputs(recorded,current,'recorded-context')
        current['cpu'] = 'other host'
        with self.assertRaisesRegex(ValueError, 'host attestation'):
            verify_host_inputs(recorded,current,'recorded-context')
        current['cpu'] = recorded['cpu']
        current['manifests']['benchmarks/external-preview.json'] = '0'*64
        with patch('tools.benchmark.layout_identity.command', return_value=text):
            with self.assertRaisesRegex(ValueError, 'recipe/corpus'):
                verify_host_inputs(recorded,current,'recorded-context')
        current['manifests']['benchmarks/external-preview.json'] = 'a'*64
        with patch('tools.benchmark.layout_identity.command', return_value='{}'):
            with self.assertRaisesRegex(ValueError, 'manifest/context'):
                verify_host_inputs(recorded,current,'recorded-context')
        old['cases'][0]['id'] = 'different-case'
        text = json.dumps(old)
        recorded['manifests'][name] = hashlib.sha256((text+'\n').encode()).hexdigest()
        with patch('tools.benchmark.layout_identity.command', return_value=text):
            with self.assertRaisesRegex(ValueError, 'case or recipe changed'):
                verify_host_inputs(recorded,current,'recorded-context')

    def test_valid_bad_cost_is_fatal_and_retained(self):
        class Monitor:
            def snapshot(self): return snapshot(end=1e20)
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp)/'bad.json'
            with patch('tools.benchmark.screen.sample', return_value=screen.Sample(10,0,0,0,'')):
                with self.assertRaisesRegex(ValueError, 'positive'):
                    measure(screen.CaseSpec('external/s/c','whole',Path('/case')),
                        Path('/cand'),Path('/base'),1,None,1,output,Monitor())
            self.assertFalse(json.loads(output.read_text())['complete'])
            self.assertEqual(len(output.with_suffix('.jsonl').read_text().splitlines()),2)


if __name__ == '__main__':
    unittest.main()
