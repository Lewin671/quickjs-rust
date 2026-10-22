"""Seven clean canonical iMac builds, serialized by imac-bench, never timing."""
import hashlib
import json
import os
import shutil
import subprocess
import sys
from pathlib import Path

from transform import BASE, EXECUTE, OBJECT, BUILDER, generate, layout_probe

OUT = Path(os.environ['BENCH_OUT']).resolve()
assert 'no-timing' in (OUT.parent / 'status').read_text()
MATRIX = [('r0a', False, 'R0'), ('r0b', False, 'R0'),
          ('h8', False, 'H8'), ('c8', False, 'C8'),
          ('o-r0', True, 'R0'), ('o-h8', True, 'H8'), ('o-c8', True, 'C8')]
sha = lambda p: hashlib.sha256(p.read_bytes()).hexdigest()
request = {'base': BASE, 'matrix': MATRIX, 'timing': False,
           'attempts_consumed': 0, 'clean_same_source_and_target_path': True,
           'recipe': 'tools.benchmark.build_cache.RUST_RECIPE',
           'producer_sha256': {p.name: sha(p) for p in [Path(__file__), Path(__file__).with_name('transform.py')]},
           'rustc': subprocess.check_output(['rustc', '-vV'], text=True)}
(OUT / 'request.json').write_text(json.dumps(request, indent=2) + '\n')
source, target = OUT / 'source', OUT / 'canonical-target'
records = {}
for role, outlined, perturbation in MATRIX:
    dest = OUT / role
    dest.mkdir()
    for p in (source, target):
        if p.exists():
            shutil.rmtree(p)
    source.mkdir()
    archive = subprocess.check_output(['git', 'archive', BASE])
    subprocess.run(['tar', '-x', '-C', str(source)], input=archive, check=True)
    generate(source, outlined, perturbation)
    originals = {p: (source / p).read_text() for p in [EXECUTE, OBJECT, BUILDER]}
    for p, text in originals.items():
        (dest / p.name).write_text(text)
    # Independent type-layout probe; never pass its flags/target to canonical build.
    layout_probe(source)
    env = os.environ.copy()
    env['CARGO_TARGET_DIR'] = str(OUT / 'layout-target')
    with (dest / 'layout.log').open('w') as log:
        subprocess.run(['cargo', 'test', '-p', 'qjs-runtime', '--features', 'agents',
                        'diagnostic_t064_layout', '--', '--nocapture'],
                       cwd=source, env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    for p, text in originals.items():
        (source / p).write_text(text)
    with (dest / 'build.log').open('w') as log:
        subprocess.run([sys.executable, '-m', 'tools.benchmark.build_cache', 'build-rust',
                        '--source', str(source), '--target-dir', str(target)],
                       stdout=log, stderr=subprocess.STDOUT, check=True)
    binary = target / 'x86_64-apple-darwin/release/qjs'
    shutil.copy2(binary, dest / 'qjs')
    smoke = subprocess.check_output([str(dest / 'qjs'), '--raw', '-e',
        'let s=0; for(let i=0;i<100;i++)s+=i; s;'], text=True)
    assert smoke.strip() == '4950', smoke
    records[role] = {'sha256': sha(dest / 'qjs'),
                     'source_sha256': {str(p): sha(source / p) for p in [EXECUTE, OBJECT, BUILDER]},
                     'layout': [s for s in (dest / 'layout.log').read_text().splitlines()
                                if s.startswith(('LAYOUT ', 'OFFSET '))]}
    (OUT / 'builds.json').write_text(json.dumps(records, indent=2) + '\n')
    print('BUILT', role, records[role], flush=True)
assert records['r0a']['source_sha256'] == records['r0b']['source_sha256']
# All other source inputs came from the exact same archive and canonical recipe.
print('MATRIX_COMPLETE', flush=True)
