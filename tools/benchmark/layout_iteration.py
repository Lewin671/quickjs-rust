"""One frozen layout-rule attempt: build all variants, screen natural, finish K=6.

Intermediate migration stages use cumulative regression budgets; final stages
and leaf units must earn the frozen target payoff.
"""
from __future__ import annotations

import argparse
import json
from pathlib import Path

from .layout_identity import ROOT, write_json
from .layout_policy import load_policy
from .layout_panel import measure_panel
from .layout_rule import evaluate
from .layout_power import needs_panel
from .layout_variants import build
from .performance_decision import load_unit


def evaluate_attempt(unit, policy, packets):
    """Use cumulative regression gates until a migration reaches its payoff stage."""
    gate = unit['layout_gate']
    migration = unit.get('migration', {})
    stage = (unit.get('unit_kind') == 'migration'
             and migration['current_stage'] < migration['stages'])
    targets = (migration['cumulative_target_ids'] if stage
               else unit['fast_gate']['target_ids'])
    return evaluate(policy, packets[0], targets,
        unit['fast_gate']['target_max_candidate_over_base'],
        panel=packets if len(packets) == 6 else None,
        instruction_blind=gate['mechanism_class'] == 'instruction-blind',
        memory_targets=gate['memory_wall_targets'], panel_required=gate['panel_required'],
        stage=stage, stage_limit=migration.get('stage_max_candidate_over_base', 1.25))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--candidate', default='HEAD')
    args = parser.parse_args()
    unit, unit_hash = load_unit(args.plan)
    gate = unit['layout_gate']
    policy = load_policy(ROOT / gate['policy_path'], gate['policy_sha256'])
    # After v4 publication, frozen v2/v3 plans are screened with v4's held-out/lane rule.
    from .layout_policy import with_held_out_successor
    policy, _ = with_held_out_successor(policy, gate['policy_sha256'])
    # Candidate commits are required; do not silently omit working-tree code.
    from .layout_identity import command
    if command(['git', 'status', '--porcelain', '--untracked-files=no']):
        raise ValueError('layout-rule iteration requires a committed clean candidate')
    out = args.output.resolve()
    out.mkdir(parents=True, exist_ok=False)
    build(unit['base_sha'], out / 'base000')
    build(args.candidate, out / 'cand000')
    base = out / 'base000/identity.json'
    candidate = out / 'cand000/identity.json'
    development = 'development' in policy['lanes']
    evidence = measure_panel(base, candidate, out / 'screen', natural=True, development=development)
    result = evaluate_attempt(unit, policy, evidence['panels'])
    mandatory = gate['panel_required'] or gate['mechanism_class'] == 'instruction-blind' or needs_panel(policy)
    if mandatory or (result['panel_required'] and not result['failures']):
        evidence = measure_panel(base, candidate, out / 'screen', complete=True, development=development)
        result = evaluate_attempt(unit, policy, evidence['panels'])
    write_json(out / 'verdict.json', {'unit_sha256': unit_hash, **result})
    print(json.dumps(result, indent=2))
    return 0 if result['passed'] else 3


if __name__ == '__main__':
    raise SystemExit(main())
