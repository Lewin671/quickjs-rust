"""Queued, non-timing K0 clean-build matrix; never edits the input checkout.

Reuse the spike's same archive/source/target paths, independent layout test,
canonical build_cache recipe, smoke and retained source/build identities.
A/A failure prevents interpreting perturbations. Each requested build is clean.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import re
import shutil
import subprocess
import sys

from .build_cache import RUST_RECIPE
from .build_cache_identity import (cargo_configuration_identity,
                                   effective_build_environment, version_identity)
from .kernel_isolation import compare, fingerprint
from .kernel_macho import apple_tool_identity
from .kernel_perturb import (BUILDER, OBJECT, install_layout_probe, perturb,
                             validate_layout)

TARGET = 'x86_64-apple-darwin'
# Decoder qualification is explicit; compiler upgrades need a new matrix/audit.
RUST = '1.95.0'
LLVM = '22.1.2'


def run(argv, **kwargs):
    return subprocess.check_output([str(a) for a in argv], text=True, **kwargs).strip()


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


def matrix(full, kind):
    return [('r0a', 'R0'), ('r0b', 'R0'), ('h8', 'H8'), ('c8', 'C8')] if full else [
        ('r0a', 'R0'), (kind.lower(), kind)]


def admit_host(output):
    queue = os.environ.get('BENCH_OUT')
    if not queue or 'no-timing' not in (Path(queue).parent / 'status').read_text():
        raise ValueError('canonical builds require an imac-bench run job without --timing')
    if not output.is_relative_to(Path(queue).resolve()):
        raise ValueError('output must be inside BENCH_OUT')
    rust = run(['rustc', '-vV'])
    if (platform.system(), platform.machine()) != ('Darwin', 'x86_64') or any(
            line not in rust.splitlines() for line in (
                'host: ' + TARGET, 'release: ' + RUST, 'LLVM version: ' + LLVM)):
        raise ValueError('unknown host/toolchain; K0 qualifies only Darwin x86_64 Rust1.95.0/LLVM22.1.2')
    env = effective_build_environment()
    unexpected = {k: v for k, v in env.items() if v and
                  k not in RUST_RECIPE['environment']}
    if unexpected:
        raise ValueError('unreviewed build environment: ' + ', '.join(unexpected))
    return {'rustc': rust, 'recipe': RUST_RECIPE,
            'environment': env, 'cargo_configuration': cargo_configuration_identity(),
            'tools': {**{t: version_identity(t) for t in ('rustc', 'cargo')},
                      **{t: apple_tool_identity(t) for t in ('cc', 'ld', 'nm', 'otool')}}}


def parse_layout(log):
    result = {}
    for line in log.splitlines():
        match = re.fullmatch(r'(?:LAYOUT|OFFSET) (\w+) (\d+)', line)
        if match:
            if match[1] in result:
                raise ValueError('duplicate layout witness')
            result[match[1]] = int(match[2])
    return result


def validate_probe(base, candidate, kind):
    validate_layout(base['layout'], candidate['layout'], kind)
    if kind != 'R0' and candidate['sha256'] == base['sha256']:
        raise ValueError('optimized-away perturbation: canonical image unchanged')


def build_one(root, ref, output, name, kind):
    source, target = output / 'source', output / 'canonical-target'
    dest = output / name
    dest.mkdir()
    for path in (source, target):
        if path.exists():
            shutil.rmtree(path)
    source.mkdir()
    archive = subprocess.check_output(['git', 'archive', ref], cwd=root)
    subprocess.run(['tar', '-x', '-C', str(source)], input=archive, check=True)
    if run(['rustc', '-vV'], cwd=source) != run(['rustc', '-vV'], cwd=root):
        raise ValueError('source revision selects a different compiler/toolchain')
    perturb(source, kind)
    for path in (OBJECT, BUILDER):
        shutil.copy2(source / path, dest / path.name)
    original = install_layout_probe(source)
    env = os.environ.copy()
    # Independent probe flags never enter the canonical build. Reuse only its
    # test dependencies, as in the spike; canonical targets are always removed.
    env['CARGO_TARGET_DIR'] = str(output / 'layout-target')
    env['CARGO_INCREMENTAL'] = '0'
    with (dest / 'layout.log').open('w') as log:
        subprocess.run(['cargo', 'test', '--locked', '-p', 'qjs-runtime', '--features', 'agents',
                        'kernel_isolation_layout_probe', '--', '--nocapture'], cwd=source,
                       env=env, stdout=log, stderr=subprocess.STDOUT, check=True)
    (source / OBJECT).write_text(original)
    layout = parse_layout((dest / 'layout.log').read_text())
    with (dest / 'build.log').open('w') as log:
        subprocess.run([sys.executable, '-m', 'tools.benchmark.build_cache', 'build-rust',
                        '--source', str(source), '--target-dir', str(target)], cwd=root,
                       stdout=log, stderr=subprocess.STDOUT, check=True)
    shutil.copy2(target / TARGET / 'release/qjs', dest / 'qjs')
    if run([dest / 'qjs', '--raw', '-e', 'let s=0; for(let i=0;i<100;i++)s+=i; s;']) != '4950':
        raise ValueError('canonical smoke failed')
    report = fingerprint(dest / 'qjs', dest / 'fingerprint.json')
    record = {'kind': kind, 'sha256': sha(dest / 'qjs'), 'layout': layout,
              'source_sha256': {str(p): sha(source / p) for p in (OBJECT, BUILDER)}}
    return record, report


def execute(root, ref, output, full, kind):
    identity = admit_host(output)
    output.mkdir(parents=True, exist_ok=False)
    ref = run(['git', 'rev-parse', ref + '^{commit}'], cwd=root)
    producer = run(['git', 'rev-parse', 'HEAD'], cwd=root)
    requested = matrix(full, kind)
    write(output / 'request.json', {'base': ref, 'producer': producer, 'matrix': requested,
          'timing': False, 'attempts_consumed': 0, 'identity': identity,
          'producer_sha256': {p.name: sha(p) for p in sorted(Path(__file__).parent.glob('kernel_*.py'))}})
    records, comparisons, reference = {}, {}, None
    for name, probe in requested:
        if admit_host(output) != identity:
            raise ValueError('build toolchain/environment identity changed during matrix')
        record, report = build_one(root, ref, output, name, probe)
        records[name] = record
        write(output / 'builds.json', records)
        if reference is None:
            reference = report
            validate_layout(record['layout'], record['layout'], 'R0')
        else:
            validate_probe(records['r0a'], record, probe)
            comparisons[name] = compare(reference, report)
            write(output / 'comparisons.json', comparisons)
            if probe == 'R0' and not comparisons[name]['pass']:
                raise ValueError('A/A failed; perturbations are uninterpretable')
        print('BUILT', name, record['sha256'], flush=True)
    if admit_host(output) != identity:
        raise ValueError('build toolchain/environment identity changed during matrix')
    result = {'pass': all(c['pass'] for c in comparisons.values()), 'comparisons': comparisons}
    write(output / 'result.json', result)
    # Keep images/logs/fingerprints; fetching the job must not copy build trees.
    for name in ('source', 'canonical-target', 'layout-target'):
        shutil.rmtree(output / name)
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--ref', default='HEAD', help='committed source revision (working edits excluded)')
    parser.add_argument('--output', type=Path, required=True, help='new directory inside BENCH_OUT')
    parser.add_argument('--perturbation', choices=('H8', 'C8'), default='H8')
    parser.add_argument('--full', action='store_true', help='four clean builds: A/A + H+8 + C+8')
    args = parser.parse_args()
    output = args.output.resolve()
    was_present = output.exists()
    try:
        result = execute(Path.cwd(), args.ref, output, args.full, args.perturbation)
    except (ValueError, KeyError, OSError, subprocess.SubprocessError) as error:
        if not was_present and output.is_dir():
            write(output / 'result.json', {'pass': False, 'error': str(error)})
        print('K0 error: ' + str(error), file=sys.stderr)
        return 2
    finally:
        if not was_present and (output / 'request.json').is_file():
            for name in ('source', 'canonical-target', 'layout-target'):
                if (output / name).is_dir():
                    shutil.rmtree(output / name)
    print('K0 PASS' if result['pass'] else 'K0 FAIL (see comparisons.json)', flush=True)
    return 0 if result['pass'] else 1


if __name__ == '__main__':
    raise SystemExit(main())
