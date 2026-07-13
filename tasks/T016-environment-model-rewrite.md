# T016: Environment / binding model rewrite (shared upvalue cells)

## Goal

Replace the per-frame `HashMap<String, Value>` snapshot + `captured_env` cell +
`CaptureWriteback` heuristic trio with a single representation: **slot-indexed
locals plus indexed shared upvalue cells** (`Upvalue = Rc<RefCell<Value>>`).
This is the keystone named in `AGENTS.md`. Full design and slice rationale:
`docs/design/env-model-rewrite.md`.

This campaign subsumes:

- `T014-var-closure-binding-staleness.md` — capture staleness is removed at the
  root because every captured binding is one shared cell, read/written by index.
- `T011-call-performance.md` — `with_frame_locals` / `function_capture_env` stop
  cloning a per-call name-keyed locals map, cutting the dominant nested-call
  cost behind the ~536 `TypedArray/*` timeouts.

It also targets the M2 class-method capture, per-iteration `let`, generator
resume-capture, and direct-`eval` capture failure clusters, which share the same
root.

## Why a campaign (not a drive-by)

Blast radius is ~90 call sites across 12 files in `bytecode/**` and
`function/**`, against ~42k passing cases. The only safe path is flag-gated
coexistence: the old name-keyed model and the new cell model run side by side,
and each binding *class* flips to cells independently with its own gate, until
the final slice deletes the old model. See the design doc for the structural map.

## Slices

Each slice is one reviewable unit. Verify with the slice's focused command and
`./scripts/check.sh` + `./scripts/compare-qjs.sh` before push; revert on any
regression (no half-finished cutover).

- [x] **S1 — Upvalue type + resolver classification (no behavior change).**
  - [x] `function::upvalue::Upvalue` shared-cell type (commit "Add Upvalue
    shared-cell type", unit-tested for shared-write visibility + `ptr_eq`).
  - [x] `bytecode::upvalue_resolver`: pure classification (cell slots, received
    upvalue slots, per-child indexed `UpvalueSource`) over `Op::NewFunction`
    captures, six unit tests (commit "Add upvalue classification resolver").
  - [x] Call-path benchmark (`function_call`/`closure_call` in
    `tests/benchmarks/quickjs/microbench.js`). **Baseline: a user call is ~70µs
    in quickjs-rust vs ~0.12µs in QuickJS-NG (~570x).** This is the root of the
    Test262 `timeout` bucket (109 cases, incl. RegExp `\p{}` ~44 which is NOT a
    feature gap — `\p{}` is implemented and binary-search-fast; the cost is the
    harness building/scanning ~1M code points through the ~70µs-per-call VM).
    Caller-local count does not affect call cost (the snapshot's
    caller-binding copy is not the dominant term); the ~70µs is distributed
    across the per-call `HashMap`/`Vec`/`String` allocation in
    `function::call::function_env` + `CallEnv` construction + `Vm` setup.
  - Re-scoped: rather than add distinct `LoadUpvalue`/`StoreUpvalue` ops, S2
    folded cell access into the existing local ops through
    `Vm.local_upvalues`, the alternative explicitly allowed by the design.
- [x] **S2 — Cells for the simplest captured `let`/`const`** (non-shadowing,
  one nested closure, read+write). `Function.upvalues` carries the shared cells
  into the callee frame; `load_local` reads the cell first and `store_local`
  updates it, so the declaring frame and sibling closures observe one binding.
  The legacy name-keyed environment remains as the coexistence path for binding
  classes removed by S3-S6. Gate: the T014 counter-callback repro and focused
  parent/sibling lexical-cell tests in `closure_state` pass.
- [x] **S3 — Shadowing + nested/multiple closures + per-iteration loop cells.**
  Generic lexical bindings now retain their source names; distinct slot and
  `Upvalue` identities replace `\0lexical:<name>:<slot>` mangling. Dynamic
  `CallEnv` round-trips select the innermost active same-named slot. A received
  cell no longer also takes the legacy name-based writeback/global-sync path,
  preventing shadowed bindings from aliasing; module namespace live exports
  retain an explicitly marked bridge until S5. `FreshIterationScope` replaces
  each captured loop-head cell from the current value at the first iteration
  boundary and every back-edge. Gate: exact Test262 `statements/for` and
  `statements/class` scans report zero actionable gaps; focused shadowing,
  class inner-name, Annex B, module-live-binding, and C-style `for` cell tests
  pass.
  - Note (2026-06-22, commit c84162c3): the **per-iteration shadowing leak** that
    this slice targets — a `for (let x of/in …)` head whose `x` shadows an outer
    `let x` writing the inner value back onto the outer slot/cell — was fixed
    *contained* in the current name-keyed model (the inner binding rides under its
    mangled key; `apply_env` skips the plain-name alias when a shadowing lexical
    is active, and the per-iteration write-skip records both spellings). So this
    leak no longer needs the cell migration. S3 subsequently removed the
    generic mangled-key mechanism and covered M2 class inner-name capture plus
    the C-style `for(;;)` CreatePerIterationEnvironment split between the
    initializer binding and per-iteration copies.
- [x] **S4 — Generators/async + parameter/function-scope captures.** Parameters
  and body `var`/function declarations captured by a nested function are now
  classified as parent-local cells; only true received captures occupy the
  incoming upvalue vector. Generator, async-function, and async-generator
  snapshots retain `upvalues` plus the declaring frame's `local_upvalues`, so
  suspension preserves cell identity. The per-step generic capture refresh and
  write-back passes are deleted. Dynamic Function constructors with an explicit
  marked realm synchronize existing global properties at the write site, and
  generator caller-refresh paths exclude all call-frame bindings. The literal
  string-append fast path reads received cells before mutating, and TLA modules
  explicitly retain the temporary live-export cell bridge. Direct-eval source
  keeps dynamically introduced/deletable function-level bindings on the legacy
  name bridge until S6, while its ordinary block lexicals retain the S3 cell
  bridge. Gate: focused resolver/compiler, closure, module, generator,
  async-function, and async-generator suites pass, including new parameter +
  `var` capture tests across suspension.
- [ ] **S5 — Delete the old model.** Remove `Function.env`/`captured_env`/
  `capture_writeback`, the `vm_capture.rs` refresh family,
  `write_through_capture_writeback_slot`, and the `CallEnv` locals HashMap +
  `with_frame_locals` clone. Realizes the T011 win; record a burndown entry and
  re-measure the benchmark.
- [ ] **S6 — direct-`eval` / `with` deopt on cells.** Name→cell deopt map gated
  on `contains_direct_eval`/`with`; close the eval-capture failures.

## Scope

- Allowed paths: `crates/qjs-runtime/src/bytecode/**`,
  `crates/qjs-runtime/src/function/**`, plus the resolver in the compiler
  (`bytecode/compiler*.rs`, `bytecode/ir.rs`). New ops touch `bytecode/ir.rs`.
- Forbidden paths: `third_party/**`. No `qjs-parser`/`qjs-ast` changes (AST is
  unchanged; this is a lowering/runtime rewrite).
- Owner boundary: **serialize on one branch.** Touches shared VM binding code;
  never run in parallel with other runtime work.

## Acceptance criteria

- Each slice's gate passes; the engine is green at every slice boundary.
- After S5: the per-call locals HashMap clone is gone, the call benchmark
  improves measurably, and a full `--exact --all` burndown shows
  `actionable_gap` and the timeout bucket down, with no previously-passing case
  lost.
- After S3/S6: the M2, per-iteration, and eval-capture clusters pass.

## Verification

```sh
# T014 root repro (must return 1 once S2 lands):
cargo run -p qjs-cli -- -e 'var c=9; function inc(){c++;} function f(){c=0;inc();} f(); c;'
./scripts/check.sh
./scripts/compare-qjs.sh
./scripts/find-qjsng-gaps.sh --exact --all --filter test/language
./scripts/test262-burndown.sh --report <dir>   # after S5 full scan
```

## Notes

Proposed 2026-06-21 after the goal was retargeted to performance + full
conformance (`AGENTS.md`). The prior incremental fixes under T014 (leaf-call
slot refresh) stay correct but are made redundant by S2+; do not extend the
heuristic model further — land cells instead.
