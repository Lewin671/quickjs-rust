"""Append power-allocated A/A pairs to immutable pilot samples and executables.

Every segment retains its own monitor history and source hash. No old pair,
iteration calibration, executable path or order is replaced on continuation.
"""
from __future__ import annotations

import argparse
import json
import platform
from pathlib import Path

from . import screen
from .layout_identity import digest, host_identity, write_json
from .layout_policy import analyze
from .layout_power import PILOT, CAP, allocation
from .pair_disturbance import Monitor, quieting_preflight, verify_run
from .pair_recording import measure
from .screen_replay import replay_case


def load_case(pilot, name):
    identity = json.loads((pilot.parent/'identity.json').read_text())
    binaries = dict(zip(('base','candidate'),identity['binaries']))
    # Verify the actual immutable measured paths, not a new copied executable.
    if len(set(binaries.values())) != 2 or any(digest(Path(p)) != identity['sha256'] for p in binaries.values()):
        raise ValueError('continuation executable identity changed')
    raw = json.loads((pilot.parent/'cases'/(name.replace('/','--')+'.json')).read_text())
    replay_case(raw,PILOT,binaries)
    packet = json.loads(pilot.read_text())
    if raw['ratios'] != next(c['ratios'] for c in packet['cases'] if c['id']==name):
        raise ValueError('continuation pilot report/raw mismatch')
    if 'attempts' not in raw:
        raise ValueError('continuation needs timestamped pilot attempts')
    verify_run([raw],packet['disturbance'])
    return identity, binaries, raw


def replay(path, pilot, name):
    """Return all original and appended ratios, checking every immutable prefix."""
    identity, binaries, original = load_case(pilot,name)
    value = json.loads(path.read_text())
    if value['pilot'] != {'path':str(pilot), 'sha256':digest(pilot)} or value['case_id'] != name:
        raise ValueError('continuation pilot binding mismatch')
    if value['previous']:
        previous = Path(value['previous']['path'])
        if digest(previous) != value['previous']['sha256']:
            raise ValueError('continuation predecessor hash mismatch')
        ratios, offset, _ = replay(previous,pilot,name)
    else:
        ratios, offset = original['ratios'],len(original['attempts'])
    current = len(ratios['wall_ns'])
    pilot_stats = analyze({name:original['ratios']['wall_ns']})
    current_stats = analyze({name:ratios['wall_ns']})
    expected = allocation(pilot_stats,current,current_stats['precise'])
    if expected <= current or value['required_pairs'] != expected or value['previous_pairs'] != current:
        raise ValueError('continuation count is not the frozen power allocation')
    raw_path = path.parent/value['raw_file']
    if digest(raw_path) != value['raw_sha256']:
        raise ValueError('continuation raw hash mismatch')
    raw = json.loads(raw_path.read_text())
    if raw.get('pair_index_offset') != offset or raw['iterations'] != original['iterations'] or raw['id'] != name:
        raise ValueError('continuation order/iteration identity mismatch')
    appended = replay_case(raw,expected-current,binaries)
    if verify_run([raw],value['disturbance']) != value['disturbance_counts']:
        raise ValueError('continuation disturbance mismatch')
    merged = {m:ratios[m]+appended[m] for m in ('instructions','wall_ns')}
    if merged != value['ratios'] or value['sha256'] != identity['sha256']:
        raise ValueError('continuation ratio/binary conservation failed')
    return merged,offset+len(raw['attempts']),value['host']


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--pilot',type=Path,required=True)
    parser.add_argument('--case',required=True)
    parser.add_argument('--previous',type=Path)
    parser.add_argument('--pilot-context',required=True)
    parser.add_argument('--output',type=Path,required=True)
    args=parser.parse_args()
    pilot=args.pilot.resolve()
    from .layout_qualification import replay_calibration, verify_host_inputs
    identity,binaries,original=load_case(pilot,args.case)
    replay_calibration(pilot,identity['sha256'])
    host=host_identity()
    verify_host_inputs(identity['host'],host,args.pilot_context)
    quieting=quieting_preflight()
    ratios,offset=(original['ratios'],len(original['attempts']))
    if args.previous:
        ratios,offset,prior_host=replay(args.previous.resolve(),pilot,args.case)
        if prior_host != host:raise ValueError('continuation host changed')
    count=len(ratios['wall_ns'])
    pilot_stats=analyze({args.case:original['ratios']['wall_ns']})
    stats=analyze({args.case:ratios['wall_ns']})
    needed=allocation(pilot_stats,count,stats['precise'])
    if needed<=count:raise ValueError('no power continuation is required')
    out=args.output.resolve();out.mkdir(parents=True,exist_ok=False)
    (out/'work').mkdir()
    cases=(screen.resolve_cases([args.case],screen.DEFAULT_CACHE,out/'work') if not args.case.startswith('held-out/') else
           __import__('tools.benchmark.screen_recording',fromlist=['held_out_cases']).held_out_cases(screen.DEFAULT_CACHE,out/'work'))
    case=next(c for c in cases if c.id==args.case)
    monitor=Monitor.from_queue()
    with screen.measurement_lock('layout-power-continuation'):
        if screen.wait_for_builds(300):raise ValueError('competing builds remain')
        screen.check_load(8,True,300)
        measure(case,Path(binaries['candidate']),Path(binaries['base']),needed-count,
                screen.counter_tool(),120,out/'case.json',monitor,
                iterations=original['iterations'],pair_offset=offset)
    raw=json.loads((out/'case.json').read_text())
    disturbance=monitor.snapshot()
    appended=raw['ratios']
    for binary in binaries.values():
        if digest(Path(binary))!=identity['sha256']:raise ValueError('continuation binary changed')
    value={'artifact_type':'quickjs-layout-continuation','schema_version':1,'case_id':args.case,
        'pilot':{'path':str(pilot),'sha256':digest(pilot)},
        'previous':{'path':str(args.previous.resolve()),'sha256':digest(args.previous)} if args.previous else None,
        'host':host,'host_node':platform.node(),'pilot_context':args.pilot_context,'quieting':quieting,'sha256':identity['sha256'],'previous_pairs':count,'required_pairs':needed,
        'raw_file':'case.json','raw_sha256':digest(out/'case.json'),
        'disturbance':disturbance,'disturbance_counts':verify_run([raw],disturbance),
        'ratios':{m:ratios[m]+appended[m] for m in ('instructions','wall_ns')}}
    write_json(out/'continuation.json',value)
    replay(out/'continuation.json',pilot,args.case)
    print(json.dumps({'case':args.case,'valid_pairs':needed,'precision':analyze({args.case:value['ratios']['wall_ns']})}))


if __name__=='__main__':main()
