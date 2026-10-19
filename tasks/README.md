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
| [T058](T058-regexp-lazy-quantifiers.md) RegExp lazy quantifiers | active | Promotion pending: formal continuation, held-out K=6 completion and decision queued with lead approval. |
| [T057](T057-string-code-units.md) string code units | active | Stage-1 PROMOTE under v4; integrated into main; stages 2 onward require new plans. |
| [T056](T056-interpreter-core-v2.md) interpreter core v2 | active | Target design for names, frames and values; unit 0 and M2 re-plan complete; M1 stage 1 complete on its branch (v4 screen pass), integration next. |
| [T018](T018-broad-performance.md) broad performance | active | The performance campaign contract and its targets against QuickJS-NG. Start performance work here, then follow [the workflow](../docs/performance-workflow.md). |
| [T040](T040-imac-general-performance.md) x86_64 general optimization | active | Fresh current-main comparison and exact-binary profiles, followed by independent general runtime units. |
| [T045](T045-typed-loop-boxed-demand-provenance.md) boxed demand provenance | active | Ordinary identity-flow joins lose boxed demand; exact profiles and seed diagnosis precede a compiler-only unit. |
| [T022](T022-performance-priority-controller.md) priority controller | active | The rules that select and stop performance units. |
| [T038](T038-audio-dft-variance.md) audio-dft variance | active | Top infrastructure item: isolated-case cycle variance and allocation/ASLR diagnosis. |
| [T037](T037-integration-batch-1.md) integration batch 1 | active | Both promotions retained under the lead amendment; implementation is integrated into main; audio placement remains open. |
| [T033](T033-wide-tier-interpreter-exits.md) wide-tier interpreter exits | active | Code is present; the frozen unit was formally rejected; candidates for the next unit are listed there. |
| [T036](T036-mixed-call-execution-design.md) mixed-call migration | active | Pricing/attribution and standalone replay; four-stage migration not approved, leaf units re-ranked. |
| [T017](T017-performance-benchmark-system.md) benchmark system | active | The measurement platform; an x86_64 measurement host is characterized, the fixed-hardware milestones are open. |
| [T013](T013-temporal-campaign.md) Temporal | blocked | Not started. Temporal is outside ES2025 and the reference engine's Test262 configuration skips it. |

The last recorded full scan passes every configured case (see
[conformance records](../docs/conformance/README.md)); [T043 history](archive/T043-derived-constructor-eval-context.md) records
two remaining baseline programs outside that standing. Take broader conformance work from `./scripts/find-qjsng-gaps.sh`.

Performance entries record scope and outcomes, not priority. Priority comes
from a freshly generated opportunity queue. Reopening a closed proposal needs
a new profile and a newly frozen plan; a rejected implementation does not ban
its whole mechanism family (T022 states the rule).

## Closed

Each file is under `archive/` and opens with its own resume block. The
outcome column says what happened, not what to do next.

| Task | Outcome |
| --- | --- |
| `T062-formal-precision-rule.md` | Prospective v6 two-look exact-median formal gates; v5 measurement/bands retained; historical attempts unchanged. |
| `T061-quick-triage.md` | Queue-only v5 fixed-subset diagnostics; T058 validation took 9.01 min, verdict unclear, no acceptance changes. |
| `T060-parallel-layout-policy-v5.md` | Workers-4 policy v5 qualified (SHA-256 `f3e8ef40...`): case-sharded collection, screens and K=6 panels 2.55x faster than serial. |
| `T059-held-out-layout-policy-v4.md` | Policy v4 qualified (SHA-256 `1eb7de46...`): six-layout held-out bands, held-out and lane events resolved by K=6, candidate formal continuation. |
| `T054-execution-layout-regions.md` | Original page residues all preserved, but one control and sentinel lane failed sole neutrality screen; no production build change retained. |
| `T052-general-performance-command-code.md` | Structural symbol matching/raw audit, canonical scan recipe and byte-identical natural order; full gates pass, no runtime optimization retained. |
| `T053-ordinary-own-slot-hot-read-fat-lto.md` | Target -8.3% but six controls failed; sole budget exhausted, runtime restored and exact original binary rebuilt. |
| `T051-counter-collection-smoke-contract.md` | Collector smoke test checks positive cycles without a nonportable absolute floor; fast/full gates passed on iMac. |
| `T050-vm-layout-causality.md` | Isolated sampled typed-loop placement reproduces the 8-13% control regressions; stale x86 runtime order symbols confirmed; no runtime change retained. |
| `T049-property-access-path-study.md` | Current-recipe 30-block baseline and path-cost study; single-probe candidate failed its only complete-portfolio screen and was reverted; no runtime optimization retained. |
| `T048-preview-cargo-config-compatibility.md` | Fixed preview rejection of the audited previous Rust-flag order; main CI, full Test262 coverage and hosted preview passed. |
| `T047-compiler-recipe-adoption.md` | Adopted fat LTO/16 units/abort for CLI release and performance builds; exact measured binary reproduced; correctness gates passed. |
| `T046-compiler-configuration-study.md` | Completed same-source CLI compiler study; fat LTO/16 units/abort recommended on x86_64 macOS; production defaults unchanged. |
| `T044-selected-stream-virtual-bank-typed-loops.md` | Two failed screens; runtime/layout reverted, generic tools and semantic coverage retained. |
| `T043-derived-constructor-eval-context.md` | Shared derived-this cell and arrow owner verified; two unrelated baseline gaps remain recorded. |
| `T042-prepared-readonly-eval.md` | Two failed screens exhausted the budget; runtime reverted, general measurement fixes retained. |
| `T041-own-data-cache-hot-answers.md` | Frozen x86_64 screen rejected the implementation; runtime reverted, owner commit retained for diagnosis. |
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

- T057: [preliminary development baseline](archive/T057-development-baseline.md) [policy v3 qualification](archive/T057-development-policy.md) and [pre-stage-1 log](archive/T057-log.md).
- T056: [unit 0 part 1 evidence](archive/T056-unit0-part1-evidence.md) (standing, attribution limits, RSS, ABI probes), [part 2](archive/T056-unit0-part2-evidence.md) (mechanism counters, development profiles, first-unit ranking and plan binding) and [M1 stage 1](archive/T056-m1-stage1-evidence.md) (atom linking and Value facade).
- T055: [qualified policy adoption](archive/T055-layout-noise-control.md), [feedback-2 record](archive/T055-feedback-2-qualification.md), [feedback-1 record](archive/T055-feedback-1-qualification.md), [complete A2 data](archive/T055-phase-a2-data.md) and [B frozen plans/blocked qualification log](archive/T055-phase-b-plan-and-log.md).
- T055 phase A: [superseded frozen plan](archive/T055-phase-a-plan.md) and [results](archive/T055-phase-a-log.md); static/correctness pass, invalid retired-sampler timing.
- [T050 complete diagnostic tables](archive/T050-vm-layout-diagnostic-data.md), including the unattributed dynamic-read regression.

- First batch-1 formal decisions: `T037-first-formal-decision.md`.
- Audio DFT one-factor diagnostics: `T038-audio-dft-diagnostic-data.md`; [Gaussian engine tails](archive/T038-gaussian-wall-tail-data.md).
- Executable launch controls: `T017-launch-preflight-diagnostic.md`.

- Design measurement record: [T036-mixed-call-execution-measurements.md](archive/T036-mixed-call-execution-measurements.md).
- Case-level formal standing: `T018-af4d0d3c-case-standing.md`.
- Prior formal standing: `T018-e54e33eb-standing.md`.
- Batch 1 leaf handoffs: `T018-batch-1-leaf-screens.md`.
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
