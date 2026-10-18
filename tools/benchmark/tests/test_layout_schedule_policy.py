"""Schedule-specific envelopes and frozen qualification refusal paths."""
import copy
import unittest
from unittest.mock import patch

from tools.benchmark.case_schedule import binding
from tools.benchmark.layout_schedule_bands import estimate
from tools.benchmark.layout_schedule_qualification import qualify, compare_bands


IDS = ['broad/a','sentinel/a','external/s/a','development/s/a']
HELD = ['held-out/s/a']


def packet(ids):
    return {'pairs':30, 'host':{}, 'measurement_schedule':binding(4),
            'cases':[{'id':n,'ratios':{'wall_ns':[1.]*30,'instructions':[1.]*30}} for n in ids]}


def seed():
    bands = {n:{'wall_band':1.01,'instruction_band':1.001} for n in IDS + HELD}
    return {'host':{},'host_node':'host','measurement_schedule':binding(4),
            'formal_lane_schedule':'workers-4', 'cases':{n:bands[n] for n in IDS},
            'held_out':{n:bands[n] for n in HELD}, 'development_manifest_sha256':'a'*64,
            'held_out_manifest_sha256':'b'*64,
            'lanes':{n:1.01 for n in ('broad','sentinel','external','development','portfolio','held-out')}}


def stats(values):
    return {'ratio':1.,'confidence_interval':{'lower':.999,'upper':1.001},
            'relative_half_width':.001,'blocks':len(next(iter(values.values()))),'precise':True}


class PolicyTests(unittest.TestCase):
    def test_all_envelopes_come_from_parallel_samples_without_serial_floor(self):
        raw = {(k,n):{'ratios':{'wall_ns':[1.]*5,'instructions':[1.]*5},
                     'costs':{r:{'wall_ns':[100]*5} for r in ('base','candidate')}}
               for n in IDS+HELD for k in range(6)}
        raw[(1,'external/s/a')]['ratios']['wall_ns']=[1.04]*5
        raw[(1,'external/s/a')]['costs']['candidate']['wall_ns']=[104]*5
        with patch('tools.benchmark.layout_held_out_policy.analyze',side_effect=stats):
            result=estimate(raw,packet(HELD),IDS,HELD)
        self.assertAlmostEqual(result['cases']['external/s/a']['wall_band'],1.04)
        self.assertEqual(result['cases']['broad/a']['wall_band'],1.)
        self.assertAlmostEqual(result['lanes']['external'],1.04)
        self.assertEqual(result['held_out']['held-out/s/a']['wall_band'],1.0011)
        raw.pop((5,'broad/a'))
        with self.assertRaisesRegex(ValueError,'inventory'): estimate(raw,packet(HELD),IDS,HELD)

    def test_qualified_policy_keeps_seed_and_original_lane_cohorts(self):
        s=seed();before=copy.deepcopy(s)
        pilots={'portfolio':packet(IDS[:3]),'development':packet(IDS[3:]),'held-out':packet(HELD)}
        formal={n:[1.]*30 for n in IDS}
        observed=[]
        def analyze(values):
            if len(values)>1: observed.append({len(v) for v in values.values()})
            return stats(values)
        with patch('tools.benchmark.layout_schedule_qualification.analyze',side_effect=analyze):
            result=qualify(s,pilots,formal)
        self.assertTrue(result['qualified']);self.assertEqual(s,before)
        for group in ('cases','held_out','lanes'):self.assertEqual(result[group],s[group])
        self.assertTrue(all(n=={30} for n in observed))
        self.assertEqual(compare_bands(s,result)['lanes']['portfolio'],{'v4':1.01,'v5':1.01})

    def test_reject_mixed_schedule_instruction_event_and_lane_escape(self):
        s=seed();pilots={'portfolio':packet(IDS[:3]),'development':packet(IDS[3:]),'held-out':packet(HELD)}
        formal={n:[1.]*30 for n in IDS}
        bad=copy.deepcopy(pilots);bad['development'].pop('measurement_schedule')
        with self.assertRaisesRegex(ValueError,'schedule mismatch'):qualify(s,bad,formal)
        bad=copy.deepcopy(pilots);bad['development']['cases'][0]['ratios']['instructions']=[1.02]*30
        with patch('tools.benchmark.layout_schedule_qualification.analyze',side_effect=stats):
            result=qualify(s,bad,formal)
        self.assertFalse(result['qualified']);self.assertTrue(any('instruction' in f for f in result['failures']))
        def escape(values):
            v=stats(values);v['confidence_interval']['upper']=1.02;return v
        with patch('tools.benchmark.layout_schedule_qualification.analyze',side_effect=escape):
            result=qualify(s,pilots,formal)
        self.assertFalse(result['qualified']);self.assertEqual(result['lanes'],s['lanes'])


if __name__=='__main__':unittest.main()
