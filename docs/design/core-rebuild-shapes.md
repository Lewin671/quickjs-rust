# Core rebuild: shape sharing and structural edits

Transition label is `(KeyId, attributes, descriptor kind)`; a hit allocates no
shape, interns no key and copies no field list. Tail misses share an append-only descriptor/key buffer and incremental ID
index with prefix shapes; each shape limits lookup/enumeration to its prefix.
Tail growth is amortized O(1); a branch copies its prefix once. Buffers through
four keys use linear probes without a hash allocation; weak sequence interning
shares zero/one/two-key buffers across prototype-bound layouts. Unobserved transient bindings omit
parent links; observed bindings publish the canonical transition path. Private edits
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
Observer-free arguments construction validates each predicted key/descriptor
then fills live slots/accessors; mismatch rebuilds the completed prefix.
Predicted construction moves its provided prototype from builder to receiver;
cached layout predictions keep only weak prototype identities.
Realm-owned construction retains at most 32 shared layouts with
only that realm's intrinsic prototype/domain and already-owned well-known symbol
keys. Values, parameter cells, poison accessors and iterator values stay per
receiver. Cache retention hits acquire no extra schema owner.
Typed-loop invariant reads may cross calls only after entry proves scalar
arguments and pure native callees/receivers. Failed proofs select the normal
region before effects. Hoisted values stay in cleared activation registers,
never IC metadata; every entry reloads them.
Code-owned append plans may retain string-only transition metadata, as literal links do;
they never retain receivers, symbol handles or prototype values. Store-add guards
the old layout and the transitive prototype generation before publishing a prefix.
Constructor cold metadata links an empty root by domain/prototype identity.
Receiver allocation reserves its observed append width (at most 16), without
publishing future fields. Root width hints are nonsemantic metadata; keys,
attributes and slot order remain immutable on shared layouts.
Class definitions retain per-field prefix plans and publish only after each
initializer. Observer-free primitive segments fill slots in order, then publish
one final layout under their exclusive borrow; no future field precedes an
effectful initializer. Segment and single-field plans share one metadata cell.
JSON parses retain the preceding string-key sequence per object depth: direct
key comparisons fill owning slots; the first mismatch resumes generic insertion
from its completed prefix. Reviver source records retain the metadata path.
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
requires no descriptor edit. Individual attribute changes also cache transitions. Computed additions retain a known live shared transition. After more than eight
append misses at a layout, an uncached computed addition enters private dictionary
mode. A saturating byte survives expired child edges without retaining keys. Static construction plans retain shared layouts. Computed writes reuse
JsString leases and consume one own-slot lookup before chain proof/creation.
A structural mutation prepares storage first, publishes layout+slots together, then bumps
validity; slot reuse never occurs under an unchanged dictionary version.
Value writes touch no shape/version. Own slots load live; prototype constants
use the separate write-invalidated [IC contract](core-rebuild-objects.md#inline-caches).

Shared structural tokens use weak allocation identity only; dictionary tokens
also compare a nonzero version. Rollover publishes a fresh dictionary allocation.
Private dictionaries with weak observers use safe Rc::make_mut dissociation,
moving rather than copying fields when uniquely owned. S2 retires the legacy
exact-value revision and moves prototype ownership into NamedStore; the ordinary
hot payload is 96 nominal bytes (the CLI allocation class stays 112 with its Rc header). Debug structural edits reconstruct layout invariants;
data-only writes check the affected slot and unchanged cardinalities.

The semantic receiver and IC contract is [objects](core-rebuild-objects.md).
