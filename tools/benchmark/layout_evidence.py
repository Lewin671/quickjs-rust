"""Replay layout-panel raw costs and bind them to formal engine identities.

Summaries alone are insufficient: each measured pair is rebuilt from retained
process samples, whose command paths and checksums must match its two roles.
"""
from __future__ import annotations

import json
from pathlib import Path

from .layout_identity import digest, OFFSETS
from .layout_policy import rows
from .screen import MIN_INCREMENT_NS


def replay(path: Path, policy, engines=None, *, held=False):
    evidence = json.loads(path.read_text())
    if (evidence.get('schema_version') != 1 or evidence.get('artifact_type') != 'quickjs-layout-panel'
            or evidence.get('host') != policy['host'] or evidence.get('held_out') != held
            or evidence.get('host_node') != policy.get('host_node')):
        raise ValueError('layout evidence schema/host/lane mismatch')
    if len(evidence['panels']) not in ((1,) if held else (1, 6)):
        raise ValueError('incomplete layout panel')
    pairs = evidence['pairs']
    if type(pairs) is not int or pairs < (30 if held else 5):
        raise ValueError('invalid layout evidence pair count')
    paths = []
    for role in ('base', 'candidate'):
        matrix = evidence[role]
        if (matrix['host'] != policy['host'] or matrix.get('host_node') != policy.get('host_node')
                or not matrix['static_pass'] or matrix['offsets'] != list(OFFSETS)):
            raise ValueError('layout matrix identity/audit mismatch')
        if len(matrix['variants']) != 6:
            raise ValueError('layout matrix variant inventory mismatch')
        for k, variant in enumerate(matrix['variants']):
            if variant['index'] != k or variant['pad_bytes'] != OFFSETS[k]:
                raise ValueError('layout matrix index mismatch')
            audit = variant['audit']
            if k and (audit['body_drift'] != 0 or audit['address_drift'] != 0):
                raise ValueError('layout matrix failed static audit')
            paths.append(variant['path'])
        if engines is not None:
            engine = engines[role]
            if (matrix['revision'] != engine['source_revision']
                    or matrix['variants'][0]['sha256'] != engine['binary_sha256']):
                raise ValueError('layout natural engine differs from formal bundle')
    if len(set(paths)) != 12 or len({len(p) for p in paths}) != 1:
        raise ValueError('layout paths must be distinct and equal length')
    expected_files = set()
    expected_ids = policy['held_out' if held else 'cases']
    for k, packet in enumerate(evidence['panels']):
        rows(packet, expected_ids, pairs)
        for case in packet['cases']:
            relative = f"variant{k}/{case['id'].replace('/', '--')}.json"
            expected_files.add(relative)
            file = path.parent / relative
            if digest(file) != evidence['case_files'].get(relative):
                raise ValueError('layout raw case hash mismatch')
            raw = json.loads(file.read_text())
            if raw['id'] != case['id'] or raw['ratios'] != case['ratios']:
                raise ValueError('layout raw/report case mismatch')
            internal = raw['kind'] == 'internal'
            samples = raw['samples']
            if len(samples) != pairs * (4 if internal else 2):
                raise ValueError('layout raw sample count mismatch')
            costs = {r: {m: [] for m in ('wall_ns', 'instructions')} for r in ('candidate', 'base')}
            cursor = 0
            for pair in range(pairs):
                outputs = {}
                for role in (('base', 'candidate') if pair % 2 == 0 else ('candidate', 'base')):
                    taken = samples[cursor:cursor + (2 if internal else 1)]
                    cursor += len(taken)
                    binary = evidence[role]['variants'][k]['path']
                    if any(s['argv'][0] != binary for s in taken):
                        raise ValueError('layout sample binary path mismatch')
                    outputs[role] = [s['sample']['stdout'] for s in taken]
                    for metric in ('wall_ns', 'instructions'):
                        value = taken[-1]['sample'][metric]
                        if internal:
                            value -= taken[0]['sample'][metric]
                        if value <= 0 or (internal and metric == 'wall_ns' and value < MIN_INCREMENT_NS):
                            raise ValueError('invalid layout increment')
                        costs[role][metric].append(value)
                if outputs['base'] != outputs['candidate']:
                    raise ValueError('layout stdout mismatch')
            if costs != raw['costs']:
                raise ValueError('layout costs differ from raw replay')
            for metric in ('wall_ns', 'instructions'):
                rebuilt = [c / b for c, b in zip(costs['candidate'][metric], costs['base'][metric])]
                if rebuilt != case['ratios'][metric]:
                    raise ValueError('layout ratios differ from raw replay')
    if expected_files != set(evidence['case_files']):
        raise ValueError('layout raw file inventory mismatch')
    return evidence
