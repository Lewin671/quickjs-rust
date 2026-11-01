"""Automatic case-subset formal continuation bound to a sealed bundle.

A/A and candidate/base bundles follow one power rule; a chain records which
comparison it extends and replay refuses the other kind. All three roles keep
their original argv/protocol/rotation. Samples append at absolute block
indexes; the sealed pilot and each predecessor remain immutable.
"""
from __future__ import annotations

import argparse
import dataclasses
import json
import platform
import datetime
from pathlib import Path

from .formal_case_context import context, prepare, schedule, validate_result, verify_assets, launch_preflight
from .layout_identity import digest, host_identity, write_json, same_node, same_host
from .layout_policy import analyze
from .layout_power import allocation, CAP
from .process import ProcessResult, run_process
from .pair_disturbance import Monitor, quieting_preflight, verify_formal_load


from . import formal_precision


COMPARISONS=('aa','candidate-base')


def comparison_kind(bundle):
    summary=json.loads((bundle/'summary.json').read_text())
    a,b=(summary['engines'][r] for r in ('base','candidate'))
    same=(a['binary_sha256'],a['source_revision'])==(b['binary_sha256'],b['source_revision'])
    return 'aa' if same else 'candidate-base'


def source_hashes(bundle, comparison='aa'):
    from .bundle import verify
    verify(bundle/'summary.json',bundle/'report.json',bundle/'external-report.json',bundle/'sentinel-report.json')
    summary=json.loads((bundle/'summary.json').read_text())
    if comparison not in COMPARISONS or comparison_kind(bundle)!=comparison:
        raise ValueError('formal continuation requires identical A/A roles' if comparison=='aa'
                         else 'candidate/base continuation requires distinct sealed roles')
    return {n:digest(bundle/n) for n in ('summary.json',*summary['bundle']['artifacts'])}


def replay(path, bundle, name, *, pilot=None, comparison='aa', precision_rule=None):
    from .layout_formal_raw import extract
    value=json.loads(path.read_text())
    if value['artifact_type']=='quickjs-formal-continuation-set':
        latest=Path(value['latest']['path'])
        if digest(latest)!=value['latest']['sha256']:raise ValueError('formal continuation set hash mismatch')
        return replay(latest,bundle,name,pilot=pilot,comparison=comparison,precision_rule=precision_rule)
    if value.get('formal_precision_rule') != precision_rule:
        raise ValueError('formal continuation precision rule mismatch')
    if value.get('comparison','aa')!=comparison:
        raise ValueError('formal continuation comparison kind mismatch')
    ctx=context(bundle,name)
    hashes=source_hashes(bundle,comparison)
    if value['source_hashes']!=hashes or value['case_id']!=name or value['bundle']!=str(bundle):
        raise ValueError('formal continuation source binding mismatch')
    if value['artifact_type']!='quickjs-formal-case-continuation':
        raise ValueError('unsupported formal continuation')
    if value['schema_version']!=1 or not same_node(value['host_node'], ctx['templates']['base']['host']['node']):
        raise ValueError('formal continuation host mismatch')
    if pilot is None:pilot=extract(bundle,value['host_node'])[name]
    if value['previous']:
        prev=Path(value['previous']['path'])
        if digest(prev)!=value['previous']['sha256']:raise ValueError('formal continuation predecessor hash mismatch')
        ratios,prior_host=replay(prev,bundle,name,pilot=pilot,comparison=comparison,precision_rule=precision_rule)
        if prior_host!=value['host']:raise ValueError('formal continuation host changed')
    else:ratios=list(pilot)
    before=len(ratios)
    needed=(formal_precision.next_count(len(pilot),before) if precision_rule == formal_precision.RULE else
            allocation(analyze({name:pilot}),before,analyze({name:ratios})['precise'],base_n=len(pilot)))
    if needed<=before or (value['previous_blocks'],value['required_blocks'])!=(before,needed):
        raise ValueError('formal continuation power/count mismatch')
    raw=path.parent/value['raw_file']
    if digest(raw)!=value['raw_sha256']:raise ValueError('formal continuation raw hash mismatch')
    rows=[json.loads(l) for l in raw.read_text().splitlines()]
    expected=[s for s in schedule(ctx,needed) if s[0]>=before]
    if len(rows)!=len(expected):raise ValueError('formal continuation physical plan incomplete')
    costs={};last=0;last_wall=0
    for row,(block,order,role) in zip(rows,expected):
        if (row['block'],row['order'],row['role'])!=(block,order,role) or row['argv']!=ctx['templates'][role]['argv']:
            raise ValueError('formal continuation role order/argv mismatch')
        result=ProcessResult(**row['result'])
        if result.timer_started_ns<last:raise ValueError('formal continuation process windows overlap')
        last=result.timer_finished_ns
        started=datetime.datetime.fromisoformat(result.started_at).timestamp()
        if started<value['load']['anchor']:raise ValueError('formal sample precedes monitor')
        last_wall=max(last_wall,started+result.duration_ns/1e9)
        costs[(role,block)]=validate_result(ctx,role,result)
    from .pair_disturbance import PROTOCOL
    if value['load']['protocol']!=PROTOCOL or value['load']['sealed_at']<last_wall+PROTOCOL['settle_seconds']:
        raise ValueError('formal load snapshot does not cover sample guards')
    verify_formal_load(value['load']['text'],value['load']['anchor'])
    import hashlib
    if hashlib.sha256(value['load']['text'].encode()).hexdigest()!=value['load']['sha256']:
        raise ValueError('formal continuation load hash mismatch')
    merged=ratios+[costs[('candidate',b)]/costs[('base',b)] for b in range(before,needed)]
    if merged!=value['ratios']:raise ValueError('formal continuation ratio conservation mismatch')
    return merged,value['host']


def collect(bundle, name, out, previous=None, precision_rule=None):
    from .layout_formal_raw import extract
    from .screen import measurement_lock, wait_for_builds, check_load, DEFAULT_CACHE
    quiet=quieting_preflight();host=host_identity();node=platform.node()
    comparison=comparison_kind(bundle)
    ctx=context(bundle,name);hashes=source_hashes(bundle,comparison)
    from .runner import _host_metadata
    if not same_host(_host_metadata(), ctx['templates']['base']['host']):
        raise ValueError('formal continuation runtime host changed')
    pilot=extract(bundle,node)[name]
    ratios=list(pilot)
    if previous is not None:
        ratios,prior_host=replay(previous,bundle,name,pilot=pilot,comparison=comparison,precision_rule=precision_rule)
        if prior_host!=host:raise ValueError('formal continuation host/recipe changed')
    target=(formal_precision.next_count(len(pilot),len(ratios)) if precision_rule == formal_precision.RULE else
            allocation(analyze({name:pilot}),len(ratios),analyze({name:ratios})['precise'],base_n=len(pilot)))
    if target<=len(ratios):raise ValueError('formal continuation is not required')
    out.mkdir(parents=True,exist_ok=False)
    monitor=Monitor.from_queue()
    from .cycle_trust import withhold_untrusted_counters
    withhold_untrusted_counters()
    costs={}
    with measurement_lock('formal-case-continuation'):
        if wait_for_builds(300):raise ValueError('competing builds remain')
        check_load(8,True,300)
        prepare(ctx,DEFAULT_CACHE)
        preflight=launch_preflight(ctx)
        with (out/'samples.jsonl').open('x') as stream:
            for block,order,role in schedule(ctx,target):
                if block<len(ratios):continue
                argv=ctx['templates'][role]['argv'];result=run_process(argv,ctx['timeout'])
                row={'block':block,'order':order,'role':role,'argv':argv,'result':dataclasses.asdict(result)}
                stream.write(json.dumps(row)+'\n');stream.flush()
                costs[(role,block)]=validate_result(ctx,role,result)
    load=monitor.snapshot()
    write_json(out/'load.json',load)
    verify_formal_load(load['text'],load['anchor'])
    verify_assets(ctx)
    if source_hashes(bundle,comparison)!=hashes:raise ValueError('formal source bundle changed')
    value={'artifact_type':'quickjs-formal-case-continuation','schema_version':1,
        **({'comparison':comparison} if comparison!='aa' else {}),
        **({'formal_precision_rule':precision_rule} if precision_rule else {}),
        'bundle':str(bundle),'source_hashes':hashes,'case_id':name,'host':host,'host_node':node,
        'previous':{'path':str(previous),'sha256':digest(previous)} if previous else None,
        'previous_blocks':len(ratios),'required_blocks':target,'raw_file':'samples.jsonl',
        'raw_sha256':digest(out/'samples.jsonl'),'quieting':quiet,'launch_preflight':preflight,'load':load,
        'ratios':ratios+[costs[('candidate',b)]/costs[('base',b)] for b in range(len(ratios),target)]}
    write_json(out/'continuation.json',value)
    replay(out/'continuation.json',bundle,name,pilot=pilot,comparison=comparison,precision_rule=precision_rule)
    return out/'continuation.json',(formal_precision.analyze_case(value['ratios']) if precision_rule else analyze({name:value['ratios']}))


def complete(bundle, name, root, previous=None, precision_rule=None):
    root.mkdir(parents=True,exist_ok=False)
    for stage in range(2):
        for attempt in range(4):
            try:
                latest,stats=collect(bundle,name,root/f'stage{stage}-attempt{attempt}',previous,
                                     **({'precision_rule':precision_rule} if precision_rule else {}))
                break
            except ValueError as error:
                if not str(error).startswith('DISTURBED:') or attempt==3:raise
                # Failed raw samples stay at their original paths; the same
                # deficit is replaced whole, never selected by measured values.
        if stats['precise'] or stats['blocks']==CAP:break
        previous=latest
    meta=json.loads(latest.read_text())
    write_json(root/'continuation.json',{'artifact_type':'quickjs-formal-continuation-set',
        'case_id':name,'host':meta['host'],'host_node':meta['host_node'],
        **({'comparison':meta['comparison']} if 'comparison' in meta else {}),
        **({'formal_precision_rule':precision_rule} if precision_rule else {}),
        'latest':{'path':str(latest),'sha256':digest(latest)},'precision':stats})
    return root/'continuation.json'


def main():
    p=argparse.ArgumentParser(description=__doc__)
    p.add_argument('--bundle',type=Path,required=True);p.add_argument('--case',
        help='optional subset; default continues every deficient case of the sealed A/A or candidate/base bundle')
    p.add_argument('--output',type=Path,required=True);p.add_argument('--previous',type=Path)
    p.add_argument('--plan',type=Path,help='frozen plan; required to opt into prospective precision v2')
    p.add_argument('--mode',choices=('promotion','stage'),default='promotion')
    args=p.parse_args()
    bundle=args.bundle.resolve()
    unit=policy=None
    precision_rule=None
    if args.plan:
        from .performance_decision import load_unit
        from .layout_policy import load_policy
        from .performance_evidence import ROOT
        unit,_=load_unit(args.plan)
        gate=unit['layout_gate']
        policy=load_policy(ROOT/gate['policy_path'],gate['policy_sha256'])
        summary=json.loads((bundle/'summary.json').read_text())
        if (summary['engines']['base']['source_revision'] != unit['base_sha']
                or summary.get('lane_schedule') != policy.get('formal_lane_schedule')):
            raise ValueError('continuation plan base/schedule mismatch')
        precision_rule=policy.get('formal_precision_rule')
        if precision_rule and comparison_kind(bundle) != 'candidate-base':
            raise ValueError('prospective precision is not a qualification A/A rule')
    if args.case and precision_rule:
        from .layout_formal_raw import extract
        pilot=extract(bundle,platform.node())[args.case]
        if len(pilot) != unit['layout_gate']['formal_blocks']:
            raise ValueError('continuation pilot differs from frozen block budget')
        if formal_precision.ready(formal_precision.analyze_case(pilot),
                                  formal_precision.threshold(unit,policy,args.case,args.mode)):
            raise ValueError('case already resolved; prospective continuation is not required')
    if args.case:
        path=complete(bundle,args.case,args.output.resolve(),
                      args.previous.resolve() if args.previous else None, precision_rule)
        print(json.dumps({'continuation':str(path),'precision':json.loads(path.read_text())['precision']}))
    else:
        if args.previous:p.error('--previous requires --case')
        from .layout_formal_raw import extract
        args.output.mkdir(parents=True,exist_ok=False)
        paths=[]
        for name,ratios in extract(bundle,platform.node()).items():
            if precision_rule and len(ratios) != unit['layout_gate']['formal_blocks']:
                raise ValueError('continuation pilot differs from frozen block budget')
            stats=formal_precision.analyze_case(ratios) if precision_rule else analyze({name:ratios})
            deficient=(not formal_precision.ready(stats,formal_precision.threshold(unit,policy,name,args.mode))
                       if precision_rule else stats['relative_half_width']>.03)
            if deficient:
                path=complete(bundle,name,args.output.resolve()/name.replace('/','--'),precision_rule=precision_rule)
                paths.append({'path':str(path),'sha256':digest(path)})
        write_json(args.output/'power.json',{'continuations':paths})


if __name__=='__main__':main()
