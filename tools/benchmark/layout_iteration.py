"""One frozen layout-rule attempt: build all variants, screen natural, finish K=6 if needed."""
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


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--plan', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--candidate', default='HEAD')
    args = parser.parse_args()
    unit, unit_hash = load_unit(args.plan)
    gate = unit['layout_gate']
    policy = load_policy(ROOT / gate['policy_path'], gate['policy_sha256'])
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
    evidence = measure_panel(base, candidate, out / 'screen', natural=True)
    def verdict(packets):
        return evaluate(policy, packets[0], unit['fast_gate']['target_ids'],
            unit['fast_gate']['target_max_candidate_over_base'],
            panel=packets if len(packets) == 6 else None,
            instruction_blind=gate['mechanism_class'] == 'instruction-blind',
            memory_targets=gate['memory_wall_targets'], panel_required=gate['panel_required'])
    result = verdict(evidence['panels'])
    mandatory = gate['panel_required'] or gate['mechanism_class'] == 'instruction-blind' or needs_panel(policy)
    if mandatory or (result['panel_required'] and not result['failures']):
        evidence = measure_panel(base, candidate, out / 'screen', complete=True)
        result = verdict(evidence['panels'])
    write_json(out / 'verdict.json', {'unit_sha256': unit_hash, **result})
    print(json.dumps(result, indent=2))
    return 0 if result['passed'] else 3


if __name__ == '__main__':
    raise SystemExit(main())
