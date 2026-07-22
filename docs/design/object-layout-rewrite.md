# Runtime value object-layout rewrite

Status: in progress; S1 and S4 landed, S2 was rejected after measuring no
size reduction, and S3 was skipped for the same layout reason. Campaign
verification is recorded in `tasks/T019-object-layout-rewrite.md`.

## Why this campaign exists

T018 (`tasks/T018-broad-performance.md`) drives quickjs-rust toward
candidate/QuickJS-NG <= 0.50x overall wall ns/op. After 92 measured units the
internal broad-micro portfolio reached ~0.349x, but the mandatory external
generalization suites (JetStream 3 JS subset, Kraken 1.1, SunSpider 1.0)
remain 5-8x *slower* than QuickJS-NG, and the `allocation` critical family has
never dropped below QuickJS-NG parity on any broad screen (2.56x-7.56x across
recorded units). T018's own notes repeatedly identify this as the next
structural bottleneck: "B4 is therefore incomplete... allocation remained
2.559545x" (unit 64), "allocation still 2.835x QuickJS-NG" (unit 92).

`AGENTS.md` names a tracing GC/arena as the eventual replacement for
`Rc`/`RefCell`, but scopes it as "a deliberate, staged subsystem, not a
drive-by change." Before committing to that scope, this campaign checks a
narrower hypothesis first: **QuickJS-NG is not actually GC-free.** Reading
`third_party/quickjs-ng/quickjs.c` confirms it also uses primary reference
counting (`gc_decref`) plus a periodic mark-and-sweep cycle collector
(`JS_RunGC`/`mark_children`) to reclaim cycles refcounting cannot free — the
same conceptual model as `Rc`. It is not a moving/compacting tracing GC. So
QuickJS-NG's allocation-family edge is unlikely to come from "has a GC vs
doesn't"; it more plausibly comes from **`JSObject` being a hand-tuned, flat
15-year-old C struct**, while quickjs-rust's `ObjectData` carries avoidable
per-object layout bloat on top of the same one-malloc-per-object model.

This campaign is scoped to **closing that avoidable layout bloat**, not to
replacing `Rc`/`RefCell` with an arena or tracing collector. If evidence after
this campaign's slices still shows a material allocation-family gap, a
GC/arena rewrite becomes its own separately-scoped, separately-approved
campaign per `AGENTS.md` — this doc does not pre-authorize that larger change.

## Baseline evidence (pre-S1, measured locally on this repo's `object.rs`)

| Type | Size (bytes) |
| --- | ---: |
| `Value` | 16 |
| `Property` (pre-S4) | 56 |
| `PropertyStorage` (pre-S1) | 72 |
| `ObjectData` (pre-S1) | 136 |

`PropertyStorage` is an enum with four variants (`Small`, `Dynamic`,
`Shaped`, `ShapedPair`); its in-memory size is governed by the *largest*
variant regardless of which one is active. `Dynamic { properties:
HashMap<Rc<str>, Property>, order: Vec<Rc<str>> }` is the cold, unbounded
path (72 bytes: a 48-byte `HashMap` plus a 24-byte `Vec`), so every ordinary
object paid for that footprint even while using the much smaller `Shaped`
(32B)/`ShapedPair`(40B)/`Small`(24B) representations. Combined with `Rc`'s
16-byte strong/weak refcount header, a plain two-property object literal
(`{a:1,b:2}`, the T018 `object_allocation` case) allocated 152 bytes.

The official three-role harness (`scripts/benchmark.sh --filter allocation`)
against local release binaries confirmed the same-shape mechanism showed up in
`object_allocation`/`array_allocation` (candidate slower than QuickJS-NG),
while `closure_allocation_call` was roughly at parity — consistent with the
object/array header layout, not closure creation, being the material
generalizable cost.

## Slices

Each slice is one reviewable unit, following the T016/T018 pattern: implement,
verify with `cargo test -p qjs-runtime`, run
`./scripts/check-touched.sh --staged --explain`, record a local three-role
`scripts/benchmark.sh`/`scripts/compare-qjs.sh` A/B before committing, and push
promptly so hosted CI records the formal three-role evidence.

- [x] **S1 — Box the `PropertyStorage::Dynamic` payload.** Move `{ properties:
  HashMap<Rc<str>, Property>, order: Vec<Rc<str>> }` behind
  `Dynamic(Box<DynamicPropertyStorage>)`. This shrinks `PropertyStorage` from
  72 to 40 bytes and `ObjectData` from 136 to 104 bytes (an ~24% smaller
  allocation for every object, regardless of which storage variant is
  active), at the cost of one extra pointer indirection only on the already-
  cold dynamic path. No behavior change; existing `PropertyStorage` unit tests
  and the `ObjectData` size-guard test cover it.
- [x] **S2 — rejected after measurement.** Packing `ObjectData`'s single-bit
  fields left `ObjectData` at 104 bytes because Rust already placed them in
  alignment padding; the attempted indirection was reverted.
- [x] **S3 — skipped after layout arithmetic.** `ArrayData`'s four booleans
  already fit in the padding of its 56-byte aligned layout, so the proposed
  bitset could not shrink it.
- [x] **S4 — move accessor state out of `Property`.** Keep the hot value and
  flags inline; place the cold getter/setter pair behind
  `Option<Rc<AccessorState>>`. This shrinks `Property` from 56 to exactly 32
  bytes, allocates nothing for ordinary data properties, and uses
  `Rc::make_mut` to preserve descriptor clone isolation on mutation. Full
  correctness gates and a valid focused A/B passed. Hosted full/external run
  `29948718553` also passed the generalization gate: overall candidate/base was
  0.992252x and all three external suite geometric means improved.
- [ ] **S5 (open, larger blast radius, do not start without a fresh
  measurement showing it still matters) — evaluate whether `Rc`'s separate
  strong/weak count block is avoidable** for object kinds that are never
  targeted by a `Weak` reference. Requires auditing every `Weak<ObjectData>`/
  `Weak<ArrayData>` use across the runtime; likely not worth the risk unless
  S1-S4 leave a material gap.

## Non-goals

- No change to the `Rc`/`RefCell` ownership model, reference-cycle handling,
  or introduction of a tracing/arena GC. QuickJS-NG's own memory model
  (refcount + cycle-collecting mark/sweep) is the existence proof that this
  campaign's narrower layout scope can close a meaningful part of the gap
  without that larger rewrite.
- No `unsafe` code. Every slice here is a safe, mechanical layout change.
- No change to observable property/array semantics, enumeration order, or
  descriptor behavior.

## Verification

```sh
cargo test -p qjs-runtime --lib value::object::
cargo test -p qjs-runtime --lib value::array::
./scripts/check-touched.sh --staged --explain
./scripts/check.sh
./scripts/compare-qjs.sh
./scripts/benchmark.sh --candidate <after> --base <before> \
  --quickjs-ng third_party/quickjs-ng/build/qjs --filter allocation \
  --blocks 3 --output /tmp/alloc-ab.jsonl
```
