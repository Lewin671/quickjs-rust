# Property layouts: construction, transitions and retention

How an ordinary named-property store keeps its layout authority current without rebuilding it; refines
[interpreter-core-v2](interpreter-core-v2.md#properties-atoms-and-layouts-are-the-ordinary-lookup-authority).
[T056](../../tasks/T056-interpreter-core-v2.md) owns status, evidence and plans; no performance result here.

## Why the first M1.2 shape was closed

The rejected stage-2 shape republished layouts by value (whole-field copy, compare and hash per append), cached
shapes only weakly, gave builders and function tables private domains and hashed text probes twice; instructions
tracked wall time (up to 5.6x). The [diagnosis record](../../tasks/archive/T056-m1-stage2-diagnosis.md) owns the
mechanisms, ablations and jobs. The rules below remove each mechanism as a new implementation shape.

## Dependency: canonical string storage

T063 (branch `agent/string-canonical/codex`, not yet on main) makes a stored string
keeps every valid surrogate pair as its scalar. Equal code units then have equal
bytes. Keys that come from `JsString` values or linked atoms are canonical by
construction, so layout probes compare and hash their bytes directly, with a
debug assertion instead of a canonicalization scan. Untyped host `&str` entry
points (atom interning, host property APIs) canonicalize once at entry. Until
T063 is on main, probes must keep the scan.

## Store ownership

- **Domain.** A store resolves its atom domain once, when it is created: from
  the prototype for ordinary objects, and from the receiver realm for the
  ordinary named parts of functions, arrays, wrappers and builders. No store
  creates a private domain, and no store re-links after construction.
  Ordered-data builders (JSON, RegExp and String wrappers) receive the target
  domain before their first insert.
- **Lazy tables.** A function or array named table has no layout until its
  first ordinary named install. Semantic hooks (`length`, `name`, `prototype`,
  indices) need none.
- **One pointer.** The layout is one `Rc<LayoutNode>`.
  - `LayoutNode` is either a shared immutable `Shape` or a private
    `Dictionary`.
  - The dictionary node is uniquely owned and edited in place through its cell.
    No weak reference to it exists, so it never becomes shared.
  - The non-append edit counter is a `u8` in the object's existing flag padding.

## Transitions are edges

A published shape never changes; an object moves to another shape, so a shape
pointer stays a valid cache token. The `token` semantics in `layout/tests.rs`
are unchanged.

| Edit | Rule |
| --- | --- |
| Append | Edge label `(owned key, attributes, ordered, payload encoding)`. A hit compares key bytes (pointer first) and allocates nothing; no field vector is copied or compared. A miss copies the parent's fields once, interns the key, creates the child and records the edge. |
| Payload re-encoding | `Small`→`Dynamic` and `Pair`→`Dynamic` are edges from the current shape. They are not structural edits and are not counted. |
| Delete, reconfigure, reorder | A content-hash cache keyed by an incremental 64-bit hash of `(key, attributes)` per slot. A full compare runs only on a hash match. |
| Bulk freeze/seal and descriptor batches | Reconfigure only slots whose attributes changed; never re-intern every key. |
| Dictionary switch | After 32 non-append edits, or when an append would exceed 128 fields. Appends never count as edits. |

A missing-own probe that precedes an insert carries its canonical key and the
current shape into the append, so an add probes once. During an operation that
reuses a key identity, it holds a strong `AtomKey`, so a freed key allocation
cannot alias another key (no ABA).

## Edge ownership and the retention bound

- **Ownership.** Parents own their edges strongly. Children hold no reference to
  their parent, so there is no cycle. Without strong edges, every intermediate
  shape dies with the object that last used it, and the next object rebuilds the
  whole chain.
- **Roots.** One empty root per payload encoding is owned by a `DomainRoots`
  value, which realms sharing a domain hold. The atom domain itself does not own
  it; see the cycle section below.
- **Bounds.** Each shape holds at most 16 append edges and one edge per
  encoding. When full, the oldest edge is evicted. Each domain caps the shapes
  created through edges at 4096. A miss beyond the cap still creates the shape
  but records no edge, so the object keeps working without growing the tree.
- **What edges retain.** Edges keep their keys' atoms alive. Edges and roots
  therefore retain at most `16 + 5` children per live shape and 4096 shapes per
  domain, plus the atoms named on those edges.
- **Reconciling with the churn rule.** This replaces the rule "transitions are
  weakly retained". It still meets the design rule that link and transition
  caches never retain arbitrary churn forever:
  - retention is a fixed function of the bounds above, not of how many distinct
    keys a program has used;
  - eviction drops a subtree and its atoms immediately;
  - dictionaries, which absorb high churn, record no edges.
- **Test expectation.** The fixed-live-churn test
  (`dictionary_order_tombstones_stay_bounded_under_fixed_live_churn`) appends
  and removes 10,000 distinct keys after an anchor. It must now assert:
  - `domain.live_count() <= 1 + 16`: the anchor shape's edges may retain the
    first evicted-window temporaries;
  - `table_len()` stays within the sweep bound of that live count;
  - the dictionary's own order, positions and index stay exactly as today
    (one live entry);
  - after the layout and the roots are dropped, `live_count() == 0`.
  A second test churns 10,000 distinct keys across fresh objects and asserts
  that the domain's edge-created shape count never exceeds 4096.

## Literal and constructor sites

`ObjectLiteralShape` keeps one `(WeakAtomDomain, shape)` slot, replaced when a
different live domain uses the literal. It is never keyed by an address.
Creating a literal clones one pointer.

Class fields and constructor stores cache **no** final shape:
- field initializers can delete or observe properties;
- a base constructor can return another object.

They reach shared shapes through append edges, one hit per field.

## Lookup by text

- Shapes with up to 12 fields compare field text linearly: pointer first, then
  bytes. There is no domain probe and no per-shape hash index.
- Larger shapes and dictionaries keep one index keyed by atom text, so a probe
  hashes the key once.
- Bytecode-linked named operations (M1.3) carry atoms and skip text entirely.
- An identity is reused along a prototype chain only while every store shares
  the domain and the operation holds the key.

## Object size and the domain–root cycle

Stage 2 grew `ObjectData` from 112 to 144 bytes by adding a domain pointer, a
two-word state enum and an edit counter. The target is at most 120 bytes,
enforced by the size test. It is reached with:
- the one-pointer `LayoutNode`;
- the counter in existing padding;
- the domain moved into the shape and dictionary nodes, as the strong owner.

Moving the domain into shapes creates a cycle if the domain owns the roots: the
root owns its children, a child owns the domain, and the domain owns the root.
Roots therefore live in `DomainRoots`, held strongly by each realm that uses the
domain. The domain keeps only its weak caches and atom table. A shape that
outlives its realm keeps the domain alive through its own pointer, as the design
requires for escaping values, and later appends from it still intern correctly.
Nothing on the domain side points back to a shape strongly.

## Work-count obligations (perf-counters)

These are checked before any timed screen. They are counts, not timing:

- **Repeated construction.** Building N objects through the same constructor,
  class-field list, JSON shape or literal creates O(1) shapes, field copies and
  atoms after the first object, including when the previous object is dead.
- **No private domains.** `JSON.parse`, wrapper construction and function or
  array creation create no atom domain and perform no re-link.
- **Append hits.** An append edge hit performs no interning and no field-vector
  comparison. Each add probes once.
- **Lookups.** A small-shape text lookup performs no hash; a large-store lookup
  performs one.
- **Edges.** A repeated `Small`→`Dynamic` growth follows a re-encoding edge.
  Bulk freeze/seal interns no keys.
- **Bounds.** The retention bounds above hold under key churn, per shape and
  per domain.
- **Semantics.** Differential and unit coverage:
  - initializers that delete or observe the instance;
  - base constructors that return another object;
  - equal keys with different attributes;
  - delete then re-add order;
  - more than 32 and more than 128 properties;
  - freezing one literal instance;
  - literals across realms and `eval`;
  - lone-surrogate keys and keys built from separate halves.
