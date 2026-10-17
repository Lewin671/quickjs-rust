"""Qualify an additive development policy without changing frozen v2 evidence.

New case/lane envelopes come only from five-pair A2. Longer screen/formal
A/A qualifies them; it never widens them. Held-out acceptance is inherited.
"""
from __future__ import annotations

import argparse
import copy
import json
from pathlib import Path

from .development import MANIFEST_PATH, opportunity_ids
from .layout_identity import digest, write_json
from .layout_policy import analyze, load_policy, rows, selection_lanes
from .layout_power import precision_record


def qualify(parent, seed, pilot, formal, extensions=None, formal_extensions=None):
    ids = opportunity_ids()
    data = rows(pilot, ids, 30)
    if (set(seed['cases']) != set(parent['cases']) | ids or set(formal) != set(seed['cases'])
            or any(seed['cases'][n] != v for n, v in parent['cases'].items())
            or any(seed['lanes'][n] != v for n, v in parent['lanes'].items() if n != 'portfolio')):
        raise ValueError('development extension changes parent case/lane contracts')
    if pilot['host'] != parent['host']:
        raise ValueError('development A/A parent host mismatch')
    extensions, formal_extensions = extensions or {}, formal_extensions or {}
    if not set(extensions) <= ids or not set(formal_extensions) <= set(formal):
        raise ValueError('unknown development qualification continuation')
    policy = copy.deepcopy(parent)
    policy.update(cases=seed['cases'], lanes=seed['lanes'],
                  development_manifest_sha256=seed['development_manifest_sha256'])
    failures, stats = [], {'development': {}, 'formal': {}}
    precision = copy.deepcopy(parent['precision'])
    precision['formal'] = {}

    def assess(name, original, combined, group, n):
        result = analyze({name: combined})
        initial = analyze({name: original})
        record = precision_record(initial, result, n, base_n=len(original))
        precision.setdefault(group, {})[name] = record or {'pairs': n, 'status': 'incomplete'}
        if record is None or record['status'] == 'needs cap continuation':
            failures.append(name + ': automatic precision continuation incomplete')
        return result

    for name, original in data.items():
        ratios = extensions.get(name, original)
        n = len(ratios['wall_ns'])
        rows({'pairs': n, 'cases': [{'id': name, 'ratios': ratios}]}, [name], n)
        stats['development'][name] = assess(name, original['wall_ns'], ratios['wall_ns'], 'cases', n)
        if min(ratios['instructions']) > max(1.01, seed['cases'][name]['instruction_band']):
            failures.append(name + ': A/A instruction control event')
    lane = analyze({n: r['wall_ns'] for n, r in data.items()})
    stats['development']['lane'] = lane
    ceiling = seed['lanes']['development']
    if (not lane['precise'] or lane['confidence_interval']['upper'] > ceiling
            or lane['confidence_interval']['lower'] < 1 / ceiling):
        failures.append('development: screen A/A interval outside frozen lane envelope')
    for name, original in formal.items():
        combined = formal_extensions.get(name, original)
        stats['formal'][name] = assess(name, original, combined, 'formal', len(combined))
    for name in selection_lanes(policy):
        values = {n: r for n, r in formal.items() if name == 'portfolio' or n.startswith(name + '/')}
        result = analyze(values)
        stats['formal'][name] = result
        ci, ceiling = result['confidence_interval'], seed['lanes'][name]
        if not result['precise'] or ci['upper'] > ceiling or ci['lower'] < 1 / ceiling:
            failures.append(name + ': formal A/A interval outside frozen lane envelope')
    policy.update(qualified=not failures, failures=failures, precision=precision,
                  development_qualification=stats)
    return policy


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--parent-policy', type=Path, required=True)
    parser.add_argument('--parent-sha256', required=True)
    parser.add_argument('--seed', type=Path, required=True)
    parser.add_argument('--a2', type=Path, required=True)
    parser.add_argument('--original-a2', type=Path, required=True)
    parser.add_argument('--portfolio', type=Path, required=True, help='development-only 30-pair A/A')
    parser.add_argument('--formal-bundle', type=Path, required=True)
    parser.add_argument('--formal-revision', required=True)
    parser.add_argument('--host-attestation', type=Path, required=True)
    parser.add_argument('--formal-load-log', type=Path, required=True)
    parser.add_argument('--continuation', type=Path, action='append', default=[])
    parser.add_argument('--formal-continuation', type=Path, action='append', default=[])
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    parent = load_policy(args.parent_policy, args.parent_sha256)
    seed = json.loads(args.seed.read_text())
    from .layout_development_a2 import replay, bands
    a2, raw = replay(args.a2, seed['calibration_binary_sha256'])
    original_seed = json.loads((Path(__file__).parent / 'tests/fixtures/layout-policy/a2-bands.json').read_text())
    if digest(args.original_a2) != original_seed['source_sha256']:
        raise ValueError('original A2 estimator source hash mismatch')
    rebuilt = bands(parent, json.loads(args.original_a2.read_text()), raw)
    if any(seed[k] != v for k, v in rebuilt.items()):
        raise ValueError('development seed differs from frozen A2 estimators')
    attestation = json.loads(args.host_attestation.read_text())
    if a2['host'] != parent['host'] or a2['host_node'] != parent['host_node'] or attestation['node'] != parent['host_node'] or attestation['host'] != parent['host']:
        raise ValueError('development qualification host mismatch')
    from .layout_qualification import replay_calibration, verify_formal_identity
    pilot = replay_calibration(args.portfolio, seed['calibration_binary_sha256'])
    from .pair_disturbance import verify_formal_load
    verify_formal_load(args.formal_load_log.read_text(), attestation['started_at'])
    from .bundle import verify, development_report_path
    from .performance_evidence import replay_reports, validate_bundle, validate_development
    folder = args.formal_bundle
    verify(folder / 'summary.json', folder / 'report.json', folder / 'external-report.json', folder / 'sentinel-report.json')
    dev = development_report_path(folder / 'summary.json')
    if dev is None:
        raise ValueError('development qualification requires a sealed development lane')
    replay_reports(folder / 'report.json', folder / 'external-report.json', folder / 'sentinel-report.json', dev)
    summary = json.loads((folder / 'summary.json').read_text())
    verify_formal_identity(summary, args.formal_revision)
    broad, external, sentinel = [json.loads((folder / n).read_text()) for n in ('report.json', 'external-report.json', 'sentinel-report.json')]
    problems = validate_bundle(summary, broad, external, sentinel)
    problems += validate_development(json.loads(dev.read_text()), summary['engines'], broad['run']['host'])
    if problems:
        raise ValueError('formal bundle unhealthy: ' + '; '.join(problems))
    from .layout_formal_raw import extract
    formal = extract(folder, parent['host_node'], development=True)
    from .layout_continuation import replay as replay_screen
    from .layout_formal_continuation import replay as replay_formal
    extended, formal_extended = {}, {}
    for paths, target, reader in ((args.continuation, extended, replay_screen),
                                   (args.formal_continuation, formal_extended, replay_formal)):
        for path in paths:
            name = json.loads(path.read_text())['case_id']
            if name in target:
                raise ValueError('duplicate latest continuation')
            if target is extended:
                ratios, _, host = reader(path.resolve(), args.portfolio.resolve(), name)
            else:
                ratios, host = reader(path.resolve(), folder.resolve(), name, pilot=formal[name])
            if host != parent['host']:
                raise ValueError('continuation host mismatch')
            target[name] = ratios
    policy = qualify(parent, seed, pilot, formal, extended, formal_extended)
    paths = {'parent': args.parent_policy, 'a2': args.a2, 'original-a2': args.original_a2,
             'seed': args.seed, 'development-aa': args.portfolio,
             'formal-summary': folder / 'summary.json', 'formal-load': args.formal_load_log,
             'host-attestation': args.host_attestation}
    paths.update({'screen-continuation/' + str(i): p for i, p in enumerate(args.continuation)})
    paths.update({'formal-continuation/' + str(i): p for i, p in enumerate(args.formal_continuation)})
    policy['development_evidence'] = {n: {'path': str(p), 'sha256': digest(p)} for n, p in paths.items()}
    write_json(args.output, policy)
    print(json.dumps({'qualified': policy['qualified'], 'failures': policy['failures'], 'lanes': policy['lanes']}))
    return 0 if policy['qualified'] else 3


if __name__ == '__main__':
    raise SystemExit(main())
