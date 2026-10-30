# T077 S1 implementation brief: ordinary property authority

Ready-to-assign scope, based on source `6663738f`; design, not a runtime patch.
Parent [T077](../T077-core-rebuild.md) owns authorization, acceptance and stage
order; [object contract](../../docs/design/core-rebuild-objects.md) owns layouts,
lifetimes and API invariants. S1 may be performance-neutral; S2 is its payer.

## Before editing

Record implementation branch/base and owner paths in T077. Confirm a clean owner
worktree, read the affected module headers and preserve unrelated changes.
Receive the lead's baseline receipt for job
`20261030T045926Z-core-rebuild-baseline` when available; do not submit a duplicate.
No old T056 plan/attempt is reopened. Timing needs the lead's serialized queue.

Inventory the actual consumers of `PropertyStorage`, `ObjectLiteralShape`,
`DynamicPropertyStorage`, `OwnDataPropertyRead/Write`, `property_revision`,
`layout_revision`, ObjectRef constructors and symbol descriptors using rg.
List every mutation and construction entry, including internal/host/test helpers;
do not port only bytecode literals. The named-property API remains semantic,
even where its internal key becomes an ID.

## Freeze these source contracts first

- `property/atom.rs`: extend the existing domain with a unified non-reused u32
  KeyId namespace and `KeyLease`; retain canonical string equality/index parsing.
  Symbols receive IDs by identity, never description. The domain owns weak
  indices; layouts and linked code own leases. No second atom domain per object.
- `value/layout/`: `Layout`, `ShapeEntry`, `NamedStore`, transition/dictionary
  APIs, key ownership, structural validity and deterministic order iteration.
  `lookup(&KeyLease)` returns slot/kind/attributes; insertion consumes the miss
  proof and current version, not a second text/ID probe. Miss proofs cannot cross
  a callback. `load_data`, `replace_data`, `define`, `remove`, `keys` expose no
  mutable descriptor/shape internals to builtins.
- Object constructors accept an explicit owning domain (or typed Context) even
  with null/no prototype. Realm setup creates the domain before its intrinsics;
  shared-realm setup passes it through. Host creation requires an explicit runtime
  owner. Migrate context-free internal/test constructors; do not hide a new
  default/private domain or TLS current-realm lookup inside them. Document public
  constructor source migration; never clone a live object graph to relink it.
- `OrderedDataPropertyBuilder` receives its domain before insertion, feeds the
  common transition tree and preserves duplicate-last-definition/order behavior.
  Literal input-slot maps remain immutable compiler metadata, not lookup authority.
- Data writes preserve layout identity; structural edits publish store+layout
  atomically before callbacks can observe them. Keep old revision APIs only while
  existing cache consumers require them; each revision has one explicit meaning.

Review those signatures and size assertions before separate owners begin ports.
This is source/API review, not a request to reauthorize the rebuild.

## S1 source API freeze (implementation owner, base `dd0905b2`)

`KeyId` is the unified nonzero u32; `KeyLease` owns `KeyRecord`. `Atom` and
`AtomId` name the same lease/ID types for prelinked string operands.
`AtomDomain::{intern,intern_symbol,lease}` share one weak ID directory.
`PropertyOwner::new(AtomDomain)` explicitly owns the domain's shape root and
bounded retention cache outside the domain. A weak domain attachment makes
owner creation idempotent while an owner is live. Realms/host creation share that
owner; `empty_store()` starts from its root. Layouts keep the domain strongly
and the cache owner weakly, so escaping stores remain usable without a cycle.
`NamedStore::lookup(&mut self, &KeyLease)` returns an exclusive borrowed
`Lookup`; consuming `define`/`remove` uses its single ID probe. Foreign leases
return `ForeignKey` before an ID probe. `get`, `data_slot`, `load_data`,
`replace_data`, `keys`, `layout` and `epoch` are the remaining scoped APIs.
No descriptor or mutable layout internals escape to semantic consumers.
The foundation asserts KeyId 4, KeyLease 8, ShapeEntry 16, NamedStore 48 and
RefCell<NamedStore> 56 bytes on the current 64-bit target. Other-target size
verification and the ordinary hot payload remain stage-close requirements.

## Implementation sequence within S1

1. Add key-ID/lease tests and layout transition/dictionary implementation. Unit
   tests may use test builders; no production object consults both old/new stores.
2. Replace ObjectData.properties with NamedStore in one construction/mutation
   cutover. Port ordinary string **and symbol** paths together. Move Property
   attributes to shape entries and getter/setter pairs to cold accessor slots.
3. Delete all four PropertyStorage variants, `dynamic_storage.rs`, the literal
   payload lookup index and ordinary ObjectColdData.symbol_properties. Preserve
   private/buffer/generator/namespace internal slots and Prototype's live identity.
   `Property` may remain a descriptor API result; it is not a stored data slot.
4. Port every ordinary creation path (JSON, classes, wrappers, host intrinsics,
   null prototype) to domain/transition APIs. Class fields must execute each
   initializer/add/delete in order; no preinstalled final shape. Host HashMap
   input with explicit order validates that order; it cannot hash-order JS keys.
5. Update slot-read APIs and **all** existing executor/cache consumers to the new
   store. Remove old storage-form/literal-shape tests from cache guards; use layout
   identity or the existing structural revision as appropriate. Do not introduce
   a second lookup index to keep an old helper signature. Full IC redesign is S2.
6. Audit all mutation bypasses and deletion inventory. Update current architecture
   and module headers only to facts now implemented; update T077 resume/evidence.
   No temporary test store/feature/compatibility payload survives stage closure.

S1 intentionally leaves arrays/functions' independent named stores until S2;
each receiver still has exactly one authority. S1 does not claim full named-store
unification or introduce fallback between stores on one receiver. S2 starts from
this same API, with exotic index/length hooks outside NamedStore.

## Discriminators and debug modes

| Boundary | Focused tests / work proof |
| --- | --- |
| Creation sharing | Repeated fixed-key constructor/class/JSON/literal/host creation reuses shapes even when previous objects die; transition hits copy/intern zero fields/keys. |
| Descriptors | Data↔accessor, writable/configurable errors, nonextensible/seal/freeze, accessor recursion and throw; data overwrites preserve layout/slot cache and read live values. |
| Order | Integer keys vs `01`/2^32−1, strings then symbols, duplicate keys, delete/re-add, huge dictionaries and bounded order tombstones. |
| Key equality | Astral pairs assembled in halves, lone surrogates, real sentinel-range scalars; distinct symbols with same description; key-ID exhaustion without wrap. |
| Prototype/receiver | Live array/function/proxy prototypes, inherited getter/setter this, Reflect/super receiver, prototype edits, global delete/accessor/recreate cells. |
| Exotic boundary | Lazy String indices, namespace live exports, typed-array detach/canonical numeric keys, Array length and function lazy descriptors; named storage never overrides these hooks. |
| Ownership | Escaping object outlives realm handle; domains/atoms/shapes release after owners drop; symbol-key cycles remain known RC leaks until S6, not falsely counted as reclaimed. |
| Bounds | At most 16 cached child edges per shape, 4096 cached nodes per domain; >5000 live shapes still correct, cache retention bounded independently of history. |

Debug-only layout verification reconstructs descriptors/order from entries and
compares slot counts/kinds/index counts after every mutation; dictionary edit
versions and old revision consumers are checked. Diagnostic generic lookup must
not re-run getters, proxies or key coercions. Use deterministic counters behind
perf-counters for probes, intern, field copies, transitions and retained nodes;
do not assert timing or a workload-specific IC hit rate in unit tests.

Run focused qjs-runtime tests at the lowest useful layer, NG fixtures for disputed
semantics and curated object/descriptor/enumeration/prototype/symbol/JSON/class
Test262 slices. Then run full check.sh, compare-qjs.sh, the complete configured
exact scan and its burndown command from the conformance runbook. Preserve all
configured cases, default/agents/perf-counters behavior and source spans. S1 has
no unsafe justification and adds no unsafe module or dependency.

## Owner paths and handoff

Lead owns `value/object.rs`, `bytecode/linked_code.rs`, cache/executor API edits,
cross-family constructor signatures/call sites, docs and integration. Owner A
owns `property/{atom,key}.rs` and new `value/layout/`; owner B owns
`value/object/{slot_reads,key_order,ordered_data_builder,tests}.rs` plus the
deletion of `dynamic_storage.rs` after APIs freeze. Semantic property wrappers
in `property/mod.rs` are lead-owned. No concurrent edits to shared types.
Other constructor family ports can run independently only after the lead records
an explicit file list; they consume the same domain/NamedStore APIs.

Exit receipt: source revision and source diff, type sizes on both targets,
zero old-store/index references (aside from history), debug/work-count results,
full correctness/exact/burndown, whole-portfolio neutral-or-better evidence,
every local regression and lead decision. Measure uninstrumented exact binaries;
compare with baseline and NG, never infer speed from counter decreases. While
the iMac measures, design S2 and run correctness on another owner branch.
If correctness/retention/neutrality fails, keep the last accepted main and repair
or revert S1 source. Never keep the old stores as a production fallback.
