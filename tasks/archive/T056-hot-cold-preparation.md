# T056 sparse hot/cold object header and instruction repair

- Status: preparation active; no owner timing and no new acceptance attempt.
- Implementation: header-only8dc59f64 present/pushed; inline summary, atom reuse and host batches52380c2e pushed; shared-key append repair present in working tree.
- Verified at:8dc59f64 full gate/NG255/subset5170; job220159Z done0 canonical count/native/static audit.
  52380 full/NG/subset and canonical86 pass (job221049Z); shared-key follow-up full/qualification pending.
- Evidence: [architecture plan](../diagnostics/t056-hot-cold/implementation-plan.json),
  [instruction plan](../diagnostics/t056-hot-cold/instruction-plan.json), [producers/receipts](../diagnostics/t056-hot-cold/README.md).
- Unresolved: whole-program instruction residuals, shifted relative PCs and wall qualification of new image.
- Next action: exact final image counts/86/native/PC and six-layout image checks; prepare fresh lead chain. PMU stays closed.

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
The static decoder finds equal normalized local numeric bytes, but relative PCs differ3–6B in portions
of selected paths for both frames. Equal normalized bytes alone is not fast-family qualification.
Raw selected-path offsets/branches are in `header-relative-pcs.json` (SHA a57110612020143eb0551b34122efc6eb80616134d6b3fd1c263179e11c69af1).

Native MallocStackLoggingNoCompact, same REPL eval source on all images, every ALLOC including frees;
classes queried with actual host malloc_good_size, not assumed16B rounding:

| Whole unpack | allocations | requested bytes | class bytes | class128 count | class144 count |
| --- | ---: | ---: | ---: | ---: | ---: |
| f58 |821493|52708162|61246096|910|255|
|8f898|834720|56276243|65031360|429|341|
|8dc59|834728|56278979|65033632|531|239|

136B requests fall110→8; only102 allocations cross back. Eight additional cold boxes mean total bytes
rise2736 requested/2272 class bytes, not a footprint win. Empty8f898/8dc59 both8106 events,
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
