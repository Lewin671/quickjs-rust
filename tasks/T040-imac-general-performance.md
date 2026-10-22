# T040: General runtime optimization on the x86_64 measurement host

- Status: active.
- Implementation: no new runtime optimization retained in the 2026-10-16
  campaign. T049/T053 cache candidates and T054 build-region prototype are
  rejected; original runtime and executable 926ee634 restored. T052 tooling
  maintenance is integrated into main 153c1fe5, with green remote pipelines.
- Verified at: main 153c1fe5, fast/full gates (469 tool tests, 5170 Test262),
  post-integration full gate 20261016T143237Z; CI 38917460092, Test262 Coverage
  38918285393 and Performance Preview 38917460111 all pass.
- Evidence: [T053](archive/T053-ordinary-own-slot-hot-read-fat-lto.md) records
  the fresh 76x30 baseline, exact profiles, rejected cache candidate and
  restoration; [T054](archive/T054-execution-layout-regions.md) records the
  independent frozen build experiment and complete failed neutrality screen.
- Unresolved: no retained runtime gain; current binary's property-read and
  Value/call costs remain opportunities, while page-residue equality alone
  did not establish neutral layout maintenance.
- Next action: stop after this round at the user's request; no further unit
  or measurement is queued. On an explicit future resume, use a fresh
  current-binary comparison and exact profiles, then freeze the next plan
  before implementation. Closed budgets stay exhausted; T045 is not promoted.

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

## Current measured standing (2026-10-22, after T066)

Formal job `20261022T182921Z-t066-formal60` (60 blocks, workers-4): candidate `9420a6a5` (T066 allocator; runtime as main after the
T066 merge), base `9eba7a1d`, NG `f7830186`. Candidate/base: development 0.898, JetStream subset 0.936, Kraken 0.917, SunSpider 0.927,
broad 0.986, sentinel 1.028. Candidate/NG wall geomeans:

| Lane | Rust / NG |
| --- | ---: |
| Broad | 0.977 |
| Sentinel | 1.071 |
| Development (10 npm libraries) | 2.751 |
| JetStream subset / Kraken / SunSpider | 1.092 / 0.719 / 1.007 |

Worst development cases against NG: js-yaml 3.84, immutable 3.40, moment 3.29, meriyah 3.04, diff 2.93. See
[T066](archive/T066-size-class-allocator.md) for the unit decision.

## Current measured standing (2026-10-21)

Job `20261022T041140Z-current-main-9eba7a1d-formal` (exit 0, 2665 s, workers-4, 30 blocks): candidate main
`9eba7a1d` (binary `f5822aa3…`), base `9a0eeeca` (`926ee634…`), NG `f7830186` (`919f7235…`).
Summary SHA-256 `205e8829…`; queue `target/comparison/current-main-9eba7a1d-queue.json` SHA-256 `f6666a68…`
(workspace-local, like the bundle under `outputs/imac-bench/<job>/`). Readiness inconclusive only because base
`9a0eeeca` has no lodash comparison; every other case is complete and healthy.

| Lane | Cases | Rust / NG wall geomean |
| --- | ---: | ---: |
| Broad | 25 | 0.990 |
| Sentinel | 6 | 1.045 |
| Development (npm libraries) | 10 | 3.071 |
| JetStream subset / Kraken / SunSpider | 5 / 14 / 26 | 1.163 / 0.780 / 1.084 |

Worst against NG: development js-yaml 4.34, immutable 4.01, moment 3.65, diff 3.62, meriyah 3.34, lodash 2.76;
broad property_read 5.15, property_write 3.78, array_read 3.26, dynamic_method_call 3.09. External/sentinel
standing matches 2026-10-16 within noise. Ordinary-path costs (named access, env/call, value glue, allocation and
the interpreter loop itself) dominate real programs; see [T056 part 2](archive/T056-unit0-part2-evidence.md#sampled-shares-canonical-image-main-thread).

## Current measured standing (2026-10-16)

Job 20261016T122024Z-general-current-baseline measured base/candidate 9a0eeeca
under the canonical fat/16/abort recipe, both executable 926ee634. Main
153c1fe5 has identical runtime/build inputs and the natural-order replay was
whole-executable byte-identical. All 76 cases x 30 blocks, four workers, are
valid and ready_for_unit_gates, with no imprecise case or host disturbance.
NG source f7830186 and binary 919f7235 are unchanged.

| Lane or selected external suite | Rust / NG wall-time geometric mean |
| --- | ---: |
| Broad | 0.979341 |
| Sentinel | 1.040126 |
| JetStream JavaScript subset (5 programs) | 1.159785 |
| Kraken (13 programs) | 0.776244 |
| SunSpider (26 programs) | 1.089004 |

These are same-host selected-program diagnostics, not official suite scores
or completion of the fixed-hardware campaign targets. Raw sealed artifacts
are workspace-local; the named revision and observed ratios are recorded here.
The historical sections below remain evidence only for their named revisions.

## Starting point

Starting revision: `06f0ecd3cb4ab7bea384d3b58700e3d958c0aeb0`. Batch 1 is already
an ancestor of this main revision; no batch-1 runtime code awaits integration.
Its historical evidence remains bound to `af4d0d3c`, not to this revision.

The prior x86_64 bundle measured `bf553ad6`, not current main. It is useful for
choosing diagnostic workloads but does not establish current priority. This
task's new 30-block comparison uses that revision as base, `06f0ecd3` as
candidate, and the qualified four-worker schedule. All decision evidence uses
the exact candidate executable and same-host schedule.

## Historical 2026-10-13 standing and infrastructure

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

## Current standing after correctness integration

Source `27ca5140`, exact binary
`b97911efb7bfc31df8bd672922b64888aa16b8c5f2cc049d60f4720e2b94a1c2`.
All 76 cases and 30 blocks are complete, workers four, no DISTURBED or
measurement issues. Summary readiness is inconclusive because captured_write,
property_dynamic_read and string-fasta have imprecise intervals; no rerun is
ordered to obtain a favorable outcome. Candidate/NG wall geomeans are
broad `1.078263`, sentinel `1.084706`, JetStream subset `1.247017`, Kraken
`0.829364`, SunSpider `1.179602`. No overall superiority is established.

Top broad gaps are property_read `6.0097x`, property_write `4.1470x` and
array_read `3.8335x` NG. Largest external gap remains tofte `1.8474x`.
Fresh broad stacks and a supplemental existing-counter trace expose compiled
original typed plans declining against erased virtual aliases; T044 connects
selected code and actual virtual scalar banks. It does not add a duplicate
heap-property hoist or use benchmark shape recognition.

Names cloning in short tofte samples is real but explicitly observed at only
3/110, 3/104 and 3/100; larger env envelopes include necessary work. String ABI
candidates are not supported as large costs there. Those read-only proposals
remain unimplemented and do not establish a new plan or priority override.
