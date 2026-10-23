"""Replay T076 raw retired-instruction processes; report the frozen unit gate.

Input is collect.py's complete non-timing job output, not wall/triage samples.
No runtime or benchmark input is modified. Print every target/control ratio.
"""
import argparse
import hashlib
import json
import math
from pathlib import Path
import statistics

from tools.benchmark.screen import parse_macos_time


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def replay(folder, unit):
    request = json.loads((folder / 'request.json').read_text())
    complete = json.loads((folder / 'complete.json').read_text())
    processes = json.loads((folder / 'processes.json').read_text())
    declared = json.loads((folder / 'instructions.json').read_text())
    assert complete['instructions_sha256'] == digest(folder / 'instructions.json')
    assert request['timing'] is False and request['pairs'] == 5
    assert len(processes) == complete['processes']
    assert len(declared) == complete['cases'] == len(request['cases']) == 86
    assert {r['id'] for r in declared} == set(request['cases'])
    identities = {role: json.loads((folder / name / 'identity.json').read_text())
                  for role, name in (('base', 'base000'), ('candidate', 'cand000'))}
    targets = set(request['targets'][unit])
    rows = []
    for row in declared:
        counts = {}
        for role in identities:
            counts[role] = []
            for pair in range(5):
                raw = [p for p in processes if p['id'] == row['id']
                       and p['role'] == role and p['pair'] == pair]
                assert len(raw) in (1, 2)
                raw.sort(key=lambda p: p['iterations'] or 0)
                for process in raw:
                    path = folder / process['raw']
                    assert digest(path) == process['raw_sha256']
                    assert process['binary_sha256'] == identities[role]['sha256']
                    actual = parse_macos_time(path.read_text(), cycles=False)['instructions']
                    assert actual == process['instructions'] > 0
                value = raw[-1]['instructions']
                if len(raw) == 2:
                    assert [p['iterations'] for p in raw] == [1048576, 2097152]
                    value -= raw[0]['instructions']
                assert value > 0
                counts[role].append(value)
        assert counts == row['counts']
        ratios = [c / b for b, c in zip(counts['base'], counts['candidate'])]
        assert ratios == row['ratios'] and statistics.median(ratios) == row['ratio']
        rows.append(dict(id=row['id'], target=row['id'] in targets,
                         ratio=statistics.median(ratios), counts=counts, ratios=ratios))
    assert targets <= {r['id'] for r in rows}
    geomean = math.exp(statistics.mean(math.log(r['ratio']) for r in rows if r['target']))
    regressions = [r['id'] for r in rows if not r['target'] and r['ratio'] > 1.003]
    return dict(unit=unit, revisions=request['revisions'], target_geomean=geomean,
                target_pass=geomean <= 0.995, control_regressions=regressions,
                pass_gate=geomean <= 0.995 and not regressions, cases=rows,
                request_sha256=digest(folder / 'request.json'),
                processes_sha256=digest(folder / 'processes.json'))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('folder', type=Path)
    parser.add_argument('unit', choices=['u1', 'u2', 'u3'])
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    result = replay(args.folder, args.unit)
    args.output.write_text(json.dumps(result, indent=2, sort_keys=True) + '\n')
    print(f"{args.unit}: {result['target_geomean']:.9f}; pass={result['pass_gate']}")
    print('| case | role | instructions candidate/base |')
    print('| --- | --- | ---: |')
    for row in result['cases']:
        print(f"| {row['id']} | {'target' if row['target'] else 'control'} | {row['ratio']:.9f} |")


if __name__ == '__main__':
    main()
