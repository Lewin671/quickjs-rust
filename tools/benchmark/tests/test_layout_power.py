"""Direction-free sample allocation and immutable continuation order."""
import json
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

from tools.benchmark.layout_power import required_pairs, allocation, diagnostic_only
from tools.benchmark import screen


class PowerTests(unittest.TestCase):
    def test_formula_limits_and_fixed_cap_stage(self):
        self.assertEqual(required_pairs(.03),30)
        self.assertEqual(required_pairs(.08134702043130693),221)
        self.assertEqual(required_pairs(.04,60),107)
        self.assertEqual(allocation({'relative_half_width':.04},60,False,base_n=60),107)
        self.assertEqual(required_pairs(.059945073264363824),120)
        self.assertEqual(required_pairs(.2),240)
        pilot={'relative_half_width':.059945073264363824}
        self.assertEqual(allocation(pilot),120)
        self.assertEqual(allocation(pilot,120,False),240)
        self.assertEqual(allocation(pilot,120,True),120)
        self.assertEqual(allocation(pilot,240,False),240)
        with self.assertRaises(ValueError):allocation(pilot,100,False)
        for invalid in (-1,float('nan'),True):
            with self.assertRaises(ValueError): required_pairs(invalid)

    def test_fallback_is_explicit_metadata(self):
        policy={'precision':{'cases':{'x':{'status':'imprecise on this host'}}}}
        self.assertTrue(diagnostic_only(policy,'x'))
        self.assertFalse(diagnostic_only(policy,'y'))
        self.assertFalse(diagnostic_only(policy,'x',held=True))

    def test_pair_continuation_keeps_iteration_and_global_order(self):
        from tools.benchmark.pair_recording import measure
        from tools.benchmark.screen_replay import replay_case
        from tools.benchmark.pair_disturbance import PROTOCOL
        import hashlib
        class Monitor:
            def snapshot(self):
                return {'protocol':PROTOCOL,'anchor':1,'text':'',
                        'sha256':hashlib.sha256(b'').hexdigest(),'sealed_at':1e20}
        with tempfile.TemporaryDirectory() as tmp:
            out=Path(tmp)/'raw.json'
            def take(tool,argv,timeout):
                n=int(argv[-1]);return screen.Sample(n*10,0,0,n*60_000_000,'QJS_BENCH_RESULT '+json.dumps({'operations':n,'checksum':n}))
            with patch('tools.benchmark.screen.sample',side_effect=take), \
                 patch('tools.benchmark.screen.calibrate') as calibrate:
                measure(screen.CaseSpec('sentinel/c','internal',Path('/case'),'c'),
                        Path('/cand'),Path('/base'),2,None,10,out,Monitor(),
                        iterations=2,pair_offset=31)
                calibrate.assert_not_called()
            value=json.loads(out.read_text())
            self.assertEqual(value['attempts'][0]['index'],31)
            self.assertEqual(value['attempts'][0]['samples'][0]['argv'][0],'/cand')
            self.assertEqual(value['calibration'],[])
            self.assertEqual(value['iterations'],2)
            replay_case(value,2,{'base':'/base','candidate':'/cand'})
            value['pair_index_offset']=30
            with self.assertRaises(ValueError):replay_case(value,2,{'base':'/base','candidate':'/cand'})



    def test_only_imprecise_cap_can_be_diagnostic_policy(self):
        from tools.benchmark.layout_power import precision_record,validate_precision
        pilot={'relative_half_width':.06,'precise':False}
        stats={'relative_half_width':.04,'precise':False}
        record=precision_record(pilot,stats,240)
        self.assertEqual(record['status'],'imprecise on this host')
        p={'cases':{'x':{}},'held_out':{},'precision':{'cases':{'x':record},'held_out':{}}}
        validate_precision(p)
        record['pairs']=239
        with self.assertRaises(ValueError):validate_precision(p)

    def test_extended_cases_keep_original_joint_lane_cohort(self):
        from tools.benchmark.layout_qualification import qualify
        p={'cases':{'broad/x':{'wall_band':1.1,'instruction_band':1.}},
           'host':{},'lanes':{n:1.1 for n in ('broad','sentinel','external','portfolio')}}
        # Use the existing complete inventory while mocking only deterministic
        # interval calculation; lane calls must continue to see 30-pair rows.
        from tools.benchmark.tests.test_layout_rule import policy,packet
        p=policy();data=packet(p);data['pairs']=30;data['host']=p['host']
        for c in data['cases']:c['ratios']={m:[1.]*30 for m in ('instructions','wall_ns')}
        name='sentinel/recursive_call_tree'
        held={'pairs':30,'host':p['host'],'cases':[{'id':'held-out/lib/program','ratios':{m:[1.]*30 for m in ('instructions','wall_ns')}}]}
        observed=[]
        def analyze(values):
            lens={len(v) for v in values.values()};observed.append((len(values),lens))
            h=.059945073264363824 if list(values)==[name] and lens=={30} else .01
            return {'ratio':1.,'confidence_interval':{'lower':.999,'upper':1.001},'relative_half_width':h,'precise':h<=.03}
        with patch('tools.benchmark.layout_qualification.analyze',side_effect=analyze), \
             patch('tools.benchmark.held_out.opportunity_ids',return_value={'held-out/lib/program'}):
            out=qualify(p,data,held,extensions={name:{m:[1.]*120 for m in ('instructions','wall_ns')}})
        self.assertTrue(out['qualified'])
        self.assertEqual(out['precision']['cases'][name]['pairs'],120)
        self.assertTrue(all(lens=={30} for count,lens in observed if count>1))

    def test_formal_cap_fallback_qualifies_and_forces_panel_without_changing_lanes(self):
        from tools.benchmark.layout_qualification import qualify
        from tools.benchmark.layout_power import needs_panel,validate_precision
        from tools.benchmark.tests.test_layout_rule import policy,packet
        p=policy();data=packet(p);data.update(pairs=30,host=p['host'])
        for c in data['cases']:c['ratios']={m:[1.]*30 for m in ('instructions','wall_ns')}
        held={'pairs':30,'host':p['host'],'cases':[{'id':'held-out/lib/program','ratios':{m:[1.]*30 for m in ('instructions','wall_ns')}}]}
        name='sentinel/recursive_call_tree'
        formal={c['id']:[1.]*30 for c in data['cases']};formal[name]=[1.00001]*30
        observed=[]
        def analyze(values):
            observed.append({len(v) for v in values.values()})
            h=.04 if list(values)==[name] and values[name][0]==1.00001 else .01
            return {'ratio':1.,'confidence_interval':{'lower':.999,'upper':1.001},
                    'relative_half_width':h,'precise':h<=.03}
        with patch('tools.benchmark.layout_qualification.analyze',side_effect=analyze), \
             patch('tools.benchmark.held_out.opportunity_ids',return_value={'held-out/lib/program'}):
            incomplete=qualify(p,data,held,formal)
            self.assertFalse(incomplete['qualified'])
            out=qualify(p,data,held,formal,formal_extensions={name:[1.00001]*240})
        self.assertTrue(out['qualified'])
        self.assertEqual(out['precision']['formal'][name]['status'],'imprecise on this host')
        self.assertTrue(diagnostic_only(out,name));self.assertTrue(needs_panel(out))
        self.assertEqual(out['lanes']['portfolio'],p['lanes']['portfolio'])
        validate_precision(out)
        self.assertEqual(out['cases'],p['cases'])


class ContinuationTests(unittest.TestCase):
    def test_replay_keeps_all_pilot_pairs_and_rejects_tampering(self):
        import hashlib,copy
        from tools.benchmark.pair_disturbance import PROTOCOL,verify_run
        from tools.benchmark.layout_continuation import replay
        from tools.benchmark.layout_identity import digest
        def raw(n,offset,binaries):
            attempts=[]
            for i in range(n):
                index=offset+i;rows=[]
                for j,role in enumerate(('base','candidate') if index%2==0 else ('candidate','base')):
                    start=10+index*5+j*2
                    rows.append({'argv':[binaries[role],'--raw','/work.js'],
                                 'sample':{'wall_ns':100,'instructions':10,'stdout':'ok'},
                                 'started_at':start,'ended_at':start+1})
                attempts.append({'index':index,'samples':rows,'started_at':rows[0]['started_at'],'ended_at':rows[-1]['ended_at']})
            snap={'protocol':PROTOCOL,'anchor':1,'text':'','sha256':hashlib.sha256(b'').hexdigest(),'sealed_at':1e20}
            return {'id':'external/s/c','kind':'whole','iterations':None,'complete':True,
                    'pair_index_offset':offset,'attempts':attempts,'calibration':[],
                    'rounds':[{'attempt_count':n,'load':snap}], 'valid_attempt_indices':list(range(n)),
                    'samples':[row for a in attempts for row in a['samples']],
                    'costs':{role:{'wall_ns':[100]*n,'instructions':[10]*n} for role in binaries},
                    'ratios':{'wall_ns':[1.]*n,'instructions':[1.]*n}},snap
        with tempfile.TemporaryDirectory() as tmp:
            root=Path(tmp);pilot=root/'pilot';pilot.mkdir();(pilot/'cases').mkdir()
            paths={}
            for role,folder in (('base','base0'),('candidate','cand0')):
                binary=pilot/folder/'qjs';binary.parent.mkdir();binary.write_bytes(b'unchanged')
                paths[role]=str(binary)
            original,snap=raw(30,0,paths)
            (pilot/'cases/external--s--c.json').write_text(json.dumps(original))
            (pilot/'identity.json').write_text(json.dumps({'binaries':list(paths.values()),'sha256':digest(Path(paths['base']))}))
            packet={'cases':[{'id':'external/s/c','ratios':original['ratios']}],'disturbance':snap}
            source=pilot/'screen.json';source.write_text(json.dumps(packet))
            out=root/'extension';out.mkdir();extension,snap=raw(90,30,paths)
            f=out/'case.json';f.write_text(json.dumps(extension))
            value={'pilot':{'path':str(source),'sha256':digest(source)},'case_id':'external/s/c',
                   'previous':None,'required_pairs':120,'previous_pairs':30,'raw_file':'case.json',
                   'raw_sha256':digest(f),'disturbance':snap,'disturbance_counts':verify_run([extension],snap),
                   'ratios':{'wall_ns':[1.]*120,'instructions':[1.]*120},
                   'sha256':digest(Path(paths['base'])),'host':{}}
            path=out/'continuation.json';path.write_text(json.dumps(value))
            def stats(values):return {'relative_half_width':.059945073264363824,'precise':False}
            with patch('tools.benchmark.layout_continuation.analyze',side_effect=stats):
                ratios,offset,host=replay(path,source,'external/s/c')
                self.assertEqual(len(ratios['wall_ns']),120);self.assertEqual(offset,120)
                for field in ('required_pairs','raw_sha256','ratios'):
                    bad=copy.deepcopy(value)
                    if field=='required_pairs':bad[field]=119
                    if field=='raw_sha256':bad[field]='0'*64
                    if field=='ratios':bad[field]['wall_ns'][0]=.9
                    path.write_text(json.dumps(bad))
                    with self.assertRaises(ValueError):replay(path,source,'external/s/c')

if __name__=='__main__':unittest.main()
