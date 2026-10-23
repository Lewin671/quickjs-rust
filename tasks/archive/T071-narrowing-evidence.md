# T071: Narrowing attempts at 9adf5662

- Status: historical-campaign; pre-final narrowing attempts, superseded by the receiver repair in the active T071 record.
- Implementation: present at `9adf5662`, subsequently changed; the measurement below is not final-source evidence.
- Verified at: engine `9adf5662` fast/full; fixture revision `da9b1d42` full, 2026-10-23; 2,402 runtime tests, 117 focused and 5,170 curated Test262 cases.
- Evidence: gates, probes and immutable triage table below; [original work](T071-lexical-context-gaps.md), [prior review](T071-review-verification.md).
- Unresolved: this revision has a new receiver-export regression; triage cannot establish performance of its later repair.
- Next action: follow [active T071](../T071-lexical-context-review.md); no push or merge.

## Lead decision, 2026-10-24

The eval-only outer-capture experiment introduced live-map, const/TDZ and module
receiver regressions. Lead stops incremental eval scope repairs. Remove that
experiment rather than patch it; [T074](../T074-direct-eval-scope-model.md) requires
a reviewed architecture before any direct-eval model implementation.

Restored baseline compiler capture selection, eval declaration handling,
call/frame binding maps, mutation metadata and tests of unrelated existing gaps.
Removed dynamic-name capture expansion, `fork_cells` call-entry use,
outer-binding provenance, owns-eval-var paths and declaration-shadowing helpers.
Removed tests/fixtures whose success required original bug 4 to be fixed.

Keep deferred keys lexical for this/arguments/new.target/home/super, with the
class private environment and shared derived-this cell. Only called frames may
materialize receiver slots; module/eval/global code must use its existing
context. Only ordinary arguments owners materialize that binding. Arguments
syntax demand planning and explicit forwarding of that call-context cell are
retained for keys reached through eval; neither captures arbitrary eval names.
At this revision the try cleanup ownership exemption was retained for implicit call slots.
Removing it reproduces the derived-this failure after a caught TDZ read: the
constructor throws before returning even after super. The focused retained
test passes with the exemption (negative control logged during this narrowing).
No ordinary call/frame executor changes remain against the recorded main base.

P1-a liveness and nested rebinding have exact regression tests, including var,
function and generator variants. P1-b guards outer const/TDZ cell integrity;
main's missing TypeError is explicitly T074, not encoded as correct semantics.
P1-c covers module keys and arrows plus global receiver identity. The earlier
key eval test now uses an explicit outer name in its field initializer: its
eval-only field capture belonged to bug 4 and is deferred to T074.

## Narrowed verification

- Fast gate exit 0: format/clippy, runtime tests, file/context guards and 117
  selected Test262 cases. Full gate exit 0: workspace/features tests and
  clippy, 732 benchmark Python tests, 35 script tests, hooks/shell/context/size
  checks and all 5,170 curated Test262 cases. Engine revision `9adf5662`.
- Scope validator against review handoff `66e500a7` passes. Call/env/global
  eval and frame binding/initialization sources are byte-identical to main
  `6e899d24`; compiler capture selection also matches main (test added only).
- Original `outer(8)()` independently still throws ReferenceError; no new
  test expects this wrong result. Exact reviewer const example still returns
  3, the main behavior; missing TypeError remains T074.
- Negative control without the try-cleanup exemption fails the derived-this
  case; the retained source passes it and the source-arguments cleanup case.
- Baseline queue `20261023T162158Z-t071-narrow-before`, ref `6e899d24`:
  exit 0 in 356 seconds. All five complete filtered `--all` comparisons pass:

| Area (`test/` prefix) | Configured / Rust pass before | NG pass before |
| --- | ---: | ---: |
| language/expressions/class | 4,049 | 4,049 |
| language/statements/class | 4,351 | 4,349 |
| built-ins/eval | 10 | 10 |
| language/eval-code | 347 | 347 |
| annexB/language/eval-code | 469 | 469 |

Total 9,226 configured cases, 26 feature skips. Rust failures, timeouts,
not-run and NG-pass/Rust-nonpass are zero in every area. NG's two statement
class failures are the same baseline grammar cases from prior T071 records.
These are filtered areas, not a complete-suite burndown; no burndown appended.

Final reference queue `20261023T162944Z-t071-narrow-reference` and sole
narrowed triage `20261023T162945Z-t071-narrow-triage` pin `9adf5662`.
Triage has the original five development targets and fixed controls, versus
`6e899d24`, no acceptance chain. Its queued command waits outside local
02:45-05:00 with a 30-minute start margin.

Temporary local log SHA-256 (results above remain here after cleanup):
- Fast: `a820557f140d3c6352b202ba7cfa9eaccd58cb3292b949fcddac75209ee092da`.
- Full: `b6e2c0cd2b781d9112706eb771e175dcc9c9c72cc5c97fee5df285e0f7c068c4`.
- Negative control: `574d832c1991289a1a04d1c57564ff4a09855e9785068610426ca8d52098b7ae`.

Baseline job log SHA-256:
`d08c080c7945053e92ce85d831b0cb13550b041f3d5f415f37bc69deba1a488c`.
Baseline summary set SHA-256:
`48dffc4960a3ec1c707f192d38e167262ff37aa770475d2bdac7442bef1f1892`.
Initial post-gate cleanup removed 7,327 files / 4.5 GiB and restored the
tracked target fixture. Fixture diagnosis later rebuilt the local CLI; final
cleanup is still required. No origin push or merge.

The first final reference job exited 1 before module/area scans: the new scope
fixture used a line comment that swallowed the remainder when the harness
flattened it, and the arguments fixture nested a global arrow around its
otherwise independent contexts. That nesting exercises the unresolved
arguments-absence model constraint now in T074. The fixture is corrected to
independent expression contexts, and the comment becomes a block comment.
Fixture correction alone did not change `9adf5662` engine source. A later
module-context probe then required an engine repair; this triage predates it.

## Pre-final engine quick triage

Job `20261023T162945Z-t071-narrow-triage`: base `6e899d24`, candidate
`9adf5662`, exit 0, verdict **unclear**. All 105/105 pairs valid; zero invalid
or heavy samples, budget met, zero plan attempts consumed. No obvious
regressions. Possible wall regressions: capturing_closure_call,
heterogeneous_property_read, polymorphic_call_site and prototype_method_call.
Ordinary instruction growth from the rejected review experiment is largely
absent: binary-trees +0.13%, cdjs +0.21%, raytrace +0.08%; these are near-flat
measurements, not exact codegen neutrality or formal performance acceptance.
No second narrowed triage or acceptance chain. Fixture/docs-only `da9b1d42`
has identical engine/build inputs to the measured revision.

Measured command: iMac local 2026-10-24 00:54:50-00:59:13 CST, outside the
02:45-05:00 exclusion. End-to-end 334.42 s including 70.73 s queue quieting;
command 263.69 s, base build 0.88 s, candidate build 71.22 s, collection
179.00 s. Five natural-layout pairs; 95% intervals below are exploratory,
unadjusted and do not establish layout robustness or complete-portfolio safety.
Ratios are candidate/base; below 1 is faster/lower work.

| case | wall ratio [95% exploratory interval] | instructions [95% exploratory interval] |
| --- | ---: | ---: |
| development/lodash/transform (target) | 0.9887 [0.9417, 1.0603] | 0.9994 [0.9991, 0.9998] |
| development/immutable/persistent (target) | 0.9862 [0.9580, 1.0071] | 1.0004 [1.0003, 1.0005] |
| development/diff/compare (target) | 0.9929 [0.9320, 1.0454] | 0.9969 [0.9969, 0.9970] |
| development/graphlib/paths (target) | 1.0068 [0.9918, 1.0311] | 0.9990 [0.9989, 0.9992] |
| development/moment/dates (target) | 1.0181 [0.9902, 1.0293] | 0.9999 [0.9998, 0.9999] |
| sentinel/capturing_closure_call | 0.9815 [0.8206, 1.0596] | 1.0000 [0.9999, 1.0001] |
| sentinel/heterogeneous_property_read | 1.0368 [0.9882, 1.0508] | 1.0000 [0.9998, 1.0001] |
| sentinel/polymorphic_call_site | 1.0337 [0.9183, 1.0732] | 1.0001 [1.0000, 1.0001] |
| sentinel/prototype_method_call | 1.0509 [0.8903, 1.2317] | 1.0009 [1.0007, 1.0011] |
| sentinel/recursive_call_tree | 0.9703 [0.9550, 0.9876] | 1.0000 [0.9998, 1.0001] |
| sentinel/string_key_map_churn | 1.0159 [0.9787, 1.0431] | 1.0000 [0.9999, 1.0002] |
| broad/property_dynamic_read | 0.9961 [0.9739, 1.0588] | 1.0000 [1.0000, 1.0002] |
| broad/object_allocation | 1.0048 [0.9735, 1.0111] | 1.0000 [0.9998, 1.0001] |
| broad/string_slice | 0.9736 [0.9458, 0.9876] | 1.0000 [0.9999, 1.0001] |
| broad/top_level_function_call | 1.0091 [0.9871, 1.0246] | 1.0000 [0.9999, 1.0001] |
| external/kraken-1.1/json-stringify-tinderbox | 1.0023 [0.9892, 1.0185] | 0.9999 [0.9996, 1.0004] |
| external/kraken-1.1/json-parse-financial | 0.9869 [0.9520, 0.9947] | 1.0000 [0.9999, 1.0001] |
| external/jetstream3-js-subset/raytrace-public-class-fields | 1.0185 [0.9913, 1.0306] | 1.0008 [1.0007, 1.0008] |
| external/sunspider-1.0/access-binary-trees | 0.9892 [0.9791, 0.9996] | 1.0013 [1.0011, 1.0014] |
| external/kraken-1.1/audio-dft | 1.0359 [0.9902, 1.0491] | 1.0000 [0.9985, 1.0010] |
| external/jetstream3-js-subset/cdjs | 1.0073 [0.9940, 1.0131] | 1.0021 [1.0017, 1.0025] |

Immutable triage artifact SHA-256:
- Request: `681751ac249c647dad07d93f86cca796e68c3437ee5defd83c6d9956b9e5a765`.
- Result: `4b5706833804d883c8f41abb5c1ab0e6ee1ea3f42e9c6b10a058b72f1d2466ee`.
- Markdown: `c2b5cf99a8a83031956fb9f3835bf709d831f4c5cece329671cb9a83f637663f`.
- Base image: `cbb471c7047917f02e13d0ecf83516d9d6e57c050f2170ddde3b5e6e17ed735e`.
- Candidate image: `8d72a72c019eab2e4cb7f8c3780a64b120e8eede77d8d28112cf0c50870532dd`.

Corrected reference job `20261023T165631Z-t071-narrow-reference-corrected`
pins `da9b1d42`; it includes a diagnostic of the pre-existing nested-global-arrow
arguments gap using the triage's exact base binary, before the NG fixture and
five-area checks. This diagnostic does not assert the wrong behavior as correct.

Corrected fixture revision `da9b1d42` also passes the full gate (exit 0),
including all workspace/features, 2,402 runtime tests, 732 tool tests,
35 script tests and 5,170 Test262 cases. Local flattened-expression checks:
arguments contexts `undefined:undefined:undefined:8:function:changed`;
eval constraints `5:5:5:5:2,2:3:true:function`. No engine edits were needed.
- Corrected full log: `f199f58517ef5fc95f050a4282b4ae5a1f33dfabcb9b1e825fd6b366b70a73f0`.
- Fixture fast log: `10c010c0408319e4172c2abc1b4ac2ac620e26493eb5e13368b5df44567d836c`.
- First failed reference log: `0dbc6e0277ef7d7ddcc718d5a6027f358c0b968d9afa8b25ff9537481e3e583a`.

Corrected job `20261023T165631Z-t071-narrow-reference-corrected` passes all
260 script fixtures and confirms main also throws for nested-global-arrow
arguments absence. It then exits 1 at the module comparison, before area scans:
prior module eval plus a later eval-capable static block overwrites the module
receiver in the inherited dynamic map. Moving keys to module entry temporarily
made the fixture pass, but that was not accepted as a fix. The explicit base
probe below proved this is a new T071 regression, not a legacy T074 gap.

## Receiver probe invalidates the pre-final handoff

Queue `20261023T172729Z-t071-narrow-module-reference` probes module
`eval('1'); class Outer { static { class C { #p; [(#p in {}, eval('1'))]() {} } } } print(typeof this);`
with exact triage binaries: main `6e899d24` prints `undefined`, candidate
`9adf5662` prints `function`, NG prints `undefined`. This is a new regression.
Root cause: key demand manufactured a parent this slot; dynamic map export
then overwrote the module receiver. Existing receiver preparation already
supplies key call context. The final repair removes that artificial slot and
restores the original combined module test order, with a dedicated regression.
The try-cleanup exemption then becomes unnecessary: all 2,403 runtime tests,
including the derived-this/caught-block cases, pass with baseline cleanup.
The earlier negative cleanup control applies only to the old parent-slot design.
