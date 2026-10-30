# T077 S1 ordinary authority cutover

Parent `4e39af1f`, implementation branch `agent/t077-s1-property-authority/codex`,
common base `dd0905b2`. The implementation uses only this worktree. T077's resume
block binds the final source and exact-scan receipt; no push or merge.

## Authority and deletion receipt

ObjectData has one NamedStore for strings and symbols, keyless values, and cold
shared accessor pairs. Layout entries own attributes and leases; a single ID
index resolves slots. Every constructor, class field, ordered JSON/host builder
and literal uses the same realm-owned domain/transition mechanism. Literal
input-slot maps remain compiler metadata, with bounded per-domain layout links;
objects never consult literal metadata for property lookup. Duplicate literal
values compact in the producer's vector in source order, without another slot
vector. Host HashMap order is validated before key creation.

Deleted from current Rust code:

- `PropertyStorage::{Small,Dynamic,Shaped,ShapedPair}` and their storage routing,
  descriptor mutation and text lookup implementations in `value/object.rs`.
  That file's cutover diff removes 1,011 lines and adds 176 before receipt/docs.
- `value/object/dynamic_storage.rs`: all 68 lines, including its key index,
  entries and order storage; its module import is gone.
- `ObjectLiteralShape.lookup`: the payload name index is gone. Only transient
  compiler deduplication, immutable keys/input slots and runtime layout links remain.
- `ObjectColdData.symbol_properties`: the independent ordinary symbol table and
  its direct mutation/read paths are gone. Arrays/functions keep their separate
  named tables until S2, as authorized.
- Old slot readers: the initial cutover of `object/slot_reads.rs` removes 345
  lines and adds 129, with further lease-based read helpers in this unit.

Audit: current runtime Rust has no PropertyStorage, DynamicPropertyStorage,
shared_dynamic_slot_value or literal_data_slot references. Historical source
inventories and archived evidence remain. The tracked CLI linker order lists
still contain historical retired mangled symbols; the linker ignores unmatched
entries. They were left unchanged rather than silently changing the measured
layout recipe; the lead must revalidate/regenerate them with formal measurement.

## Invariants and S1 design corrections

- Lead decision 2026-10-30: key IDs recycle only after the last lease dies;
  public interning takes Rust's allocation-error abort path at resource
  exhaustion. No property/native/install/module error ABI was widened.
  [Decided record](../proposals/T077-s1-fallibility.md).
- Legacy bare epoch guards could alias an earlier version after epoch rollover.
  Structural validity now pairs weak allocation identity with epoch across all
  existing consumers, including enumeration, creation and global-slot proofs.
  A uniquely owned dictionary with weak observers uses safe Rc::make_mut
  dissociation rather than copying fields; old weak guards expire.
- The legacy value revision is observed lazily in ObjectColdData. Structural
  validity comes from NamedStore; packed flags and a u32 index count keep the
  hot object at the approved 96-byte budget without a duplicate domain field.
- Debug structural edits reconstruct descriptor/order/index/slot invariants.
  Data-only writes verify the affected slot and unchanged cardinalities; their
  API cannot alter shape/order/index/kind. This avoids rescanning immutable
  metadata on every global/data assignment while checking every changed invariant.
- String indices are pure exotic descriptors, reusing String's canonical-index
  logic. Reads/enumeration do not intern or materialize indices. Live namespace
  cells and array/function/proxy prototype identities remain in their hooks.
- Ordinary named interpreter operands keep prelinked leases through lookup.
  Reads and writes pass resolved slot metadata to caches without another ID
  lookup; insertion consumes the same miss proof. Getter/setter/exotic cases
  release the borrow before the semantic path. Constructors execute fields in
  order; no final class shape is preinstalled before initializer effects.

Property remains a 32-byte cold descriptor API. Cold accessor states share the
existing Property accessor Rc, avoiding an allocation on each descriptor read.
Owning Value remains 16 bytes. No dependency, crate, unsafe block, FFI, Rust
async, binding snapshot, workload recognizer or fallback store was added.

## Sizes and work proofs

Compile-time assertions on aarch64-apple-darwin and x86_64-apple-darwin verify:
KeyId 4 B; KeyLease 8 B; ShapeEntry 16 B; NamedStore 48 B;
RefCell<NamedStore> 56 B; ObjectData 96 B; Value 16 B. x86_64 verification is
cross-compilation, not a claim that its runtime suite or timing ran on this Mac.

Focused tests prove shared constructor/class/JSON/literal/null-prototype layouts,
post-object-death cache retention, zero append-hit field copies/interns and one
ID probe, descriptor transitions, dictionary width/edit limits, forced ID-index
collisions/rebuilds/tombstones, symbol identity/order, UTF-16 key equality,
nonextensible inherited setters, live slots after data writes, virtual String
indices, expired IC rejection across key reuse and epoch rollover, domain
release, bounded edges/nodes and more than 5,000 simultaneously live shapes.
Text/identity boundary probes and cold index rebuild work are counted separately;
no time or workload-specific cache hit-rate assertion is used.

## Verification boundary

Final cutover working tree, 2026-10-30 UTC:

- `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check.sh`: passes; 2,411 default runtime tests, default/agents/perf-counters clippy and selected feature tests, 753 benchmark-tool tests, 35 script tests, context/file-size/hook checks, and 5,170/5,170 Test262 subset cases.
- `./scripts/compare-qjs.sh`: 257/257 fixtures pass on the final tree.
- `cargo test -p qjs-runtime --features agents,perf-counters --lib --quiet`: 2,451/2,451 pass, including the exact 128-key reconfiguration boundary.
- `cargo check -p qjs-runtime --target x86_64-apple-darwin --features agents,perf-counters`: passes the production size assertions on the final source.
- The commit hook supplies `check-touched.sh --staged --explain` on the final tree.

No complete exact scan or new burndown standing is asserted here; the scan
will bind the committed candidate. Source/API invariants are mapped in the
current architecture and object contract, not duplicated as acceptance here.

Both pinned top-level reference/input submodules were initialized in this
worktree; no reference source or nested NG submodule was edited. Full/subset
checks used the shared read-only Test262 input. The exact driver uses this
worktree's same pinned corpus. Disk stayed above 30 GiB before the release scan.
No iMac job or local timing measurement was submitted; formal stage performance
and the pending rebuild baseline receipt remain lead-owned.
