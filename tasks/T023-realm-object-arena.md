# T023: Realm-local object arena

## Status: rejected after S1 fast gate (2026-07-30)

The implementation was deliberately removed after the single frozen attempt.
It replaced `Rc<ObjectData>` with a one-word intrusive strong/weak cell and
routed ordinary constructor receivers through stable Realm-local blocks. The
focused ownership tests, call tests, and a constructor smoke test passed, but
the performance mechanism failed before broad controls or full conformance
were warranted.

The three-block, seeded external fast screen is preserved under
`target/performance-realm-object-arena-fast-external/`. Its external report is
SHA-256 `2123ea9e6b9fd0020c04e4f38349046a3368d928c7b2ccd3f280ac2c127d0843`
and compares candidate binary
`e77efdea39eea6005cf8bc0fa6bd7b9075ff59c868b4d7817a0c977283f4a48a`
against the exact `32a00b0e`-equivalent baseline binary
`568549b735c590a0787d93a84f67d2f3c65312f917e863714a1bb28e6f757ad4`.

| Frozen case | Candidate/base | Gate | Result |
| --- | ---: | ---: | --- |
| HashMap target | 1.016x | <= 0.950x | failed |
| A* control | 1.088x | <= 1.030x | failed |
| cdjs / raytrace | 1.026x / 1.026x | <= 1.030x | within control ceiling, but target already failed |
| access-nbody / Tagcloud | 1.015x / 1.013x | <= 1.030x | within control ceiling, but regressing |
| controlflow-recursive | 0.991x | <= 1.030x | neutral |

The removed object-cell protocol paid more in ordinary strong/weak count and
handle traffic than it saved in allocator traffic. `max_attempts: 1` in the
bound performance unit is exhausted. Do not rebrand or retry intrusive
object-header counts, Realm fixed-block cells, a different packed-count
encoding, or broader object-family admission. A tracing GC is a different
architecture only after a new profile establishes its own shared cost and a
new T022 plan freezes its targets.

## Goal

Remove the per-object system-allocation boundary from ordinary JavaScript
object construction while preserving pointer identity, strong/weak ownership,
and the current Test262 result. This is the next allocation/ownership campaign
after T019 exhausted the safe `Rc`/layout reductions. It is not a benchmark
specialization and it is not a moving or tracing collector yet.

The first slice replaces `Rc<ObjectData>` with a pointer-sized intrusive
strong/weak handle backed by stable, Realm-local object blocks. Ordinary
constructor receivers allocate from those blocks; other allocation sites keep
the same handle protocol and can migrate one semantic family at a time.

## Current evidence and priority

The exact T022 queue at `32a00b0e` ranks `hash-map` second at 5.351608x and
`ai-astar` third at 4.811007x candidate/QuickJS-NG. The rank-one recursive
executor path is closed by independently measured frame-stack, compact-core,
and scalar-recursion attempts, so this campaign legitimately starts at rank
two.

| Priority | Candidate | Expected benefit | Cost | Decision |
| --- | --- | --- | --- | --- |
| 1 | Realm-local fixed-block `ObjectCell` arena | Predicted 5-10% HashMap improvement | High: shared object ownership and weak-reference boundary | **Rejected: 1.016x HashMap, 1.088x A*** |
| 2 | Extend the same arena to ordinary literals/native empty objects | Additional allocation-family coverage after S1 proves the handle protocol | Medium | forbidden: S1 did not prove the handle protocol |
| 3 | Property-entry/`Vec` arena | Potentially material, but it needs a separate current profile and must not repeat T019's rejected descriptor/shape mechanisms | High | separate future candidate only |

The current profile receipts are `/tmp/qjs-current-external.Vyd00J/hash-map.sample`
(`a275eec13951b4f9e86d8cede379eb0f6a46a24bca97d5994075d993a371ae4`) and
`/tmp/qjs-current-external.Vyd00J/ai-astar.sample`
(`0fd5e57fe0e7b3bea21027f94af13593fc78e2a0763d218768a803c7fd5cbf49`).
MallocStackLogging on the first records 23,760 `ObjectRef::with_prototype_slot`
allocations (3,801,600 bytes); A* independently records 10,000 ordinary
receiver allocations (1,600,000 bytes). This is a shared allocation class,
not a source-name or constructor-name condition.

## S1: intrusive object cells and constructor arena routing

`ObjectRef` remains one machine word, but points to an `ObjectCell` containing
explicit packed strong/weak counts and an initialized `ObjectData`. A
`RealmState` owns a fixed-block arena. The arena is held alive while an object
or weak handle from it exists, so a `Value` returned after its VM/Realm drops
remains valid. A weak handle upgrades only while the strong count is nonzero;
dead cells are never reused until all weak handles are gone. Cycles retain the
same reference-counting behavior as today; cycle collection is a later,
separately measured slice.

The implementation may use localized `unsafe` only inside the arena module:
stable block slots make a non-null cell pointer valid for the lifetime proven
by its strong/weak counts. Every raw-pointer operation needs a `SAFETY:`
comment and a focused ownership test. No unsafe access escapes through the
public `ObjectRef` API.

### Scope

- Allowed paths: `crates/qjs-runtime/src/value/object/**`,
  `crates/qjs-runtime/src/value/object.rs`, `crates/qjs-runtime/src/function/env.rs`,
  `crates/qjs-runtime/src/function/call.rs`, focused tests, this task, the
  matching performance-unit plan, and the linked architecture note.
- Forbidden paths: `third_party/**`, benchmark workload identities, input- or
  iteration-specific conditions, global allocators, FFI, and a second VM.
- Owner boundary: serialize on the integration branch; `ObjectRef` is shared
  by all runtime subsystems.

### Semantic risks

- final strong-drop must destroy `ObjectData` exactly once, while a weak cell
  keeps stable identity but cannot expose destroyed data;
- `ObjectWeakRef::{clone,upgrade,drop}` must retain current cache behavior;
- a Realm drop must not invalidate an escaping object or weak reference;
- prototypes, cyclic graphs, property caches, descriptor operations, and
  cross-realm construction must retain pointer identity and ordinary behavior;
- no source shape, property key, or benchmark can affect arena admission.

### S1 acceptance gate

- focused ownership, reuse, weak-upgrade, escaping-value, and constructor
  tests pass;
- `hash-map` is at most 0.95x candidate/base in the frozen same-host fast
  screen; A*, controlflow-recursive, Tagcloud, and broad allocation/call
  controls stay at most 1.03x;
- only a retained fast screen proceeds to complete broad/external evidence,
  curated Test262, `check.sh`, and `compare-qjs.sh`.

## Closed follow-up slices

S2 cannot route bytecode object literals or native empty ordinary objects:
that would broaden a failed ownership mechanism. S3 property-entry payload
storage and any tracing/cycle collector need independent profiles, plans,
memory-pressure policy, and Test262 weak-reference evidence rather than an
opportunistic continuation of S1.

## Verification

```sh
cargo test -p qjs-runtime --lib value::object::
./scripts/check-touched.sh --staged --explain
./scripts/benchmark.sh --candidate <after> --base <before> \
  --quickjs-ng third_party/quickjs-ng/build/qjs --filter allocation \
  --blocks 3 --output /tmp/qjs-object-arena-allocation.jsonl
./scripts/check.sh
./scripts/compare-qjs.sh
./scripts/test262-subset.sh
```
