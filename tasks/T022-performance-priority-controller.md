# T022: Evidence-bound performance priority controller

## Goal

Prevent an optimization campaign from spending successive commits on a
low-reach leaf path while the largest external bottleneck remains unchanged.
The controller turns the existing candidate/base/QuickJS-NG artifacts into a
current opportunity queue, binds each optimization to a predeclared plan, and
records an explicit retain/reject/inconclusive decision after measurement.

This is process infrastructure, not a new runtime fast path. It does not turn
variable GitHub-hosted preview numbers into fixed-hardware performance claims.

## Why it exists

The external portfolio is the anti-overfitting boundary, but task notes and
agent instructions can become stale after a structural migration lands. T016,
for example, completed the slot-local/shared-upvalue migration; treating its
old proposal as the current priority would misroute later work. A current
external ratio identifies a workload to profile, not a code change to make.

## Mechanism

1. `scripts/performance-decision.sh queue` consumes one exact preview bundle
   (`summary.json`, `report.json`, and `external-report.json`) and emits a
   SHA-bound queue of broad and external opportunities above the campaign
   target ratio.
2. A future optimization declares
   `tasks/performance-units/<unit>.json` before implementation. The plan fixes
   its base SHA, queue SHA, target/control cases, profile receipts, general
   mechanism, semantic risks, fast gate, and two-attempt stop budget.
3. `validate-unit` refuses a stale queue, targets not backed by a profile, or a
   queue-mode plan that does not address a top-ranked opportunity. A lower
   priority task requires a written override rather than silently claiming ROI.
4. `decide` evaluates the candidate/base metrics against the frozen targets and
   controls. A complete promotion also requires complete broad/external
   evidence and zero Test262 parity gaps. Valid negative evidence is recorded
   as `rejected`; incomplete/noisy evidence is `inconclusive`, never progress.

Raw timing artifacts remain outside Git. The queue and decision bind their
SHA-256 values so a small reviewable plan cannot be detached from its evidence.

## Scope

- Allowed paths: `tools/benchmark/**`, `scripts/performance-decision.sh`,
  `docs/benchmarking.md`, `tasks/performance-units/**`, and this task.
- Forbidden paths: runtime benchmark workarounds keyed to workload names,
  checksums, paths, or iteration counts; `third_party/**`.
- Owner boundary: serialize changes to benchmark policy/schema and global
  performance documentation on the integration branch.

## Acceptance criteria

- [x] The queue only ranks complete candidate/base/QuickJS-NG external rows
  and broad cases still above the campaign target.
- [x] A unit plan is SHA-bound to its exact evidence queue and base revision.
- [x] A plan cannot choose target cases after measurement, omit current profile
  coverage, or use an unexplained priority override.
- [x] Fast and promotion decisions distinguish retained, rejected, and
  inconclusive evidence, including zero-gap Test262 validation.
- [ ] Add a repository ruleset-required `Performance decision` check once
  fixed-hardware or approved same-host promotion infrastructure is available.

## Verification

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest \
  tools.benchmark.tests.test_performance_decision -v
./scripts/performance-decision.sh --help
./scripts/check.sh
```

## Operating rule

The next runtime optimization starts by generating a queue from the exact
parent revision's preview artifact and profiling the top external opportunities.
It may proceed only after its plan passes `validate-unit`. Two failed fast
screens close that mechanism and require a new profile; they do not justify a
third variation of the same leaf specialization.
