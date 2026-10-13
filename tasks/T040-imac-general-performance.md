# T040: General runtime optimization on the x86_64 measurement host

- Status: active.
- Implementation: T041's candidate is reverted on the lead branch and retained
  on its owner branch for diagnosis; T042 is frozen and being implemented.
  Neither has performance promotion evidence.
- Verified at: `06f0ecd3`, 2026-10-12; full local `scripts/check.sh`, including
  5170 curated Test262 cases, passed. The new comparison is complete and sealed.
- Evidence: measurement job `20261013T032141Z-campaign-current-06f0ecd3`;
  profile job `20261013T033011Z-campaign-profile-top8-receipts`. Raw artifacts
  are workspace-local and not retained in this repository.
- Unresolved: unit screens, formal promotion and campaign performance gaps.
- Next action: finish T042's independent correctness review, verify its layout
  and use its bounded screens. T041's frozen attempt budget is exhausted.

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

## Current standing and execution infrastructure

The 30-block bundle completed successfully with all 25 broad, 6 sentinel and
45 external cases, no imprecise cases or measurement issues, and no host
disturbance. Its summary SHA-256 is
`f70c84595f4794365d0f733c345801e36398faa3239f24556208eb84d1c7bc37`;
all nine sealed artifact hashes match. Base and candidate executable hashes
are identical (`19ddbb62222a9c3b4b8578d65d891bd0893f354efdff28ddc092016bd9b648f7`),
providing a current same-binary comparison as well as the NG standing.

| Lane or external suite | Candidate / NG wall-time geometric mean |
| --- | ---: |
| Broad | 1.077836 |
| Sentinel | 1.083702 |
| JetStream JavaScript subset | 1.251098 |
| Kraken | 0.828511 |
| SunSpider | 1.178491 |

These are complete same-host unit-selection measurements, not a fixed-hardware
qualification or official upstream suite scores. The campaign is not complete.

During the layout-tool owner's commit, the old pre-commit hook inherited Git
variables into nested tool tests, which changed the shared repository's
`core.bare` to true. No commit completed. The lead restored `core.bare=false`
and verified all branch/worktree identities and the main checkout's original
unrelated deletion. Both hooks now isolate repository-local environment and
choose the invoking checkout's scripts. The new regression rejects the old
hook, passes both fixed hooks, and exercises a real linked-worktree commit.
Its full gate passed. The layout symbol portability change (`0fcc58f4`) also
passed its full gate and remote CI, and is integrated on the lead branch.
An exact x86_64 symbol check found 130 admissible std filler functions and
successfully formed gaps from 32 through 2048 bytes. This validates symbol
selection, not old ARM64 offsets or a performance improvement.

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
