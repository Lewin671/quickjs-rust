# T040: General runtime optimization on the x86_64 measurement host

- Status: active.
- Implementation: none; selecting a new unit from exact current evidence.
- Verified at: `06f0ecd3`, 2026-10-12; full local `scripts/check.sh`, including
  5170 curated Test262 cases, passed. The new comparison is running.
- Evidence: measurement job `20261013T032141Z-campaign-current-06f0ecd3`;
  profile job `20261013T033011Z-campaign-profile-top8-receipts`. Raw artifacts
  are workspace-local and not retained in this repository.
- Unresolved: current x86_64 standing, exact-binary hotspots, and a frozen plan.
- Next action: verify the comparison seal and host monitor, derive the queue,
  read its exact-binary profiles, then validate and commit a plan before coding.

## Goal and ownership

Continue [T018](T018-broad-performance.md) through general engine mechanisms.
The lead owns integration, documentation and serialized measurements. Coding
owners use separate branches and worktrees; code review and correctness tests
may run locally while the measurement host is busy. The environment/cell model,
benchmark inventory, workload validation and promotion requirements remain the
existing contracts.

The user's 2026-10-12 clarification requires general optimization, without
benchmark-identity or source-template recognition. Workload profiles identify
costs; they do not define admission rules. Multiple independent external
workloads and the complete regression portfolio must check generalization.

## Starting point

Starting revision: `06f0ecd3cb4ab7bea384d3b58700e3d958c0aeb0`. Batch 1 is already
an ancestor of this main revision; no batch-1 runtime code awaits integration.
Its historical evidence remains bound to `af4d0d3c`, not to this revision.

The prior x86_64 bundle measured `bf553ad6`, not current main. It is useful for
choosing diagnostic workloads but does not establish current priority. This
task's new 30-block comparison uses that revision as base, `06f0ecd3` as
candidate, and the qualified four-worker schedule. All decision evidence uses
the exact candidate executable and same-host schedule.

## Mechanisms under investigation

No runtime change is authorized by these hypotheses alone:

- Reuse the existing, unpromoted `95c3af21` OwnSlot hot-answer implementation
  only if new profiles justify it. Preserve its guards and add coverage for
  selected-entry replacement, non-selected entry conversion, and observable
  getter/proxy fallback. Old plans, attempt counts and ARM64 results remain
  historical; do not relabel or overwrite them.
- Cache immutable captured-slot analysis per compiled body if current general
  frame construction profiles establish material cost. Cache metadata only;
  each activation still creates its own cells and preserves dynamic eval,
  arguments and module routes. Do not combine this with removing live-cell
  validation unless separately priced and planned.

## Verification

Follow [the performance workflow](../docs/performance-workflow.md). Record every
screen attempt and formal decision here; rejection is evidence to preserve,
not permission to change thresholds or retry until a favorable sample appears.
