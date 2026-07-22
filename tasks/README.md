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
latest `docs/conformance/burndown.jsonl` entry before starting new work. This
table is historical planning input, not the current pass/fail status.

| Task | Campaign | Unlocks (approx.) | Mostly blocked as |
| --- | --- | ---: | --- |
| `T006-class-campaign.md` | class | 7,138 | not-run (syntax filter) |
| `T007-async-foundation-campaign.md` | async/jobs | 5,567 | not-run (async filter) |
| `T008-destructuring-completion-campaign.md` | destructuring | 2,580 | engine failures |
| `T009-typedarray-buffers-campaign.md` | TypedArray/buffers | 2,480 | engine failures |
| `T010-generators-iteration-campaign.md` | generators/iteration | 1,960 | engine failures |
| `T012-modules-campaign.md` | ES modules | 1,621 | not-run (module filter) + fails |
| `T013-temporal-campaign.md` | Temporal | 4,603 | engine failures (not implemented) |
| `T015-explicit-resource-management-campaign.md` | using/await using | ~113 | parser landed; disposal runtime pending |

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

## Keystone

- `T016-environment-model-rewrite.md` — replace the snapshot + `captured_env` +
  `CaptureWriteback` trio with slot-indexed locals + indexed shared upvalue
  cells (`docs/design/env-model-rewrite.md`). This is the keystone for both
  goals: it removes capture staleness at the root and the per-call locals-map
  clone. **Subsumes T011 and T014** — do not extend the heuristic model;
  land cells instead. Serialize on one branch.

## Engine Correctness

- `T014-var-closure-binding-staleness.md` — **subsumed by T016.** A `var`
  mutated by one function is lost when a sibling reassigned it first
  (snapshot-model desync). The leaf-call fix landed; the full fix is T016 S2+.

## Performance

- `T018-broad-performance.md` — establish the 25-case, eight-family broad
  black-box benchmark, then drive candidate/QuickJS-NG overall wall ns/op to
  at most 0.50x without regressing any critical family above 1.00x or weakening
  correctness. This is the active performance campaign; each runtime change
  remains a separately measured, reviewable unit.
- `T019-object-layout-rewrite.md` — feeds T018 B3/B4. Shrinks `ObjectData`/
  `ArrayData`/`PropertyStorage` layout to close the `allocation` critical
  family's persistent QuickJS-NG gap, as a narrower alternative to a full
  GC/arena rewrite (`docs/design/object-layout-rewrite.md`). S1 (box the cold
  `PropertyStorage::Dynamic` payload) and S4 (`Property` 56B -> 32B with cold
  accessor state) landed; S2/S3 proved bit packing could not reduce aligned
  layouts. Serialize on one branch.
- `T020-realm-binding-cell-unification.md` — **landed** (`bfcd53da`). Feeds
  T018's `call`/global-var families. `RealmState`'s raw
  `HashMap<String, Value>` and its separate `binding_cells` registry are
  unified into one `DynamicBindings` map, so a cell-backed global no longer
  costs two name-table hash lookups per store. Verified with the full test
  suite, Test262 subset, `compare-qjs.sh`, and exact gap scans across
  eval/module/global-code/with/for/Function; zero regressions.
- `T021-single-vm-frame-stack.md` — active structural performance unit. Move
  ordinary synchronous bytecode calls onto one explicit VM frame stack, then
  compact that same execution core into register/superinstructions. This is
  the next T018 unit; do not create a second independent VM or expand the
  direct-leaf eligibility predicate in the frame-stack commit.
- `T017-performance-benchmark-system.md` — versioned candidate/base/QuickJS-NG
  black-box benchmark platform. M0-M4 landed, including independent throughput,
  resource lanes, and diagnostic public-boundary Criterion lifecycle benches;
  M5 governance now records five blocked source-pinned candidates and two
  excluded evidence-backed decisions in a deny-only v1 registry; future
  admission requires a separately reviewed v2 audit bundle before any gate.
  Hosted same-repository PRs now publish strict three-block informational
  previews from a base-owned `pull_request_target` harness, while every `main`
  push uses the after revision as head-owned harness/candidate and the before
  revision as base. Both paths retain complete provenance and phase-aware
  durable failure status; fork previews are unsupported and fail-closed M6/M7 policy infrastructure is ready,
  while fixed-hardware A/A calibration and every performance gate remain
  intentionally incomplete and disabled.
- `T011-call-performance.md` — **subsumed by T016.** Cut per-call
  environment-cloning cost. The leaf-call activation-snapshot clone landed; the
  remaining per-call locals-map clone is deleted by T016 S5, which unblocks the
  `TypedArray/*` cases that time out under heavy nested-call load.
