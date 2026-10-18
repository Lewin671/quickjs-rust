"""Qualify schedule-specific v5 bands without inheriting serial gates/precision."""
from __future__ import annotations

import copy
import json
from pathlib import Path

from .case_schedule import require_same
from .layout_identity import OFFSETS, digest
from .layout_policy import HELD_OUT_RULE, analyze, rows
from .layout_power import precision_record


def qualify(seed, pilots, formal, extensions=None, formal_extensions=None):
    if set(pilots) != {'portfolio', 'development', 'held-out'}:
        raise ValueError('qualification needs three complete pilots')
    require_same(seed, *pilots.values())
    if any(p['host'] != seed['host'] for p in pilots.values()):
        raise ValueError('qualification pilot host mismatch')
    selection = {n for n in seed['cases'] if not n.startswith('development/')}
    dev = set(seed['cases']) - selection
    data = {**rows(pilots['portfolio'], selection, 30), **rows(pilots['development'], dev, 30)}
    held = rows(pilots['held-out'], seed['held_out'], 30)
    if set(formal) != set(data) or any(len(v) != 30 for v in formal.values()):
        raise ValueError('qualification formal inventory/block mismatch')
    extensions, formal_extensions = extensions or {}, formal_extensions or {}
    if not set(extensions) <= set(data) | set(held) or not set(formal_extensions) <= set(formal):
        raise ValueError('unknown qualification continuation')
    stats, precision, failures = {}, {'cases': {}, 'held_out': {}, 'formal': {}}, []

    def assess(name, original, combined, group):
        result = analyze({name: combined})
        record = precision_record(analyze({name: original}), result, len(combined), base_n=len(original))
        precision[group][name] = record or {'pairs': len(combined), 'status': 'incomplete'}
        if not record or record['status'] == 'needs cap continuation':
            failures.append(name + ': automatic precision allocation incomplete')
        return result

    def lane(label, values, ceiling):
        result = stats[label] = analyze(values)
        ci = result['confidence_interval']
        if not result['precise'] or ci['lower'] < 1 / ceiling or ci['upper'] > ceiling:
            failures.append(label + ': A/A interval outside frozen envelope')

    for group, original in (('cases', data), ('held_out', held)):
        stats[group] = {}
        for name, pairs in original.items():
            combined = extensions.get(name, pairs)
            n = len(combined['wall_ns'])
            rows({'pairs': n, 'cases': [{'id': name, 'ratios': combined}]}, [name], n)
            stats[group][name] = assess(name, pairs['wall_ns'], combined['wall_ns'], group)
            if min(combined['instructions']) > max(1.01, seed[group][name]['instruction_band']):
                failures.append(name + ': A/A instruction control event')
    # Development was a separately collected pilot: do not fabricate a joint
    # 86-case cohort; the sealed formal run qualifies its portfolio ceiling.
    for name in ('broad', 'sentinel', 'external', 'development'):
        lane('screen/' + name, {n: r['wall_ns'] for n, r in data.items() if n.startswith(name + '/')}, seed['lanes'][name])
    lane('screen/portfolio-76', {n: r['wall_ns'] for n, r in data.items() if not n.startswith('development/')}, seed['lanes']['portfolio'])
    lane('screen/held-out', {n: r['wall_ns'] for n, r in held.items()}, seed['lanes']['held-out'])
    stats['formal'] = {n: assess(n, values, formal_extensions.get(n, values), 'formal') for n, values in formal.items()}
    for name in ('broad', 'sentinel', 'external', 'development', 'portfolio'):
        lane('formal/' + name, {n: v for n, v in formal.items() if name == 'portfolio' or n.startswith(name + '/')}, seed['lanes'][name])
    policy = {k: copy.deepcopy(seed[k]) for k in ('host', 'host_node', 'measurement_schedule', 'formal_lane_schedule',
              'cases', 'held_out', 'lanes', 'development_manifest_sha256', 'held_out_manifest_sha256')}
    return {**policy, 'schema_version': 1, 'artifact_type': 'quickjs-layout-policy', 'qualified': not failures,
            'failures': failures, 'offsets': list(OFFSETS), 'held_out_rule': HELD_OUT_RULE,
            'precision': precision, 'qualification': stats}


def replay_formal(folder, seed, revision, attestation, load):
    from .bundle import verify, development_report_path
    from .performance_evidence import replay_reports, validate_bundle, validate_development
    from .layout_qualification import verify_formal_identity
    from .pair_disturbance import verify_formal_load
    from .layout_formal_raw import extract
    verify(folder/'summary.json', folder/'report.json', folder/'external-report.json', folder/'sentinel-report.json')
    dev = development_report_path(folder/'summary.json')
    if dev is None:
        raise ValueError('formal qualification missing development lane')
    replay_reports(folder/'report.json', folder/'external-report.json', folder/'sentinel-report.json', dev)
    summary = json.loads((folder/'summary.json').read_text())
    if summary.get('lane_schedule') != seed['formal_lane_schedule']:
        raise ValueError('formal qualification schedule mismatch')
    verify_formal_identity(summary, revision)
    reports = [json.loads((folder/n).read_text()) for n in ('report.json','external-report.json','sentinel-report.json')]
    errors = validate_bundle(summary, *reports)
    errors += validate_development(json.loads(dev.read_text()), summary['engines'], reports[0]['run']['host'])
    if errors:
        raise ValueError('formal qualification unhealthy: ' + '; '.join(errors))
    host = json.loads(attestation.read_text())
    if host['host'] != seed['host'] or host['node'] != seed['host_node']:
        raise ValueError('formal qualification host/recipe mismatch')
    verify_formal_load(load.read_text(), host['started_at'])
    return extract(folder, seed['host_node'], development=True)


def compare_bands(parent, policy):
    """Full auditable differences, with no selected-case summary."""
    return {group: {name: {'v4': old, 'v5': policy[group][name]} for name, old in parent[group].items()}
            for group in ('cases', 'held_out', 'lanes')}
