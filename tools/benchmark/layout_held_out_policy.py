"""Policy v4: six-layout held-out bands, qualified without touching parent gates.

Held-out B_i/I_i and the lane ceiling come from five-pair placement A2 of the
band-source image plus the parent's existing 30-pair A/A, as the maximum of
the estimators the other lanes use. A fresh A/A qualifies them and never
widens them. Every selection band, ceiling and precision record is inherited.
"""
from __future__ import annotations

import argparse
import copy
import json
import statistics
from pathlib import Path

from .held_out import MANIFEST_PATH, opportunity_ids
from .layout_identity import ROOT, digest, write_json
from .layout_policy import HELD_OUT_RULE, analyze, geomean, load_policy, round_up, rows
from .layout_power import precision_record

ORIGINAL_SEED = Path(__file__).parent / 'tests/fixtures/layout-policy/a2-bands.json'


def reciprocal(value):
    return max(value, 1 / value)


def spread(values):
    return max(values) / min(values)


def aa_envelope(ratios):
    """The parent's held-out estimator: rounded reciprocal 95% A/A endpoints."""
    ci = analyze(ratios)['confidence_interval']
    return round_up(max(1, ci['upper'], 1 / ci['lower']))


def bands(parent, original, development_raw, raw, aa):
    ids = sorted(opportunity_ids())
    if set(raw) != {(k, n) for k in range(6) for n in ids}:
        raise ValueError('held-out A2 inventory mismatch')
    old = {c['id']: c['metrics'] for c in original['cases']}
    natural = [r for (k, _), r in sorted(development_raw.items()) if k == 0] + [raw[(0, n)] for n in ids]
    bias = {m: max([reciprocal(old[n][m]['aa_median']) for n in old] +
                   [reciprocal(statistics.median(r['ratios'][m])) for r in natural])
            for m in ('wall_ns', 'instructions')}
    pairs = rows(aa, ids, 30)
    if aa_envelope({n: pairs[n]['wall_ns'] for n in ids}) != parent['lanes']['held-out']:
        raise ValueError('parent held-out lane ceiling does not reproduce from its A/A')
    paired, result = {}, {}
    for n in ids:
        envelope = aa_envelope({n: pairs[n]['wall_ns']})
        if envelope != parent['held_out'][n]['wall_band']:
            raise ValueError(n + ': parent held-out band does not reproduce from its A/A')
        paired[n] = [statistics.median(raw[(k, n)]['ratios']['wall_ns']) for k in range(6)]
        normalized = [1.] + [statistics.median(raw[(k, n)]['costs']['candidate']['wall_ns']) /
                             statistics.median(raw[(k, n)]['costs']['base']['wall_ns']) for k in range(1, 6)]
        instructions = raw[(0, n)]['ratios']['instructions'] + pairs[n]['instructions']
        result[n] = {'wall_band': max(spread(normalized), spread([1.] + paired[n][1:]), bias['wall_ns'], envelope),
                     'instruction_band': round_up(max(bias['instructions'], max(instructions), 1 / min(instructions)))}
    placements = [1.] + [geomean([paired[n][k] for n in ids]) for k in range(1, 6)]
    lane = max(spread(placements), reciprocal(geomean([paired[n][0] for n in ids])), parent['lanes']['held-out'])
    return {'held_out': result, 'held_out_lane': lane, 'held_out_manifest_sha256': digest(MANIFEST_PATH)}


def inputs(parent, parent_sha, a2, original, development_a2, aa, image):
    """Replay every frozen estimator input and return the seed it determines."""
    from .layout_development_a2 import replay
    from .layout_qualification import replay_calibration
    dev = parent.get('development_evidence', {})
    fixture = json.loads(ORIGINAL_SEED.read_text())
    if (digest(original) != fixture['source_sha256'] or digest(original) != dev.get('original-a2', {}).get('sha256')
            or digest(development_a2) != dev.get('a2', {}).get('sha256')
            or digest(aa) != parent['evidence']['held-out']['sha256']):
        raise ValueError('held-out estimator input differs from the parent policy evidence')
    held, raw = replay(a2, image, 'held-out')
    development, development_raw = replay(development_a2, image)
    if held['host'] != parent['host'] or held['host_node'] != parent['host_node']:
        raise ValueError('held-out A2 host differs from parent policy')
    packet = replay_calibration(aa, fixture['calibration_binary_sha256'])
    from .case_schedule import require_same
    require_same(parent, held, development, packet)
    seed = bands(parent, json.loads(original.read_text()), development_raw, raw, packet)
    seed.update(artifact_type='quickjs-held-out-a2-bands', schema_version=1, parent_policy_sha256=parent_sha,
                calibration_binary_sha256=image, source_revision=held['source']['revision'],
                a2_sha256=digest(a2), original_a2_sha256=digest(original),
                development_a2_sha256=digest(development_a2), held_out_aa_sha256=digest(aa))
    return seed


def qualify(parent, seed, pilot, extensions=None):
    from .case_schedule import require_same
    require_same(parent, seed, pilot)
    ids = opportunity_ids()
    data = rows(pilot, ids, 30)
    if set(seed['held_out']) != ids or pilot['host'] != parent['host']:
        raise ValueError('held-out qualification inventory/host mismatch')
    extensions = extensions or {}
    if not set(extensions) <= ids:
        raise ValueError('unknown held-out continuation')
    policy = copy.deepcopy(parent)
    policy.update(held_out=seed['held_out'], held_out_rule=HELD_OUT_RULE,
                  held_out_manifest_sha256=seed['held_out_manifest_sha256'])
    policy['lanes']['held-out'] = seed['held_out_lane']
    failures, stats = [], {}
    precision = copy.deepcopy(parent['precision'])
    for name, original in data.items():
        ratios = extensions.get(name, original)
        n = len(ratios['wall_ns'])
        rows({'pairs': n, 'cases': [{'id': name, 'ratios': ratios}]}, [name], n)
        stats[name] = analyze({name: ratios['wall_ns']})
        record = precision_record(analyze({name: original['wall_ns']}), stats[name], n)
        precision['held_out'][name] = record or {'pairs': n, 'status': 'incomplete'}
        if record is None or record['status'] == 'needs cap continuation':
            failures.append(name + ': automatic precision continuation incomplete')
        if min(ratios['instructions']) > max(1.01, seed['held_out'][name]['instruction_band']):
            failures.append(name + ': A/A instruction control event')
    lane = stats['lane'] = analyze({n: r['wall_ns'] for n, r in data.items()})
    ceiling = seed['held_out_lane']
    if (not lane['precise'] or lane['confidence_interval']['upper'] > ceiling
            or lane['confidence_interval']['lower'] < 1 / ceiling):
        failures.append('held-out: A/A interval outside frozen lane envelope')
    policy.update(qualified=not failures, failures=failures, precision=precision,
                  qualification={**parent['qualification'], 'held_out_v4': stats})
    return policy


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='command', required=True)
    for name in ('a2', 'qualify'):
        p = sub.add_parser(name)
        for flag in ('--parent-policy', '--original-a2', '--development-a2', '--held-out-aa', '--output'):
            p.add_argument(flag, type=Path, required=True)
        p.add_argument('--parent-sha256', required=True)
    sub.choices['a2'].add_argument('--matrix', type=Path, required=True)
    sub.choices['a2'].add_argument('--sha256', required=True)
    q = sub.choices['qualify']
    for flag in ('--seed', '--a2', '--qualification-aa'):
        q.add_argument(flag, type=Path, required=True)
    q.add_argument('--continuation', type=Path, action='append', default=[])
    args = parser.parse_args(argv)
    parent = load_policy(args.parent_policy, args.parent_sha256)
    if args.command == 'a2':
        from .layout_development_a2 import collect
        path = collect(args.matrix, args.output / 'a2', args.sha256, 'held-out')
        seed = inputs(parent, args.parent_sha256, path, args.original_a2, args.development_a2,
                      args.held_out_aa, args.sha256)
        write_json(args.output / 'seed.json', seed)
        print(json.dumps({'seed_sha256': digest(args.output / 'seed.json'), 'held_out': seed['held_out'],
                          'held_out_lane': seed['held_out_lane']}, indent=2))
        return 0
    seed = json.loads(args.seed.read_text())
    rebuilt = inputs(parent, args.parent_sha256, args.a2, args.original_a2, args.development_a2,
                     args.held_out_aa, seed['calibration_binary_sha256'])
    if rebuilt != seed:
        raise ValueError('held-out seed differs from its replayed frozen estimators')
    from .layout_qualification import replay_calibration
    pilot = replay_calibration(args.qualification_aa, seed['calibration_binary_sha256'])
    if json.loads((args.qualification_aa.parent / 'identity.json').read_text()).get('lane') != 'held-out':
        raise ValueError('qualification A/A is not a held-out calibration')
    from .layout_continuation import replay as replay_continuation
    extensions, paths = {}, {}
    for path in args.continuation:
        name = json.loads(path.read_text())['case_id']
        if name in extensions:
            raise ValueError('duplicate latest continuation')
        extensions[name], _, host = replay_continuation(path.resolve(), args.qualification_aa.resolve(), name)
        if host != parent['host']:
            raise ValueError('continuation host mismatch')
        paths['continuation/' + name] = path
    policy = qualify(parent, seed, pilot, extensions)
    paths.update({'parent': args.parent_policy, 'seed': args.seed, 'a2': args.a2,
                  'original-a2': args.original_a2, 'development-a2': args.development_a2,
                  'held-out-aa': args.held_out_aa, 'qualification-aa': args.qualification_aa})
    policy['held_out_evidence'] = {
        n: {'path': str(p.resolve().relative_to(ROOT) if p.resolve().is_relative_to(ROOT) else p.resolve()),
            'sha256': digest(p)}
        for n, p in paths.items()}
    write_json(args.output, policy)
    print(json.dumps({'qualified': policy['qualified'], 'failures': policy['failures'],
                      'held_out': policy['held_out'], 'held_out_lane': policy['lanes']['held-out']}))
    return 0 if policy['qualified'] else 3


if __name__ == '__main__':
    raise SystemExit(main())
