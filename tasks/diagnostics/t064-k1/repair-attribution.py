"""Fixed three-image, four-case non-timing attribution for K1 repair 1.

Whole-process instructions use original workload bundles. Independent sample
runs repeat external bodies 40 times to retain short-lived processes; their
sample shares are diagnostic, never per-symbol retired-instruction counts.
Existing canonical builds, process counters, sampler and profile receipts own
the measurements. No acceptance threshold, timing decision or runtime routing.
"""
import hashlib
import json
import os
from pathlib import Path
import re
import subprocess
from functools import lru_cache

from tools.benchmark.kernel_macho import select_roles
from tools.benchmark.layout_symbols import parse_symbol
from tools.benchmark.layout_tables import text_symbols
from tools.benchmark.process import run_process
from tools.benchmark.screen import DEFAULT_CACHE, _argv, _check_output, resolve_cases

CASES = ['development/big.js/arith', 'external/sunspider-1.0/access-nbody',
         'broad/math_abs', 'sentinel/string_key_map_churn']
ITERATIONS = {'broad/math_abs': 100_000_000, 'sentinel/string_key_map_churn': 10_000_000}
BASE = '9eba7a1d6ae8b67397d20e0d6c93ac79037f5e30'
OLD = '671e99431aef710a4c1abc23a172b9cf468e5245'
EXPECTED = {'M': 'f5822aa32e4c7a3def2aaf5b8eff2107ba005a76252afb25f06077fe25f2e8eb',
            'cold': '0847b950b2279dbf194cca7bdf234e87b1b8410ea38f6177e64fa458e3b67a30'}
PREFIX = 'qjs_runtime::bytecode::typed_loop::execute::'
LINE = re.compile(r'^(?P<prefix>[\s+!:|]*)(?P<count>\d+) (?P<rest>.+)$')


def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write(path, value):
    path.write_text(json.dumps(value, indent=2) + '\n')


@lru_cache(maxsize=None)
def canonical(raw):
    parsed = parse_symbol(raw) or parse_symbol('_' + raw)
    return parsed.canonical if parsed else raw


def sample_summary(path, hot_symbols):
    graph = path.read_text().split('Call graph:', 1)[1].split('Total number in stack', 1)[0]
    nodes = []
    for line in graph.splitlines():
        match = LINE.match(line)
        if not match:
            continue
        raw = re.split(r'\s+\(in |\s+\[0x', match['rest'], maxsplit=1)[0].strip()
        nodes.append((len(match['prefix']), int(match['count']), raw))
    assert nodes and 'Thread_' in nodes[0][2], 'missing main-thread root'
    depth, total, _ = nodes[0]
    groups = dict.fromkeys(['service_body', 'service_callees', 'hot_execute_body',
                            'hot_callees', 'caller', 'other'], 0)
    symbols, stack = {}, []

    def finish(node, ancestors):
        _, count, raw, children = node
        own = count - children
        assert own >= 0, 'inconsistent sampling tree'
        if not own:
            return
        name = canonical(raw)
        service = lambda s: canonical(s).startswith(PREFIX + 'service::execute::<')
        if service(raw):
            group = 'service_body'
        elif any(service(a[2]) for a in ancestors):
            group = 'service_callees'
        elif raw.lstrip('_') in hot_symbols:
            group = 'hot_execute_body'
        elif any(a[2].lstrip('_') in hot_symbols for a in ancestors):
            group = 'hot_callees'
        elif (name.startswith(PREFIX + 'run::') or
              ('qjs_runtime::bytecode::' in name and any(s in name for s in
               ['::activation::run', '::Vm::run_current_activation', '::Vm::run_completion']))):
            group = 'caller'
        else:
            group = 'other'
        groups[group] += own
        key = (group, name)
        symbols[key] = symbols.get(key, 0) + own

    for index, (d, count, raw) in enumerate(nodes):
        if index and d <= depth:
            break
        while stack and stack[-1][0] >= d:
            node = stack.pop()
            finish(node, stack)
        if stack:
            stack[-1][3] += count
        stack.append([d, count, raw, 0])
    while stack:
        node = stack.pop()
        finish(node, stack)
    assert sum(groups.values()) == total, 'self-sample partition does not conserve main-thread samples'
    return {'main_samples': total, 'groups': groups,
            'symbols': [{'group': group, 'symbol': symbol, 'self_samples': count}
                        for (group, symbol), count in sorted(symbols.items(), key=lambda kv: -kv[1])]}


def main():
    root = Path.cwd()
    out = Path(os.environ['BENCH_OUT']).resolve()
    assert 'no-timing' in (out.parent / 'status').read_text()
    revision = subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip()
    revisions = {'M': BASE, 'cold': OLD, 'repair': revision}
    cache = root / 'target/perf-loop/compiler-fat16-abort-v1'
    binaries = {role: cache / ('rev-' + ref) / 'qjs' for role, ref in revisions.items()}
    identities, hot = {}, {}
    for role, binary in binaries.items():
        digest = sha(binary)
        if role in EXPECTED:
            assert digest == EXPECTED[role], 'wrong original exact image'
        inventory = text_symbols(binary)
        family, selected, _ = select_roles(inventory)
        hot[role] = {name.lstrip('_') for name in selected.values()}
        identities[role] = {'revision': revisions[role], 'binary': str(binary), 'sha256': digest,
                            'family': family, 'symbols': [
            {'symbol': canonical(name), 'bytes': inventory.size_of(name)}
            for name in inventory.names if canonical(name).startswith(PREFIX)]}
    work = out / 'attribution-work'
    work.mkdir()
    write(out / 'attribution-request.json', {'images': identities, 'cases': CASES,
          'internal_iterations': ITERATIONS, 'sample_external_repeats': 40,
          'sample_duration_seconds': 8, 'sample_interval_ms': 1,
          'timing': False, 'attempts_consumed': 0, 'producer_sha256': sha(Path(__file__))})
    records = []
    for case in resolve_cases(CASES, DEFAULT_CACHE, work):
        expected_stdout = None
        for role, binary in binaries.items():
            name = role + '-' + case.id.replace('/', '--')
            argv = _argv(binary, case, ITERATIONS.get(case.id))
            result = run_process(argv, 240)
            assert result.exit_code == 0 and not result.timed_out and result.instructions
            checked = _check_output(case, result.stdout)
            if expected_stdout is None:
                expected_stdout = checked
            assert checked == expected_stdout, 'three-image output mismatch'
            # Save all original whole-process evidence, not just its ratio.
            write(out / (name + '.process.json'), result.__dict__)
            profile_argv = argv.copy()
            workload = case.workload
            if case.kind != 'internal':
                workload = work / (case.id.replace('/', '--') + '-repeat40.js')
                workload.write_text('for (var __diagnostic_pass=0; __diagnostic_pass<40; __diagnostic_pass++) {\n'
                                    + case.workload.read_text() + '\n}\n')
                profile_argv = [str(binary), '--raw', str(workload)]
            raw = out / (name + '.sample')
            with (out / (name + '.sample.stdout')).open('x') as stdout, (out / (name + '.sample.stderr')).open('x') as stderr:
                process = subprocess.Popen(profile_argv, stdout=stdout, stderr=stderr)
                command = ['sample', str(process.pid), '8', '1', '-mayDie', '-file', str(raw)]
                try:
                    subprocess.run(command, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=30)
                    status = process.wait(timeout=240)
                finally:
                    if process.poll() is None:
                        process.kill()
                        process.wait()
            assert status == 0 and raw.is_file(), 'profile process failed'
            summary = sample_summary(raw, hot[role])
            receipt = out / (name + '-receipt')
            subprocess.run(['python3', '-m', 'tools.benchmark.profile', '--input', str(raw),
                '--binary', str(binary), '--workload', str(workload), '--base-sha', revisions[role],
                '--opportunity-id', case.id, '--tool', '/usr/bin/sample macOS built-in 1ms',
                '--command-json', json.dumps(command), '--output-dir', str(receipt)], check=True)
            record = {'case': case.id, 'role': role, 'argv': argv,
                      'workload_sha256': sha(case.workload), 'instructions': result.instructions,
                      'process_sha256': sha(out / (name + '.process.json')),
                      'profile_argv': profile_argv, 'receipt_sha256': sha(receipt / 'receipt.json'),
                      'sample_sha256': sha(raw), **summary}
            records.append(record)
            write(out / 'attribution.json', {'images': identities, 'records': records})
            print('ATTRIBUTED', case.id, role, result.instructions, summary['groups'], flush=True)
    assert len(records) == 12
    write(out / 'attribution-complete.json', {'cases': 4, 'images': 3, 'records': 12,
          'attribution_sha256': sha(out / 'attribution.json'), 'timing': False})


if __name__ == '__main__':
    main()
