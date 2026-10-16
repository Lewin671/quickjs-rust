"""Lossless screen samples and costs, with calibration kept out of measured pairs.

The recorder wraps the existing screen sampler outside timed subprocesses.
It refuses incomplete/nonpositive/short increments instead of filtering pairs.
"""
from __future__ import annotations

import dataclasses
from pathlib import Path

from . import screen
from .layout_identity import write_json


def measure(case, candidate, base, pairs, tool, timeout, output: Path):
    samples = []
    original = screen.sample

    def record(*args, **kwargs):
        sample = original(*args, **kwargs)
        samples.append({'argv': list(args[1]), 'sample': dataclasses.asdict(sample)})
        return sample

    screen.sample = record
    try:
        result = screen.screen_case(case, candidate, base, pairs, tool, timeout)
    finally:
        screen.sample = original
    internal = case.kind == 'internal'
    count = pairs * (4 if internal else 2)
    measured = samples[-count:]
    if len(measured) != count:
        raise screen.ScreenError(f'{case.id}: incomplete samples')
    costs = {role: {m: [] for m in ('wall_ns', 'instructions')} for role in ('base', 'candidate')}
    cursor = 0
    for pair in range(pairs):
        for role in (('base', 'candidate') if pair % 2 == 0 else ('candidate', 'base')):
            rows = measured[cursor:cursor + (2 if internal else 1)]
            cursor += len(rows)
            for metric in ('wall_ns', 'instructions'):
                value = rows[-1]['sample'][metric]
                if internal:
                    value -= rows[0]['sample'][metric]
                if value <= 0 or (internal and metric == 'wall_ns' and value < screen.MIN_INCREMENT_NS):
                    raise screen.ScreenError(f'{case.id}: invalid {role} {metric} increment {value}')
                costs[role][metric].append(value)
    for metric in ('wall_ns', 'instructions'):
        ratios = [c / b for c, b in zip(costs['candidate'][metric], costs['base'][metric])]
        if ratios != result.ratios[metric] or len(ratios) != pairs:
            raise screen.ScreenError(f'{case.id}: ratio/sample conservation failed')
    packet = {'id': case.id, 'kind': case.kind, 'iterations': result.iterations,
              'calibration': samples[:-count], 'samples': measured, 'costs': costs,
              'ratios': result.ratios}
    write_json(output, packet)
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
