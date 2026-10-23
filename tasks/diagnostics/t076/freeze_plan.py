"""Bind the prospectively selected T076 bundle to its fresh queue/profiles.

Usage: PYTHONPATH=. python3 tasks/diagnostics/t076/freeze_plan.py PROFILE_OUT QUEUE
Run only before runtime implementation. The committed JSON is never rewritten.
"""
import hashlib
import json
from pathlib import Path
import re
import sys

from tools.benchmark.layout_symbols import parse_symbol

from collect import BASE, POLICY, UNION


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def fraction(path):
    # Same first-main-thread tree/count conservation as T075. Ordinary symbols
    # can merge PCs here: only a function-family inclusive union is claimed.
    nodes, stack = [], []
    graph = path.read_text().split('Call graph:\n', 1)[1]
    for line in graph.splitlines():
        match = re.match(r'^([ +|:!]*)(\d+) (.+)$', line)
        if not match:
            if nodes:
                break
            continue
        depth, count, text = len(match[1]), int(match[2]), match[3]
        if nodes and depth <= nodes[0]['depth']:
            break
        while stack and nodes[stack[-1]]['depth'] >= depth:
            stack.pop()
        name = text.split('  (in ', 1)[0]
        parsed = parse_symbol(name) if name.startswith('_R') else None
        canonical = parsed.canonical if parsed else name
        hit = any(s in canonical for s in (
            'compact_fn::wide::activation::run', 'typed_loop::execute::run'))
        node = dict(depth=depth, count=count, self=count,
                    selected=hit or bool(stack and nodes[stack[-1]]['selected']))
        if stack:
            nodes[stack[-1]]['self'] -= count
        stack.append(len(nodes))
        nodes.append(node)
    assert nodes and all(n['self'] >= 0 for n in nodes)
    assert sum(n['self'] for n in nodes) == nodes[0]['count']
    selected = sum(n['self'] for n in nodes if n['selected'])
    return selected, nodes[0]['count']


def main():
    root, queue_path = map(Path, sys.argv[1:])
    queue = json.loads(queue_path.read_text())
    assert queue['candidate_sha'] == BASE
    binary_sha = json.loads((root / 'base000/identity.json').read_text())['sha256']
    assert binary_sha == queue['engines']['candidate']['binary_sha256']
    evidence = []
    for case in UNION:
        for repeat in range(3):
            source = Path('profiles') / case.replace('/', '--') / str(repeat) / 'receipt/receipt.json'
            receipt = json.loads((root / source).read_text())
            assert receipt['base_sha'] == BASE and receipt['binary_sha256'] == binary_sha
            sampled, total = fraction(root / source.parent / receipt['profile']['path'])
            evidence.append(dict(source=str(source), sha256=digest(root / source),
                                 base_sha=BASE, opportunity_ids=[case],
                                 inclusive_fraction=sampled / total,
                                 shared_cost=f'Exact-base canonical complete-process 1ms sample, repeat {repeat}: '
                                 f'wide/typed execution inclusive union {sampled}/{total} main-thread samples; '
                                 'recursive descendants counted once. Broad reach includes required operations and '
                                 'callees, not savings. All three repeats bound. Targets were selected before '
                                 'candidate measurement by T075 above-mean transport/indexed/received-index self '
                                 'fractions, not this supplementary sample. T075 retains exact-PC subpopulation evidence.'))
    plan = dict(schema_version=3, unit_kind='leaf', artifact_type='quickjs-performance-unit',
                unit_id='executor-bundle', base_sha=BASE,
                queue=dict(candidate_sha=BASE, sha256=digest(queue_path)),
                priority=dict(mode='queue', opportunity_ids=UNION, rank_ceiling=10, override_reason=None),
                mechanism=dict(
                    summary='T076 bundles T075 proposals 1, 2 and 4: destination-aware immediate publication, '
                    'scalar numeric indices through boxed consumers, immutable received-upvalue indices. '
                    'Per-unit non-timing retired-instruction targets are frozen in tasks/T076-executor-bundle.md '
                    '(above each T075 population mean); target geomean <=0.995 and every control <=1.003, all 86 '
                    'selection cases measured, full correctness gate per unit. A miss is kept on a side branch '
                    'and dropped without wall tuning. Combined seven-development-program triage wall geomean '
                    '<=0.97 is required before a screen. Two K=6 screen attempts, formal 30, held-out 30 K=6, '
                    'RSS 15 alternating pairs, T067 host resources. Following T069, a complete screen with improved '
                    'development/external/portfolio aggregates permits remaining evidence collection despite '
                    'strict local payoff misses; preserve the strict verdict. Incomplete/health evidence cannot '
                    'advance. T022 overall-first: development, external, '
                    'all-case portfolio and held-out are overall gates; lane ceilings, individual microbenchmarks/'
                    'sentinels and RSS are local, reported to the lead. Correctness and zero Test262 gaps are '
                    'never waived. RSS defaults per-case <=1.10 and development geomean <=1.05. No final decide '
                    'or promotion step is authorized.',
                    generality='Admission depends only on primitive Value variants, register data flow with '
                    'exact scalar/boxed recovery, and compiler slot-to-cell metadata under existing runtime '
                    'owner checks. No source/library/benchmark/bytecode-template recognition, Value ABI change, '
                    'new tier, per-call name-keyed snapshot, new dependency or unsafe. T045 identity-join '
                    'candidate is not imported; T070 ownership-taking and T075 hot/cold cache proposal are excluded.',
                    semantic_risks=[
                        'U1 preserves self-copy, immediate bit patterns, owned alias counts and old-value destruction order; '
                        'TDZ markers stay opaque owning Values and checked lexical operations keep their checks.',
                        'U2 uses existing indexed operations only when scalar data flow proves the same key; '
                        'multi-successor joins, source overwrites, zero-trip loops, exact deopt stack/IP and local '
                        'writeback remain correct. Unknown provenance stays boxed; no speculative tier or identity assumptions.',
                        'U3 metadata indexes the same live shared upvalue cells, including writes and TDZ; '
                        'runtime source-owner/layout admission and lexical initialization checks are retained.',
                        'Codegen and layout may perturb unrelated controls. No wall-driven per-unit tuning; '
                        'T073-style bimodality is reported with raw processes instead of rerunning away the tail. '
                        'Timing never starts during iMac local 02:45-05:00; DISTURBED jobs are replaced whole.',
                    ]),
                profile_evidence=evidence,
                fast_gate=dict(target_ids=UNION, target_max_candidate_over_base=0.97, max_attempts=2),
                promotion_gate=dict(require_complete_broad=True, require_complete_external=True,
                                    require_test262_zero_gap=True),
                layout_gate=dict(policy_path=str(POLICY), policy_sha256=digest(POLICY),
                                 mechanism_class='cpu-work', memory_wall_targets=[], panel_required=True,
                                 panel_pairs=5, held_out_pairs=30, formal_blocks=30))
    path = Path('tasks/performance-units/executor-bundle.json')
    with path.open('x') as stream:
        stream.write(json.dumps(plan, indent=2) + '\n')
    print(path, digest(path))


if __name__ == '__main__':
    main()
