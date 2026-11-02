#!/usr/bin/env python3
"""Interleave instruction-only replay of the frozen 109 public receipt inputs."""
import argparse
import hashlib
import json
import math
import os
from pathlib import Path
import re
import subprocess
from tools.benchmark import screen


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--before', type=Path, required=True)
    p.add_argument('--after', type=Path, required=True)
    p.add_argument('--output', type=Path, required=True)
    a = p.parse_args()
    a.output.mkdir(parents=True, exist_ok=False)
    work = a.output / 'programs'
    work.mkdir()
    cases = screen.resolve_cases(['development', 'external', 'broad', 'sentinel'], screen.DEFAULT_CACHE, work)
    programs = {c.id: c.workload for c in cases}
    for name, original, old, new in [
        ('audio-fixed-clock', 'external/kraken-1.1/audio-beat-detection', '(new Date()).getTime()', '(__s3cTimestamp += 16)'),
        ('aes-fixed-clock', 'external/sunspider-1.0/crypto-aes', '(new Date()).getTime()', '1234567890000'),
    ]:
        path = work / (name + '.js')
        source = programs[original].read_text().replace(old, new)
        path.write_text(('var __s3cTimestamp = 0;\n' if name.startswith('audio') else '') + source)
        programs['diagnostic/' + name] = path
    programs.update({'repro/' + f.stem: f for f in Path('tasks/diagnostics/T077-s3b-r4/repro').glob('*.js')})
    frozen = json.loads(Path('tasks/diagnostics/T077-s3c-counts.json').read_text())['records']
    assert len(frozen) == 109 and set(programs) == {r['id'] for r in frozen}
    source = lambda: {str(f): digest(f) for f in sorted(Path('crates/qjs-runtime/src').rglob('*.rs'))}
    bins = {k: str(getattr(a, k).resolve()) for k in ['before', 'after']}
    request = {'revision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
               'runtime_sha256': source(), 'binaries': {k: digest(v) for k, v in bins.items()},
               'inventory_sha256': digest('tasks/diagnostics/T077-s3c-counts.json'),
               'pairs': 3, 'order': 'AB BA AB', 'metric': 'instructions retired; no timing', 'held_out': False}
    (a.output / 'request.json').write_text(json.dumps(request, indent=2) + '\n')
    env = {k: v for k, v in os.environ.items() if not k.startswith(('QJS_', 'LLVM_PROFILE_FILE'))}
    env['TZ'] = 'UTC'
    rows = []
    for old in frozen:
        ident = old['id']; program = programs[ident]
        assert digest(program) == old['sha256'], ident
        counts = {k: [] for k in bins}; stdout = None
        for pair in range(3):
            for key in (['before', 'after'] if pair % 2 == 0 else ['after', 'before']):
                r = subprocess.run(['/usr/bin/time', '-l', bins[key], '--raw', str(program), *old['args']],
                                   text=True, capture_output=True, env=env, timeout=600)
                assert r.returncode == 0, (ident, key, r.stderr)
                if stdout is None: stdout = r.stdout
                assert stdout == r.stdout, (ident, key, 'output changed')
                counts[key].append(int(re.search(r'(\d+)\s+instructions retired', r.stderr)[1]))
        ratio = sorted(counts['after'])[1] / sorted(counts['before'])[1]
        rows.append({'id': ident, 'sha256': old['sha256'], 'args': old['args'],
                     'stdout_sha256': hashlib.sha256(stdout.encode()).hexdigest(), 'instructions': counts, 'ratio': ratio})
        (a.output / 'records.json').write_text(json.dumps(rows, indent=2) + '\n')
        print(ident, ratio, flush=True)
    assert request['runtime_sha256'] == source()
    assert request['binaries'] == {k: digest(v) for k, v in bins.items()}
    geomean = lambda xs: math.exp(sum(math.log(x) for x in xs) / len(xs))
    summary = {'count': len(rows), 'geomean': geomean([r['ratio'] for r in rows]),
               'lanes': {lane: geomean([r['ratio'] for r in rows if r['id'].startswith(lane + '/')])
                         for lane in ['development', 'external', 'broad', 'sentinel', 'repro', 'diagnostic']},
               'worst': sorted(rows, key=lambda r: r['ratio'], reverse=True)[:10]}
    (a.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print(json.dumps({k: v for k, v in summary.items() if k != 'worst'}), flush=True)

if __name__ == '__main__':
    main()
