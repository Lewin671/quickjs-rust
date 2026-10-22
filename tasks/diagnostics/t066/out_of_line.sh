#!/usr/bin/env bash
# T066 out-of-line entry check: non-timing symbol sizes and call targets for the
# interpreter's hot functions in base, 4e7dda32 (inlined allocator) and HEAD.
set -euo pipefail
export PYTHONDONTWRITEBYTECODE=1
. scripts/lib.sh
cargo_bin="$(qjs_resolve_cargo)"
base="$(qjs_build_revision "$PWD" "$cargo_bin" 9eba7a1d6ae8b67397d20e0d6c93ac79037f5e30)"
pre="$(qjs_build_revision "$PWD" "$cargo_bin" 4e7dda328afdb5dcd848d16a4e2e7f6fca29bcc4)"
post="$(qjs_build_revision "$PWD" "$cargo_bin" "$(git rev-parse HEAD)")"
python3 - "$base" "$pre" "$post" "$BENCH_OUT" <<'PY'
import hashlib, json, re, subprocess, sys
from collections import defaultdict
from pathlib import Path
from tools.benchmark import symbols

base, pre, post, out = map(Path, sys.argv[1:5])
roles = [('base', base, '9eba7a1d'), ('inline', pre, '4e7dda32'), ('out-of-line', post, 'HEAD')]
HOT = [
    ('wide activation::run', 'compact_fn::wide::activation::run', 'wide10activation3run'),
    ('compact activation::run', 'compact_fn::activation::run', 'compact_fn10activation3run'),
    ('Vm::run_current_activation', 'Vm>::run_current_activation', '2Vm22run_current_activation'),
    ('Vm::run_completion', 'Vm>::run_completion', '2Vm14run_completion'),
]
LABEL = re.compile(r'^_+([A-Za-z0-9_.$]+):$')
ALLOC = ('9allocator', '__rust_alloc', '__rust_dealloc', '__rust_realloc')


def disassembly(binary):
    text = subprocess.run(['otool', '-tV', str(binary)], capture_output=True,
                          text=True, check=True).stdout
    current = None
    calls = defaultdict(lambda: defaultdict(int))
    for line in text.splitlines():
        match = LABEL.match(line.strip())
        if match:
            current = match.group(1)
            continue
        if current is None:
            continue
        parts = line.split('\t')
        if len(parts) < 2:
            continue
        op = parts[1].strip()
        if op not in ('callq', 'call', 'bl', 'blx'):
            continue
        target = parts[2].strip().lstrip('_') if len(parts) > 2 else ''
        if not target:
            continue
        if any(token in target for token in ALLOC):
            calls[current]['allocator'] += 1
        elif any(token in target for token in ('malloc', 'calloc', 'realloc', 'free')):
            calls[current]['system'] += 1
        else:
            calls[current]['other'] += 1
    return calls


report = {'revisions': {}, 'cases': {}}
for role, binary, revision in roles:
    sizes = symbols.symbol_sizes(binary)
    calls = disassembly(binary)
    report['revisions'][role] = {
        'revision': revision,
        'binary_sha256': hashlib.sha256(binary.read_bytes()).hexdigest(),
        'hot': {},
    }
    for label, size_suffix, call_pattern in HOT:
        # Apple nm on the measurement host does not demangle Rust names, while
        # the local Xcode nm does; accept either spelling for the size lookup.
        size = next((value for name, value in sizes.items()
                     if name.endswith(size_suffix) or call_pattern in name), None)
        allocator_calls = sum(kinds.get('allocator', 0)
                              for name, kinds in calls.items() if call_pattern in name)
        system_calls = sum(kinds.get('system', 0)
                           for name, kinds in calls.items() if call_pattern in name)
        report['revisions'][role]['hot'][label] = {
            'size_bytes': size,
            'allocator_calls': allocator_calls,
            'system_calls': system_calls,
        }
    report['revisions'][role]['allocator_entries'] = {
        name: size for name, size in sizes.items()
        if 'SizeClassAllocator' in name or 'as core::alloc::global::GlobalAlloc' in name
    }
    report['revisions'][role]['demangled'] = any(
        '<' in name or '::' in name for name in sizes)
    report['revisions'][role]['size_samples'] = dict(sorted(
        ((name, size) for name, size in sizes.items() if 'activation' in name),
        key=lambda row: -row[1])[:5])
(out / 'symbols.json').write_text(json.dumps(report, indent=2) + '\n')
for role in report['revisions']:
    print('REVISION', role, report['revisions'][role]['revision'])
    for label, row in report['revisions'][role]['hot'].items():
        print(f"  {label}: size={row['size_bytes']} allocator_calls={row['allocator_calls']} "
              f"system_calls={row['system_calls']}")
    print('  allocator entries:', report['revisions'][role]['allocator_entries'])
PY
