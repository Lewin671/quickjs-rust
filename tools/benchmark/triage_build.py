"""Canonical committed builds for triage; shared Cargo and identity-checked caches.

Only disposable clean worktrees are built. Cache keys include source, host,
recipe and toolchain; immutable role copies keep equal filesystem path lengths.
"""
from __future__ import annotations

import shutil
import subprocess
import tempfile
from pathlib import Path

from . import build_cache
from .layout_identity import ROOT, digest, write_json


def build(revision: str, output: Path) -> dict:
    with tempfile.TemporaryDirectory(prefix='qjs-triage-') as temporary:
        source = Path(temporary) / 'source'
        subprocess.run(['git', 'worktree', 'add', '--quiet', '--detach', str(source), revision],
                       cwd=ROOT, check=True)
        try:
            spec = build_cache.rust_spec(source)
            spec_path = output.parent / (output.parent.name + '-cache.json')
            write_json(spec_path, spec)
            cache = ROOT / 'target/quick-triage-cache' / spec['key_sha256']
            hit, _ = build_cache.materialize(cache, spec_path, output)
            if not hit:
                # Reuse dependencies from the established revision-build cache.
                target = ROOT / 'target/perf-loop/compiler-fat16-abort-v1/build-rev'
                built = subprocess.check_output([
                    'python3', '-m', 'tools.benchmark.build_cache', 'build-rust',
                    '--source', str(source), '--target-dir', str(target)], cwd=ROOT, text=True).strip()
                build_cache.store(cache, spec_path, Path(built))
                if not build_cache.materialize(cache, spec_path, output)[0]:
                    raise ValueError('new triage build cache failed validation')
            return {'revision': revision, 'path': str(output), 'sha256': digest(output),
                    'cache_hit': hit, 'cache_key': spec['key_sha256'],
                    'cache_spec_sha256': digest(spec_path)}
        finally:
            subprocess.run(['git', 'worktree', 'remove', '--force', str(source)], cwd=ROOT, check=True)
            # TemporaryDirectory also cleans any failed build's source tree.
            if source.exists():
                shutil.rmtree(source)
