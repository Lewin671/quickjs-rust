# T011: Call and harness hot-path performance

> **Status (2026-06-21): subsumed by `T016-environment-model-rewrite.md`.**
> The remaining per-call locals-map clone is removed by T016 S5 (shared upvalue
> cells), which is the structural fix behind the `TypedArray/*` timeouts. Prior
> landed slices stay; new perf work on the binding path goes through T016.

## Goal

Keep Test262 harness-heavy cases from timing out by making ordinary JavaScript
calls, method dispatch, prototype property reads, and dense array element reads
cheap without changing observable semantics. This task tracks the remaining
performance work after the broad environment-model redesign landed.

## Scope

- Allowed paths: `crates/qjs-runtime/**`, focused Test262 allowlist/baseline
  updates when a performance timeout is promoted into a reliable pass, and this
  task note.
- Forbidden paths: `third_party/**`.
- Owner boundary: runtime call/VM/property work is shared architecture; serialize
  broad edits on one branch. Small, isolated builtin fast paths may be split
  only after a focused gap scan identifies independent subtrees.

## Current State

The original flat `HashMap<String, Value>` call-environment model has been
replaced by `CallEnv` (`crates/qjs-runtime/src/function/env.rs`):

- `realm`: shared `Rc<RefCell<HashMap<String, Value>>>` for intrinsics and true
  globals, so reassigned builtins and sloppy globals are visible across frames
  without copying or write-back scans.
- `locals`: per-frame bindings (`this`, parameters, captured locals, caller
  scope bindings). Cloning a `CallEnv` shares the realm and copies only locals.
- Reads clone values out of a short realm borrow. Do not hold a realm borrow
  across user callbacks, getters, setters, Proxy traps, iterators, or
  `valueOf`/`toString`.

Related landed performance work:

- Leaf function calls skip activation captured-env snapshots when the body
  cannot create nested closures/classes.
- Prototype-chain data-property gets use VM fast paths for ordinary reads.
- Dense array present-index reads use `ArrayRef::direct_dense_index_value`
  before falling back to generic property resolution.
- `eval_function_bytecode` returns the live `CallEnv` directly instead of
  rebuilding a frame env at every function return.

## Current Evidence

At commit `18be69650953106355d425fd64412a13c384c648`:

- Latest CI and Test262 Coverage are green.
- Full CI aggregate burndown is recorded in
  `docs/conformance/burndown.jsonl` for 2026-06-15.
- Test262 comparison moved to:
  - quickjs-rust pass: 39781
  - quickjs-rust fail: 2429
  - quickjs-rust timeout: 396
  - quickjs-rust not-run: 66
  - actionable gap: 2773
- Local release probes on this machine:
  - `function f(x){return x+1}` loop, 20k calls: ~0.54s
  - `a.indexOf(4)` loop, 50k calls: ~0.20s
- The known
  `test/built-ins/TypedArray/prototype/set/typedarray-arg-src-backed-by-resizable-buffer.js`
  case still times out at the default 10s case timeout, despite measurable array
  index improvements. Remaining cost is still dominated by high-frequency
  Test262 harness calls such as `assert._isSameValue` and `compareArray`.

## Remaining Work

1. Profile one remaining timeout subtree at a time with `QJS_CLI_PROFILE=release`
   and a focused `find-qjsng-gaps.sh --filter <area> --all` run.
2. Prefer runtime fast paths that are generally valid for ordinary execution:
   function call setup, argument binding, local/global lookup, dense arrays,
   typed-array indexed reads/writes, and non-accessor method dispatch.
3. Keep semantic guard tests near the affected runtime behavior before removing
   any timeout exclusion or adding Test262 cases to the curated subset.
4. After a complete unfiltered CI coverage run, append the generated
   `test262-burndown` artifact with `./scripts/test262-burndown.sh --entry`.

Avoid papering over real runtime cost with broader timeouts or xfail updates
unless the case is intentionally stress-shaped and documented as such.

## Verification Gates

For runtime changes:

```sh
cargo test -p qjs-runtime
./scripts/compare-qjs.sh
./scripts/check.sh
```

For timeout/gap work, also run the focused subtree before and after the change:

```sh
QJS_CLI_PROFILE=release ./scripts/find-qjsng-gaps.sh --filter test/<area> --all --recommend-queue 20
```

For complete conformance accounting, use the CI `Test262 Coverage` artifact or a
local complete unfiltered scan. Do not record partial probes in the burndown
time series.
