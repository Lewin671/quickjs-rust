# T040: General runtime optimization on the x86_64 measurement host

- Status: active.
- Implementation: T041/T042/T044 runtime candidates are rejected and restored
  to main runtime on the lead; measured sources remain for diagnosis. Generic
  layout-scan and preflight tools and semantic fixtures remain; no candidate
  performance code is promoted. T044 removal full gate and UTC NG247 pass at
  5c9e3ac0; runtime/layout match 27ca5140 exactly.
- Verified at: `27ca5140`, 2026-10-13; integrated T043 correctness, full local
  gate (2267 runtime, 5170 Test262, 238 NG), remote CI and x86 layout/regression.
  Main is updated. CI exact full coverage at that commit passes all 42672
  configured cases, with zero failures/timeouts/not-run or NG-pass gaps. A new
  complete comparison and exact-binary profiles exist.
- Evidence: comparison `20261013T071551Z-campaign-derived-this-27ca5140`, fresh
  queue/top-eight profiles `20261013T073337Z-campaign-fresh-top8-profiles`, and
  valid broad receipts `20261013T092227Z-campaign-broad3-valid-receipts`.
  Raw artifacts are workspace-local, not retained; current standing below.
- Unresolved: new performance units and campaign gaps. Three discovery cases
  have imprecise intervals, so this comparison is not promotion evidence.
- Next action: collect the frozen first screen of
  [T045](T045-typed-loop-boxed-demand-provenance.md). Its compiler-only candidate
  is independently reviewed and correctness-verified on an isolated branch;
  it is not promoted or integrated into the lead runtime.
  All three closed attempt budgets remain exhausted;
  [T044 history](archive/T044-selected-stream-virtual-bank-typed-loops.md)
  preserves its target gains, control failures and ABI correction.

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
