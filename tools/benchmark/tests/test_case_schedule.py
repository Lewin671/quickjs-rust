"""Exercise forked ownership and lossless recovery without timing workloads."""
import copy
import json
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.benchmark import screen
from tools.benchmark.case_schedule import binding, collect, plan, require_same, verify
from tools.benchmark.layout_identity import digest
from tools.benchmark.pair_disturbance import verify_run
from tools.benchmark.screen_replay import replay_case
from tools.benchmark.tests.test_pair_disturbance import snapshot


class FakeMonitor:
    def snapshot(self):
        return snapshot('00:00:10 200 daemon\n', 1e20)


def recover(case, candidate, base, pairs, tool, timeout, output, monitor, **kwargs):
    # Each fork gets a fresh clock for this case/layout. The initial invalid
    # pair is retained, and attempt 5 must still start candidate-first.
    from tools.benchmark.pair_recording import measure
    ticks = iter([10,11,11,12,20,21,21,22,30,31,31,32,
                  40,41,41,42,50,51,51,52,60,61,61,62])
    with patch('tools.benchmark.pair_recording.time.time', side_effect=lambda: next(ticks)), \
            patch('tools.benchmark.screen.sample', return_value=screen.Sample(100,0,0,100,'ok')):
        result = measure(case, candidate, base, pairs, tool, timeout, output, monitor, **kwargs)
    output.with_suffix('.pid').write_text(str(os.getpid()))
    return result


class ScheduleTests(unittest.TestCase):
    def test_deterministic_conservation_and_validation(self):
        ids = [str(i) for i in range(11)]
        shards = plan(ids, 4)
        self.assertEqual(shards, [ids[i::4] for i in range(4)])
        self.assertEqual(sorted(sum(shards, [])), sorted(ids))
        for workers in (0, -1, 9, True):
            with self.assertRaises(ValueError): binding(workers)
        for ids in ([], ['a','a']):
            with self.assertRaises(ValueError): plan(ids, 4)
        require_same({}, {'measurement_schedule': binding(1)})
        with self.assertRaisesRegex(ValueError, 'schedule mismatch'):
            require_same({}, {'measurement_schedule': binding(4)})

    def test_forked_layouts_keep_one_owner_replay_every_attempt(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            cases = [screen.CaseSpec('external/s/c'+str(i), 'whole', Path('/case')) for i in range(7)]
            binaries = {k: {'base':Path('/base'), 'candidate':Path('/cand')} for k in range(6)}
            with patch('tools.benchmark.screen.counter_tool'):
                packets, scheduling = collect(cases, list(range(6)), binaries, 5, None,
                                               120, root, FakeMonitor(), 4, measure=recover)
            raws = [json.loads(p.read_text()) for p in sorted(root.glob('variant*/*.json'))]
            verify(scheduling, [c.id for c in cases], raws)
            self.assertEqual(len(raws), 42)
            self.assertEqual(verify_run(raws, FakeMonitor().snapshot()),
                {'attempts':252, 'valid':210, 'invalid':42, 'heavy_samples':1})
            owner_pids = {}
            for case in cases:
                pids = {(root/f'variant{k}'/(case.id.replace('/','--')+'.pid')).read_text() for k in range(6)}
                self.assertEqual(len(pids),1)
                owner_pids[case.id] = pids.pop()
            self.assertEqual(len(set(owner_pids.values())),4)
            self.assertEqual(owner_pids[cases[0].id],owner_pids[cases[4].id])
            for raw in raws:
                replay_case(raw, 5, {'base':'/base','candidate':'/cand'})
                self.assertEqual(raw['valid_attempt_indices'], [1,2,3,4,5])
                self.assertEqual(raw['attempts'][5]['samples'][0]['argv'][0],'/cand')
            self.assertEqual([c['id'] for c in packets[0]['cases']], [c.id for c in cases])
            for mutate in ('owner','missing','duplicate','schedule'):
                bad=copy.deepcopy(scheduling); samples=copy.deepcopy(raws)
                if mutate=='owner': samples[0]['case_shard']=99
                if mutate=='missing': bad['case_shards'][0].pop()
                if mutate=='duplicate': bad['case_shards'][0].append(cases[0].id)
                if mutate=='schedule': samples[0].pop('measurement_schedule')
                with self.assertRaises(ValueError): verify(bad,[c.id for c in cases],samples)

    def test_worker_cap_failure_keeps_load_only_failure_classification(self):
        def fail(*args, **kwargs):
            raise ValueError('DISTURBED: invalid pairs exceed 20% cap')
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp)
            cases=[screen.CaseSpec('external/s/a','whole',Path('/case'))]
            with patch('tools.benchmark.screen.counter_tool'):
                with self.assertRaisesRegex(ValueError,'^DISTURBED:'):
                    collect(cases,[0],{0:{'base':Path('/base'),'candidate':Path('/cand')}},
                            5,None,120,root,FakeMonitor(),4,measure=fail)
            self.assertIn('20% cap', (root/'worker-error-0.json').read_text())

    def test_parallel_raw_and_summary_cannot_be_used_by_serial_policy(self):
        from tools.benchmark.tests.test_layout_evidence import fixture
        from tools.benchmark.layout_evidence import replay
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);e,p,r,f=fixture(root)
            sch=binding(4)
            e.update(measurement_schedule=sch, case_shards=plan([r['id']],4))
            e['panels'][0]['measurement_schedule']=sch
            r.update(measurement_schedule=sch,case_shard=0,worker_pid=42)
            f.write_text(json.dumps(r));e['case_files'][str(f.relative_to(root))]=digest(f)
            path=root/'panel.json';path.write_text(json.dumps(e))
            with self.assertRaisesRegex(ValueError,'schedule mismatch'): replay(path,p)
            p['measurement_schedule']=sch
            replay(path,p)
            r.pop('measurement_schedule');f.write_text(json.dumps(r))
            e['case_files'][str(f.relative_to(root))]=digest(f);path.write_text(json.dumps(e))
            with self.assertRaisesRegex(ValueError,'schedule mismatch'): replay(path,p)

    def test_completion_allows_new_processes_only_at_a_sealed_run_boundary(self):
        from tools.benchmark.tests.test_layout_evidence import fixture
        from tools.benchmark.layout_evidence import replay
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);e,p,r,f=fixture(root)
            sch=binding(4);ids=[r['id']]
            e.update(measurement_schedule=sch,case_shards=plan(ids,4))
            p['measurement_schedule']=sch
            e['panels']=[copy.deepcopy(e['panels'][0]) for _ in range(6)]
            e['case_files']={}
            for k,packet in enumerate(e['panels']):
                packet['measurement_schedule']=sch
                raw=copy.deepcopy(r)
                raw.update(measurement_schedule=sch,case_shard=0,worker_pid=42 if k==0 else 43,attempts=[])
                path=root/f'variant{k}'/f.name;path.parent.mkdir(exist_ok=True)
                path.write_text(json.dumps(raw));e['case_files'][str(path.relative_to(root))]=digest(path)
            e['disturbance_runs']=[{'variants':[0],'load':{},'counts':{}},
                                    {'variants':list(range(1,6)),'load':{},'counts':{}}]
            path=root/'complete.json';path.write_text(json.dumps(e))
            with patch('tools.benchmark.layout_evidence.replay_case'), \
                    patch('tools.benchmark.pair_disturbance.verify_run',return_value={}):
                replay(path,p)
                e['disturbance_runs']=[{'variants':list(range(6)),'load':{},'counts':{}}]
                path.write_text(json.dumps(e))
                with self.assertRaisesRegex(ValueError,'process identity'): replay(path,p)

    def test_npm_report_schedule_replays_ownership(self):
        from types import SimpleNamespace as Obj
        from tools.benchmark.npm_workers import replay_schedule
        manifest=Obj(suites=[Obj(id='s',cases=[Obj(id='a'),Obj(id='b')])])
        records=[{'suite_id':'s','case_id':name,'measurement_schedule':binding(4),'case_shard':i,'worker_pid':42+i}
                 for i,name in enumerate(('a','b'))]
        self.assertEqual(replay_schedule(manifest,records)['case_shards'],[['s/a'],['s/b'],[],[]])
        records[1]['case_shard']=0
        with self.assertRaises(ValueError): replay_schedule(manifest,records)


if __name__=='__main__': unittest.main()
