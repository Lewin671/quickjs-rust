"""Lossless screen samples and costs, with calibration kept out of measured pairs.

The queue-backed recorder retains load-invalid attempts and appends rounds.
Valid nonpositive/short increments fail; measured values never select pairs.
"""
from __future__ import annotations

from pathlib import Path

from . import screen


from .pair_recording import measure


def development_cases(cache: Path, work: Path):
    """Selectable npm workloads use the same whole-process checksum contract."""
    from .development import load_development_manifest, MANIFEST_PATH, fetch_sources
    from .external_preview import _bundle_source, _EXPECTED_STDOUT_BY_ROLE
    manifest = load_development_manifest(MANIFEST_PATH)
    fetch_sources(manifest, cache)
    result = []
    for suite in manifest.suites:
        for case in suite.cases:
            path = work / f'development--{suite.id}--{case.id}.js'
            path.write_text(_bundle_source(suite, case, cache.resolve()), encoding='utf-8')
            result.append(screen.CaseSpec(f'development/{suite.id}/{case.id}', 'whole', path,
                expected_stdout=f'CHECKSUM {case.expected_checksum}\n' + _EXPECTED_STDOUT_BY_ROLE['candidate'],
                timeout_seconds=manifest.measurement.timeout_for(suite.id, case.id)))
    return result


def held_out_cases(cache: Path, work: Path):
    from .held_out import load_held_out_manifest, MANIFEST_PATH, fetch_sources
    from .external_preview import _bundle_source, _EXPECTED_STDOUT_BY_ROLE
    manifest = load_held_out_manifest(MANIFEST_PATH)
    fetch_sources(manifest, cache)
    result = []
    for suite in manifest.suites:
        for case in suite.cases:
            path = work / f'held-out--{suite.id}--{case.id}.js'
            path.write_text(_bundle_source(suite, case, cache.resolve()), encoding='utf-8')
            result.append(screen.CaseSpec(f'held-out/{suite.id}/{case.id}', 'whole', path,
                expected_stdout=f'CHECKSUM {case.expected_checksum}\n' + _EXPECTED_STDOUT_BY_ROLE['candidate']))
    return result
