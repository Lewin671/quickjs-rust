# T077 S2 verification ledger (in progress)

Executable candidate: `fc5da912bdd646af2ae05f991fe0618afc961195`, base
`b2daec46`; branch `agent/t077-s2-named-stores/codex`. The subsequent counter
receipt and release-test cfg/counter-label correction do not change executable
engine behavior. No push, merge or main-checkout edit.

## Implementation and deletions

- Functions/arrays use NamedStore; old text/symbol tables and function ordering
  index are gone. Domain ownership reaches every array constructor and installer.
- `value/named.rs` is the borrowed family facade. Map/Set carriers are borrowed
  on hits. Namespace/Proxy/numeric/string/lazy/private hooks stay explicit.
- One weak shape IC serves interpreter, compact/wide, typed loops and leaves:
  mono, allocated four-way, then permanent megamorphic generic resolution.
  Entries store no Value or strong receiver. No legacy exact-value epoch remains.
- Prototype and enumeration dependencies invalidate before structural edits;
  value-only writes preserve them. The first edge guards contextual array
  prototype wrapper identity. Shared-layout broadcasts are deliberately conservative.

Deletion diff at executable candidate vs S2 base (removed/added lines, including
associated migrations rather than claiming every removed line was storage):

| Path under runtime src | Removed | Added |
| --- | ---: | ---: |
| function/value.rs | 519 | 24 |
| value/array.rs | 139 | 204 |
| bytecode/named_property_cache.rs | 597 | 389 |
| bytecode/enumerate_keys_cache.rs | 188 | 63 |
| bytecode/typed_loop/mod.rs | 127 | 2 |
| bytecode/typed_loop/execute.rs | 120 | 28 |
| value/object/slot_reads.rs | 106 | 31 |

The new semantic modules replace deleted implementations; no second named store,
old fallback, dependency, crate, unsafe or binding-model change was added.
Runtime Rust has no CacheProbe, CachedValue, ExactWay, InheritedWay,
LazyFunctionProperties or symbol_properties references in these subsystems.

## Sizes

Native aarch64 and executed x86_64 test binaries agree: ArrayData 64 B,
ArrayColdData 112 B, FunctionData 288 B, FunctionAuxiliaryState 24 B,
IC entry 32 B, IC handle 8 B; debug site state 88 B. Release tests measure site
state 64 B. S1 compile-time assertions remain: KeyId 4, KeyLease 8, ShapeEntry
16, NamedStore 48, RefCell wrapper 56, ObjectData 96, Value 16 bytes.
x86 tests are correctness/size verification, not iMac timing.

## Completed checks (2026-10-30 UTC)

- `TEST262_DIR=/Users/qingyingliu/Code/co-work-space/projects/quickjs-rust/third_party/test262 ./scripts/check.sh`: passes on executable candidate: 2,417 runtime tests, 753 benchmark-tool tests, 35 script tests, feature gates, format/context/size/hooks, 5,170/5,170 curated cases.
- `QJS_VERIFY_IC=1 cargo test -p qjs-runtime --lib --quiet`: 2,417/2,417.
- `QJS_VERIFY_IC=1 TEST262_DIR=... ./scripts/test262-subset.sh`: 5,170/5,170.
- `cargo test -p qjs-runtime --features agents,perf-counters --lib --quiet`: 2,458/2,458.
- `./scripts/compare-qjs.sh`: 259/259 fixtures, including named-shape-hooks.
- `cargo check -p qjs-runtime --target x86_64-apple-darwin --features agents,perf-counters`: pass.
- `cargo test -p qjs-runtime --target x86_64-apple-darwin --lib --features agents,perf-counters named_property_cache::tests -- --nocapture`: 9/9.
- Native/x86 array/function size tests pass; release/perf-counters cache tests 9/9 (the cfg correction makes the test itself compile without debug verification fields).
- Every implementation commit passed its staged fast gate; final documentation gate pending ledger completion.

Focused proofs cover weak lifetime/key recycling, shared live values, four-way
promotion/mega permanence, dictionary rollover/reconfiguration, receiver edge,
all intermediate prototype dependencies, accessor exclusion, namespace cache
exclusion, lazy function slots and array/function own slots. Differential hooks
cover Reflect/super receivers, Proxy effects, ordering/freeze, resize and detach.
Existing module tests cover live namespace cells; configured exact scans remain
the final coverage authority.

## Pending exact and performance standing

Cached and IC-off complete configured release/agents comparisons are running;
no complete-scan count or burndown is asserted yet. Frozen CLI SHA-256:
`4bb228117947e01b6d85a6f54bd75928433e1bb40c1c45f3ebf8579eec8fbb29`.

[Ten-program counter receipt](../archive/T077-s2-counters.md) records checksums,
exact diagnostic binaries, per-program/family events, before/after work and
instrumentation limits. Higher materialization/intern/field-copy work is an
open risk. No timing was collected and no job name or speedup is asserted.
The iMac runner pushes temporary refs; clarification of the user's no-push rule
is pending. The lead owns formal four-aggregate acceptance and historical linker
order revalidation. S2 is not accepted on counters or curated correctness alone.
