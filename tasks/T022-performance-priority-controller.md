# T022: Evidence-bound performance priority controller

- Status: active; standing controller, with schema-3 tooling awaiting host qualification.
- Implementation: present; new layout/held-out gates are fail-closed until
  [T055](T055-layout-noise-control.md) produces a qualified policy.
- Verified at: 1600baa9, 2026-10-16, final iMac full gate and 543 tool tests.
  Legacy retained decisions remain verified only at their recorded revisions.
- Evidence: T055 owns B jobs and host-band status; [T037](T037-integration-batch-1.md)
  owns historical unstable-base evidence and exact retained outputs.
- Unresolved: T055 applies the frozen standing formal A/A continuation;
  final qualification pending. Ruleset check unconfigured.
- Next action: complete T055 automatic qualification;
  require a fully qualified host policy before any schema-3 promotion.

Commands and their interpretation are in
[the performance workflow](../docs/performance-workflow.md); schemas and
protocol mechanics are in `docs/benchmarking.md`. This file states the rules.

## Goal

Prevent an optimization campaign from spending successive commits on a
low-reach leaf path while the largest external bottleneck remains unchanged.
The controller turns the existing candidate/base/QuickJS-NG artifacts into a
current opportunity queue, binds each optimization to a predeclared plan, and
records an explicit retain/reject/inconclusive decision after measurement.

This is process infrastructure, not a new runtime fast path. It does not turn
variable GitHub-hosted preview numbers into fixed-hardware performance claims.

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
5. A plan declaring `"unit_kind": "migration"` (schema 2) is a staged
   architectural program instead of a leaf fast path. Its intermediate stages
   are classified by `decide --mode stage` as `advance`, `abort`, or
   `inconclusive` against a regression budget, never as `retained` or
   `rejected`; the migration itself reaches the ordinary payoff gate only at
   its final stage. Full contract in `docs/benchmarking.md`.

## Evidence hardening (2026-09-05)

- Queue schema 2 binds all three engine identities and optionally ranks the
  generic sentinels. The CLI replays colocated raw evidence before queue or
  decision generation; source revisions, binary hashes and exact inventories
  must agree across lanes.
- `validate-unit --profile-root` reads content-hashed receipts and verifies the
  actual profile/workload assets against the queue candidate executable.
  `check-unit` remains an archival structural check.
- Decisions require at least 30 paired blocks, narrow 95% intervals, and all
  six sentinels. Promotion applies the regression ceiling to every broad and
  external case, not just the hand-selected controls.
- `python3 -m tools.benchmark.compare` produces a complete local bundle and
  lane-separated summary. Detailed commands and interpretation live in
  [the performance workflow](../docs/performance-workflow.md).

## Layout-robust decisions

New units use the host-bound schema-3 rule; adoption requires the qualified
policy produced under [T055](T055-layout-noise-control.md). Older plans and their rejected budgets
are historical and never acquire an extra attempt from this rule change.

- Freeze targets/payoffs, controls, mechanism class, memory wall targets,
  offsets and the complete measurement/attempt budget before implementation.
- A target must beat both its payoff and reciprocal case wall band B_i,
  with every pair improving. A declared full panel uses all six target
  medians as an additional geometric aggregate; natural and panel targets
  both require every pair to improve.
- A control instruction change opens an event only when every pair exceeds
  max(1.01,I_i). A median-only excess is an unresolved instruction alert.
  A consistent excess with median above 1.03 rejects at the hard cap.
  Otherwise instructions alone do not reject: require all six matching
  layouts' wall medians above B_i to establish persistent cost.
- A single-layout wall median above B_i opens a wall event. Six medians
  above B_i reject, even with flat instructions; a complete mixed panel
  resolves the event as nonpersistent. An absent panel stays incomplete.
- Every lane and the equal-case portfolio must fit their own wall ceilings;
  a portfolio gain cannot hide a failed lane. Natural placement and every
  measured panel layout must pass. Formal intervals stay mandatory.
- Instruction-blind changes (Value, memory layout, allocation) predeclare
  memory-sensitive wall targets, always run K=6, retain resource evidence,
  and require longer formal wall evidence. Flat instructions do not waive
  costs. No post-failure target, offset or threshold selection.
- Held-out is mandatory promotion acceptance: every program within its own
  measured band and its lane within its measured ceiling, with formal
  precision. It never selects work. Missing evidence is incomplete.
- Every calibration/formal A/A automatically uses frozen precision allocation
  n=min(240,ceil(base_n*(h/.03)^2)); pilot retained; predicted stage then cap.
  At240 imprecise bands are diagnostic, mandatory K=6, including held-out
  and canonical formal cases. Other payoff/budget/precision gates remain.
- Policy data is versioned/hash-bound to the host and compiler recipe.
  A different host or recipe recalibrates. Measurements use distinct stable
  equal-length binary paths created once. Apply the [load-only recovery contract](../docs/layout-policy-artifacts.md#load-only-pair-recovery);
  cap failures/formal DISTURBED jobs replace whole. Valid failures spend attempts.

The full K=6 panel is six screens total, not six independent attempts.
Correctness, exact engine/profile identity, formal precision and the original
migration/final-payoff requirements remain in force. Schema 1/2 keeps the
previous cycles-first rule for replay; schema 3 uses wall plus instructions.
Commands live in [the workflow](../docs/performance-workflow.md#screen-gate)
and artifact contracts in [benchmarking](../docs/benchmarking.md#layout-policy-and-panel-artifacts).

## Unstable frozen-base cases: lead amendment, 2026-10-12

This historical waiver applies to schema-1/2 decisions; schema 3 retains its
full wall-precision gates. The lead made this rule change on **2026-10-12**.
A frozen base can itself be imprecise; requiring every candidate to repair its
ratio would hold unrelated decisions hostage. A case can be unstable_on_base
only with same-protocol base/base A/A evidence at least as long as the bundle,
an A/A interval wider than the precision limit, and candidate/base instructions
inside that case's measured A/A instruction floor. Evidence is hashed into the
decision. Such a control uses the instruction gate; instruction growth fails.
It is excluded from timing precision readiness, never counted as a passed
target; a declared target remains inconclusive. Every other gate is unchanged.
The lead authorizes re-deciding the sealed batch-1 bundle with this evidence
before applying the split/re-measure rule. Mechanics and floor definition:
[benchmarking](../docs/benchmarking.md#unstable-frozen-base-evidence).

## Why staged migrations exist

The one-attempt leaf rule is correct for a recognizer and fatal for an
architecture. Every structural attempt this campaign made was closed after a
single implementation: the contiguous direct-call frame stack (correct, 1,920
tests passing, regressed two targets), the realm object arena (1.016x/1.088x),
transition-shape object storage, the compact generic bytecode core, and
default data-property storage (two of three targets improved 3%, the third
regressed 2.2%). Each rejection recorded "do not retry this mechanism", so the
only changes able to close a 3-6x generic-path gap were made unavailable one
at a time.

A migration's early stages move real execution onto a new representation
before any of it is faster. Requiring each to pay for itself is requiring a
bridge to carry traffic after the first pier. The stage gate keeps every final
standard — complete broad and external evidence, zero Test262 gap, a real
improvement — and changes only how the work is allowed to reach that gate. An
`abort` closes one implementation shape; it does not close "frames", "shapes",
or "arenas".

Raw timing artifacts remain outside Git. The queue and decision bind their
SHA-256 values so a small reviewable plan cannot be detached from its evidence.
For a migration whose plan freezes an older base, dispatch the hosted preview
from current `main` with that ancestor as `base_sha`; immediate-parent push
previews are useful incremental diagnostics but cannot satisfy the controller's
base-SHA binding.

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
- [x] A staged architectural migration can reach that gate: its stages are
  judged against a bounded regression budget, measured cumulatively against one
  migration base, and an aborted stage closes an implementation rather than a
  mechanism family.
- [x] Promotion evidence that a single shared development host can produce:
  cycle-based decisions with a wall-time divergence guard.
- [ ] Add a repository ruleset-required `Performance decision` check once
  fixed-hardware or approved same-host promotion infrastructure is available.

## Verification

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s tools/benchmark/tests -v
./scripts/performance-decision.sh --help
./scripts/check.sh
```

## Operating rule

The next runtime optimization starts by generating a queue from the exact
parent revision's preview artifact and profiling the top external
opportunities. The queue is a generated artifact: `performance-decision.sh
queue --output <path>` writes it under `target/`, and it is not stored in the
repository. Generate it for the revision being worked on; `validate-unit`
refuses a stale one. The work may proceed only after its plan passes
`validate-unit`. Each implementation attempt is first checked with the
layout-robust dual-metric screen; only a passing screen spends a formal run, and
screened units planned from the same queue may share one batched promotion
run (gates and batching rules in
[the performance workflow](../docs/performance-workflow.md#screen-gate)).
Exhausting the plan's attempt budget (`max_attempts`, at most two failed
screens) closes that mechanism and requires a new profile; it does not
justify another variation of the same leaf specialization.

A neutrality control must execute the path it is guarding. Broad portfolio
cases do not: at 100,000 nominal iterations `plain_function_call` performs five
real calls and `property_read` eleven real property operations, because the
loop is folded whole (measured before the loop-template plans were removed on
2026-10-09; re-check each case with counters before relying on it). Use
`benchmarks/generic-sentinels-manifest.json` and the `perf-counters` build to
control generic-path work.
