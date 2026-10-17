"""Apply the host-bound rule beside sealed formal precision/correctness gates.

Formal case statistics may extend through replayed candidate/base precision
continuations; lane statistics keep the sealed joint blocks.
"""
from __future__ import annotations

from pathlib import Path

from .layout_evidence import replay
from .layout_identity import digest
from .layout_policy import HELD_OUT_RULE, load_policy, analyze, rows, selection_lanes, with_held_out_successor
from .layout_rule import evaluate
from .layout_power import diagnostic_only, needs_panel
from .performance_evidence import ROOT


def evaluate_formal(unit, summary, broad, external, sentinel, mode, panel_path, held_path, formal_bundle=None, resource_report=None, resource_raw=None, formal_continuations=()):
    gate = unit['layout_gate']
    policy = load_policy(ROOT / gate['policy_path'], gate['policy_sha256'], live=False)
    policy, held_policy_sha = with_held_out_successor(policy, gate['policy_sha256'])
    missing, failed = [], []
    if panel_path is None:
        return {'label': 'incomplete', 'passed': False, 'failures': [],
                'missing': ['layout evidence missing']}, ['layout evidence missing'], [], {}
    evidence = replay(panel_path, policy, summary['engines'])
    panels = evidence['panels']
    if evidence['pairs'] != gate['panel_pairs']:
        missing.append('layout panel differs from frozen pair budget')
    held = replay(held_path, policy, summary['engines'], held=True) if held_path else None
    if held and held['pairs'] != gate['held_out_pairs']:
        missing.append('held-out evidence differs from frozen pair budget')
    persistence = policy.get('held_out_rule') == HELD_OUT_RULE
    held_panel = held['panels'] if held and persistence and len(held['panels']) == 6 else None
    target_ids = unit['fast_gate']['target_ids']
    payoff = unit['fast_gate']['target_max_candidate_over_base']
    verdict = evaluate(policy, panels[0], target_ids, payoff,
        panel=panels if len(panels) == 6 else None,
        held_out=held['panels'][0] if held else None, held_out_panel=held_panel,
        instruction_blind=gate['mechanism_class'] == 'instruction-blind',
        memory_targets=gate['memory_wall_targets'], promotion=mode == 'promotion',
        panel_required=gate['panel_required'], stage=mode == 'stage',
        stage_limit=unit.get('migration', {}).get('stage_max_candidate_over_base', 1.25))
    missing.extend(verdict['missing'])
    failed.extend(verdict['failures'])
    if gate['panel_required'] and len(panels) != 6:
        missing.append('frozen plan requires K=6')
    wall = {}
    if formal_bundle is None:
        missing.append('canonical raw formal bundle required')
    else:
        from .layout_formal_raw import extract
        development = 'development' in policy['lanes']
        if development:
            import json
            from .bundle import verify, development_report_path
            from .performance_evidence import replay_reports, validate_development
            verify(formal_bundle / 'summary.json', formal_bundle / 'report.json',
                   formal_bundle / 'external-report.json', formal_bundle / 'sentinel-report.json')
            dev_path = development_report_path(formal_bundle / 'summary.json')
            if dev_path is None:
                raise ValueError('development policy requires a sealed development formal lane')
            replay_reports(formal_bundle / 'report.json', formal_bundle / 'external-report.json',
                           formal_bundle / 'sentinel-report.json', dev_path)
            missing.extend(validate_development(json.loads(dev_path.read_text()), summary['engines'], broad['run']['host']))
        wall = extract(formal_bundle, policy['host_node'], development=development)
    if wall and any(len(values) != gate['formal_blocks'] for values in wall.values()):
        missing.append('formal evidence differs from frozen block budget')
    if set(wall) != set(policy['cases']):
        missing.append('formal wall inventory does not match policy')
    extended, continuation_hashes = continued_cases(policy, formal_bundle, wall, formal_continuations)
    formal_stats = {}
    for case, band in policy['cases'].items():
        if case not in wall:
            continue
        stats = analyze({case: extended.get(case, wall[case])})
        formal_stats[case] = stats
        lower, upper = stats['confidence_interval']['lower'], stats['confidence_interval']['upper']
        diagnostic=diagnostic_only(policy,case)
        if diagnostic and len(panels)!=6:
            missing.append(case + ': diagnostic-only case requires K=6')
        if not stats['precise'] and not diagnostic:
            missing.append(case + ': formal interval imprecise')
            continue
        limit = (payoff if diagnostic_only(policy,case) else min(payoff, 1 / band['wall_band'])) if case in target_ids and mode != 'stage' else band['wall_band']
        if mode == 'stage':
            limit = min(limit, unit['migration']['stage_max_candidate_over_base'])
        if diagnostic_only(policy,case) and (case not in target_ids or mode == 'stage'):
            if len(panels) != 6:missing.append(case + ': diagnostic-only case requires K=6')
            if mode == 'stage':
                limit=unit['migration']['stage_max_candidate_over_base']
                if lower > limit:failed.append(case + ': formal stage budget exceeded')
                elif upper > limit:missing.append(case + ': formal stage interval crosses budget')
            continue
        if upper <= limit:
            continue
        if case in target_ids and mode != 'stage':
            if lower > limit:
                failed.append(case + ': formal target payoff failed')
            else:
                missing.append(case + ': formal target interval crosses threshold')
        elif len(panels) != 6:
            missing.append(case + ': formal wall interval requires K=6')
        elif verdict['cases'][case]['label'] == 'reject':
            failed.append(case + ': persistent panel regression')
    for lane in selection_lanes(policy):
        values = {name: ratios for name, ratios in wall.items()
                  if lane == 'portfolio' or name.startswith(lane + '/')}
        if not values:
            continue
        stats = analyze(values)
        formal_stats[lane] = stats
        if not stats['precise']:
            missing.append(lane + ': formal lane interval imprecise')
        lower, upper = stats['confidence_interval']['lower'], stats['confidence_interval']['upper']
        if persistence and upper > policy['lanes'][lane]:
            # v4 lane event: the six-layout panel verdict decides, as in the screen.
            if len(panels) != 6:
                missing.append(lane + ': formal lane interval above ceiling requires K=6')
        elif lower > policy['lanes'][lane]:
            failed.append(lane + ': formal lane ceiling exceeded')
        elif upper > policy['lanes'][lane]:
            missing.append(lane + ': formal lane interval crosses ceiling')
    verdict['formal_statistics'] = formal_stats
    verdict['formal_continued_blocks'] = {n: len(r) for n, r in extended.items()}
    if held and persistence:
        missing.extend(held_out_intervals(policy, held, held_panel is not None))
    elif held:
        if needs_panel(policy,held=True) and len(held['panels']) != 6:
            missing.append('diagnostic-only held-out case requires K=6 held-out panel')
        for extra in held['panels'][1:]:
            check=evaluate(policy,panels[0],target_ids,payoff,
                panel=panels if len(panels)==6 else None,held_out=extra,promotion=True,
                instruction_blind=gate['mechanism_class']=='instruction-blind',
                memory_targets=gate['memory_wall_targets'],panel_required=gate['panel_required'])
            failed.extend(check['failures']);missing.extend(check['missing'])
        values = rows(held['panels'][0], policy['held_out'])
        for case, ratios in values.items():
            stats = analyze({case: ratios['wall_ns']})
            upper = stats['confidence_interval']['upper']
            if not stats['precise'] or (not diagnostic_only(policy,case,held=True) and upper > policy['held_out'][case]['wall_band']):
                missing.append(case + ': held-out formal precision/upper interval gate')
        stats = analyze({n: r['wall_ns'] for n, r in values.items()})
        if not stats['precise'] or stats['confidence_interval']['upper'] > policy['lanes']['held-out']:
            missing.append('held-out lane formal precision/upper interval gate')
    if gate['mechanism_class'] == 'instruction-blind':
        from .layout_resources import verify
        resources, resource_missing = verify(resource_report, resource_raw, policy, summary['engines'])
        verdict['resources'] = resources
        missing.extend(resource_missing)
    hashes = {'policy_sha256': gate['policy_sha256'], 'layout_panel_sha256': digest(panel_path),
              'held_out_panel_sha256': digest(held_path) if held_path else None,
              **({'formal_continuation_sha256': continuation_hashes} if continuation_hashes else {}),
              **({'held_out_policy_sha256': held_policy_sha} if held_policy_sha else {})}
    verdict['label'] = 'reject' if failed else ('incomplete' if missing else 'pass')
    verdict['passed'] = not failed and not missing
    return verdict, missing, failed, hashes


def continued_cases(policy, bundle, wall, paths):
    """Latest candidate/base continuation per case; replay checks every prefix."""
    extended, hashes = {}, {}
    if paths and bundle is None:
        raise ValueError('formal continuation requires its sealed formal bundle')
    import json
    from .layout_formal_continuation import replay as replay_formal
    for path in paths:
        name = json.loads(Path(path).read_text())['case_id']
        if name in extended or name not in wall:
            raise ValueError('duplicate or unknown formal continuation case: ' + name)
        ratios, host = replay_formal(Path(path).resolve(), Path(bundle).resolve(), name,
                                     pilot=wall[name], comparison='candidate-base')
        if host != policy['host']:
            raise ValueError('formal continuation host/recipe differs from policy')
        extended[name], hashes[name] = ratios, digest(Path(path))
    return extended, hashes


def held_out_intervals(policy, held, complete_panel):
    """v4: an upper bound above B_i or the lane ceiling needs the K=6 panel."""
    missing = []
    values = rows(held['panels'][0], policy['held_out'])
    for case, ratios in values.items():
        stats = analyze({case: ratios['wall_ns']})
        diagnostic = diagnostic_only(policy, case, held=True)
        if not stats['precise'] and not diagnostic:
            missing.append(case + ': held-out formal precision gate')
        elif (not diagnostic and stats['confidence_interval']['upper'] > policy['held_out'][case]['wall_band']
              and not complete_panel):
            missing.append(case + ': held-out upper interval crosses band; K=6 held-out panel required')
    stats = analyze({n: r['wall_ns'] for n, r in values.items()})
    if not stats['precise']:
        missing.append('held-out lane formal precision gate')
    elif stats['confidence_interval']['upper'] > policy['lanes']['held-out'] and not complete_panel:
        missing.append('held-out lane upper interval crosses ceiling; K=6 held-out panel required')
    return missing
