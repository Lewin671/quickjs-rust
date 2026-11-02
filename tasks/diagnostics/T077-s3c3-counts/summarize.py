#!/usr/bin/env python3
"""Summarize the complete C3 public receipt against the frozen S3b receipt."""
import argparse
from collections import Counter
import json
import math
from pathlib import Path
import statistics

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--counts', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
args = parser.parse_args()
base = json.loads(Path('tasks/diagnostics/T077-s3c-counts.json').read_text())
candidate = json.loads(args.counts.read_text())
old = {row['id']: row for row in base['records']}
new = {row['id']: row for row in candidate['records']}
assert len(old) == len(new) == 109 and old.keys() == new.keys()
lanes = {}
for lane in ['development', 'external', 'broad', 'sentinel', 'diagnostic', 'repro']:
    ids = [key for key in old if key.startswith(lane + '/')]
    ratios = []
    totals = {'s3b': Counter(), 'c3': Counter()}
    outputs = []
    for key in ids:
        before, after = old[key], new[key]
        assert before['sha256'] == after['sha256'] and before['args'] == after['args']
        ratios.append(statistics.median(after['instructions']) / statistics.median(before['instructions']))
        if before['stdout_sha256'] != after['stdout_sha256']:
            outputs.append(key)
        totals['s3b'].update(before['coverage'])
        totals['c3'].update(after['coverage'])
    lanes[lane] = {'cases': len(ids), 'instructions_geomean': math.exp(statistics.mean(map(math.log, ratios))),
                   'coverage_totals': {role: dict(values) for role, values in totals.items()},
                   'stdout_hash_changes': outputs}
args.output.write_text(json.dumps({
    'comparison': base['base'], 'timing': False,
    'interpretation': 'Unweighted case geomeans against historical S3b instruction observations; diagnostic only. Coverage totals sum the frozen invocations, not equivalent API costs. Original records retain all counter differences.',
    'cases': 109, 'lanes': lanes,
}, indent=2) + '\n')
