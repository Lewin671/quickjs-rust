# Tasks

Active and blocked tasks open with a resume block; historical records live under `archive/`.
For a new task, copy [TEMPLATE.md](TEMPLATE.md). One slice of a task is one
reviewable unit: verify it with its focused command before and after, and
update the resume block in the commit that lands it. When a task closes, move
it to `archive/` with `git mv` and move its row below.

## Active

| Task | Status | What it is |
| --- | --- | --- |
| [T068](T068-post-t066-profile.md) post-T066 profiles | active | All ten development programs profiled on exact f49be95c/NG; owner attribution complete, ranked incremental proposals await selection. |
| [T057](T057-string-code-units.md) string code units | active | Stage 1 and UTF-16 correctness extraction integrated; stage 2 abandoned (0 attempts); linear-search follow-up awaits screening. |
| [T065](T065-interpreter-core-v3.md) interpreter core v3 | active | Ablation complete; rewrite not approved (lead, 2026-10-23); incremental units come from a fresh post-T066 profile. |
| [T056](T056-interpreter-core-v2.md) interpreter core v2 | active | Remaining M2/M3 superseded by T065 pending review; M1 A/B blocked after T064 K abort; old failure/reserve unchanged. |
| [T018](T018-broad-performance.md) broad performance | active | The performance campaign contract and its targets against QuickJS-NG. Start performance work here, then follow [the workflow](../docs/performance-workflow.md). |
| [T040](T040-imac-general-performance.md) x86_64 general optimization | active | Fresh current-main comparison and exact-binary profiles, followed by independent general runtime units. |
| [T045](T045-typed-loop-boxed-demand-provenance.md) boxed demand provenance | active | Ordinary identity-flow joins lose boxed demand; exact profiles and seed diagnosis precede a compiler-only unit. |
| [T022](T022-performance-priority-controller.md) priority controller | active | The rules that select and stop performance units ([staged-migration rationale](archive/T022-staged-migration-rationale.md)). |
| [T038](T038-audio-dft-variance.md) audio-dft variance | active | Top infrastructure item: isolated-case cycle variance and allocation/ASLR diagnosis. |
| [T037](T037-integration-batch-1.md) integration batch 1 | active | Both promotions retained under the lead amendment; implementation is integrated into main; audio placement remains open. |
| [T033](T033-wide-tier-interpreter-exits.md) wide-tier interpreter exits | active | Code is present; the frozen unit was formally rejected; candidates for the next unit are listed there. |
| [T036](T036-mixed-call-execution-design.md) mixed-call migration | active | Pricing/attribution and standalone replay; four-stage migration not approved, leaf units re-ranked. |
| [T017](T017-performance-benchmark-system.md) benchmark system | active | The measurement platform; an x86_64 measurement host is characterized, the fixed-hardware milestones are open. |
| [T013](T013-temporal-campaign.md) Temporal | blocked | Not started. Temporal is outside ES2025 and the reference engine's Test262 configuration skips it. |

The last recorded full scan passes every configured case (see
[conformance records](../docs/conformance/README.md)); [T043 history](archive/T043-derived-constructor-eval-context.md) records
two remaining baseline programs outside that standing. Take broader conformance work from `./scripts/find-qjsng-gaps.sh`.

Performance entries record scope and outcomes, not priority. Priority comes from a freshly generated opportunity queue. Reopening a closed proposal needs a new profile and a newly frozen plan; a rejected implementation does not ban
its whole mechanism family (T022 states the rule).

## Closed

Each file is under `archive/` with a resume block; outcomes record what happened.

| Task | Outcome |
| --- | --- |
| [T070 ownership transfer](archive/T070-value-transport.md) | Closed without promotion: attempt 1 declined; broader triage 0.975245 misses <=0.97; one screen unused; implementation retained on owner branch. |
| `T069-call-activation.md` | Promoted by the lead under overall-first: formal development 0.957, external 0.998, portfolio 0.992, held-out 0.930, zero Test262 gaps; deferred-key receiver regression found in review and repaired. |
| `T067-x86-resource-manifest.md` | Resource lane derives a qualified manifest for the measuring host (iMac x86_64 fat16-abort); base-vs-base qualification size 1.0, fresh 0.990, RSS 1.0. |
| `T066-size-class-allocator.md` | Promoted under the lead decision: CLI size-class allocator, development 0.898× main (NG 3.07×→2.75×), held-out 0.72–0.96; sentinel lane cost accepted by the user; resource gate unsatisfiable on x86 (tooling gap). |
| [T064 kernel isolation](archive/T064-kernel-isolation.md) | Stage K aborted before timing: K1/Repair 1 static and correctness gates pass, same 46/86 instruction cases block; outlining abandoned; [plan closure/ledger](archive/T064-kernel-layout-authority-closure.md), M1 A/B blocked, T056 moves to M2. |
| `T063-string-canonical.md` | Surrogate pairs stored canonically at construction; instruction repair; regression screen passed (lanes 0.998-1.008); integrated ([correctness](archive/T063-correctness-verification.md), [instructions](archive/T063-instruction-diagnosis.md)). |
| `T058-regexp-lazy-quantifiers.md` | Fixed-candidate v6 unit retained: papaparse/csv 0.0806 at formal, 86/86 selection and 9/9 held-out pass, Test262 42,672/42,672; candidate `736200aa`, integration pending. |
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

- T069: [receiver repair](archive/T069-call-context-fix.md), [measurements](archive/T069-final-evidence.md), [first candidate](archive/T069-first-candidate.md). T070: [triage](archive/T070-triage.md), [ownership producers](archive/T070-producer-attribution.md), [first screen](archive/T070-screen-attempt1.md), [broader triage](archive/T070-broader-transport.md). T068: [per-program profiles and receipt manifest](archive/T068-profile-evidence.md). T065: [rounds 1–3 ablation](archive/T065-ablation-r1.md), [gap anatomy and both-engine instruction baseline](archive/T065-gap-anatomy.md), [independent design review](archive/T065-design-review.md); T066: [allocator evidence](archive/T066-allocator-evidence.md), [remote-free test diagnosis](archive/T066-remote-free-test.md).
- T064: [K1/Repair 1 evidence](archive/T064-k1-record.md), [K0 checker](archive/T064-k0-checker.md), [successor inputs](archive/T064-successor-plan-inputs.md), [lead review](proposals/T064-successor-review.md), [old-plan closure](archive/T064-layout-edges-closure.md), [spike](archive/T064-causal-outlining-spike.md), [isolation/cost inputs](archive/T064-kernel-isolation-inputs.md).
- T058: [fixed-candidate v6 promotion](archive/T058-regexp-lazy-quantifiers.md), [attempt 1 incomplete closure](archive/T058-attempt1-incomplete.md), [attempt 2 inconclusive](archive/T058-attempt2-inconclusive.md), [v6 rejection](archive/T058-v6-rejection.md), [instruction diagnosis](archive/T058-instruction-diagnosis.md).
- T057: [linear-search repair](archive/T057-search-linear.md), [stage-1 promotion evidence](archive/T057-stage1-evidence.md), [preliminary development baseline](archive/T057-development-baseline.md) [policy v3 qualification](archive/T057-development-policy.md) and [pre-stage-1 log](archive/T057-log.md).
- T056: [unit 0 part 1 evidence](archive/T056-unit0-part1-evidence.md) (standing, attribution limits, RSS, ABI probes), [part 2](archive/T056-unit0-part2-evidence.md) (mechanism counters, development profiles, first-unit ranking and plan binding) [M1 stage 1](archive/T056-m1-stage1-evidence.md) (atom linking and Value facade), [M1.2 rejection](archive/T056-m1-stage2-evidence.md), [M1.2 diagnosis](archive/T056-m1-stage2-diagnosis.md) (rejected layout shape: mechanisms and ablations) [design inputs](archive/T056-design-inputs.md) [M2 design inputs](archive/T056-m2-design-inputs.md) (development-lane call/frame and compact-driver costs), [M2 plan inputs](archive/T056-m2-plan-inputs.md) (draft targets, reach, budgets and exact-M receipts) and [task history](archive/T056-task-history.md) (owner notes, unit 0 summary, M1 stage 1, design checklist).
- T055: [qualified policy adoption](archive/T055-layout-noise-control.md), [feedback-2 record](archive/T055-feedback-2-qualification.md), [feedback-1 record](archive/T055-feedback-1-qualification.md), [complete A2 data](archive/T055-phase-a2-data.md) and [B frozen plans/blocked qualification log](archive/T055-phase-b-plan-and-log.md).
- T055 phase A: [superseded frozen plan](archive/T055-phase-a-plan.md) and [results](archive/T055-phase-a-log.md); static/correctness pass, invalid retired-sampler timing.
- [T050 complete diagnostic tables](archive/T050-vm-layout-diagnostic-data.md), including the unattributed dynamic-read regression.
- First batch-1 formal decisions: `T037-first-formal-decision.md`.
- Audio DFT one-factor diagnostics: `T038-audio-dft-diagnostic-data.md`; [Gaussian engine tails](archive/T038-gaussian-wall-tail-data.md).
- Executable launch controls: `T017-launch-preflight-diagnostic.md`; resource-lane launch admission: [T017-resource-launch-admission](archive/T017-resource-launch-admission.md); loaded-host test failures: [T017-test-wall-timeouts](archive/T017-test-wall-timeouts.md) and [follow-up](archive/T017-git-lock-and-race-tests.md).
- Design measurement record: [T036-mixed-call-execution-measurements.md](archive/T036-mixed-call-execution-measurements.md).
- Case-level formal standing: `T018-af4d0d3c-case-standing.md`; prior formal standing: `T018-e54e33eb-standing.md`; batch 1 leaf handoffs: `T018-batch-1-leaf-screens.md`.
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

Search before repeating an experiment: `grep -rn -i "<mechanism>" tasks/`. Records describe their named revisions and never establish current priority.
