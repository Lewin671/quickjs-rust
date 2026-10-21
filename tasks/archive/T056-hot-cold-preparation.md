# T056 sparse hot/cold object header and instruction repair

- Status: owner preparation complete; static fast-family prerequisite fails, timing blocked; no new acceptance attempt.
- Implementation:112B header, hot boolean summary, atom/host batches and shared append text reuse present/pushed at5dd3a810.
- Verified at:8dc59f64 full gate/NG255/subset5170; job220159Z done0 canonical count/native/static audit.
  52380 full/NG/subset and canonical86 pass (job221049Z);5dd3 full/NG255/subset5170 and CI39685975633 pass;222209Z86/native done0, static fast-family fails.
- Evidence: [architecture plan](../diagnostics/t056-hot-cold/implementation-plan.json),
  [instruction plan](../diagnostics/t056-hot-cold/instruction-plan.json), [producers/receipts](../diagnostics/t056-hot-cold/README.md).
- Unresolved: instruction residuals and failed fast-family PCs; no new wall qualification or latency-cause claim.
- Next action: finish non-timing image qualification and gated lead packet. Do not time this image; consider kernel/service isolation under its own plan. PMU stays closed.

## Input standing

Lead feedback binds8f898dd9:211111Z full exact pass;213443Z86/86 pass;213757Z triage unclear;
214401Z mini A/A failed, hence routing inconclusive. Global medians .902/.989/.911/1.026/.897/1.048
satisfy disappearance in5/6 layouts but are not a qualified routing result. Unpack is slow in all6
(1.052/1.043/1.057/1.037/1.034/1.033), triage1.0339[1.0237,1.0433], instructions1.0105.
Class-fields triage1.0434[1.0132,1.0587], instructions1.0095; empty instructions1.0721.
The frozen unit and its last remaining attempt across A+B are unchanged.

## Architecture and instruction work

Move sparse exact own-index counts into the existing cold box, keeping Cell<usize>. Absent cold means zero.
Named-only construction and nonindexed constant class segments allocate none. Central add/remove updates
preserve index order, prototype interception and lazy String materialization. Keep a separate inline boolean
summary for common indexless checks; do not pack it with the flags or narrow the exact count. Both64-bit
headers are112B; Rc requests128 rather than136. Preserve property payload/revisions/prototype/hot brands.

Atom creation reuses an expired weak-table entry's canonical text, always with a fresh id; omit the redundant
remove/hash probe. Sweeping/exhaustion/domain/canonicalization rules stay intact. Complete host global
helper groups, Object prototype methods and Object constructor statics publish final descriptor batches.
GlobalThis identity and IsHTMLDDA boundaries stay explicit; function defaults remain receiver-owned.
Ordinary dynamic mutation still uses transition edges, with no environment snapshot or tier/source recognizer.

## Header-only attribution (not the final repaired candidate)

Job `20261021T220159Z-t056-hot-cold-attribution`, no-timing/done0, compares f58c696e,
8f898dd9 and8dc59f64 (canonical image `c4cdffa245a5301e92ddf6a66ec7aef68670e02e56fff3c0252791a8f0a8f54b`).
Three warmed whole-process retired-instruction medians, same cohort:

| Image | empty | unpack | class-fields |
| --- | ---: | ---: | ---: |
| f58 |13849998|1031120036|7402145139|
|8f898|14850489|1041795853|7476896378|
|8dc59|14868041|1044428930|7454963082|

Header-only does not fix unpack instructions; common cold-pointer reads motivate the inline summary.
The static decoder finds equal normalized local numeric bytes, but relative PCs differ3–12B in portions
of selected paths for both frames. Equal normalized bytes alone is not fast-family qualification.
Raw selected-path offsets/branches are in `header-relative-pcs.json` (SHA a57110612020143eb0551b34122efc6eb80616134d6b3fd1c263179e11c69af1).

Native MallocStackLoggingNoCompact, same REPL eval source on all images, every ALLOC including frees;
classes queried with actual host malloc_good_size, not assumed16B rounding:

| Whole unpack | allocations | requested bytes | class bytes | class128 count | class144 count |
| --- | ---: | ---: | ---: | ---: | ---: |
| f58 |821493|52708162|61246096|910|255|
|8f898|834720|56276243|65031360|429|341|
|8dc59|834728|56278979|65033632|531|239|

136B requests fall110→8; only102 allocations cross back. The36 old400B cold boxes move to408B (class400→416), and8 new408B boxes appear on first-index
creation. Cold class cost rises3904B;102 hot Rc objects save1632B, net+2272B. Requested bytes net+2736B.
[Exact cold-stack accounting](../diagnostics/t056-hot-cold/header-cold-accounting.json) separates unrelated
400B CallEnv requests and binds source stack hashes. This is not a footprint win. Empty8f898/8dc59 both8106 events,
1128731/1128395 requested bytes. Header crossing is not established as the principal unpack wall cause.
Full histograms and raw-history/stack hashes: `header-native-summary.json`
(SHA4ea1dfc7fe38127f11e2b80ce5ef44a52e7b8c8bb418393645e1b0266d1052dd).
These counts do not measure live/peak memory or latency; logging/wrapper perturb execution.

## Ownership follow-up

Native histories retain4868 repeated24B text allocations on unpack append misses: ordinary-object writes
already own Rc keys but erased ownership before atom construction. [Shared-key plan](../diagnostics/t056-hot-cold/shared-key-plan.json)
keeps that ownership on misses through the existing shape/dictionary authority. Existing edge hits still
intern nothing; borrowed host definitions retain the borrowed path, normalization and fresh ids are unchanged.
No shape/header grows. Both paths share transition publication; focused atom/layout/property tests pass.
No counter/timing result from52380 applies automatically to this next source.

## Why the static fast-family requirement fails

The direct encoding difference is four inlined brand comparisons in WideLoopFrame execute. Parent
module-namespace checks use0x82 displacements at offsets0x838/0xf92;112B checks use0x7b and shrink
8→5/7→4 bytes. Symbol-primitive checks use0x83 at0x1ccc/0x1ceb;112B uses0x7c and shrinks7→4
each. Consequently the numeric Add payload store moves0x29ca→0x29be (12 bytes), exactly the
previously undesirable relative position; normalized numeric instruction bytes/counts remain equal.
[Emitted operands/lengths](../diagnostics/t056-hot-cold/repair52380-displacements.json) retain exact
source-image/assembly hashes. This explains code-address drift, not the hardware latency mechanism.

A112B ObjectData plus Rc's16B ownership prefix puts each inline brand byte below displacement128.
Retaining the old inline checks therefore crosses the x86 signed8-bit displacement threshold. The
new sparse count stays usize: count narrowing/flag packing is not necessary for this encoding change.
Do not add padding, instruction patches or choose a favorable layout to restore an address. Separating
typed numeric dispatch from object services would require its own target architecture/migration plan
([prior architecture review](T056-architecture-review.md)); it is not silently added to this header unit.

`fast-family.json` is explicitly false on the new canonical image. Prepared03/mini call a fail-closed
static prerequisite; validators test its rejection, then deliberately mock it only for fake collection
simulation. All frozen numerical AA/routing/full gates and the remaining attempt are preserved.

## Final5dd3 canonical/non-timing result

Job `20261021T222209Z-t056-shared-key-qualification` done0/519s, canonical image
`26ed840a9cbb5bc17c2993ad7fb61380bbeab03b3a5bf39acbe220512da783fc`.
All86 original instruction gates pass; max ratio audio-beat-detection1.018181 (limit1.03).
CI39685975633 is green at5dd3. Full gate/NG255/subset5170 pass locally; fast gate reports no
matching focused Test262 filter for shared-key paths (the full subset supplied coverage).52380's
fast gate additionally ran117 selected global/eval cases. Full exact5dd3 remains pending lead.

Three warmed canonical medians in one cohort (point estimates, no timing/CI claim):

| Image | empty | unpack | class-fields |
| --- | ---: | ---: | ---: |
| f58 |13855950|1031098976|7400552891|
|8f898|14859332|1041825868|7468075338|
|5dd3|14840576|1043898757|7469609443|

Final/base instruction ratios are1.071062/1.012414/1.009331. Versus the120B parent, final ratios
are0.998738/1.001990/1.000205. The requested whole-program residuals **are not eliminated**;
removed source work/event counts cannot be presented as a successful instruction or wall repair.
CLI-only final/base1.005478 is a separate diagnostic, not a startup cost subtraction or a timing gate.
The source/image changes also alter whole-program codegen; the stages are not an additive decomposition.

| Whole unpack | allocations | requested bytes | malloc-good-size class bytes | class128 | class144 |
| --- | ---: | ---: | ---: | ---: | ---: |
| f58 |821493|52708162|61246096|910|255|
|8f898|834720|56276243|65031360|429|341|
|5dd3|829528|56143868|64856112|532|240|

5192 events/132375 requested/175248 class bytes are removed versus8f898, including24B requests
16616→11512 (-5104).136B requests110→8. Net footprint still exceeds f58 by3435706 requested and
3610016 class bytes; the header crossing is only102 events. Empty8033 events/1111284 bytes versus
parent8106/1128731. These are allocation-history totals under the matched REPL wrapper, not peak/live
usage, actual per-block usable-size measurements or latency. The class table queries actual host
malloc_good_size for every observed request, including large sizes. `native-summary.json` preserves
all request/class bins, input/image hashes and full raw-history/stack-group hashes.

Instrumented events (same final source, separate image/extra perf-counters, never timed): startup
398 shapes/6163 copied fields versus earlier441/6981; globals935→458 copied fields, Object391→50.
Unpack927 shapes/39974 copied fields,758425 text comparisons,468 index builds/32894 indexed fields,
6620 weak-table text reuses. Class-fields799041 zero-index constant segments still execute, with
15956322 text-match calls; call counts do not price their pointer/byte-comparison branches. No index
admission policy changed in this follow-up. Arrays/prototypes/index classification/cache invalidation,
atom identity/exhaustion/canonicalization/churn and shared-key pointer ownership have focused tests.

Final emitted displacement audit reproduces the same four3B shortenings and Add29be. The artifact
`displacements.json` binds final/parent assembly/image hashes; `relative-pcs.json` retains all decoded
numeric paths for both frames. Static fast-family failure remains decisive for this packet.

## Sealed lead packet

Non-timing K=6 qualification `20261021T222300Z-t056-hot-cold-k6-qualification` done0/399s:
all12 prescribed images pass NG255 each and five matching-output smokes; uniform declared shifts and
relocation-normalized body/address drift0 pass. Qualification SHA
`993b62f8e7687fd779ea7891b26af8a23d665590371a722ceb5f3192d0b9b241`.
The build derives its natural image hash from the exact86 receipt; [producer proof](../diagnostics/t056-hot-cold/panel/producer-proof.json)
preserves the executed protocol source and verifies it differs only in that resolved hash literal.
No source/placement selection. Five-case mini600 units/1440 calls fits the unchanged30min watchdog.

Fresh01→02→03→mini bodies, executable chain, identity/status/disturbance checks, original numerical
AA/routing rules and timeout/night/DST guards are sealed. Local validator simulates every600 units with
fake processes/counters and explicitly mocked future receipts/static pass; real static rejection is tested
separately. Readiness is false for timing/last attempt. Current source is on stage-A, not main, and is not
promoted. The full gate passes for final runtime and packet; a fresh full exact scan is still lead work.
