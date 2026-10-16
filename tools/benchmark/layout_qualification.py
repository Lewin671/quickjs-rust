"""Replay frozen 30-block A/A and publish a host-bound policy if qualified."""
from __future__ import annotations

import argparse
import hashlib
import json
import os
import statistics
from pathlib import Path

from .layout_identity import OFFSETS, digest, write_json
from .layout_policy import analyze, round_up, rows


def qualify(seed, portfolio, held_out, formal=None):
    data = rows(portfolio, seed['cases'], 30)
    from .held_out import opportunity_ids
    held = rows(held_out, opportunity_ids(), 30)
    if portfolio['host'] != held_out['host']:
        raise ValueError('A/A hosts disagree')
    results = {'cases': {}, 'lanes': {}, 'held_out': {}}
    failures = []
    for name, ratios in data.items():
        stats = analyze({name: ratios['wall_ns']})
        results['cases'][name] = stats
        if not stats['precise']:
            failures.append(name + ': imprecise 30-block A/A')
        if min(ratios['instructions']) > max(1.01, seed['cases'][name]['instruction_band']):
            failures.append(name + ': A/A instruction control event')
    for lane in ('broad', 'sentinel', 'external', 'portfolio'):
        stats = analyze({n: r['wall_ns'] for n, r in data.items()
                         if lane == 'portfolio' or n.startswith(lane + '/')})
        results['lanes'][lane] = stats
        ci = stats['confidence_interval']
        ceiling = seed['lanes'][lane]
        if not stats['precise'] or ci['upper'] > ceiling or ci['lower'] < 1 / ceiling:
            failures.append(lane + ': formal A/A interval outside frozen lane envelope')
    if formal is not None:
        if set(formal) != set(seed['cases']):
            raise ValueError('canonical formal inventory mismatch')
        results['formal'] = {}
        for name, ratios in formal.items():
            stats = analyze({name: ratios})
            results['formal'][name] = stats
            if not stats['precise']:
                failures.append(name + ': canonical formal A/A imprecise')
        for lane in ('broad', 'sentinel', 'external', 'portfolio'):
            stats = analyze({n: ratios for n, ratios in formal.items()
                             if lane == 'portfolio' or n.startswith(lane + '/')})
            results['formal'][lane] = stats
            ci = stats['confidence_interval']
            ceiling = seed['lanes'][lane]
            if not stats['precise'] or ci['upper'] > ceiling or ci['lower'] < 1 / ceiling:
                failures.append(lane + ': canonical formal A/A interval outside frozen envelope')
    held_bands = {}
    for name, ratios in held.items():
        stats = analyze({name: ratios['wall_ns']})
        results['held_out'][name] = stats
        if not stats['precise']:
            failures.append(name + ': imprecise held-out A/A')
        ci = stats['confidence_interval']
        held_bands[name] = {'wall_band': round_up(max(1, ci['upper'], 1 / ci['lower']))}
    stats = analyze({n: r['wall_ns'] for n, r in held.items()})
    results['lanes']['held-out'] = stats
    if not stats['precise']:
        failures.append('held-out: imprecise lane A/A')
    ci = stats['confidence_interval']
    lanes = {**seed['lanes'], 'held-out': round_up(max(1, ci['upper'], 1 / ci['lower']))}
    return {'schema_version': 1, 'artifact_type': 'quickjs-layout-policy',
            'qualified': not failures, 'failures': failures, 'host': portfolio['host'],
            'offsets': list(OFFSETS), 'cases': seed['cases'], 'held_out': held_bands,
            'lanes': lanes, 'qualification': results}


def verify_formal_identity(summary, expected_revision):
    """A/A roles share one frozen revision/image; older band-source code may differ."""
    engines = summary['engines']
    if any(engines[r]['source_revision'] != expected_revision for r in ('base', 'candidate')):
        raise ValueError('formal A/A source differs from its frozen revision')
    if engines['base']['binary_sha256'] != engines['candidate']['binary_sha256']:
        raise ValueError('formal A/A roles must be byte-identical')


def replay_calibration(path, binary_sha256):
    """Read the collector's summaries only after every raw case conserves costs."""
    from .screen_replay import replay_case
    folder = path.parent
    identity = json.loads((folder / 'identity.json').read_text())
    if identity['sha256'] != binary_sha256:
        raise ValueError('calibration executable differs from frozen A2 natural binary')
    binaries = identity['binaries']
    if len(set(binaries)) != 2 or len({len(os.fsencode(p)) for p in binaries}) != 1:
        raise ValueError('calibration paths are not distinct/equal-length')
    for role in ('base0', 'cand0'):
        if digest(folder / role / 'qjs') != identity['sha256']:
            raise ValueError('calibration binary hash changed')
    packet = json.loads(path.read_text())
    expected_files = set()
    for case in packet['cases']:
        file = folder / 'cases' / (case['id'].replace('/', '--') + '.json')
        expected_files.add(file.name)
        raw = json.loads(file.read_text())
        if raw['id'] != case['id'] or raw['ratios'] != case['ratios']:
            raise ValueError('calibration raw/report mismatch')
        replay_case(raw, 30, dict(zip(('base', 'candidate'), binaries)))
    if {p.name for p in (folder / 'cases').glob('*.json')} != expected_files:
        raise ValueError('calibration raw inventory mismatch')
    if packet['host'] != identity['host'] or packet['pairs'] != 30:
        raise ValueError('calibration host/protocol mismatch')
    return packet


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--a2-bands', type=Path, required=True)
    parser.add_argument('--portfolio', type=Path, required=True)
    parser.add_argument('--held-out', type=Path, required=True)
    parser.add_argument('--screen-context', required=True, help='committed calibration tooling revision')
    parser.add_argument('--formal-bundle', type=Path, required=True)
    parser.add_argument('--host-attestation', type=Path, required=True,
                        help='queue-collected host node and full host identity')
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    paths = {'a2': args.a2_bands, 'portfolio': args.portfolio, 'held-out': args.held_out}
    seed = json.loads(args.a2_bands.read_text())
    packets = {'a2': seed,
               'portfolio': replay_calibration(args.portfolio, seed['calibration_binary_sha256']),
               'held-out': replay_calibration(args.held_out, seed['calibration_binary_sha256'])}
    attestation = json.loads(args.host_attestation.read_text())
    recorded = packets['portfolio']['host']
    current = attestation['host']
    if seed.get('source_host_node') != attestation['node'] or seed.get('source_host') != recorded:
        raise ValueError('A2 bands belong to a different host/recipe: recalibrate the layout envelope')
    if {k: v for k, v in recorded.items() if k != 'manifests'} != {
            k: v for k, v in current.items() if k != 'manifests'}:
        raise ValueError('host attestation differs from calibration host')
    # The canonical snapshot protocol was independently requalified. Screen
    # inputs may differ only in that protocol metadata, never in case/recipe.
    from .layout_identity import command, ROOT
    for name, sha in recorded['manifests'].items():
        if sha == current['manifests'][name]:
            continue
        if name not in ('benchmarks/manifest.json', 'benchmarks/generic-sentinels-manifest.json'):
            raise ValueError('calibration recipe/corpus changed')
        old_text = command(['git', 'show', args.screen_context + ':' + name]) + '\n'
        if hashlib.sha256(old_text.encode()).hexdigest() != sha:
            raise ValueError('recorded calibration manifest/context mismatch')
        old, new = json.loads(old_text), json.loads((ROOT / name).read_text())
        old.pop('protocol'); new.pop('protocol')
        if old != new:
            raise ValueError('calibration case or recipe changed')
    from .performance_evidence import replay_reports
    from .bundle import verify
    folder = args.formal_bundle
    verify(folder / 'summary.json', folder / 'report.json', folder / 'external-report.json', folder / 'sentinel-report.json')
    summary = json.loads((folder / 'summary.json').read_text())
    verify_formal_identity(summary, seed['formal_source_revision'])
    replay_reports(folder / 'report.json', folder / 'external-report.json', folder / 'sentinel-report.json')
    from .layout_formal_raw import extract
    formal = extract(folder, attestation['node'])
    policy = qualify(packets['a2'], packets['portfolio'], packets['held-out'], formal)
    paths['formal-summary'] = folder / 'summary.json' 
    policy['screen_host'] = recorded
    policy['host'] = current
    policy['host_node'] = attestation['node']
    paths['host-attestation'] = args.host_attestation
    policy['evidence'] = {k: {'path': str(p), 'sha256': digest(p)} for k, p in paths.items()}
    write_json(args.output, policy)
    print(json.dumps({'qualified': policy['qualified'], 'failures': policy['failures'], 'lanes': policy['lanes']}))
    return 0 if policy['qualified'] else 3


if __name__ == '__main__':
    raise SystemExit(main())
