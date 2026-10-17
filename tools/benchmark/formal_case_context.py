"""Sealed case contracts for formal A/A continuation, without re-calibration.

Original argv paths, role schedules, images and workload bytes are authoritative.
Only removed transient assets may be restored; sealed evidence is never edited.
"""
from __future__ import annotations

import dataclasses
import hashlib
import json
import shutil
import statistics
from fractions import Fraction
from pathlib import Path

from .layout_identity import digest
from .planning import role_orders, measurement_plan
from .process import ProcessResult

ROLES=('candidate','base','quickjs-ng')


def context(bundle, name):
    summary=json.loads((bundle/'summary.json').read_text())
    images={e['role']:e for e in json.loads((bundle/'report.json').read_text())['run']['engines']}
    if name.startswith('external/'):
        from .external_preview import load_manifest
        manifest=load_manifest(bundle/'external-manifest.json')
        _,suite_id,case_id=name.split('/')
        si,suite=next((i,s) for i,s in enumerate(manifest.suites) if s.id==suite_id)
        ci,case=next((i,c) for i,c in enumerate(suite.cases) if c.id==case_id)
        raw=bundle/'external-raw.jsonl'
        seed=manifest.measurement.seed+si*10000+ci*100
        rows=[json.loads(l) for l in raw.read_text().splitlines()]
        selected=[r for r in rows if r.get('phase')=='measurement' and r['suite_id']==suite_id and r['case_id']==case_id]
        count=manifest.measurement.blocks
        # The archived manifest keeps its default preview count; raw carries
        # the actual sealed canonical block count.
        count=selected[0]['measurement_blocks']
        timeout=manifest.measurement.timeout_for(suite_id,case_id)
        startup={}
    else:
        from .performance_evidence import load_archived_manifest
        from .raw_validation import validate_run
        lane,case_id=name.split('/',1);prefix='' if lane=='broad' else 'sentinel-'
        manifest=load_archived_manifest(bundle/(prefix+'manifest.json'),lane)
        run=validate_run(bundle/(prefix+'raw.jsonl'),manifest)
        case=next(c for c in manifest.cases if c.id==case_id)
        raw=bundle/(prefix+'raw.jsonl');rows=[json.loads(l) for l in raw.read_text().splitlines()]
        selected=[r for r in rows if r.get('phase')=='measurement' and r['case_id']==case_id]
        count=run.blocks;seed=run.start['seed'];timeout=case.timeout_seconds
        startup={r:statistics.median(run.startup[(r,case_id)]) for r in ROLES}
    templates={r:next(x for x in selected if x['role']==r) for r in ROLES}
    if len(selected)!=count*3 or any(x['status']!='ok' for x in selected):
        raise ValueError('formal pilot case input is incomplete')
    for role in ROLES:
        t=templates[role]
        if t['binary_sha256']!=summary['engines'][role]['binary_sha256']:
            raise ValueError('formal template image differs from sealed identity')
        if any(x['argv']!=t['argv'] for x in selected if x['role']==role):
            raise ValueError('formal pilot argv changed')
    return {'bundle':bundle,'name':name,'summary':summary,'images':images,'manifest':manifest,
            'case':case,'templates':templates,'count':count,'seed':seed,'timeout':timeout,
            'startup':startup,'raw':raw,'external':name.startswith('external/'),
            'selected_cases':None if name.startswith('external/') else run.start['selected_cases']}


def schedule(ctx, total):
    if ctx['external']:
        return [(block,order,role) for block,roles in enumerate(role_orders(list(ROLES),total,ctx['seed']))
                for order,role in enumerate(roles)]
    return [(p.block,p.order,p.role) for p in measurement_plan(list(ROLES),ctx['selected_cases'],total,ctx['seed'])
            if p.case_id==ctx['case'].id]


def validate_result(ctx, role, result):
    if (type(result.timer_started_ns) is not int or type(result.timer_finished_ns) is not int
            or type(result.exit_code) is not int or type(result.duration_ns) is not int or result.duration_ns<=0
            or result.timer_finished_ns-result.timer_started_ns!=result.duration_ns):
        raise ValueError('formal continuation timer mismatch')
    template=ctx['templates'][role]
    if (result.instructions is None)!=(template['instructions'] is None) or (result.cycles is None)!=(template['cycles'] is None):
        raise ValueError('formal continuation counter protocol differs')
    from .raw_contract import validate_counters
    validate_counters(dataclasses.asdict(result),'formal continuation')
    if ctx['external']:
        from .external_preview import _sample_status
        status,error=_sample_status(result,role)
        if status!='ok':raise ValueError('formal continuation sample '+status+': '+str(error))
        return result.duration_ns
    from .raw_contract import validate_success
    row={**template,**{k:v for k,v in dataclasses.asdict(result).items() if k in template}}
    validate_success(row,ctx['case'],'formal continuation')
    if (result.duration_ns<ctx['case'].min_window_ms*1e6 or
            Fraction(ctx['startup'][role],result.duration_ns)>ctx['case'].startup_max_fraction):
        raise ValueError('formal continuation sample is timer-limited')
    return result.duration_ns/row['operations']


def _restore(source, target, expected, *, data=None, mode=0o500):
    if target.exists():
        if digest(target)!=expected:raise ValueError('existing formal asset hash mismatch')
        return
    if (hashlib.sha256(data).hexdigest() if data is not None else digest(source)) != expected:
        raise ValueError('formal source asset changed')
    target.parent.mkdir(parents=True,exist_ok=True)
    with target.open('xb') as out:
        if data is not None:out.write(data)
        else:
            with source.open('rb') as inp:shutil.copyfileobj(inp,out)
    target.chmod(mode)
    if digest(target)!=expected:raise ValueError('restored formal asset hash mismatch')


def prepare(ctx, cache):
    for role,t in ctx['templates'].items():
        _restore(Path(ctx['images'][role]['binary_source_path']),Path(t['argv'][0]),t['binary_sha256'])
        workload=Path(t['argv'][-1] if ctx['external'] else t['workload_snapshot_path'])
        if ctx['external']:
            from .external_preview import _bundle_source,fetch_corpora
            fetch_corpora(ctx['manifest'],cache)
            suite=next(s for s in ctx['manifest'].suites if s.id==t['suite_id'])
            data=_bundle_source(suite,ctx['case'],cache).encode()
            _restore(None,workload,t['bundle_sha256'],data=data,mode=0o400)
        else:
            _restore(ctx['case'].workload,workload,t['workload_sha256'],mode=0o400)


def verify_assets(ctx):
    for t in ctx['templates'].values():
        workload=Path(t['argv'][-1] if ctx['external'] else t['workload_snapshot_path'])
        expected=t['bundle_sha256'] if ctx['external'] else t['workload_sha256']
        if digest(Path(t['argv'][0]))!=t['binary_sha256'] or digest(workload)!=expected:
            raise ValueError('formal measured asset changed')
