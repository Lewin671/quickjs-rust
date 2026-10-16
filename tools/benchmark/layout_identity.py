"""Host and recipe identity for host-bound layout noise policies."""
from __future__ import annotations

import hashlib
import json
import platform
import subprocess
from pathlib import Path

from .build_cache import RUST_RECIPE
from .build_cache_identity import effective_build_environment, cargo_configuration_identity

ROOT = Path(__file__).resolve().parents[2]
OFFSETS = (0, 16, 64, 240, 512, 1008)


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def command(args) -> str:
    return subprocess.check_output(list(map(str, args)), text=True).strip()


def host_identity() -> dict:
    if platform.system() != 'Darwin' or platform.machine() != 'x86_64':
        raise ValueError('layout policy requires x86_64 macOS; recalibrate on a different host')
    return {
        'system': platform.system(), 'machine': platform.machine(),
        'os': platform.mac_ver()[0],
        'hardware': command(['sysctl', '-n', 'hw.model']),
        'cpu': command(['sysctl', '-n', 'machdep.cpu.brand_string']),
        'memory': command(['sysctl', '-n', 'hw.memsize']),
        'rustc': command(['rustc', '-vV']), 'cargo': command(['cargo', '-V']),
        'linker': command(['cc', '--version']),
        'recipe': RUST_RECIPE,
        'build_environment': effective_build_environment(),
        'cargo_configuration': cargo_configuration_identity(),
        'manifests': {str(p): digest(ROOT / p) for p in (
            'benchmarks/manifest.json', 'benchmarks/generic-sentinels-manifest.json',
            'benchmarks/external-preview.json', 'benchmarks/held-out-preview.json',
            'rust-toolchain.toml')},
    }


def write_json(path: Path, value) -> None:
    with path.open('x', encoding='utf-8') as handle:
        handle.write(json.dumps(value, indent=2, sort_keys=True, allow_nan=False) + '\n')
