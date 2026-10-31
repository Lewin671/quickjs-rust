# Core rebuild: prototype ownership and validity

The [object contract](core-rebuild-objects.md) owns named-property semantics and ICs.

Prototype holders retain their own shared immutable layouts. Per-receiver cold
metadata owns accessor slots, an optional validity state and a deferred empty
guard root. Empty ordinary objects retain only an inline prototype edge and a
shared neutral seed until a guard is issued or a named property is added. A
neutral seed never validates a prototype-sensitive guard. Existing roots are
reused at construction; newly demanded roots share the empty descriptor buffer.
IC admission publishes an ordinary receiver's deferred root into its direct
layout field once, preserving an already-issued guard identity.
Array/function empty seeds retain their compact shared edge representation.

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

