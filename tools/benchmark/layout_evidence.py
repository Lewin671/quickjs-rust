"""Replay layout-panel raw costs and bind them to formal engine identities.

Summaries alone are insufficient: each measured pair is rebuilt from retained
process samples, whose command paths and checksums must match its two roles.
"""
from __future__ import annotations

import json
import os
from pathlib import Path

from .layout_identity import digest, OFFSETS
from .layout_policy import rows
from .screen_replay import replay_case


def replay(path: Path, policy, engines=None, *, held=False):
    evidence = json.loads(path.read_text())
    if (evidence.get('schema_version') != 1 or evidence.get('artifact_type') != 'quickjs-layout-panel'
            or evidence.get('host') != policy['host'] or evidence.get('held_out') != held
            or evidence.get('host_node') != policy.get('host_node')):
        raise ValueError('layout evidence schema/host/lane mismatch')
    if len(evidence['panels']) not in (1, 6):
        raise ValueError('incomplete layout panel')
    if not held and 'development_manifest_sha256' in policy:
        if evidence.get('development_manifest_sha256') != policy['development_manifest_sha256']:
            raise ValueError('layout evidence development manifest mismatch')
    from .case_schedule import require_same, verify
    require_same(policy, evidence, *evidence['panels'])
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
    if len(set(paths)) != 12 or len({len(os.fsencode(p)) for p in paths}) != 1:
        raise ValueError('layout paths must be distinct and equal length')
    expected_files = set()
    raw_panels = {}
    expected_ids = policy['held_out' if held else 'cases']
    for k, packet in enumerate(evidence['panels']):
        rows(packet, expected_ids, pairs, allow_health=True)
        raw_panels[k] = []
        for case in packet['cases']:
            relative = f"variant{k}/{case['id'].replace('/', '--')}.json"
            expected_files.add(relative)
            file = path.parent / relative
            if digest(file) != evidence['case_files'].get(relative):
                raise ValueError('layout raw case hash mismatch')
            raw = json.loads(file.read_text())
            if (raw['id'] != case['id'] or raw.get('ratios', {}) != case['ratios']
                    or raw.get('health') != case.get('health')):
                raise ValueError('layout raw/report case mismatch')
            raw_panels[k].append(raw)
            if 'health' in raw:
                replay_case(raw, pairs, {role: evidence[role]['variants'][k]['path']
                                         for role in ('base', 'candidate')}, allow_health=True)
            else:
                replay_case(raw, pairs, {role: evidence[role]['variants'][k]['path']
                                         for role in ('base', 'candidate')})
    case_ids = [c['id'] for c in evidence['panels'][0]['cases']]
    verify(evidence, case_ids)
    all_raw = [r for panel in raw_panels.values() for r in panel]
    if expected_files != set(evidence['case_files']):
        raise ValueError('layout raw file inventory mismatch')
    if any('attempts' in r for panel in raw_panels.values() for r in panel):
        from .pair_disturbance import verify_run
        covered = []
        for run in evidence['disturbance_runs']:
            covered.extend(run['variants'])
            raws = [r for k in run['variants'] for r in raw_panels[k]]
            verify(evidence, case_ids, raws)
            if verify_run(raws, run['load']) != run['counts']:
                raise ValueError('panel disturbance count mismatch')
        if sorted(covered) != list(raw_panels):
            raise ValueError('panel disturbance run inventory mismatch')
    else:
        verify(evidence, case_ids, all_raw)
    return evidence
