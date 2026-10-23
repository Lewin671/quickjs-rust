"""Diagnostic-only replay of recorded screens; never changes a unit's standing."""
from __future__ import annotations

import argparse
import json
from pathlib import Path

from .layout_identity import digest
from .layout_policy import load_policy
from .layout_rule import evaluate
from .performance_decision import _atomic_write


def run(args):
    if args.policy:
        if not args.policy_sha256:
            raise ValueError('--policy requires --policy-sha256')
        policy = load_policy(args.policy, args.policy_sha256, live=False)
        policy_sha = args.policy_sha256
    else:
        # An explicit unqualified diagnostic is useful when qualification failed.
        # It cannot be supplied to decide or validate-unit.
        policy = json.loads(args.diagnostic_bands.read_text())
        policy['held_out'] = {}
        policy_sha = digest(args.diagnostic_bands)
    packet = json.loads(args.screen.read_text())
    if packet.get('artifact_type') == 'quickjs-layout-panel':
        from .layout_evidence import replay
        packet = replay(args.screen, policy)
        panels = packet['panels']
        result = evaluate(policy, panels[0], args.target, args.payoff,
                          panel=panels if len(panels) == 6 else None)
    else:
        result = evaluate(policy, packet, args.target, args.payoff)
    payload = {'artifact_type': 'quickjs-layout-diagnostic-replay',
               'claim_eligible': False, 'historical_standing_changed': False,
               'qualified_policy': bool(args.policy), 'policy_sha256': policy_sha,
               'screen_sha256': digest(args.screen), **result}
    _atomic_write(args.output, payload)
    print(json.dumps(payload, sort_keys=True))
    return 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    source = parser.add_mutually_exclusive_group(required=True)
    source.add_argument('--policy', type=Path)
    source.add_argument('--diagnostic-bands', type=Path)
    parser.add_argument('--policy-sha256')
    parser.add_argument('--screen', type=Path, required=True)
    parser.add_argument('--target', action='append', default=[])
    parser.add_argument('--payoff', type=float, default=.97)
    parser.add_argument('--output', type=Path, required=True)
    return run(parser.parse_args())


if __name__ == '__main__':
    raise SystemExit(main())
