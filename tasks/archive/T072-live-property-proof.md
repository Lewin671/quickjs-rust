# T072: Live receiver and slot proof for named properties

- Status: closed without promotion, 2026-10-23; the unchanged second screen also stops on a collection failure, exhausting both attempts under the lead decision.
- Implementation: present; selected hot slot proofs plus shared cold receiver/storage views, descriptor-derived write completion and feature-only outcome counters.
- Verified at: final runtime `6c122407`, 2026-10-23; fast/full gates, 2,395 runtime tests, 257 NG fixtures and complete 42,672-case configured Test262 scan pass; triage passes collection checks.
- Evidence: [frozen plan](../performance-units/live-named-property-proof.json), [first measurements and job ledger](../diagnostics/t072/measurements.md), [lead-authorized attempt 2](../diagnostics/t072/attempt2.md).
- Unresolved: no complete K=6 screen, formal, held-out, RSS or resource standing; collection failure cause, moment and binary-trees regressions remain unresolved. Both screen attempts are charged.
- Next action: lead reviews closure and routes the measurement-infrastructure finding to T017/T038; no further screen, runtime iteration or promotion decision in this unit.

## Scope and prospective selection

Owner `agent/t072-property-proof/codex`, this worktree; base above includes
T066 and the repaired T069. T071 separately owns class/eval/call-context
conformance. No push or merge, dependency, unsafe, parser, binding-model,
benchmark, policy, compiler-recipe or third-party change is authorized.
Allowed runtime paths: named-property caches/handlers, object storage helpers
and their focused tests; reuse `168ce2c0`'s feature-only diagnostic counters.
Task/plan indexes and the subsystem's owning architecture statement also belong
to this unit. iMac collection uses only the serialized workspace wrapper.

Select every development program whose T068 pooled named-cache plus
named-other **self** fraction exceeds the equal-program mean, 8.844%.
This fixes meriyah (955/6745, 14.159%), moment (3434/25986, 13.215%),
diff (1756/14708, 11.939%), js-yaml (1024/11024, 9.289%), and matter-js
(1087/7820, 13.900%) before any candidate. All remaining policy cases are
controls. No held-out program informs design or selection. These shares
identify reach, not removable work or an expected speedup. The cache/handler
family's whole-development deletion envelope is only 8.94%; storage work
cannot simply be added to the expected gain.

## Target architecture and migration stages

1. Freeze exact-base queue/profile hashes and the qualified iMac workers-4
   v5 collection / v6 analysis plan before runtime edits.
2. Introduce a short-lived typed receiver/storage view. It borrows the live
   property table once per named operation, validates a cache's identity,
   revision, shape or interned key, and resolves an actual data slot. A
   consume-once slot proof reads or writes that value; slot/shape metadata
   from the same lookup installs a cache without another text search.
3. Share that view across cache probing, own lookup and named get/set/method
   handlers. Preserve live values, weak identities, layout invalidation,
   descriptor writability and exotic fallbacks. Prototype holders upgrade
   once; an own miss is valid only for the receiver/layout that proved it.
   Release every cache/storage borrow before getters, setters, Proxy traps,
   coercion, constructors or any other user code. Array/Function receiver
   paths keep their observable own-property and prototype precedence.
4. Verify focused mutation/reentrancy tests, existing feature-only outcome
   partitions and full correctness. Run quick triage before any screen.
   Only a target wall-geomean <=0.97 with no correctness issue spends a
   K=6 attempt. After two honest designs that miss this condition, stop and
   record closure. No implementation/layout fishing or favorable rerun.

T049's duplicate-probe deletion missed its target and regressed controls;
T041's hot-slot screen had a non-improving pair; T053 gained 8.3% on its
target but failed six controls. This unit must price shared live validation
and storage lookup rather than repeat those isolated shortcuts. T065's
negative lookup memo was unsound and is not reused.

## Acceptance contract

Frozen plan: [live-named-property-proof](../performance-units/live-named-property-proof.json).
T022 overall-first applies prospectively: development, external, equal-case
portfolio and held-out are overall gates. Lane ceilings, individual
sentinels/microbenchmarks and RSS are local gates and every regression is
reported. RSS defaults: >=15 alternating pairs, per-case median <=1.10,
development geomean <=1.05; use the T067 host-derived resource manifest.
Correctness and zero Test262 gaps are never waived. The lead alone decides
promotion; this owner never runs final decide. A promising candidate proceeds
through screen, formal, held-out and resource/RSS collection with the frozen
plan; an unpromising two-design result is a complete valid outcome.

## Current evidence and frozen candidate

Plan freeze `f2ac7ec3` precedes all runtime edits; validation binds all 15
exact-base receipts. Plan SHA-256
`476cb44858ba0ca74de9ba94a3b200676fb1d4c069cc865c9f47c6a581d381b4`;
queue SHA-256 `c6063619ce1daee74f4d7c48254b2844d3d48d76102e9f0f6e9c2876fe7dbb06`.

First design `80b2eecc`: target wall 1.003921, instructions 0.974620;
no screen spent. The final design `6c122407` adds direct selected hot proofs
and descriptor-derived write receipts: target wall **0.949774**, instructions
**0.913469**, with full correctness checks passing. The target rule and
frozen plan remain unchanged. Moment wall 1.076441 and binary-trees
instructions 1.0447 remain diagnostic losses, not waived or hidden.

The first launch failed preflight on default workers 1 and spends nothing.
The workers-4 attempt builds/audits all twelve images, then stops on a
load-valid 19.998 ms local_read increment below the fixed 50 ms minimum.
At that stop it closes incomplete and consumes one attempt, leaving one for lead review. The
qualified contract makes valid short increments fatal, not replaceable.
No panel, formal, held-out, RSS or resource result is inferred from the
partial collection. No further implementation/design iteration is taken.
Complete per-case intervals, counters, raw hashes and every job are in the
[measurement record](../diagnostics/t072/measurements.md). No origin push, merge
or promotion decision occurs. Final source remains frozen at `6c122407`.

## Lead decision and infrastructure finding

Lead Claude authorizes attempt 2 of 2 on 2026-10-23, retaining exact candidate
`6c122407`, targets, policy and thresholds. Attempt 1 remains charged and its
partial samples are not reused. The authorized launch requires explicit `--workers 4` after a
queue/clock check. No timing job may start if its expected duration overlaps
iMac local 02:45–05:00: allow 55 minutes for screen, 35 for formal and 80 for
held-out. A completed screen proceeds through formal 30, held-out 30 K=6,
RSS 15 and resources, then stops for lead review. Any second screen failure
closes the unit; no further attempt or implementation change is authorized.

Measurement-infrastructure finding for the lead to route to T017/T038:
one slow N process invalidates the whole K=6 screen. In attempt 1's
`broad/local_read` candidate pair 1, N wall time was 501.623129 ms versus
about 265 ms in the other pairs, with 490 ms user CPU; 2N was 521.621365 ms
with 510 ms user CPU. Their valid 19.998236 ms increment fails the fixed
50 ms minimum despite the normal approximately 4.545-billion instruction
increment. The protocol correctly fails the attempt; this is not a T072
runtime change or a candidate-performance conclusion. The cause is unresolved.

## Closure after attempt 2

Fresh screen `20261023T154044Z-t072-screen-attempt2` runs exact `6c122407`
with explicit workers 4 and unchanged plan/targets/policy/thresholds. It exits
1 after 1,137 seconds: shard 3 fails after 68 natural-case collection starts.
Four light background samples remain below the disturbance threshold.
Again `broad/local_read` fails: candidate pair 0 N/2N wall 572.438206 /
522.485450 ms produces a load-valid negative increment, -49.952756 ms.
No complete natural/K=6 panel or qualifying verdict is produced. The
[second measurement record](../diagnostics/t072/attempt2.md) owns raw details,
identities, hashes and the export ledger.

The lead's second-failure condition closes the unit with both attempts charged.
No formal 30, held-out 30 K=6, RSS 15 or resource job is launched. Partial
samples are not pooled or used for lane conclusions. Runtime remains present
for review; closure makes no candidate-performance or promotion decision.

## Correctness and handoff

Full exact scan: 53,572 unique corpus paths; 10,900 NG-config skips;
42,672/42,672 configured Rust passes, zero fail/timeout/not-run/gaps.
The complete record is appended to `docs/conformance/burndown.jsonl`.
The code is retained for review, with no performance promotion claim.
No new dependency, unsafe, binding-model or public crate API change occurs.
The measurement record owns all raw identities, tables and the stop rationale.
