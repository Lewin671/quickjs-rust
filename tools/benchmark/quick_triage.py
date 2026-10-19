"""Queue-only, natural-layout five-pair diagnostics, never acceptance evidence.

The prospective request fixes revisions, selection-only inventory and heuristic
before building. Existing qualified workers-4 case ownership and lossless pair
recording are reused; no unit plan, attempt ledger or acceptance artifact is written.
"""
from __future__ import annotations

import argparse
import json
import os
import platform
import subprocess
import time
from pathlib import Path

from . import screen
from .case_schedule import binding, collect, require_same, verify
from .layout_identity import ROOT, command, digest, write_json
from .layout_policy import analyze, load_policy
from .pair_disturbance import Monitor, quieting_preflight, verify_run
from .screen_replay import replay_case
from .triage_build import build

POLICY = 'benchmarks/policies/imac-x86_64-rust1.95-fat16-layout-v5.json'
POLICY_SHA = 'f3e8ef40b8b45601b5b86edaf4b47c2d9d904d0045eb024e837f1c447a52bec4'
PAIRS = 5
# Fixed before the first run. Every sentinel is included. External controls
# cover M1.2's worst construction losses and known audio/layout sensitivity;
# development controls exercise ordinary calls, properties, strings and dates.
CONTROLS = (
    'sentinel/capturing_closure_call', 'sentinel/heterogeneous_property_read',
    'sentinel/polymorphic_call_site', 'sentinel/prototype_method_call',
    'sentinel/recursive_call_tree', 'sentinel/string_key_map_churn',
    'broad/property_dynamic_read', 'broad/object_allocation',
    'broad/string_slice', 'broad/top_level_function_call',
    'development/lodash/transform', 'development/diff/compare', 'development/moment/dates',
    'external/kraken-1.1/json-stringify-tinderbox',
    'external/kraken-1.1/json-parse-financial',
    'external/jetstream3-js-subset/raytrace-public-class-fields',
    'external/sunspider-1.0/access-binary-trees',
    'external/kraken-1.1/audio-dft', 'external/jetstream3-js-subset/cdjs',
)
CAVEAT = ('Diagnostic only: triage cannot promote anything, spend/reset a plan attempt, '
          'waive a required run, select an acceptance run to keep, or change an existing decision. '
          'Five-pair 95% bootstrap intervals are exploratory, unadjusted, and do not cover layout bias '
          'or unmeasured cases. Natural layout only; no formal precision or complete-portfolio claim.')


def inventory(targets, policy):
    if not targets or len(set(targets)) != len(targets):
        raise ValueError('declare unique nonempty --target selection case IDs before timing')
    ids = list(dict.fromkeys([*targets, *CONTROLS]))
    if set(ids) - set(policy['cases']):
        raise ValueError('triage allows only policy selection cases; no held-out/custom targets')
    return ids


def summarize(packets, targets, policy):
    rows = []
    losses, ambiguous = [], []
    for case in packets[0]['cases']:
        name = case['id']
        metrics = {m: analyze({name: case['ratios'][m]}) for m in ('wall_ns', 'instructions')}
        band = policy['cases'][name]['wall_band']
        instruction_limit = max(1.03, policy['cases'][name]['instruction_band'])
        wall = metrics['wall_ns']['confidence_interval']
        instructions = metrics['instructions']['confidence_interval']
        if wall['lower'] > band or instructions['lower'] > instruction_limit:
            losses.append(name)
        if wall['upper'] > band or instructions['upper'] > instruction_limit:
            ambiguous.append(name)
        rows.append({'id': name, 'target': name in targets, 'wall_band': band,
                     'instruction_limit': instruction_limit, **metrics})
    target_rows = [r for r in rows if r['target']]
    win = bool(target_rows) and all(
        r['wall_ns']['confidence_interval']['upper'] < 1 / r['wall_band'] for r in target_rows)
    verdict = 'likely loss' if losses else 'likely win' if win and not ambiguous else 'unclear'
    return {'verdict': verdict, 'cases': rows, 'obvious_regressions': losses,
            'possible_regressions': ambiguous}


def render(result):
    lines = [f"Triage: {result['verdict']} (candidate/base; lower is faster).", CAVEAT, '',
             '| case | wall ratio [95% exploratory interval] | instructions [95% exploratory interval] |',
             '| --- | ---: | ---: |']
    for row in result.get('cases', []):
        cells = []
        for metric in ('wall_ns', 'instructions'):
            value = row[metric]
            ci = value['confidence_interval']
            cells.append(f"{value['ratio']:.4f} [{ci['lower']:.4f}, {ci['upper']:.4f}]")
        lines.append(f"| {row['id']}{' (target)' if row['target'] else ''} | " + ' | '.join(cells) + ' |')
    if result.get('error'):
        lines.append('Incomplete: ' + result['error'])
    lines += ['', 'Stage seconds: ' + json.dumps(result.get('stage_seconds', {}), sort_keys=True),
              'Budget met (includes queue quieting, excludes waiting behind jobs): ' + str(result['budget_met'])]
    return '\n'.join(lines) + '\n'


def run(args):
    command_started = time.time()
    if not os.environ.get('BENCH_OUT'):
        raise ValueError('submit through imac-bench.sh run --timing; BENCH_OUT/load monitor required')
    monitor = Monitor.from_queue()
    status = monitor.path.with_name('status').read_text()
    if not status.startswith('running ') or 'no-timing' in status:
        raise ValueError('active queue timing job required (use --timing)')
    # The dispatcher writes status immediately before quieting. Its mtime
    # includes quieting even when prolonged; queued waiting is separately visible.
    queue_start = monitor.path.with_name('status').stat().st_mtime
    started = time.monotonic()
    policy = load_policy(ROOT / POLICY, POLICY_SHA)
    require_same(policy, {'measurement_schedule': binding(4)})
    ids = inventory(args.target, policy)
    revisions = {r: command(['git', 'rev-parse', '--verify', ref + '^{commit}'])
                 for r, ref in (('base', args.base), ('candidate', args.candidate))}
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=False)
    request = {'schema_version': 1, 'artifact_type': 'quickjs-triage-request',
               'decision_evidence': False, 'attempts_consumed': 0, 'created_at': time.time(),
               'tooling_revision': command(['git', 'rev-parse', 'HEAD']),
               'producer_sha256': digest(Path(__file__)), 'policy_path': POLICY, 'policy_sha256': POLICY_SHA,
               'host': policy['host'], 'host_node': platform.node(), 'revisions': revisions,
               'targets': args.target, 'case_ids': ids, 'pairs': PAIRS, 'layout': 'natural',
               'measurement_schedule': binding(4), 'budget_seconds': 1800,
               'heuristic': 'v1: loss if wall CI lower>B_i or instructions CI lower>max(1.03,I_i); '
                            'win if all targets wall CI upper<1/B_i and no case upper exceeds those loss limits; else unclear',
               'caveat': CAVEAT}
    write_json(output / 'request.json', request)  # Sealed BEFORE builds/results.
    stages = {'queue_quieting': max(0, command_started - queue_start),
              'validate_and_freeze': time.monotonic() - started}
    result = {'schema_version': 1, 'artifact_type': 'quickjs-triage',
              'decision_evidence': False, 'attempts_consumed': 0, 'verdict': 'unclear',
              'request_sha256': digest(output / 'request.json'), 'caveat': CAVEAT}

    def stage(name, action):
        begin = time.monotonic()
        try:
            return action()
        finally:
            stages[name] = time.monotonic() - begin
            print(f'STAGE {name}: {stages[name]:.3f} s', flush=True)

    try:
        # Hold the same checkout lock across builds and timing.
        with screen.measurement_lock('quick-triage'):
            if stage('build_quieting', lambda: screen.wait_for_builds(300)):
                raise ValueError('competing builds remain')
            quieting = quieting_preflight()
            binaries = {}
            builds = {}
            for role, directory in (('base', 'base000'), ('candidate', 'cand000')):
                (output / directory).mkdir()
                binary = output / directory / 'qjs'
                builds[role] = stage('build_' + role, lambda r=role, b=binary: build(revisions[r], b))
                binaries[role] = binary
            result['builds'] = builds
            (output / 'work').mkdir()
            cases = stage('prepare_workloads', lambda: screen.resolve_cases(ids, screen.DEFAULT_CACHE, output / 'work'))
            # resolve_cases must neither omit nor silently expand this inventory.
            if [c.id for c in cases] != ids:
                raise ValueError('resolved triage inventory changed')
            result['workload_sha256'] = {c.id: digest(c.workload) for c in cases}
            def preflight():
                screen.check_load(8, True, 300)
                return screen.launch_preflight(list(binaries.values()), 120)
            result['launch_preflight'] = stage('settle_and_launch', preflight)
            tool = screen.counter_tool()
            packets, schedule = stage('collection', lambda: collect(
                cases, [0], {0: binaries}, PAIRS, tool, 120, output, monitor, 4))
            def seal():
                final = monitor.snapshot()
                paths = sorted(output.glob('variant0/*.json'))
                raw = [json.loads(p.read_text()) for p in paths]
                verify(schedule, ids, raw)
                counts = verify_run(raw, final)
                for packet in raw:
                    replay_case(packet, PAIRS, {r: str(b) for r, b in binaries.items()})
                if any(digest(c.workload) != result['workload_sha256'][c.id] for c in cases):
                    raise ValueError('triage workload changed during collection')
                if any(digest(binaries[r]) != builds[r]['sha256'] for r in binaries):
                    raise ValueError('triage executable changed during collection')
                return {'load': final, 'counts': counts, 'quieting': quieting, **schedule,
                        'raw_sha256': {str(p.relative_to(output)): digest(p) for p in paths}}
            result.update(stage('seal_and_replay', seal))
            result.update(stage('analysis', lambda: summarize(packets, args.target, policy)))
    except (ValueError, RuntimeError, OSError, subprocess.SubprocessError) as error:
        result.update(verdict='unclear', error=str(error))
    stages['command_total'] = time.monotonic() - started
    stages['end_to_end'] = time.time() - queue_start
    result.update(stage_seconds=stages, budget_met=stages['end_to_end'] <= 1800)
    if not result['budget_met']:
        result.update(verdict='unclear', error='30-minute budget exceeded; diagnostic incomplete')
    write_json(output / 'triage.json', result)
    report = render(result)
    (output / 'triage.md').write_text(report)
    print(report, flush=True)
    return 2 if result.get('error') else 0


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', required=True, help='committed base ref')
    parser.add_argument('--candidate', required=True, help='committed candidate ref')
    parser.add_argument('--target', action='append', required=True, help='selection case ID; repeatable')
    parser.add_argument('--output', type=Path, required=True, help='fresh immutable run directory')
    args = parser.parse_args()
    try:
        return run(args)
    except (ValueError, OSError, subprocess.SubprocessError) as error:
        parser.exit(2, f'error: {error}\n')


if __name__ == '__main__':
    raise SystemExit(main())
