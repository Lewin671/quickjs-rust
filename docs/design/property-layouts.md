# Property layouts: construction, transitions and retention

How an ordinary named-property store keeps its layout authority current without rebuilding it; refines
[interpreter-core-v2](interpreter-core-v2.md#properties-atoms-and-layouts-are-the-ordinary-lookup-authority).
[T056](../../tasks/T056-interpreter-core-v2.md) owns status, evidence, plans and the work-count tests that
check these rules; no performance result here.

## Why the first M1.2 shape was closed

The rejected stage-2 shape republished layouts by value (whole-field copy, compare and hash per append), cached
shapes only weakly, gave builders and function tables private domains and hashed text probes twice; instructions
tracked wall time (up to 5.6x). The [diagnosis record](../../tasks/archive/T056-m1-stage2-diagnosis.md) owns the
mechanisms, ablations and jobs. The rules below remove each mechanism as a new implementation shape.

## Dependency and replacement

T063 (branch `agent/string-canonical/codex`, not yet on main) keeps every valid surrogate pair of a stored
string as its scalar, so equal code units have equal bytes. Keys from `JsString` values or linked atoms are then
canonical by construction: probes compare and hash their bytes directly, behind a debug assertion. Untyped host
`&str` entry points (atom interning, host property APIs) canonicalize once at entry.

The layout **replaces** payload lookup authority in the same stage; it is not layered on top:
- an add probes once, and the miss result (canonical key, current shape) feeds the append;
- the payload-side indexes go in the same stage: `Dynamic`'s name map and `ObjectLiteralShape.lookup`;
- payloads keep slots and values only.

## Store ownership

- **Domain.** A store resolves its atom domain once, at creation. No store creates a private domain or
  re-links later.
  - Ordinary objects take it from the prototype. Objects with a null prototype (and `PropertyStorage::dynamic`
    hosts without one) take it from the current realm.
  - Function, array and wrapper named parts take it from the receiver realm.
  - Ordered-data builders (JSON, RegExp/String wrappers) receive it before their first insert.
  - `PropertyStorage::dynamic` (objects built from a host map) builds its layout by appending the ordered keys
    through edges from the domain root. It does not publish a whole field list.
- **Lazy tables.** A function or array named table has no layout until its first ordinary named install;
  semantic hooks (`length`, `name`, `prototype`, indices) need none.
- **One pointer.** The layout is one `Rc<LayoutNode>`: a shared immutable `Shape` or a private `Dictionary`.
  The dictionary node is uniquely owned and edited in place through its cell.
- **Dictionary identity.** A dictionary keeps an independent `identity: Rc<()>` beside its `u64` version. An IC
  may hold a `Weak` of it, so a dead identity can never match a later dictionary at a reused address.
  - The alternative is a monotonic `u64` id per domain. It was rejected because it needs either a domain
    lookup or a stored domain to compare ids across domains.
  - The `Rc<()>` is one allocation per dictionary, which only exists after heavy churn.

## Transitions are edges

A published shape never changes; an object moves to another shape, so a shape pointer stays a valid cache
token (the `token` semantics in `layout/tests.rs` are unchanged).

| Edit | Rule |
| --- | --- |
| Append | Edge label `(owned key, attributes, ordered, payload encoding)`. A hit compares key bytes (pointer first), allocates nothing and copies or compares no field vector. A miss copies the parent's fields once, interns the key, creates the child and records the edge. |
| Payload re-encoding | `Small`→`Dynamic` and `Pair`→`Dynamic` are edges from the current shape. They are not structural edits and are not counted. |
| Delete, reconfigure, reorder | Content-hash cache. The incremental 64-bit hash folds the payload encoding, then `(key, attributes)` per slot, then the order sequence. A full compare runs only on a hash match and also compares encoding, fields and order. |
| Bulk freeze/seal and descriptor batches | Reconfigure only slots whose attributes changed; never re-intern every key. |
| Dictionary switch | After 32 non-append edits, or when an append would exceed 128 fields. Appends never count as edits. |

An operation that reuses a key identity holds a strong `AtomKey` while it does, so a freed key allocation cannot
alias another key (no ABA).

## Edge ownership and the retention bound

- **Ownership.** Parents own their edges strongly; children hold no parent reference, so there is no cycle. With
  weak edges, intermediate shapes die with the object that last used them and every object rebuilds the chain.
- **Roots.** One empty root per payload encoding lives in a `DomainRoots` value.
  - `new_realm` creates one `Rc<DomainRoots>` with the domain.
  - `new_realm_sharing_atoms` passes the same `Rc` on, so every realm of one domain shares one set of roots.
  - The atom domain does not own the roots; see the cycle section.
- **Edge bound.** Each shape holds at most 16 append edges plus one edge per encoding. When full, the oldest edge
  is evicted: its parent stops holding it, and the child is freed with its subtree and their atoms once no object
  uses them.
  - After eviction, a later miss can create a shape with the same content as a live evicted one but a different
    token. That only adds a form an IC may see; it does not affect correctness.
- **Domain budget.** The 4096-shape budget counts *live* shapes created by append or re-encoding misses. The
  counter lives in the domain (shared by all its realms), not in `DomainRoots`. Each shape holds its domain, so
  its `Drop` can decrement the counter.
  - A miss while the counter is at or above the budget still creates the shape, which is counted, but records no
    edge. The object keeps working and the tree does not grow.
- **What is retained.** At most 16 + 5 children per live shape and 4096 live edge-created shapes per domain, plus
  the atoms named on their edges.
- **Churn rule.** This replaces "transitions are weakly retained" and still satisfies "link and transition
  caches never retain arbitrary churn forever":
  - retention is a fixed function of these bounds, not of how many keys a program has used;
  - evicted subtrees are released when unused;
  - dictionaries, which absorb high churn, record no edges.

## Literal and constructor sites

`ObjectLiteralShape` keeps one `(WeakAtomDomain, shape)` slot, replaced when a different live domain uses the
literal; it is never keyed by an address. Creating a literal clones one pointer.

Class fields and constructor stores cache **no** final shape. Field initializers can delete or observe
properties, and a base constructor can return another object. They reach shared shapes through append edges, one
hit per field.

## Lookup by text

- Shapes with up to 12 fields compare field text linearly, pointer first and then bytes. There is no domain probe
  and no per-shape index.
- Larger shapes and dictionaries keep one index keyed by atom text, so a probe hashes once.
- Bytecode-linked named operations (M1.3) carry atoms and skip text entirely.
- An identity is reused along a prototype chain only while every store shares the domain and the operation holds
  the key.

## Object size and the domain–root cycle

**Layout today.** On main `ObjectData` is 112 bytes:
- `RefCell<PropertyStorage>`: 48 bytes;
- three `u64`/`usize` cells: 24 bytes;
- `RefCell<Option<Prototype>>`: 24 bytes;
- the cold `OnceCell<Box<_>>`: 8 bytes;
- eight one-byte cells: 8 bytes. Seven are `Cell<bool>` (`extensible`, `raw_json`, `array_prototype_exotic`,
  `typed_array_exotic`, `immutable_prototype_exotic`, `module_namespace_exotic`, `string_indices_pending`) and
  one is `Cell<SymbolBrand>`.

Stage 2 added a domain, a two-word state and a counter to `PropertyStorage`, which made it 144 bytes.

**Target: at most 120 bytes**, locked by the size test. It is reached with:
- the one-pointer `LayoutNode`, which adds 8 bytes to `PropertyStorage`;
- the eight flags packed into one `Cell<u16>` bit set (7 bools plus 2 bits of brand, 9 bits). This frees 6 bytes;
- the `u8` edit counter placed in the freed space;
- the domain moved into the shape and dictionary nodes.

**The cycle.** Moving the domain into shapes creates a cycle if the domain owned the roots: root → child →
domain → root. `DomainRoots` therefore sits outside the domain, held by its realms. The domain keeps only its
weak caches, its atom table and the live-shape counter. A shape that outlives its realm keeps the domain alive
through its own pointer, as escaping values require, and later appends from it still intern correctly. Nothing on
the domain side points back to a shape strongly.
