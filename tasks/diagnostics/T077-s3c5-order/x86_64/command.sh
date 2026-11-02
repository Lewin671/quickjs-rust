python3 - <<'S3C_ORDER_PY'
#!/usr/bin/env python3
"""Regenerate the native order with the canonical build and public profile set.

Run only on an otherwise idle measurement host, through the runner on x86.
Only linker order changes; fail if any runtime source changes during the run.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import platform
import shutil
import subprocess
import sys

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--output', type=Path, default=Path(os.environ.get('BENCH_OUT', 'target/s3c5/order-arm')))
parser.add_argument('--target-dir', type=Path)
args = parser.parse_args()
args.output.mkdir(parents=True, exist_ok=True)
root = Path.cwd()
arch = platform.machine()
assert arch in ['arm64', 'x86_64'], arch
name = 'hot-functions-x86_64.order' if arch == 'x86_64' else 'hot-functions.order'
order = root / 'crates/qjs-cli' / name
sha = lambda path: hashlib.sha256(Path(path).read_bytes()).hexdigest()
runtime = lambda: {str(p): sha(p) for p in sorted(Path('crates/qjs-runtime/src').rglob('*.rs'))}
source = runtime()
env = {k: v for k, v in os.environ.items() if not k.startswith(('QJS_', 'LLVM_PROFILE_FILE'))}
request = {'source_commit': subprocess.check_output(['git', 'rev-parse', 'HEAD'], text=True).strip(),
           'runtime_source_sha256': source, 'architecture': arch,
           'cases': ['external', 'sentinel', 'broad'], 'seconds_per_case': 1,
           'previous_order_sha256': sha(order), 'offsets': ['0x400', '0xc40'],
           'recipe': 'tools.benchmark.build_cache.RUST_RECIPE', 'held_out': False}
(args.output / 'request.json').write_text(json.dumps(request, indent=2) + '\n')
shutil.copyfile(order, args.output / 'previous.order')
build_args = [sys.executable, '-m', 'tools.benchmark.build_cache', 'build-rust',
              '--source', str(root), '--target-dir', str(args.target_dir or args.output / 'build')]
def build():
    result = subprocess.check_output(build_args, env=env, text=True)
    return Path(result.strip().splitlines()[-1])
binary = build()
request['profile_binary_sha256'] = sha(binary)
(args.output / 'request.json').write_text(json.dumps(request, indent=2) + '\n')
subprocess.run([sys.executable, '-m', 'tools.benchmark.order_file', '--binary', str(binary),
                '--case', 'external', '--case', 'sentinel', '--case', 'broad',
                '--seconds', '1', '--output', str(order)], env=env, check=True)
shutil.copyfile(order, args.output / 'profiled.order')
for attempt in range(3):
    binary = build()
    audit = subprocess.run([sys.executable, '-m', 'tools.benchmark.layout_audit',
        '--binary', str(binary), '--order', str(order), '--strict', '--json'],
        env=env, text=True, capture_output=True)
    (args.output / f'audit-{attempt}.json').write_text(audit.stdout)
    (args.output / f'audit-{attempt}.stderr').write_text(audit.stderr)
    if audit.returncode == 0:
        break
    # Re-linking may change concrete extents. Re-pin the same declared offsets;
    # this is stabilization, never an offset scan or case-based layout choice.
    subprocess.run([sys.executable, '-m', 'tools.benchmark.layout_pin', '--binary', str(binary),
        '--order', str(order), '--offset', '0x400', '--vm-offset', '0xc40'], env=env, check=True)
else:
    raise RuntimeError('native order failed to stabilize')
assert runtime() == source, 'runtime changed during order generation'
shutil.copyfile(order, args.output / name)
summary = {'source_commit': request['source_commit'], 'architecture': arch,
           'binary_sha256': sha(binary), 'order_sha256': sha(order),
           'strict_audit_passed': True, 'relinks': attempt + 1,
           'order_lines': len(order.read_text().splitlines()), 'runtime_unchanged': True}
(args.output / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
print(json.dumps(summary), flush=True)

S3C_ORDER_PY

