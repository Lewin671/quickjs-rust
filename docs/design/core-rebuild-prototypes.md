# Core rebuild: prototype ownership and validity

The [object contract](core-rebuild-objects.md) owns named-property semantics and ICs.

Prototype holders retain their own shared immutable layouts. Per-receiver cold
metadata owns accessor slots, an optional validity state and a deferred empty
guard root. Empty ordinary objects retain only an inline prototype edge and a
shared neutral seed until a guard is issued or a named property is added. A
neutral seed has a separate identity from every semantic root, including the
null-prototype root; it is never issued as a guard. Published guards compare
the direct layout identity first, without consulting cold metadata on a hit. Existing roots are
reused at construction; newly demanded roots share the empty descriptor buffer.
Unobserved named writes use transient bound nodes without empty roots or retained
transient prefixes. Before guard/template observation, publication rebuilds the
canonical prototype/flavor transition path without copying values. Structural
edits clear any pending publication guard. Tiny key/attribute buffers are weakly
interned by sequence across prototypes; only bound layouts guard inheritance.
IC admission publishes an ordinary receiver's deferred root into its direct
layout field once, preserving an already-issued guard identity.
Array/function empty seeds retain their compact shared edge representation,
reusing a weak root memo rather than allocating a box per instance. The memo
never owns a prototype, and full NamedStores retain their edge inline.
Compact authorities request a canonical root directly, without creating an
ordinary neutral seed or a temporary value store.

A dependency request installs only the required ancestor path, iteratively.
The weak user graph links validity states, never IC sites. Reparenting leaves
obsolete registrations for amortized sweeping; registration serials distinguish
repeated returns to the same parent. Structural mutation invalidates and walks
only armed subtrees. Issuing a new token replaces invalid cells and arms every
ancestor through the first armed node, so an invalid ancestor cannot hide a new
descendant token. Trackability changes propagate even through invalid cells. The single
NamedStore write barrier also invalidates on prototype value/accessor writes.
After four value-write invalidations a holder becomes permanently value-unstable: the
transition invalidates once, future entries load live slots and value writes stop
invalidating. Structural edits always invalidate; additions do not spend the
overwrite budget. There is no subscriber cap. Inherited entries hold the first
prototype's generation, weak holder/slot and optional stable-holder constant;
the receiver layout already implies its first prototype. Numbers/booleans/null/undefined are copied; RC values and existing accessor pairs
use weak handles (no hit-time pair allocation), and a failed upgrade misses.
Missing and accessor outcomes use the same proof. Accessors execute once after all borrows end. Array prototype identity is the live array payload, with no contextual wrapper.
Proxy/unmodeled exotic chains remain uncached. Expired layouts/generations are
stale entries and can be replaced without consuming another polymorphic way.
Enumeration and creation proofs use the same shared generations and layout roots.

