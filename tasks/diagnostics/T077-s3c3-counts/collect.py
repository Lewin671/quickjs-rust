#!/usr/bin/env python3
"""Replay the frozen public 109-case receipt: instructions, counters, coverage.

No timing metric is retained. Never add held-out inputs. Use a fresh output
folder for each runtime source; a partial folder resumes only its pinned inputs.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
from tools.benchmark import screen

ROOT = Path.cwd()
PROBES = {
    'activation_builds': ('context/activation.rs', 'for_call', 0),
    'plain_activation_builds': ('context/activation.rs', 'new', 0),
    'context_materializations': ('context/activation_borrow.rs', 'materialize', 0),
    'frame_pushes': ('bytecode/frame_bank.rs', 'push', 0),
    'frame_pops': ('bytecode/frame_bank.rs', 'pop', 0),
    'typed_facade_views': ('bytecode/frame_bank.rs', 'view', 0),
    'root_facade_views': ('bytecode/frame_bank.rs', 'view', 1),
    'wide_entries': ('bytecode/compact_fn/wide/activation.rs', 'enter_boundary', 0),
    'numeric_entries': ('bytecode/compact_fn/activation.rs', 'enter_boundary', 0),
    'checked_assigns': ('binding/cell.rs', 'assign', 0),
    'initialized_mutable_replaces': ('binding/cell.rs', 'replace_initialized_mutable', 0),
    'local_slow_reads': ('bytecode/vm_bindings.rs', 'load_local_slow', 0),
    'vm_boundaries': ('bytecode/vm_references.rs', 'run_binding_boundary', 0),
    'reference_events': ('bytecode/vm_references.rs', 'run_reference_event', 0),
    'vm_reference_cleanup': ('bytecode/vm_references.rs', 'prune_references', 0),
    'wide_boundaries': ('bytecode/compact_fn/wide/activation/cold_ops.rs', 'reference_boundary', 0),
    'wide_reference_cleanup': ('bytecode/reference_admission.rs', 'cleanup_checked', 0),
    'suspension_exports': ('bytecode/vm_generator.rs', 'into_record', 0),
    'generator_prologues': ('bytecode/vm_generator.rs', 'start_suspended_at_body', 0),
    'suspended_starts': ('bytecode/vm_generator.rs', 'run_from_start', 0),
    'pure_user_calls': ('bytecode/typed_loop/execute/user_calls.rs', 'call', 0),
    'unboxed_shortcuts': ('bytecode/vm_numeric_leaf/effects.rs', 'try_eval_pure_number_shortcut', 0),
}


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def invoke(binary, program, arguments, env, instructions=False, profile=None):
    command = [str(binary), '--raw', str(program), *arguments]
    if instructions:
        command = ['/usr/bin/time', '-l', *command]
    if profile:
        env = {**env, 'LLVM_PROFILE_FILE': str(profile)}
    result = subprocess.run(command, env=env, text=True, capture_output=True, timeout=600)
    assert result.returncode == 0, (program, result.stderr)
    if instructions:
        data = int(re.search(r'(\d+)\s+instructions retired', result.stderr)[1])
    else:
        data = {k: int(v) for k, v in re.findall(r'^\s+(\w+) (\d+)$', result.stderr, re.M)}
    return result.stdout, data


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ['binary', 'counters', 'coverage', 'output']:
        parser.add_argument('--' + name, type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    work = args.output / 'programs'; work.mkdir(exist_ok=True)
    frozen = json.loads(Path('tasks/diagnostics/T077-s3c-counts.json').read_text())
    cases = screen.resolve_cases(['development', 'external', 'broad', 'sentinel'], screen.DEFAULT_CACHE, work)
    programs = {case.id: case.workload for case in cases}
    case_records = {case.id: case for case in cases}
    audio = programs['external/kraken-1.1/audio-beat-detection']
    path = work / 'audio-fixed-clock.js'
    path.write_text('var __s3cTimestamp = 0;\n' + audio.read_text().replace('(new Date()).getTime()', '(__s3cTimestamp += 16)'))
    programs['diagnostic/audio-fixed-clock'] = path
    path = work / 'aes-fixed-clock.js'
    path.write_text(programs['external/sunspider-1.0/crypto-aes'].read_text().replace('(new Date()).getTime()', '1234567890000'))
    programs['diagnostic/aes-fixed-clock'] = path
    for path in Path('tasks/diagnostics/T077-s3b-r4/repro').glob('*.js'):
        programs['repro/' + path.stem] = path
    assert len(frozen['records']) == 109 and set(programs) == {row['id'] for row in frozen['records']}
    lines = {}
    for key, (path, function, occurrence) in PROBES.items():
        source = (ROOT / 'crates/qjs-runtime/src' / path).read_text().splitlines()
        matches = [i + 1 for i, line in enumerate(source) if re.search(r'\bfn\s+' + function + r'\b', line)]
        lines[key] = (path, matches[occurrence])
    binaries = {key: getattr(args, key) for key in ['binary', 'counters', 'coverage']}
    request = {'parent': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
               'source_sha256': {str(p): digest(p) for p in sorted(Path('crates/qjs-runtime/src').rglob('*.rs'))},
               'binaries': {key: digest(path) for key, path in binaries.items()},
               'corpus': {row['id']: row['sha256'] for row in frozen['records']},
               'probe_lines': lines, 'timing': False, 'counter_repeats': 2, 'coverage_repeats': 1}
    req = args.output / 'request.json'
    if req.exists():
        assert json.loads(req.read_text()) == json.loads(json.dumps(request)), 'source/input changed during resume'
    else:
        req.write_text(json.dumps(request, indent=2) + '\n')
    llvm = Path(subprocess.check_output(['rustc', '--print', 'sysroot'], text=True).strip()) / 'lib/rustlib/aarch64-apple-darwin/bin'
    env = {k: v for k, v in os.environ.items() if not k.startswith(('QJS_', 'LLVM_PROFILE_FILE'))}; env['TZ'] = 'UTC'
    rows = []
    for old in frozen['records']:
        ident = old['id']; program = programs[ident]; arguments = old['args']
        assert digest(program) == old['sha256'], (ident, 'input differs from frozen receipt')
        key = ident.replace('/', '--'); dest = args.output / (key + '.json')
        if dest.exists():
            rows.append(json.loads(dest.read_text())); continue
        maps = []; stdout = None
        for _ in range(2):
            output, events = invoke(args.counters, program, arguments, env)
            if ident in case_records: screen._check_output(case_records[ident], output)
            if stdout is None: stdout = output
            assert stdout == output, ident
            maps.append(events)
        if ident.startswith(('development/', 'repro/', 'diagnostic/', 'broad/', 'sentinel/')):
            assert maps[0] == maps[1], (ident, 'nondeterministic semantic counters')
        instructions = []
        for _ in range(3 if ident.startswith('repro/') else 1):
            output, count = invoke(args.binary, program, arguments, env, instructions=True)
            assert output == stdout, ident
            instructions.append(count)
        raw = args.output / (key + '.profraw'); prof = raw.with_suffix('.profdata')
        output, _ = invoke(args.coverage, program, arguments, env, profile=raw)
        assert output == stdout, ident
        subprocess.run([str(llvm / 'llvm-profdata'), 'merge', '-sparse', str(raw), '-o', str(prof)], check=True, capture_output=True)
        coverage = json.loads(subprocess.check_output([str(llvm / 'llvm-cov'), 'export', str(args.coverage), '-instr-profile=' + str(prof)], text=True))['data'][0]
        hits = {}
        for name, (path, line) in lines.items():
            functions = [f for f in coverage['functions'] if f['filenames'][0].endswith('/' + path)
                         and f['regions'][0][0] == line and not f['name'].startswith('_RNC')]
            assert functions, (name, ident)
            hits[name] = sum(f['count'] for f in functions)
        row = {'id': ident, 'sha256': digest(program), 'args': arguments,
               'stdout_sha256': hashlib.sha256(stdout.encode()).hexdigest(), 'instructions': instructions,
               'counters': maps[0], 'counter_repeat': 'identical' if maps[0] == maps[1] else maps[1], 'coverage': hits}
        dest.write_text(json.dumps(row, indent=2) + '\n'); rows.append(row)
        raw.unlink(); prof.unlink()
        print(ident, flush=True)
        (args.output / 'counts.json').write_text(json.dumps({'request': request, 'records': rows}, separators=(',', ':')) + '\n')
    assert len(rows) == 109
    assert request['binaries'] == {key: digest(path) for key, path in binaries.items()}
    assert request['source_sha256'] == {str(p): digest(p) for p in sorted(Path('crates/qjs-runtime/src').rglob('*.rs'))}


if __name__ == '__main__':
    main()
