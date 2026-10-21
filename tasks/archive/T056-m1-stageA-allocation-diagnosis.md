# T056 stage A: allocation-class and adapter follow-up

- Status: implemented on the stage-A branch; local and instruction checks pass,
  new-image exact correctness and wall remain lead work. No acceptance attempt.
- Implementation:112-byte object header; absent write probes do not create
  transient shape indexes; object-only typed getter is separated from arrays.
- Verified at: dfff3bb6 / canonical3466c37a (committed reproduction055658Z);
  local full gate/NG255 and86/86 pass; identities and limits below.
- Evidence: [completed8f chain](T056-m1-stageA-aligned-scratch.md),
  [type/class/event inventory](T056-m1-stageA-allocation-inventory.md),
  [producers](../diagnostics/t056-allocation-probe/README.md).
- Unresolved: no new wall claim; unpack/string_slice instruction excess remains;
  arm64 timing unmeasured. The frozen unit's final attempt remains unspent.
- Next action: lead runs fresh correctness -> bound86 -> diagnostic triage on
  the new runtime commit; old8f receipts cannot satisfy it. No owner timing/K=6.

## Architecture and staged repair

The target is a compact object header and demand-driven lookup metadata, with
unchanged slot/cell binding, property layout authority, observer behavior and
cache admission. Migration: prove the index-key bound and physical layout;
cover flag isolation, boundary keys and index admission at the lowest layer;
then audit codegen/instructions and hand the committed image to lead for exact
correctness and diagnostic wall triage. No new crate, dependency, engine FFI,
unsafe, global state or per-entry scratch allocation.

A unique ordinary array-index key is0..=4294967294, at most u32::MAX keys.
`ObjectData.index_property_count` is u32; the constructor checks conversion
from its internal unique-key count. Five flags share one byte; SymbolBrand,
module-namespace and typed-array hot checks retain separate bytes. The count
and flags together recover a whole word: payload120 ->112, Rc request136 ->128,
host usable class144 ->128. Packing flags alone does not cross the alignment
boundary (prior T019 and the previous120-byte guard already established that).
An outside safe-enum prototype moved metadata beside the descriptor Vec,
but its payload still occupied40 bytes, giving no header reduction.
Pair values remain inline; no rejected pair-boxing experiment is revived.

The measured growing-object path repeatedly asks whether a write's own key
exists before appending. A missing write can leave that shape immediately;
its lookup must not allocate an index on that intermediate shape. The new
`LayoutNode::lookup_for_write` uses any existing index, otherwise scans once;
only a hit warms/builds the original lazy index. Read policy is unchanged;
successful writes still build it after the original probe count. Dictionary
lookups and descriptor/value mutation are unchanged. The focused test proves
missing writes keep the index absent, then repeated hits build and reuse it.
Native traces verify464 transient indexes removed; sizes/counts live in the inventory.

## Executed getter ABI and instruction growth

The8f literal loop was inside get_named_typed. Its prologue saved rbx plus
r12/r13/r14/r15, with a0x18 frame; even a non-object Array.length read saved
and restored those four extra registers. d130's earlier adapter saved only
rbx (also0x18). That explains a real extra eight save/restore instructions
per such call, independent of code address. Spectral-norm loops repeatedly
read array lengths; the growth1.018-ish ->1.027-ish appeared after that loop.

The new adapter tests Object before its prologue and tail-jumps to outlined
get_named_object_typed. On the non-object path it saves only rbx again;
assembly shows an actual JMP for the object path and no frame on that edge.
The object's literal loop retains direct f64 reads and its own five saved
registers; it does not enlarge the generic adapter or the main dispatch arm.
This is an operation/type boundary, not source or bytecode recognition.
Boundary-aware static bodies: adapter45 rows, object helper95; ordinary wide
run still reserves0x388, store_owned's normalized body is unchanged from8f.
Wide has two additional padding NOP rows; no new store_owned spill is shown.
That alone does not identify their executed cost in unpack or string_slice.

Development diagnostics retained, never timing/acceptance:
- Header-only job20261021T052421Z: image6c566fc1, class-fields1.005172,
  spectral1.024546, unpack1.014088, slice1.010797. Packing every flag was
  then narrowed so the two hot exotic checks remain plain booleans.
- Job052552Z is done1: the new helper lacked an ObjectRef import. A unit
  test also initially used the wrong delete method; both were corrected
  before all2349 runtime tests passed. Failed outputs are retained, not timed.
- Combined unpinned job053011Z: image08ec0adb, spectral1.018449,
  class-fields1.006531, unpack1.011800; final pinned86 supersedes it.

The class-fields instruction excess improves without a class-specific rule;
header-only data already shows improvement. Exact per-operation attribution
is incomplete, so do not assign all of it to malloc classes or one call.
Spectral-norm returns to the earlier1.018 range with the adapter repair.

## Final source/image and verification

Non-timing job `20261021T053535Z-t056-compact-pinned86-instructions`:
done0/no DISTURBED,258s, all86 original policy IDs exactly once, VERDICT pass.
Source f84fec3ce957b8a76f1f11d74b9f5acea12a0434 plus crates/tools patch SHA
`065fb10d6b51a2f118a375d8d4377f92316c802224c7be1b9e2e8b6ba7f8ebe1`;
canonical image SHA
`3466c37a62f2fb6d764d0123212749ff810138ca05fcad336ce1afa05bdb1e3f`.
Original recipe/explicit x86_64 host target retained; strict pin audit preserves
both executor residues0xd90/0x110 and all15 surviving reference callees. Pin
regeneration uses current body budgets and reference offsets, no offset search.

| Final pinned case | instructions C/B |
| --- | ---: |
| global_read |1.000243|
| local_read |0.999904|
| branch_arithmetic |0.999931|
| property_dynamic_read |0.926903|
| heterogeneous_property_read |0.822452|
| string_slice |1.007277|
| string-unpack-code |1.011864|
| raytrace-public-class-fields |1.009864|
| math-spectral-norm |1.018282|
| access-nbody |1.012226|
| moment/dates |0.968330|
| diff/compare |0.969272|
| meriyah/parse |0.970889|

Largest ratio graphlib/paths1.019241, under the original cutoff1.03. Unpack
is **higher in instructions** than8f (~1.0016), and slice is still above1;
do not call either wall problem fixed. The physical allocation reduction
and the scalar/heterogeneous/target gains must be tested together by lead.

Local full gate passes (runtime2349, tools663, scripts35, subset5170), NG255;
new flag/index-key/index-admission tests are included. Source did not change
after those checks; the linker order was mechanically re-pinned for the final
image. Context, diagnostic script syntax and staged fast gate pass (selected Test26265).
The complete8f exact entry (42672/42672,53572 pinned corpus) is appended once
to burndown; it is not attributed to this new image.

Committed build-only job `20261021T055658Z-t056-allocation-committed-image`
is done0/no DISTURBED,71s, reproducing the exact final SHA from
`dfff3bb6bf3db01fb52a7612ed1144900b5f08c5`; both executor and all15 callee
residues are checked again. Runtime commit is pushed only to its owner branch.
Full gate passed before push; QJS_SKIP_PRE_PUSH=1 skipped its duplicate run;
pre-commit ran the fast gate again. CI39552469487 was queued at preparation,
not claimed green. The packet's static/local validation passes, including
source identity, all28 cases, prerequisite rejection and night/DST guards.

The next workspace packet is `tmp/t056-m1a-allocation-verify`: fresh-image
full correctness with8 disjoint exact shards, then bound86, then all28
previous diagnostic cases (19 fixed controls,13 focus),5 pairs/workers4.
Unit SHA remains d184dbcc122a060a219b367ffe3f3bb02c216f542f75f51eee1925387ef23284,
stage1 and original v6 policy. Attempt1/2 stays spent; one remains across A+B.
No owner timing, final K=6 or main integration is submitted.
