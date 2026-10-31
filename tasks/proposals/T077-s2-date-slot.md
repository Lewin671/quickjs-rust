# S2 typed Date internal slot

Frozen before implementation. The cdda4bd4 public86 cohort shows date-format-xparb
1.3896 main instructions and date-format-tofte 1.0756. Date getters and setters
currently reach [[DateValue]] through the NUL string key in NamedStore, repeating
key hashing/ID/descriptor work for a spec-private scalar.

Replace this representation with a small typed Date cold payload: Cell<f64>
and an optional general-cold extension. The object's OnceCell holds either a
boxed Date payload or the existing boxed general payload. The tagged pointer
is 16 bytes, so ObjectData becomes 96 nominal bytes (from 88); with the Rc
header both requests remain in the CLI allocator's 112-byte class. A normal
Date needs only a 16-byte private box, not the large general cold allocation.
Array headers remain 56 bytes. Keep all public properties in NamedStore.

Date construction supplies a fresh receiver, so initial branding precedes any
general cold metadata; subsequent updates preserve the immutable payload kind.
Date extra/private metadata uses a nested OnceCell, keeping returned references
stable. No JS callback crosses a slot access. Date brand/value reads no longer
inherit or accept an ordinary string key; freezing public properties leaves
internal Date mutation valid. Retire the old key and permit it as ordinary JS
state, including reflection. Preserve error messages and time clipping.

Add anti-spoof, inherited-brand, frozen-update and private-field coexistence
coverage, run Date and full gates, then remeasure the same public programs.
No new dependency, unsafe, AST/parser shortcut, cached Value, binding snapshot,
native ABI/S3 change or held-out input.
