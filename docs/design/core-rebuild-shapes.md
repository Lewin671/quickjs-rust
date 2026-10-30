# Core rebuild: shape sharing and structural edits

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
Code-owned append plans may retain string-only transition metadata, as literal links do;
they never retain receivers, symbol handles or prototype values. Store-add guards
the old layout and the transitive prototype generation before publishing a prefix.
Constructor cold metadata links an empty root by domain/prototype identity.
Receiver allocation reserves its observed append width (at most 16), without
publishing future fields. Root width hints are nonsemantic metadata; keys,
attributes and slot order remain immutable on shared layouts.
Class definitions retain per-field prefix plans and publish only after each
initializer. Observer-free primitive segments share one borrow/reservation.
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
Value writes touch no shape/version. Existing property_revision consumers must
be migrated individually: cached values cannot borrow slot-cache validity.

Shared structural tokens use weak allocation identity only; dictionary tokens
also compare a nonzero version. Rollover publishes a fresh dictionary allocation.
Private dictionaries with weak observers use safe Rc::make_mut dissociation,
moving rather than copying fields when uniquely owned. S2 retires the legacy
exact-value revision and moves prototype ownership into NamedStore; the ordinary
hot payload is 88 bytes. Debug structural edits reconstruct layout invariants;
data-only writes check the affected slot and unchanged cardinalities.

The semantic receiver and IC contract is [objects](core-rebuild-objects.md).
