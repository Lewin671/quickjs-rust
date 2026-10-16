"""Apply the host-bound rule beside sealed formal precision/correctness gates."""
from __future__ import annotations

from pathlib import Path

from .layout_evidence import replay
from .layout_identity import digest
from .layout_policy import load_policy, analyze, rows
from .layout_rule import evaluate
from .performance_evidence import ROOT


def evaluate_formal(unit, summary, broad, external, sentinel, mode, panel_path, held_path, formal_bundle=None, resource_report=None, resource_raw=None):
    gate = unit['layout_gate']
    policy = load_policy(ROOT / gate['policy_path'], gate['policy_sha256'], live=False)
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
    target_ids = unit['fast_gate']['target_ids']
    payoff = unit['fast_gate']['target_max_candidate_over_base']
    verdict = evaluate(policy, panels[0], target_ids, payoff,
        panel=panels if len(panels) == 6 else None,
        held_out=held['panels'][0] if held else None,
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
        wall = extract(formal_bundle, policy['host_node'])
    if wall and any(len(values) != gate['formal_blocks'] for values in wall.values()):
        missing.append('formal evidence differs from frozen block budget')
    if set(wall) != set(policy['cases']):
        missing.append('formal wall inventory does not match policy')
    formal_stats = {}
    for case, band in policy['cases'].items():
        if case not in wall:
            continue
        stats = analyze({case: wall[case]})
        formal_stats[case] = stats
        lower, upper = stats['confidence_interval']['lower'], stats['confidence_interval']['upper']
        if not stats['precise']:
            missing.append(case + ': formal interval imprecise')
            continue
        limit = min(payoff, 1 / band['wall_band']) if case in target_ids and mode != 'stage' else band['wall_band']
        if mode == 'stage':
            limit = min(limit, unit['migration']['stage_max_candidate_over_base'])
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
    for lane in ('broad', 'sentinel', 'external', 'portfolio'):
        values = {name: ratios for name, ratios in wall.items()
                  if lane == 'portfolio' or name.startswith(lane + '/')}
        if not values:
            continue
        stats = analyze(values)
        formal_stats[lane] = stats
        if not stats['precise']:
            missing.append(lane + ': formal lane interval imprecise')
        lower, upper = stats['confidence_interval']['lower'], stats['confidence_interval']['upper']
        if lower > policy['lanes'][lane]:
            failed.append(lane + ': formal lane ceiling exceeded')
        elif upper > policy['lanes'][lane]:
            missing.append(lane + ': formal lane interval crosses ceiling')
    verdict['formal_statistics'] = formal_stats
    if held:
        values = rows(held['panels'][0], policy['held_out'])
        for case, ratios in values.items():
            stats = analyze({case: ratios['wall_ns']})
            upper = stats['confidence_interval']['upper']
            if not stats['precise'] or upper > policy['held_out'][case]['wall_band']:
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
              'held_out_panel_sha256': digest(held_path) if held_path else None}
    verdict['label'] = 'reject' if failed else ('incomplete' if missing else 'pass')
    verdict['passed'] = not failed and not missing
    return verdict, missing, failed, hashes
