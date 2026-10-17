"""Three-role append plans, frozen asset recovery and lossless raw replay."""
import copy
import dataclasses
import hashlib
import json
import tempfile
import unittest
from pathlib import Path
from types import SimpleNamespace
from unittest.mock import patch

from tools.benchmark import formal_case_context as contract
from tools.benchmark import layout_formal_continuation as continuation
from tools.benchmark.layout_identity import digest
from tools.benchmark.pair_disturbance import PROTOCOL
from tools.benchmark.process import ProcessResult


class FormalContinuationTests(unittest.TestCase):
    def context(self):
        return {'external':True,'seed':1729,'templates':{
            r:{'argv':['/'+r,'--script','/work.js'],'binary_sha256':r,
               'instructions':None,'cycles':None,'host':{'node':'host'}} for r in contract.ROLES}}

    def result(self, index=0):
        return ProcessResult('1970-01-01T00:00:10+00:00',100+index*100,150+index*100,50,
                             0,False,'__QJS_EXTERNAL_OK__\n__QJS_EXTERNAL_OK__\n','',False,False)

    def test_schedule_extends_original_rotation_and_internal_global_order(self):
        ctx=self.context()
        self.assertEqual(contract.schedule(ctx,30),contract.schedule(ctx,221)[:90])
        added=contract.schedule(ctx,221)[90:]
        self.assertEqual(len(added),573)
        for b in range(30,221):
            self.assertEqual({r for block,_,r in added if block==b},set(contract.ROLES))
        ctx.update(external=False,selected_cases=['a','b','c'],case=SimpleNamespace(id='b'))
        full=contract.measurement_plan(list(contract.ROLES),ctx['selected_cases'],221,ctx['seed'])
        expected=[(p.block,p.order,p.role) for p in full if p.case_id=='b' and p.block>=30]
        self.assertEqual(contract.schedule(ctx,221)[90:],expected)
        self.assertTrue(any(order>2 for _,order,_ in expected))

    def test_preflight_uses_role_adapters_and_never_runs_a_workload(self):
        ctx=self.context();commands=[]
        def runner(argv,timeout):
            commands.append(argv)
            self.assertEqual(argv[-2:],['-e','0'])
            self.assertNotIn('/work.js',argv)
            return self.result()
        with patch.object(contract,'digest',return_value='same'):
            records=contract.launch_preflight(ctx,runner=runner)
            self.assertEqual(len(records),3)
            self.assertEqual(commands,[['/candidate','--raw','-e','0'],
                                       ['/base','--raw','-e','0'],['/quickjs-ng','-e','0']])
            with self.assertRaisesRegex(ValueError,'launch preflight'):
                contract.launch_preflight(ctx,runner=lambda a,t:dataclasses.replace(self.result(),exit_code=1))

    def test_restore_is_exact_and_never_replaces_existing_asset(self):
        with tempfile.TemporaryDirectory() as tmp:
            src=Path(tmp)/'source';dst=Path(tmp)/'snapshot';src.write_bytes(b'image')
            contract._restore(src,dst,digest(src))
            inode=dst.stat().st_ino
            contract._restore(src,dst,digest(src))
            self.assertEqual(dst.stat().st_ino,inode)
            with self.assertRaises(ValueError):contract._restore(src,dst,'0'*64)
            missing=Path(tmp)/'missing'
            with self.assertRaises(ValueError):contract._restore(src,missing,'0'*64)
            self.assertFalse(missing.exists())
            contract._restore(None,Path(tmp)/'work','2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824',data=b'hello')

    def test_external_checks_timer_counters_and_role_output(self):
        ctx=self.context();result=self.result()
        self.assertEqual(contract.validate_result(ctx,'base',result),50)
        for change in ({'timed_out':True},{'exit_code':1},{'stdout':'wrong'},
                       {'stdout_truncated':True},{'duration_ns':49},{'instructions':10},
                       {'timer_started_ns':True}):
            with self.subTest(change=change),self.assertRaises(ValueError):
                contract.validate_result(ctx,'base',dataclasses.replace(result,**change))

    def test_internal_uses_frozen_n_per_operation_and_startup_eligibility(self):
        from tools.benchmark.schema import load_manifest
        case=load_manifest(Path('benchmarks/manifest.json')).cases[0]
        ctx=self.context();ctx.update(external=False,case=case,startup={r:0 for r in contract.ROLES})
        n=10;ops=case.expected_operations(n)
        for role,t in ctx['templates'].items():
            t.update(status='ok',error=None,iterations=n,operations=ops,
                     checksum=case.expected_checksum(n),adapter_id='qjs-rust-raw',
                     binary_snapshot_path='/'+role,workload_snapshot_path='/work.js')
            t['argv']=['/'+role,'--raw','/work.js',case.id,str(n)]
        duration=int(case.min_window_ms*1e6)+1
        output='QJS_BENCH_RESULT '+json.dumps({'case_id':case.id,'iterations':n,'operations':ops,'checksum':case.expected_checksum(n)})
        result=dataclasses.replace(self.result(),duration_ns=duration,timer_finished_ns=100+duration,stdout=output)
        # A real archived raw template contains all ProcessResult fields.
        ctx['templates']['base'].update(dataclasses.asdict(result))
        self.assertEqual(contract.validate_result(ctx,'base',result),duration/ops)
        ctx['startup']['base']=duration
        with self.assertRaisesRegex(ValueError,'timer-limited'):contract.validate_result(ctx,'base',result)
        ctx['startup']['base']=0
        bad=dataclasses.replace(result,stdout=output.replace('"iterations": 10','"iterations": 11'))
        with self.assertRaises(ValueError):contract.validate_result(ctx,'base',bad)

    def test_raw_replay_retains_pilot_and_rejects_incomplete_or_mutated_plan(self):
        ctx=self.context();pilot=[1.]*30
        stats={'relative_half_width':.08134702043130693,'precise':False}
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);raw=root/'samples.jsonl';path=root/'continuation.json'
            rows=[{'block':b,'order':o,'role':r,'argv':ctx['templates'][r]['argv'],
                   'result':dataclasses.asdict(dataclasses.replace(self.result(i),
                        stdout='__QJS_EXTERNAL_OK__\n' if r=='quickjs-ng' else self.result(i).stdout))}
                  for i,(b,o,r) in enumerate(contract.schedule(ctx,221)[90:])]
            raw.write_text(''.join(json.dumps(r)+'\n' for r in rows))
            value={'artifact_type':'quickjs-formal-case-continuation','schema_version':1,
                   'source_hashes':{'summary':'sealed'},'case_id':'external/s/c','bundle':str(root),
                   'host_node':'host','host':{},'previous':None,'previous_blocks':30,'required_blocks':221,
                   'raw_file':raw.name,'raw_sha256':digest(raw),'ratios':[1.]*221,
                   'load':{'protocol':PROTOCOL,'text':'','sha256':hashlib.sha256(b'').hexdigest(),
                           'anchor':1,'sealed_at':20}}
            def save(v,rr):
                raw.write_text(''.join(json.dumps(r)+'\n' for r in rr));v['raw_sha256']=digest(raw)
                path.write_text(json.dumps(v))
            with patch.object(continuation,'context',return_value=ctx), \
                 patch.object(continuation,'source_hashes',return_value={'summary':'sealed'}), \
                 patch.object(continuation,'analyze',return_value=stats):
                save(value,rows)
                merged,host=continuation.replay(path,root,'external/s/c',pilot=pilot)
                self.assertEqual(merged[:30],pilot);self.assertEqual(len(merged),221)
                for change in ('missing','duplicate','order','argv','timer','stdout','counter','ratio','source','load','guard'):
                    bad=copy.deepcopy(value);rr=copy.deepcopy(rows)
                    if change=='missing':rr.pop()
                    if change=='duplicate':rr[-1]=rr[-2]
                    if change=='order':rr[0]['block']=0
                    if change=='argv':rr[0]['argv'][0]='/replacement'
                    if change=='timer':rr[0]['result']['duration_ns']=1
                    if change=='stdout':rr[0]['result']['stdout']='bad'
                    if change=='counter':rr[0]['result']['cycles']=10
                    if change=='ratio':bad['ratios'][0]=.9
                    if change=='source':bad['source_hashes']={}
                    if change=='load':bad['load']['text']='00:00:10 250 intruder\n';bad['load']['sha256']=hashlib.sha256(bad['load']['text'].encode()).hexdigest()
                    if change=='guard':bad['load']['sealed_at']=10
                    save(bad,rr)
                    with self.subTest(change=change),self.assertRaises(ValueError):
                        continuation.replay(path,root,'external/s/c',pilot=pilot)

    def test_automatic_cap_stage_is_precision_only_for_both_collectors(self):
        from tools.benchmark import layout_continuation
        for module,args in ((continuation,(Path('/bundle'),'case')),
                            (layout_continuation,(Path('/pilot'),'case','revision'))):
            for first_precise,first_count,expected in ((True,221,1),(False,221,2),(False,240,1)):
                with self.subTest(module=module.__name__,first_precise=first_precise,first_count=first_count):
                    with tempfile.TemporaryDirectory() as tmp:
                        root=Path(tmp)/'out';calls=[]
                        def collect(*args):
                            out=args[-2];previous=args[-1];out.mkdir()
                            path=out/'continuation.json';path.write_text(json.dumps({'host':{},'host_node':'host'}))
                            calls.append(previous)
                            return path,{'blocks':first_count if len(calls)==1 else 240,
                                         'precise':first_precise,'ratio':.8 if len(calls)==1 else 1.2}
                        with patch.object(module,'collect',side_effect=collect):
                            module.complete(*args,root)
                        self.assertEqual(len(calls),expected)
                        if expected==2:self.assertIsNotNone(calls[1])
