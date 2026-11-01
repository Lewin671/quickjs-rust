"""Exact case-paired ratios from a sealed canonical formal bundle.

Internal samples use the canonical per-operation costs (not screen increments).
The same median-ratio and joint-bootstrap statistic is used for qualification
and final decisions; no marginal-CI averaging substitutes for it.
"""
from __future__ import annotations

import json
from pathlib import Path

from .layout_identity import same_node
from .performance_evidence import load_archived_manifest
from .raw_validation import validate_run


def extract(folder: Path, host_node: str, *, development=None):
    result = {}
    counts = set()
    for lane, prefix in (('broad', ''), ('sentinel', 'sentinel-')):
        manifest = load_archived_manifest(folder / (prefix + 'manifest.json'), lane)
        run = validate_run(folder / (prefix + 'raw.jsonl'), manifest)
        if (not run.comparison_input_complete or run.invalid_blocks or run.blocks < 30
                or not same_node(run.start['host']['node'], host_node)):
            raise ValueError('formal layout input incomplete or on a different host')
        counts.add(run.blocks)
        for case in manifest.cases:
            result[f'{lane}/{case.id}'] = [
                run.measurements[('candidate', case.id, block)] / run.measurements[('base', case.id, block)]
                for block in range(run.blocks)]
    if development is None:
        summary = json.loads((folder / 'summary.json').read_text()) if (folder / 'summary.json').exists() else {}
        development = summary.get('bundle', {}).get('schema_version') == 2
    samples = {}
    for lane in ('external', *(('development',) if development else ())):
        records = [json.loads(line) for line in (folder / (lane + '-raw.jsonl')).read_text().splitlines()]
        for row in records:
            if row.get('phase') != 'measurement' or row.get('role') not in ('base', 'candidate'):
                continue
            if row['status'] != 'ok' or not same_node(row['host']['node'], host_node):
                raise ValueError('formal whole-process layout input invalid or on a different host')
            name = f"{lane}/{row['suite_id']}/{row['case_id']}"
            key = (name, row['role'], row['block'])
            if key in samples:
                raise ValueError('duplicate formal whole-process sample')
            samples[key] = row['duration_ns']
    if len(counts) != 1:
        raise ValueError('formal lane block counts differ')
    blocks = counts.pop()
    for name in sorted({key[0] for key in samples}):
        result[name] = [samples[(name, 'candidate', block)] / samples[(name, 'base', block)]
                        for block in range(blocks)]
    return result
