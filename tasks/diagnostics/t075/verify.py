"""Independently recompute the retained T075 tables without external artifacts."""
from collections import Counter, defaultdict
import csv
import json
from pathlib import Path


def main():
    root = Path(__file__).resolve().parent
    mapping = {(r['role'],r['pc']):r for r in csv.DictReader((root/'pc-map.csv').open())}
    profiles = json.loads((root/'profiles.json').read_text())
    totals = Counter()
    expected = {}
    for row in profiles:
        key = (row['role'],row['id'],row['repeat'])
        assert key not in expected and row['ambiguous_self']==0
        expected[key] = row['executor_self']
        totals[row['role'],row['id']] += row['total']
    assert len(expected)==90
    seen,actual,counts = set(),Counter(),defaultdict(Counter)
    for row in csv.DictReader((root/'counts.csv').open()):
        role,case,repeat,count=row['role'],row['program'],int(row['repeat']),int(row['count'])
        key=(role,case,repeat,row['pc'])
        assert key not in seen and count>0
        seen.add(key)
        actual[role,case,repeat]+=count
        meta=mapping[role,row['pc']]
        assert meta['frame_ids'] and meta['mapped_pc'] and meta['instruction']!='?'
        counts[role,case][meta['category']]+=count
    assert dict(actual)==expected
    summary=json.loads((root/'summary.json').read_text())
    for role,record in summary.items():
        cases=[c for r,c in totals if r==role]
        assert len(cases)==10
        for category,value in record['main_thread_percent'].items():
            observed=sum(100*counts[role,c][category]/totals[role,c] for c in cases)/10
            assert abs(value-observed)<1e-10,(role,category)
        assert abs(sum(record['main_thread_percent'].values())-record['executor_main_percent'])<1e-10
    owner=json.loads((root/'owner-envelope.json').read_text())
    assert abs(sum(owner['mean'].values())-100)<1e-10
    canonical_profiles={(p['id'],p['repeat']):p['total'] for p in profiles if p['role']=='canonical'}
    assert {(p['id'],p['repeat']):p['total'] for p in owner['profiles']}==canonical_profiles
    assert abs(owner['mean']['loop']-summary['canonical']['executor_main_percent']-sum(owner['other_loop_symbols'].values()))<1e-10
    audits=json.loads((root/'codegen-audit.json').read_text())
    assert len(audits)==7 and all(a['mapped'] for a in audits)
    print(f'PASS: {len(profiles)} profiles, {len(seen)} PC-count rows, {len(mapping)} mapped PCs; categories and owner envelope conserve counts')


if __name__=='__main__':
    main()
