"""Build/audit six placement-only variants of any commit on x86_64 macOS.

Only an isolated git archive's CLI link inputs are changed. A never-called
assembly NOP atom is ordered before the complete natural symbol inventory;
every function must retain its normalized body and one uniform displacement.
No source checkout or runtime engine file is changed.
"""
from __future__ import annotations

import argparse
import platform
import os
import shutil
import sys
import subprocess
import tarfile
from pathlib import Path

from .build_cache import RUST_RECIPE
from .layout_identity import OFFSETS, command, digest, host_identity, write_json
from .layout_variant_audit import audit, inspect

HELPER = '''
    let manifest = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let asm = manifest.join("layout-noise.s");
    let obj = PathBuf::from(env::var("OUT_DIR").unwrap()).join("layout-noise.o");
    assert!(std::process::Command::new("cc").args(["-c", "-arch", "x86_64", "-mmacosx-version-min=10.12"]).arg(&asm).arg("-o").arg(&obj).status().unwrap().success());
    println!("cargo:rerun-if-changed={}", asm.display());
    println!("cargo:rustc-link-arg-bin=qjs={}", obj.display());
'''


def build(revision: str, output: Path) -> dict:
    host = host_identity()  # Explicit refusal before touching output on other hosts.
    revision = command(['git', 'rev-parse', '--verify', revision + '^{commit}'])
    output = output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    source = output / 'source'
    source.mkdir()
    archive = output / 'source.tar'
    with archive.open('wb') as handle:
        subprocess.run(['git', 'archive', revision], stdout=handle, check=True)
    with tarfile.open(archive) as handle:
        # git archive contains only trusted tracked inputs of the requested commit.
        handle.extractall(source, filter='data')
    archive.unlink()
    build_file = source / 'crates/qjs-cli/build.rs'
    order_file = source / 'crates/qjs-cli/hot-functions-x86_64.order'
    original_build, original_order = build_file.read_text(), order_file.read_text()
    if original_build.count('fn main() {') != 1:
        raise ValueError('unsupported CLI link producer: expected one fn main()')
    env = {**os.environ, **RUST_RECIPE['environment'],
           'CARGO_BUILD_TARGET': 'x86_64-apple-darwin',
           'CARGO_TARGET_DIR': str(output / 'build'), 'TZ': 'UTC', 'LC_ALL': 'C'}
    identity = {'schema_version': 1, 'artifact_type': 'quickjs-layout-variants',
                'revision': revision, 'host': host, 'host_node': platform.node(), 'offsets': list(OFFSETS),
                'producer_sha256': digest(Path(__file__)), 'variants': [], 'static_pass': False}
    for k, padding in enumerate(OFFSETS):
        build_file.write_text(original_build if k == 0 else
                              original_build.replace('fn main() {', 'fn main() {\n' + HELPER, 1))
        order_file.write_text(original_order if k == 0 else
                             '_qjs_layout_noise_pad\n' + '\n'.join(natural['addresses']) + '\n')
        if k:
            (source / 'crates/qjs-cli/layout-noise.s').write_text(
                '.subsections_via_symbols\n.section __TEXT,__text,regular,pure_instructions\n'
                '.p2align 4\n.globl _qjs_layout_noise_pad\n.private_extern _qjs_layout_noise_pad\n'
                '.no_dead_strip _qjs_layout_noise_pad\n_qjs_layout_noise_pad:\n.space '
                + str(padding) + ', 0x90\n')
        print('BUILD', revision, 'variant', k, 'padding', padding, flush=True)
        subprocess.run(['cargo', 'build', *RUST_RECIPE['cargo_args']], cwd=source, env=env, check=True)
        binary = output / 'binaries' / f'variant{k}' / 'qjs'
        binary.parent.mkdir(parents=True)
        shutil.copy2(output / 'build/x86_64-apple-darwin/release/qjs', binary)
        meta = inspect(binary, output, f'variant{k}')
        if k == 0:
            natural = meta
            checked = {'natural': True}
        else:
            checked = audit(natural, meta, padding)
        identity['variants'].append({'index': k, 'pad_bytes': padding, 'path': str(binary),
                                    'sha256': meta['sha256'], 'audit': checked})
    identity['static_pass'] = True
    write_json(output / 'identity.json', identity)
    return identity


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--revision', required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    try:
        build(args.revision, args.output)
    except (ValueError, OSError, subprocess.CalledProcessError) as error:
        print(f"error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
