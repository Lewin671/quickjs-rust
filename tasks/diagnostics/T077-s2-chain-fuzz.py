"""Reproduce instantiated chain IC differential seeds on a debug CLI."""
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path
import argparse
import hashlib
import json
import os
import subprocess

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--binary', type=Path, required=True)
parser.add_argument('--output', type=Path, required=True)
parser.add_argument('--seeds', type=int, default=300)
parser.add_argument('--workers', type=int, default=4)
parser.add_argument('--warm-branches', action='store_true')
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=False)
template = Path(__file__).with_suffix('.js').read_text()
assert template.count('__SEED__') == 1
prefix = "var branchKeep=[];for(var bi=0;bi<12;bi++){var branch={};branch['branch'+bi]=bi;branchKeep[bi]=branch;}\n" if args.warm_branches else ''
binary = args.binary.resolve()
env = {k: v for k, v in os.environ.items() if k not in ('QJS_DISABLE_IC', 'QJS_VERIFY_IC')}
env['TZ'] = 'UTC'
sha = lambda data: hashlib.sha256(data).hexdigest()

def run(seed):
    path = args.output / f'{seed}.js'
    source = prefix + template.replace('__SEED__', str(seed))
    path.write_text(source)
    row = {'seed': seed, 'source_sha256': sha(source.encode()), 'modes': {}}
    for mode, policy in [('on', {}), ('off', {'QJS_DISABLE_IC': '1'}), ('verify', {'QJS_VERIFY_IC': '1'})]:
        try:
            result = subprocess.run([str(binary), str(path.resolve())], env={**env, **policy}, capture_output=True, timeout=120)
            row['modes'][mode] = {'status': result.returncode, 'stdout_sha256': sha(result.stdout), 'stderr_sha256': sha(result.stderr)}
            (args.output / f'{seed}-{mode}.stdout').write_bytes(result.stdout)
            (args.output / f'{seed}-{mode}.stderr').write_bytes(result.stderr)
        except subprocess.TimeoutExpired:
            row['modes'][mode] = {'status': 124}
    modes = row['modes']
    row['same'] = modes['on'] == modes['off'] == modes['verify'] and modes['on']['status'] == 0
    return row

rows = []
with ThreadPoolExecutor(max_workers=args.workers) as pool:
    for row in pool.map(run, range(1, args.seeds + 1)):
        rows.append(row)
        if not row['same']:
            print('DIFF', json.dumps(row), flush=True)
        if len(rows) % 25 == 0:
            print('progress', len(rows), flush=True)
        (args.output / 'results.json').write_text(json.dumps({'binary_sha256': sha(binary.read_bytes()), 'template_sha256': sha(template.encode()), 'warm_branches': args.warm_branches, 'records': rows}, indent=2) + '\n')
print('COMPLETE', len(rows), 'same', sum(row['same'] for row in rows), flush=True)
raise SystemExit(0 if all(row['same'] for row in rows) else 1)
