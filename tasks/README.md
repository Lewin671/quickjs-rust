# Tasks

Which task should I open? Active and blocked tasks are files in this
directory; each opens with a resume block that says what is next. Everything
else is a historical record under `archive/`.

For a new task, copy [TEMPLATE.md](TEMPLATE.md). One slice of a task is one
reviewable unit: verify it with its focused command before and after, and
update the resume block in the commit that lands it. When a task closes, move
it to `archive/` with `git mv` and move its row below.

## Active

| Task | Status | What it is |
| --- | --- | --- |
| [T018](T018-broad-performance.md) broad performance | active | The performance campaign contract and its targets against QuickJS-NG. Start performance work here, then follow [the workflow](../docs/performance-workflow.md). |
| [T022](T022-performance-priority-controller.md) priority controller | active | The rules that select and stop performance units. |
| [T033](T033-wide-tier-interpreter-exits.md) wide-tier interpreter exits | active | Code is present; the frozen unit was formally rejected; candidates for the next unit are listed there. |
| [T017](T017-performance-benchmark-system.md) benchmark system | blocked | The measurement platform; the remaining milestones need fixed hardware. |
| [T013](T013-temporal-campaign.md) Temporal | blocked | Not started. Temporal is outside ES2025 and the reference engine's Test262 configuration skips it. |

Conformance has no open campaign: the last recorded full scan passes every
configured case (see [conformance records](../docs/conformance/README.md)).
Take conformance work from `./scripts/find-qjsng-gaps.sh`.

Performance entries record scope and outcomes, not priority. Priority comes
from a freshly generated opportunity queue. Reopening a closed proposal needs
a new profile and a newly frozen plan; a rejected implementation does not ban
its whole mechanism family (T022 states the rule).

## Closed

Each file is under `archive/` and opens with its own resume block. The
outcome column says what happened, not what to do next.

| Task | Outcome |
| --- | --- |
| `T001-lexer-coverage.md` | Bootstrap work item. |
| `T002-parser-expressions.md` | Bootstrap work item. |
| `T003-runtime-values.md` | Bootstrap work item. |
| `T004-quickjs-comparison.md` | Bootstrap work item. |
| `T005-test262-subset.md` | Bootstrap work item. |
| `T006-class-campaign.md` | Landed. |
| `T007-async-foundation-campaign.md` | Landed. |
| `T008-destructuring-completion-campaign.md` | Landed. |
| `T009-typedarray-buffers-campaign.md` | Landed. |
| `T010-generators-iteration-campaign.md` | Landed. |
| `T011-call-performance.md` | Superseded by T016. |
| `T012-modules-campaign.md` | Landed. |
| `T014-var-closure-binding-staleness.md` | Superseded by T016. |
| `T015-explicit-resource-management-campaign.md` | Landed. |
| `T016-environment-model-rewrite.md` | Landed. Slot-indexed locals and shared upvalue cells; the protected binding model. Invariants: [env-model-rewrite](../docs/design/env-model-rewrite.md). |
| `T019-object-layout-rewrite.md` | Boxing the cold dynamic property payload and the 32-byte `Property` landed; bit packing and the weak-count slices were rejected. |
| `T020-realm-binding-cell-unification.md` | Landed. One name-to-cell map per realm. |
| `T021-single-vm-frame-stack.md` | The compact register tier landed; the single-VM frame stack and virtual stack were reverted. Log: `T021-single-vm-frame-stack-log.md`. |
| `T023-realm-object-arena.md` | Rejected and reverted. A realm-local object arena regressed its frozen gate. |
| `T024-general-register-core.md` | Rejected. The dispatch-preamble split is kept; it moved the external corpus by about one percent, which falsified dispatch overhead as the main cost. |
| `T025-allocation-free-regexp-backtracking.md` | Stopped at stage 3 when a control crossed its regression budget; the earlier stages are present. |
| `T026-compact-hot-op-error-abi.md` | Rejected and reverted. |
| `T027-frame-verified-direct-local-opcodes.md` | Rejected and reverted. |
| `T028-discarded-binary-branch-superinstruction.md` | Rejected and reverted. |
| `T029-compilation-graph-static-property-names.md` | Rejected and reverted. |
| `T030-packed-typed-loop-scalars.md` | Rejected and reverted. |
| `T031-realm-code-unit-strings.md` | Retained, then removed when String wrapper index properties became lazy. |
| `T032-shaped-prototype-slot-reads.md` | Retained. |

## Other Records In `archive/`

- Experiment logs split out of task files: `T017-performance-benchmark-system-log.md`,
  `T018-broad-performance-log.md`, `T033-screen-log.md`.
- Plans and measurements moved out of current documents:
  `design-env-model-migration.md`, `design-generator-suspension-plan.md`,
  `design-object-layout-plan.md`, `design-realm-object-arena-plan.md`,
  `benchmarking-history.md`, `performance-observations.md`,
  `source-comment-measurements.md`, `harness-convergence-risks.md`,
  `task-index-2026-10-09.md`.
- Frozen unit plans and their outcomes are indexed in
  [performance-units/README.md](performance-units/README.md).

Search before repeating an experiment:
`grep -rn -i "<mechanism>" tasks/`. Records describe the revisions they name
and never establish current priority.
