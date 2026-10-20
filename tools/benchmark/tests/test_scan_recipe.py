"""Exercise layout-scan dispatch/restoration without compiling or measuring."""
from __future__ import annotations

import json
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path
from tools.benchmark.tests.bounded import run_bounded

ROOT = Path(__file__).resolve().parents[3]
FAKE_PYTHON = r'''
import json, os, sys
from pathlib import Path
root = Path(os.environ['SCAN_TEST_ROOT'])
args = sys.argv[1:]
with (root / 'calls.jsonl').open('a') as f:
    f.write(json.dumps(args) + '\n')
order = root / 'crates/qjs-cli/hot-functions-x86_64.order'
if args[0] == '-c':
    print('0x9a0' if 'executor_offsets' in args[1] else order)
elif args[:3] == ['-m', 'tools.benchmark.build_cache', 'build-rust']:
    count = root / 'build-count'
    n = int(count.read_text()) + 1 if count.exists() else 1
    count.write_text(str(n))
    if n == int(os.environ.get('FAIL_BUILD_NUMBER', '0')):
        sys.exit(7)
    target = Path(args[args.index('--target-dir') + 1])
    binary = target / 'test-target/release/qjs'
    binary.parent.mkdir(parents=True, exist_ok=True)
    binary.write_text('canonical binary')
    binary.chmod(0o755)
    print(binary)
elif args[:2] == ['-m', 'tools.benchmark.layout_pin']:
    order.write_text('pinned\n')
elif args[:2] != ['-m', 'tools.benchmark.screen']:
    raise SystemExit('unexpected command: ' + repr(args))
'''


class LayoutScanRecipeTests(unittest.TestCase):
    def run_scan(self, fail_build=0):
        with tempfile.TemporaryDirectory(prefix='qjs scan recipe ') as directory:
            root = Path(directory)
            (root / 'scripts').mkdir()
            script = root / 'scripts/layout-scan.sh'
            shutil.copyfile(ROOT / 'scripts/layout-scan.sh', script)
            (root / 'scripts/lib.sh').write_text(
                'qjs_resolve_cargo() { printf "%s\\n" /unused/cargo; }\n'
            )
            order = root / 'crates/qjs-cli/hot-functions-x86_64.order'
            order.parent.mkdir(parents=True)
            order.write_text('original\n')
            binary = root / 'bin/python3'
            binary.parent.mkdir()
            binary.write_text(f'#!{sys.executable}\n' + FAKE_PYTHON)
            binary.chmod(0o755)
            env = {**os.environ, 'PATH': str(binary.parent) + os.pathsep + os.environ['PATH'],
                   'SCAN_TEST_ROOT': str(root), 'FAIL_BUILD_NUMBER': str(fail_build)}
            result = run_bounded(['bash', str(script), '--offsets', '0x20',
                                     '--case', 'sentinel'], env=env,
                                    capture_output=True, text=True)
            calls = [json.loads(line) for line in (root / 'calls.jsonl').read_text().splitlines()]
            self.assertEqual(order.read_text(), 'original\n')
            return result, calls

    def test_build_and_relinks_use_canonical_binary_and_restore_order(self):
        result, calls = self.run_scan()
        self.assertEqual(result.returncode, 0, result.stderr)
        builds = [c for c in calls if c[:3] == ['-m', 'tools.benchmark.build_cache', 'build-rust']]
        self.assertEqual(len(builds), 3)
        for call in builds:
            self.assertEqual(call[call.index('--cargo') + 1], '/unused/cargo')
        pins = [c for c in calls if c[:2] == ['-m', 'tools.benchmark.layout_pin']]
        self.assertEqual(len(pins), 2)
        for call in pins:
            self.assertTrue(call[call.index('--binary') + 1].endswith('/test-target/release/qjs'))
            self.assertEqual(call[call.index('--vm-offset') + 1], '0x9a0')
        self.assertEqual(sum(c[:2] == ['-m', 'tools.benchmark.screen'] for c in calls), 1)

    def test_failed_relink_restores_order_without_measuring(self):
        result, calls = self.run_scan(fail_build=2)
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(any(c[:2] == ['-m', 'tools.benchmark.screen'] for c in calls))
