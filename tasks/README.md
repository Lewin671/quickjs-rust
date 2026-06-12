# Tasks

Use these as agent-sized work items. Keep each task independently testable.
Concrete task files live next to this index. For new tasks, copy
`tasks/TEMPLATE.md` and fill in scope, owner boundary, acceptance criteria, and
verification commands before assigning an agent.

## Bootstrap Tasks

Early-engine work items; largely landed, kept for reference and residual
follow-ups.

- `T001-lexer-coverage.md` — lexer comments, templates, punctuators.
- `T002-parser-expressions.md` — expression precedence parsing.
- `T003-runtime-values.md` — value types, coercion, environments, errors.
- `T004-quickjs-comparison.md` — QuickJS-NG smoke comparison runner.
- `T005-test262-subset.md` — curated Test262 subset harness.

## Conformance Campaigns

Campaigns decompose the broad features that the quickwins gap strategy
intentionally de-prioritizes. Sizes are QuickJS-NG-passing Test262 cases
measured at commit `3e7feb0` (2026-06-09 full scan); recheck against the
latest `docs/conformance/burndown.jsonl` entry before starting new work.

| Task | Campaign | Unlocks (approx.) | Mostly blocked as |
| --- | --- | ---: | --- |
| `T006-class-campaign.md` | class | 7,138 | not-run (syntax filter) |
| `T007-async-foundation-campaign.md` | async/jobs | 5,567 | not-run (async filter) |
| `T008-destructuring-completion-campaign.md` | destructuring | 2,580 | engine failures |
| `T009-typedarray-buffers-campaign.md` | TypedArray/buffers | 2,480 | engine failures |
| `T010-generators-iteration-campaign.md` | generators/iteration | 1,960 | engine failures |
| `T012-modules-campaign.md` | ES modules | 1,621 | not-run (module filter) + fails |
| `T013-temporal-campaign.md` | Temporal | 4,603 | engine failures (not implemented) |

Campaign working rules:

- One slice from a campaign's checklist is one reviewable unit; verify with
  the slice's focused command before and after, like any gap-queue area.
- Tick the slice checkbox in the task file in the same commit that lands it,
  so the next session resumes without re-deriving state.
- When `find-qjsng-gaps.sh` still offers non-hard quick wins, those stay
  first choice; switch to the highest-priority campaign slice when the
  recommendation queue is dominated by hard-hinted areas.
- Smaller feature clusters not yet campaign-sized (BigInt 830,
  regexp-unicode-property-escapes 581, Proxy 292, explicit-resource-management
  273) stay in the normal gap queue; promote one to a campaign file when its
  slices stop fitting in single reviewable units.

## Performance

- `T011-call-performance.md` — cut per-call environment-cloning cost. The
  leaf-call activation-snapshot clone is landed; the remaining
  shared-realm-globals redesign unblocks ~370 `TypedArray/*` cases that time
  out under heavy nested-call load (e.g. `fill/coerced-indexes.js`).
