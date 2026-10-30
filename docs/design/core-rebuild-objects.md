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
Compact/typed static-name misses use domain-checked weak lease links; dead keys
remain recyclable. Computed JsString keys memoize one live lease in metadata;
reads never intern, and mutation/COW clears the memo. The production string-value
header is 72 bytes: the matcher mode occupies scalar padding and the cached
slice/key lease share a 24-byte payload, with no extra allocation. No receiver/domain is pinned.

## Physical layout (S2 assertions)

| Type | Fields / size |
| --- | --- |
| `ShapeEntry` | `key:u32, slot:u32, order:u32, attrs:u16, reserved:u16` = 16 bytes |
| `NamedStore` | `layout:Rc<Layout>` 8, `values:ValueSlots` 32, `accessors:Option<Box<AccessorSlots>>` 8, `epoch:u64` 8, strong prototype owner 8 = 64 bytes; `RefCell` wrapper 72 |
| Ordinary hot payload | `named:RefCell<NamedStore>` 72, tagged cold pointer 16, index count 4, packed flags/edit count/padding 4 = 96 bytes, excluding RC header |
| Array hot/cold payload | 56-byte header: dense Vec RefCell 32, packed length/flags 8, scoped seed owner 8, cold pointer 8. Optional cold payload 104: NamedStore RefCell 72 and holes RefCell 32. Realm shares the default empty seed; probes allocate nothing. |
| `Slot` | One owning `Value`: initially 16 bytes, S5 target 8; TDZ/hole state separate from ordinary data |
| `Layout` | Immutable shape or unique dictionary; ID/domain, owned entries/key leases and one key-ID lookup index; transition cache is cold |
| `Dictionary` | Private identity + `u64` structural version; key-ID index, attributes/order and slot allocation are its sole authority |

Compile-time assertions verify these sizes on aarch64/x86_64 macOS; a layout
estimate alone is no ABI proof. Values remain keyless: zero/one/two slots are inline; larger stores use one
heap vector. Direct pair literals allocate no temporary slot buffer.
Slot storage may pool capacity, never keys. Accessor slots hold `undefined` in
the value vector and their getter/setter pair in the cold slot-indexed table;
the shape decides descriptor kind. Bulk descriptor APIs construct `Property`
only on demand. Symbol descriptors join this authority, not a second cold table. Nonempty host
ObjectRef construction requires an ordered vector; legacy HashMap constructor
entry points accept only empty input, keeping S2 empty creation sites compatible.

## Shapes and edits

The [shape contract](core-rebuild-shapes.md) owns live transitions, append-shared
metadata, byte-bounded retention, dictionary edits and integrity transitions.

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
slots retain semantic hooks. StringData and DateValue are typed private slots, never named keys. Date uses a 16-byte boxed scalar/extension payload; other cold metadata remains stable behind its box. Hooks materialize into the same store when required.
S1's intrinsic Array.prototype is an ObjectRef: its length/index definitions run
array validation/shrink hooks before NamedStore; ordinary creation and slot-write
caches decline that receiver.
No symbol/text property table survives outside it. Prototype slots retain live
array/function/Map/Set identity (`value/object.rs::Prototype`), never copied descriptors. Named values and owning prototype edges participate in iterative object/array/function teardown.

## Inline caches

One cache per named site, mono first, bounded four-way poly then megamorphic
generic lookup. Own-data guard: domain + kind + shape ID (dictionary ID/version)
+ descriptor bits + slot. It does not guard the property's value or receiver
address. Entries hold weak layout tokens, not a strong receiver or cached Value.
The implemented entry is 32 bytes: weak layout/version 16, slot 4, kind and padding 4, optional cold chain pointer
8; four-way storage is allocated only when a second guard is admitted. A fifth
independent live guard makes the site permanently megamorphic. Shared-own poly
hits compare four identities in parallel; numeric consumers load a live f64
with a value-kind check, without constructing an owning Value. Number/function
owning loads copy the current slot inline; other values keep ordinary cloning.
Typed numeric stores guard the same own writable slot and update numeric
payloads directly, with normal replacement/drop when the old type differs.
Array prototype identity checks borrow the edge without cloning an owner.
Constructor prototype links use the same weak layout/version and live own-slot
read. Lazy/accessor misses retain full Get; no prototype value is cached.
A cold store-add plan uses the prefix/prototype guards described in the
[shape contract](core-rebuild-shapes.md). Typed-loop stores explicitly exclude
additions, preserving their invariant-read hoisting proof.

Layouts include prototype identity and semantic hook flavor. Shared guards compare
weak allocation identity only; dictionary guards additionally compare versions.
A receiver owns its explicit prototype edge; layouts retain weak identities so
retained shapes cannot pin JavaScript values or realms. Roots and transition
trees are canonical per prototype/flavor. Prototype mutation re-roots the layout.

Objects used as prototypes enter private dictionary mode with one validity state.
Its weak user graph links prototype-mode children, never IC sites. Structural
mutation replaces its validity generation and cascades iteratively; value writes
and ordinary object transitions invalidate nothing. There is no subscriber cap.
Inherited entries hold the first prototype's generation and weak holder/slot;
the receiver layout already implies its first prototype. Missing and accessor
outcomes use the same proof. Accessors execute once after all borrows end.
Array prototype identity is the live array payload, with no contextual wrapper.
Proxy/unmodeled exotic chains remain uncached. Expired layouts/generations are
stale entries and can be replaced without consuming another polymorphic way.
Enumeration and creation proofs use the same shared generations and layout roots.

S2 deletes `NamedPropertyCacheEntry`'s exact-value/receiver variants and repeated
Small/Dynamic/literal probe paths. In debug mode compare every IC result with
uncached semantic lookup without replaying accessors; disable ICs entirely for
a full correctness run. Count hits/misses by receiver family, hashing, transitions,
key interning, allocations and bounded retained nodes; do not claim a 90% hit
rate independent of the operations/programs measured.


## S2 semantic ports and verification

Arrays share a realm-owned empty authority and explicit prototype; mutation
forks the same NamedStore into cold storage. Array headers are 56 bytes, cold
NamedStore/holes data 104; functions embed a 32-byte lazy authority and have
304-byte headers, excluding RC headers. Named probes allocate no array cold data.
Original lazy property order is reserved before later keys. Function virtual
state is part of layout flavor; hooks publish into the authority when observed.
Every production constructor receives the realm/prototype explicitly. Realm
intrinsics are stable creation inputs even when bindings are overwritten; an
AtomDomain can serve several realms and is never an intrinsic identity.

`value/named.rs` is the borrowed receiver facade for all cache consumers.
Interpreter, compact/wide, typed loops and leaf readers share one shape IC;
Proxy, namespace live exports and unmodeled exotic keys use their semantic hooks.
`QJS_DISABLE_IC=1` disables named/enumeration IC admission and hits.
`QJS_VERIFY_IC=1` in debug builds compares hits with complete pure uncached outcomes,
without replaying accessors, key coercion or Proxy traps. These modes are captured
once per AtomDomain/runtime group. Creation-miss proofs share prototype generations and prototype-sensitive roots; IC-off disables these and global-object slot memos. Perf counters partition IC hits/misses, generic own probes and
text-index linking queries by receiver family; they compile out of production timing builds.
