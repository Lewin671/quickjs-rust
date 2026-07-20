# T019: Runtime value object-layout rewrite

## Goal

Close the `allocation` critical family's gap to QuickJS-NG (T018's
`benchmarks/manifest.json`) by shrinking the per-object memory footprint of
`ObjectData`/`ArrayData` and their property storage, without changing
observable semantics. This is a **narrower alternative to a full GC/arena
rewrite**: `third_party/quickjs-ng/quickjs.c` shows QuickJS-NG itself uses
reference counting plus a periodic cycle-collecting mark/sweep pass
(`gc_decref`/`JS_RunGC`), not a tracing/moving GC, so its allocation-family
edge over quickjs-rust is attributed to `JSObject`'s flat, hand-tuned C
layout rather than to having a fundamentally different memory model. Full
design and slice rationale: `docs/design/object-layout-rewrite.md`.

This campaign feeds T018's B3/B4 milestones (structural bottleneck
optimization toward <= 0.50x overall, <= 1.00x per critical family). It does
not replace or block T018; each slice remains a T018-style separately
measured, reviewable unit.

## Why this was opened as its own task

The `allocation` critical family has never reached QuickJS-NG parity across
92 recorded T018 units (2.56x-7.56x depending on revision), and T018's own
notes call out layout/allocation work as the next lever twice (units 64 and
92). `AGENTS.md` scopes a GC/arena replacement of `Rc`/`RefCell` as "a
deliberate, staged subsystem, not a drive-by change" — the same reasoning
that made T016 its own campaign rather than a drive-by fix inside T011/T014.
This task exists so the narrower, lower-risk object-layout hypothesis is
tried, measured, and recorded before any larger GC/arena campaign is opened.

## Slices

Each slice is one reviewable unit. Verify with the slice's focused command,
`./scripts/check-touched.sh --staged --explain`, and a local three-role
`scripts/benchmark.sh --filter allocation` A/B before committing; push
promptly so hosted CI records the formal three-role evidence.

- [x] **S1 — Box `PropertyStorage::Dynamic`'s payload.** Commit (this task's
  first commit) moves the cold, unbounded `{ properties: HashMap<Rc<str>,
  Property>, order: Vec<Rc<str>> }` case behind `Dynamic(Box<
  DynamicPropertyStorage>)`. Measured: `PropertyStorage` 72 -> 40 bytes,
  `ObjectData` 136 -> 104 bytes (~24% smaller allocation for every object,
  regardless of which storage variant is active). The `ObjectData` size-guard
  unit test is tightened from `<= 160` to `<= 112` bytes. All existing
  `PropertyStorage`/`ObjectData` unit tests pass unchanged; no behavior or
  public-API change.
- [ ] **S2 — Pack `ObjectData`'s boolean/brand `Cell` fields into one
  `Cell<u8>`.**
- [ ] **S3 — Pack `ArrayData`'s boolean flag `Cell`s into one `Cell<u8>`.**
- [ ] **S4 — Audit and narrow `Property`'s 56-byte footprint.**
- [ ] **S5 (open, gate on a fresh measurement first) — evaluate whether
  `Rc`'s strong/weak refcount block is avoidable for object kinds never
  targeted by `Weak`.**

## Scope

- Allowed paths: `crates/qjs-runtime/src/value/**` (object/array/property
  layout only), plus focused tests next to that code.
- Forbidden paths: `third_party/**`. No change to `bytecode/**` dispatch, the
  `Rc`/`RefCell` ownership model, or environment/upvalue representation
  (T016's territory) unless a slice's design doc entry explicitly says so.
- Owner boundary: serialize on one branch — this touches the shared `Value`
  object/array representation used everywhere.

## Acceptance criteria

- [ ] Each slice's gate passes; `./scripts/check.sh` and
  `./scripts/compare-qjs.sh` stay green at every slice boundary.
- [ ] Every slice's local A/B (`scripts/benchmark.sh --filter allocation`,
  3 blocks) shows the changed mechanism does not regress any critical family;
  record the ns/op deltas in this file's Notes section per slice, matching
  T018's evidence style.
- [ ] No observable change to property/array descriptor semantics,
  enumeration order, or existing Test262 pass state (`find-qjsng-gaps.sh
  --exact --all --filter test/built-ins/Object` and `.../Array` report zero
  new actionable gaps after the campaign's final slice).

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

## Notes

Opened 2026-07-20 after confirming QuickJS-NG's memory model is refcount +
cycle-collecting mark/sweep (not a tracing/moving GC), which narrows the
allocation-family gap hypothesis from "needs a GC" to "needs a flatter object
layout."

### S1 local evidence

A local three-role `scripts/benchmark.sh` run (Apple Silicon, macOS, 3 blocks,
before/after release binaries built from the same tree with only the S1 diff
stashed/unstashed, `third_party/quickjs-ng/build/qjs` as the reference) over
all 25 broad-micro cases:

- `object_allocation`: candidate/base **0.988x** (candidate/QuickJS-NG
  1.413x, down from base's 1.430x);
- `array_allocation`: candidate/base **0.988x** (candidate/QuickJS-NG 1.227x,
  down from base's 1.242x);
- `closure_allocation_call`: candidate/base 1.004x (neutral, within noise —
  expected, since this case does not exercise `PropertyStorage`);
- no other case among the 25 moved outside ordinary run-to-run noise
  (all other candidate/base ratios were within ~±2.5%, matching this
  session's measurement noise floor, not a directional regression).

This is a modest, consistent, non-regressing win, not a family-closing one:
`object_allocation` and `array_allocation` remain well above QuickJS-NG
parity. Do not credit this campaign with closing T018's `allocation` family
until hosted three-role CI evidence and external-suite generalization
(JetStream/Kraken/SunSpider) both show a repeatable improvement, per T018's
own General Optimization Acceptance Rule, and until S2-S4 compound with S1.
