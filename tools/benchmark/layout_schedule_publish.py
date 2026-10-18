"""Replay schedule-calibration receipts; qualification and full band comparison.

The command writes a qualified or rejected artifact in the queue output. It
never installs a policy in the repository or rewrites a frozen seed.
"""
from __future__ import annotations

import json
import platform
import subprocess
from pathlib import Path

from .layout_identity import digest, host_identity, write_json
from .layout_schedule_calibration import load, receipt


def formal_stage(plan, target):
    from .pair_disturbance import Monitor, quieting_preflight, verify_formal_load
    target.mkdir()
    monitor = Monitor.from_queue()
    write_json(target/'host-attestation.json', {'host':host_identity(), 'node':platform.node(),
               'started_at':monitor.anchor, 'quieting':quieting_preflight()})
    subprocess.run(['./scripts/perf-compare.sh', '--base', plan['formal_revision'], '--candidate', plan['formal_revision'],
                    '--blocks','30','--workers','4','--output-dir',str(target/'bundle')], check=True)
    snap = monitor.snapshot()
    write_json(target/'load.json', snap)
    verify_formal_load(snap['text'], snap['anchor'])
    return target/'bundle/summary.json'


def qualify_stage(args, plan, out):
    from .layout_schedule_bands import inputs
    from .layout_schedule_qualification import qualify, replay_formal, compare_bands
    from .layout_policy import load_policy
    from .layout_qualification import replay_calibration
    from .layout_continuation import replay as replay_screen
    from .layout_formal_continuation import replay as replay_formal_continuation
    seed = load(args.seed)
    expected_images = {'portfolio':plan['portfolio_image'], 'development':plan['npm_image'], 'held-out':plan['npm_image']}
    if seed['source_images'] != expected_images:
        raise ValueError('seed image differs from frozen calibration plan')
    lanes = {load(receipt(p))['artifact_type'].removeprefix('quickjs-').removesuffix('-layout-a2'): receipt(p) for p in args.a2}
    rebuilt = inputs(lanes, receipt(args.estimator), seed['source_images'], plan['parent_sha256'])
    rebuilt['calibration_plan'] = {'path': str(args.plan), 'sha256': digest(args.plan)}
    if seed != rebuilt:
        raise ValueError('seed differs from frozen replayed estimators/plan')
    parent = load_policy(Path(plan['parent_policy']), plan['parent_sha256'])
    pilots, extensions, evidence = {}, {}, {}
    for root in args.pilot:
        path = receipt(root)
        pilot = replay_calibration(path, plan['npm_image'])
        lane = load(path.parent/'identity.json')['lane']
        if lane in pilots:
            raise ValueError('duplicate qualification pilot lane')
        pilots[lane] = pilot
        evidence['pilot/' + lane] = {'path':str(path), 'sha256':digest(path)}
        for record in load(path.parent/'power.json')['continuations']:
            ext = Path(record['path'])
            if digest(ext) != record['sha256']:
                raise ValueError('power manifest hash mismatch')
            name = load(ext)['case_id']
            ratios, _, host = replay_screen(ext, path, name)
            if host != seed['host'] or name in extensions:
                raise ValueError('screen continuation identity mismatch')
            extensions[name] = ratios
            evidence['screen-power/' + name] = record
    formal_summary = receipt(args.formal)
    folder = formal_summary.parent
    attestation = folder.parent/'host-attestation.json'
    snap = folder.parent/'load.json'
    # snapshot stores the original unmodified log text and wall anchor.
    from .pair_disturbance import verify_formal_load
    load_record = load(snap)
    verify_formal_load(load_record['text'], load_record['anchor'])
    load_file = out/'formal-load.log'
    load_file.write_text(load_record['text'])
    formal = replay_formal(folder, seed, plan['formal_revision'], attestation, load_file)
    formal_extended = {}
    power = args.formal_power/'power/power.json'
    for record in load(power)['continuations']:
        path = Path(record['path'])
        if digest(path) != record['sha256']:
            raise ValueError('formal power manifest changed')
        name = load(path)['case_id']
        ratios, host = replay_formal_continuation(path, folder, name, pilot=formal[name])
        if host != seed['host'] or name in formal_extended:
            raise ValueError('formal continuation identity mismatch')
        formal_extended[name] = ratios
        evidence['formal-power/' + name] = record
    policy = qualify(seed, pilots, formal, extensions, formal_extended)
    for label, path in (('seed', args.seed), ('plan',args.plan), ('formal-summary',formal_summary),
                        ('formal-load',snap), ('host-attestation',attestation)):
        evidence[label] = {'path':str(path), 'sha256':digest(path)}
    policy['evidence'] = {**seed['evidence'], **evidence}
    policy['parent_policy_sha256'] = plan['parent_sha256']
    write_json(out/'qualification.json', policy)
    write_json(out/'band-comparison.json', compare_bands(parent, policy))
    if policy['qualified']:
        write_json(out/'policy-v5.json', policy)
        (out/'policy-v5.sha256').write_text(digest(out/'policy-v5.json') + '\n')
    print(json.dumps({'qualified':policy['qualified'], 'failures':policy['failures'],
                      'sha256':digest(out/'qualification.json')}, indent=2))
    return 0 if policy['qualified'] else 3
