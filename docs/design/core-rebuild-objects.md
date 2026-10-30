# Core rebuild: keys, shapes, slots and ICs

Target contract for [core rebuild](core-rebuild.md); S1 ordinary storage, S2 all
named stores and caches. Paths below are under `crates/qjs-runtime/src/`.

## Keys and lifetime

Extend `property/atom.rs::AtomDomain`, not a parallel interner. One runtime group
owns the domain; communicating realms share it. `KeyId(NonZeroU32)` is 4 bytes,
recycled only after the last lease dies. Strings intern exact UTF-16; symbols
intern identity in the same namespace. Private names remain separate internal identities.
`KeyRecord` contains kind, text or strong symbol handle, cached canonical array
index and a domain identity. It never folds `"01"` into `"1"`.

`KeyLease(Rc<KeyRecord>)` is an 8-byte owner. Shapes, dictionaries and linked code
retain leases; an operation's borrowed ID is valid while its lease/domain live.
The domain's string/symbol content indices are weak and swept with the live-size
rule. The ID directory exists only in tests; production sweep sizing uses the
content indices. Symbol identities use the engine's NameHasher. Numeric IDs alone must not pin every historically interned key.
Escaping objects retain the domain through their layouts. Shape roots live on
runtime/realm owners outside the domain: no domain→root→shape→domain RC cycle.
A free list recycles dead IDs before minting new ones. Bare IDs never outlive their borrowed leases; live shapes, dictionaries,
linked code and lease-owning caches prevent reuse. Layout identities/versions
remain non-reused. Production u32 exhaustion requires more than 2^32 live key
records and follows allocation-failure abort, not a JS exception; an internal
try_intern supports test-only capped domains (lead decision 2026-10-30). Foreign
compiled names relink once by units; existing cross-realm values share the original domain. A foreign
domain cannot validate an IC by integer equality; host ingress must normalize
keys or report an explicit domain error, never reinterpret IDs.

`to_key(&mut Context, Value) -> Result<KeyLease>` performs observable ToPrimitive
once, then interns; named bytecode operands use prelinked leases. Host `&str`
entry points convert once; internal slot access never formats or hashes text.

## Physical layout (S1 assertions; S2 cache budgets remain proposed)

| Type | Fields / size |
| --- | --- |
| `ShapeEntry` | `key:u32, slot:u32, order:u32, attrs:u16, reserved:u16` = 16 bytes |
| `NamedStore` | `layout:Rc<Layout>` 8, `values:Vec<Value>` 24, `accessors:Option<Box<AccessorSlots>>` 8, `epoch:u64` 8 = 48 bytes; `RefCell` wrapper 56 |
| Ordinary hot payload | `named:RefCell<NamedStore>` 56, `prototype:RefCell<Option<Prototype>>` 24, cold pointer 8, index count 4, packed flags/edit count/padding 4 = 96 bytes, excluding RC header |
| `Slot` | One owning `Value`: initially 16 bytes, S5 target 8; TDZ/hole state separate from ordinary data |
| `Layout` | Immutable shape or unique dictionary; ID/domain, owned entries/key leases and one key-ID lookup index; transition cache is cold |
| `Dictionary` | Private identity + `u64` structural version; key-ID index, attributes/order and slot allocation are its sole authority |

S1 compile-time assertions verify these sizes on aarch64/x86_64 macOS; a layout
estimate alone is no ABI proof. Values remain in a keyless vector, with no pair
of inline Values enlarging the ordinary header.
Slot storage may pool capacity, never keys. Accessor slots hold `undefined` in
the value vector and their getter/setter pair in the cold slot-indexed table;
the shape decides descriptor kind. Bulk descriptor APIs construct `Property`
only on demand. Symbol descriptors join this authority, not a second cold table.

## Shapes and edits

Transition label is `(KeyId, attributes, descriptor kind)`; a hit allocates no
shape, interns no key and copies no field list. Tail misses share an append-only descriptor/key buffer and incremental ID
index with prefix shapes; each shape limits lookup/enumeration to its prefix.
Tail growth is amortized O(1); a branch copies its prefix once. Private edits
detach before mutation. The empty root never retains a nonempty buffer. Shared shapes apply equally to literals,
constructor stores, class fields, JSON and host creation. No whole-vector hash
or comparison on a hit. Duplicate keys preserve first insertion order and last
definition semantics. Class/JSON builders execute each observable step in order.

Weak edges never evict a live child. Up to eight edges use a vector, then a
NameMap; child destruction removes its incoming edge (geometric sweeping also
prunes dead entries when removal was borrowed). Children retain source
shapes, with append edges distinguished for last-property deletion rollback.
Strong keep-alive retention uses a 1 MiB allocation budget instead of a node
limit: descriptor/key capacities, ID buckets, text/lease estimates, shape nodes
and weak-edge capacity are tracked incrementally. Shared arrays count once.
Live shape memory also consumes this conservative budget; exceeding it drops
strong cache roots, while live objects retain their complete transition paths.
Live shapes and their weak lookup edges may exceed the keep-alive bound.
Repeated fixed-key construction within the budget reuses shapes after objects
die. Delete/reconfigure churn after 32 edits or width
over 128 entries uses dictionary mode. A dictionary never records transitions;
S1 swap-removes dictionary slots and preserves explicit order numbers, leaving
zero order tombstones; its ID-index tombstones are compacted with bounded slack.

Deleting the last appended property restores its parent shape; other deletes
keep the private-edit policy. Re-add appends a new order position; array-index keys enumerate in numeric
order, then other strings, then symbols in insertion order. Reconfiguration,
seal/freeze and preventExtensions preserve data/accessor semantics. Seal/freeze
use one cached bulk transition per source shape and integrity level, copying
descriptors once and preserving per-object accessor payloads. No-op integrity
changes copy nothing. Extensibility remains an object flag; preventExtensions
requires no descriptor edit. Individual attribute changes also cache transitions. A structural
mutation prepares storage first, publishes layout+slots together, then bumps
validity; slot reuse never occurs under an unchanged dictionary version.
Value writes touch no shape/version. Existing property_revision consumers must
be migrated individually: cached values cannot borrow slot-cache validity.

S1 structural tokens pair weak allocation identity and epoch; epoch rollover
publishes a fresh identity. A private dictionary with weak observers uses safe
Rc::make_mut dissociation, moving rather than copying fields when uniquely owned.
Legacy value revision starts observation lazily in ObjectColdData, keeping the
ordinary hot payload at 96 bytes. Debug structural edits reconstruct all layout
invariants; data-only writes check the affected slot and unchanged cardinalities.

## Shared semantic API

Builtins call `get(ctx, receiver, key)`, `set(ctx, base, key, value, receiver,
strict)`, `get_own_descriptor`, `define_own`, `delete_own`, `own_keys` and
`set_prototype`. The internal own-slot API returns a scoped `Data/Accessor/Miss/
Exotic` result; a data hit loads the current value once. Release every storage
borrow before getter, setter, Proxy trap, coercion, callback or lazy installation.
Set proves writable own data and the actual receiver; inherited setter, super,
Reflect.set, additions and strict errors retain their complete algorithms.

Arrays, functions, wrappers, Map/Set and ordinary objects embed the same NamedStore.
Dense/sparse indices, array length and mapped arguments stay exotic; typed-array
canonical numeric indices and detachment use their own algorithms. String virtual
indices, function lazy name/length/prototype, namespace live exports and private
slots retain semantic hooks. Hooks materialize into the same store when required.
S1's intrinsic Array.prototype is an ObjectRef: its length/index definitions run
array validation/shrink hooks before NamedStore; ordinary creation and slot-write
caches decline that receiver.
No symbol/text property table survives outside it. Prototype slots retain live
array/function identity (`value/object.rs::Prototype`), never copied descriptors.

## Inline caches

One cache per named site, mono first, bounded four-way poly then megamorphic
generic lookup. Own-data guard: domain + kind + shape ID (dictionary ID/version)
+ descriptor bits + slot. It does not guard the property's value or receiver
address. Entries hold weak layout tokens, not a strong receiver or cached Value.
An initial entry budget is 32 bytes: layout token 8, validity token 8, slot 4,
kind/flags 4, epoch/version 8; four-way storage is allocated only when needed.

Inherited entries additionally have a cold chain record: weak first-prototype
and holder identities, weak dependencies and a validity cell (outside the 32-byte
own-entry budget). Every hit guards the **current receiver's first prototype**
as well as its own layout; equal shapes alone do not imply equal prototypes.
Each prototype node registers layout/prototype dependencies; mutation invalidates
dependents before publication and removes dead subscriptions. Hit checks the live
token and reads the holder's current slot. Receiver prototype edits miss the
explicit edge guard; value overwrites preserve validity. Proxy/unmodeled exotic
chains remain uncached. Overflowed versions allocate new identities, never wrap.
Enumerate-keys caches use the same structural dependencies, never value epochs.

S2 deletes `NamedPropertyCacheEntry`'s exact-value/receiver variants and repeated
Small/Dynamic/literal probe paths. In debug mode compare every IC result with
uncached semantic lookup without replaying accessors; disable ICs entirely for
a full correctness run. Count hits/misses by receiver family, hashing, transitions,
key interning, allocations and bounded retained nodes; do not claim a 90% hit
rate independent of the operations/programs measured.
