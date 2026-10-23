"""Build throwaway safe address loggers from exact T072 revisions.

Only isolated queue worktrees are patched, never the runner or this branch's
engine. No unsafe/pointer dereference is used. Probe samples are supplemental.
"""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

root = Path.cwd()
out = Path(sys.argv[1]).resolve(); out.mkdir()
revisions = {'base': '6e899d24c167d1393c077bcd69a01b61571dbcd1',
             'candidate': '6c122407146614e5d480b642551d18e782c430f5'}
receipt = {}
for role, revision in revisions.items():
    source = out / ('source-' + role)
    subprocess.run(['git', 'worktree', 'add', '--detach', str(source), revision], check=True)
    try:
        path = source / 'crates/qjs-runtime/src/bytecode/typed_loop/execute.rs'
        text = path.read_text()
        anchor = '    let mut scratch = program.take_scratch();\n'
        assert text.count(anchor) == 1
        probe = '''    if std::env::var_os("T073_PROBE").is_some() {
        eprintln!(
            "T073_ADDRESS bank={:p} ops={:p} count={} program={:p} scratch={:p}",
            scratch.registers.as_ptr(), program.ops.as_ptr(), program.ops.len(),
            program, &scratch
        );
    }
'''
        path.write_text(text.replace(anchor, anchor + probe))
        (out / (role + '.patch')).write_text(subprocess.check_output(['git', '-C', str(source), 'diff'], text=True))
        command = [sys.executable, '-m', 'tools.benchmark.build_cache', 'build-rust',
                   '--source', str(source), '--target-dir', str(out / 'build')]
        built = Path(subprocess.check_output(command, text=True).strip())
        destination = out / (role[:4] + '000') / 'qjs'; destination.parent.mkdir()
        shutil.copy2(built, destination)
        receipt[role] = {'source': revision, 'binary': str(destination),
                         'sha256': hashlib.sha256(destination.read_bytes()).hexdigest(),
                         'patch_sha256': hashlib.sha256((out / (role + '.patch')).read_bytes()).hexdigest()}
    finally:
        subprocess.run(['git', 'worktree', 'remove', '--force', str(source)], check=True)
(out / 'identity.json').write_text(json.dumps(receipt, indent=2) + '\n')
