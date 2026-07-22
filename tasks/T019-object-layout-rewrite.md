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
- [x] **S2 — rejected.** Attempted packing `ObjectData`'s six `Cell<bool>`
  fields plus `symbol_brand` into one `Cell<u8>`. Measured `size_of::<
  ObjectData>()` before and after: **104 bytes both times, zero reduction.**
  Rust's default (non-`repr(C)`) struct layout already reorders and packs
  single-byte fields into existing alignment padding, so seven separately
  declared single-byte `Cell`s cost nothing beyond what one packed byte would
  — the struct's size is dictated by its largest-aligned field (pointers,
  needing 8-byte alignment) plus whatever padding remains, and that padding
  already absorbed the bools. A full three-role local A/B (25 cases, 3
  blocks) confirmed no case moved outside +/-2.1% noise. Reverted rather than
  keeping unjustified indirection, per T018's own acceptance discipline.
- [x] **S3 — skipped without implementing.** Same physics as S2:
  `ArrayData`'s four `Cell<bool>` flags (`length_writable`, `extensible`,
  `sealed`, `frozen`) already fit inside the padding between its 52 bytes of
  addressable fields and the 56-byte aligned total (`RefCell<Vec<Value>>` 32B
  + `Cell<usize>` 8B + 4 bools 4B + `OnceCell<Box<..>>` 8B = 52, padded to
  56). Packing them into one byte cannot reduce a total that padding already
  covers. Do not re-attempt bitset-packing of scattered single-byte `Cell`
  fields in this codebase without first confirming the struct's *unpacked*
  field sum crosses an alignment boundary the packed version would avoid.
- [x] **S4 — move accessor state out of every `Property`.** `Property` now
  keeps its hot `Value` and descriptor flags inline and stores the cold
  `{ get, set }` pair behind a private `Option<Rc<AccessorState>>`, shrinking
  the exact layout from 56 to 32 bytes. Plain data properties allocate no
  accessor state. Clones share immutable accessor state, while setter/getter
  mutation uses `Rc::make_mut` so cloned descriptors retain value semantics.
  The mechanical call-site migration spans the runtime modules that directly
  inspected the old fields; focused tests cover empty accessors, exact size,
  shared clones, and copy-on-write isolation. Branch CI, 1,539 runtime tests,
  the 5,147-case Test262 subset, and QuickJS-NG comparisons passed. Local A/B
  found no credible property or control regression; hosted full/external
  evidence remains the rollback gate.
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

Hosted `Performance Preview` for commit `cfde2fbd` (run `29767781513`)
completed all engine builds and measurement but exited with `report
comparison input is incomplete` — the harness's own fail-closed non-claim
policy, matching prior recorded incomplete-measurement attempts (e.g. T018
unit 92 attempt 1). The separate correctness `CI` job (`29767781545`) and
`Test262 Coverage` (`29768040632`) both passed. This is hosted-runner
measurement variance, not a regression signal; do not re-push solely to
retry the preview.

### S2/S3 result: bitset-packing scattered `Cell<bool>` fields does not help

See the S2/S3 checklist entries above. This is the campaign's most useful
negative result so far: Rust's default struct layout already packs
single-byte fields into existing alignment padding, so consolidating them
into an explicit `Cell<u8>` only helps when the *unpacked* field sum crosses
an alignment boundary the packed form would avoid. Verify this arithmetic
before proposing another bitset-packing slice.

### S4 local evidence: cold accessor state

The first seven-case run was invalid and remains diagnostic only: 70/98
formal measurements were eligible, 28 were timer-limited, and its apparent
`object_allocation` ratio of 1.057750 therefore cannot accept or reject the
change (raw SHA-256
`4cc608b654ee33630a59711a02e6a9382489508ccca6583fa7fa9be8afe37ac`).

A fresh five-case rescreen (seven blocks, seed `20250714`) produced 70/70
eligible formal measurements and all 80 expected linearity records; every
official median linearity ratio was inside `[0.85, 1.15]`. The frozen
shared-block bootstrap primitive gave:

- property aggregate: **1.000457x**, 95% CI `[0.994979, 1.009800]`;
- all five selected cases: **1.000911x**, 95% CI
  `[0.996003, 1.015615]`;
- controls: **1.001592x**, 95% CI `[0.995575, 1.029001]`;
- per-case paired ratios: `property_read` 0.999463,
  `property_dynamic_read` 0.999063, `property_write` 1.002849,
  `object_allocation` 1.003722, and `plain_function_call` 0.999466.

All intervals cross 1.0 and no control has a stable >3% regression. The raw
evidence SHA-256 is
`883dce6d95b1579b4d99255ff62dce4265169bdacfdcbd4cfcd0d3b8fc1866ab`.
Because this deliberately focused selection is not the complete 25-case
portfolio, it is not represented as a portfolio-complete report; hosted full
and external suites remain required generalization evidence.

### A much larger lever found outside this campaign's scope: global `var` sync

While re-measuring, the full 25-case candidate/QuickJS-NG standing (from the
S1 A/B raw data, not the stale initial-baseline table in T018) showed
`top_level_function_call` at **9.897x** QuickJS-NG — by far the worst case in
the portfolio, dwarfing every `allocation` case. `dynamic_method_call`
(3.854x) and `array_write` (2.446x) are also larger than anything in this
campaign's scope. Root-caused `top_level_function_call` to
`Vm::store_local_slow` (`crates/qjs-runtime/src/bytecode/vm_bindings.rs`):
every write to a hoisted top-level `var` inside `syncs_global_var` re-fetches
`globalThis` from the realm's binding map, calls
`global_this.has_own_property(name)`, clones the binding name `String` up to
three times, and does two to four more `HashMap` lookups/inserts (`realm
.borrow().contains_key`, `env.insert_realm`, `env.has_local_binding`,
`env.insert`) — all on every loop iteration, not just once. This is **not**
T019 scope (it is VM dispatch / environment-sync work, not object/array
layout) and it is **not** a safe quick fix: it sits in the same
historically fragile realm/globalThis-sync territory that took many
dedicated sessions to stabilize (see memory `Parity progress` sessions on
realm semantics). Recorded here and in `T018-broad-performance.md` as the
clear next priority, deliberately not attempted without a full session's
verification budget (focused Annex B / sloppy-var Test262 scans before and
after, not just the broad-micro portfolio).
