#!/usr/bin/env python3
"""Diagnostic instruction replays on measured (not calibration) public argv."""
import argparse
import json
import os
from pathlib import Path
import statistics
from tools.benchmark.screen import counter_tool, sample
from tools.benchmark.triage_build import build

p = argparse.ArgumentParser(description=__doc__)
p.add_argument('--revision', action='append', required=True)
p.add_argument('--triage', type=Path, required=True)
p.add_argument('--output', type=Path, required=True)
a = p.parse_args()
a.output.mkdir(parents=True, exist_ok=False)
images = {}
for revision in a.revision:
    folder = a.output / revision
    folder.mkdir()
    images[revision] = build(revision, folder / 'qjs')
cases = ['development/lodash/transform', 'development/meriyah/parse',
         'development/diff/compare', 'external/sunspider-1.0/date-format-tofte',
         'external/sunspider-1.0/string-fasta',
         'external/sunspider-1.0/access-binary-trees',
         'external/kraken-1.1/json-parse-financial',
         'broad/property_dynamic_read', 'broad/captured_write',
         'sentinel/string_key_map_churn']
tool = counter_tool()
report = {'diagnostic_only': True, 'timing_claim': False,
          'builds': images, 'triage_argv_source': str(a.triage), 'records': []}
for case in cases:
    rows = [json.loads(x) for x in
            (a.triage / 'variant0' / (case.replace('/', '--') + '.jsonl')).read_text().splitlines()]
    if case.startswith(('broad/', 'sentinel/')):
        large = max(int(r['argv'][-1]) for r in rows)
        small = large // 2
        assert large % 2 == 0 and any(int(r['argv'][-1]) == small for r in rows)
        args = [next(r['argv'][1:-1] for r in rows) + [str(n)] for n in [small, large]]
    else:
        args = [rows[-1]['argv'][1:]]
    values = {revision: [] for revision in images}
    outputs = {}
    for pair in range(2):
        order = list(images)
        if pair % 2:
            order.reverse()
        for revision in order:
            runs = []
            for index, argv in enumerate(args):
                result = sample(tool, [images[revision]['path'], *argv], 300)
                key = (pair, index)
                if key in outputs:
                    assert result.stdout == outputs[key], (case, revision)
                outputs[key] = result.stdout
                runs.append(result.instructions)
            cost = runs[-1] if len(runs) == 1 else runs[1] - runs[0]
            assert cost > 0
            values[revision].append(cost)
    medians = {ref: statistics.median(v) for ref, v in values.items()}
    base = a.revision[0]
    record = {'case': case, 'argv_without_binary': args, 'samples': values,
              'ratios': {ref: value / medians[base] for ref, value in medians.items()}}
    report['records'].append(record)
    (a.output / 'instructions.json').write_text(json.dumps(report, indent=2) + '\n')
    print(case, record['ratios'], flush=True)
