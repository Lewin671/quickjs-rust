# Generator and Async Suspension Design

Status: implemented; original design approved 2026-06-09. Implementation tracked by
`tasks/T010-generators-iteration-campaign.md` (slices S1-S4) and
`tasks/T007-async-foundation-campaign.md` (slice S5+). Keep this document in
sync as slices land.

## How the VM works today (relevant facts)

- One `Vm<'a>` per invocation (`crates/qjs-runtime/src/bytecode/vm.rs`).
  Calls are Rust-recursive: `Op::Call` -> `call_function`
  (`crates/qjs-runtime/src/function/call.rs`) -> `eval_function_bytecode` ->
  fresh `Vm` runs to completion. There is no shared frame stack to unwind,
  which makes suspension easy: a frame is just the `Vm` field set.
- Frame state = `ip`, value stack, dense slot-indexed `locals`, sparse
  `local_upvalues`, received indexed `upvalues`, `CallEnv`, retained
  `with_stack`, disposable/try stacks, and pending abrupt completions. All are
  owned; `bytecode` is the only borrow, and `Function` owns it as
  `Rc<Bytecode>`.
- Completion plumbing: `Op::Return` -> `return_value()` (runs finally blocks
  via `try_stack`), and `throw_value()` walks `try_stack`
  (`bytecode/vm_try.rs`). Captured bindings are shared `Upvalue` cells, so
  suspension and cross-frame calls require no capture refresh/writeback pass.
- Promise job queue already exists and is drained deterministically after
  script evaluation: `promise/jobs.rs::drain_promise_jobs`, called from
  `eval_bytecode` (`vm.rs:34`). The array-iterator pattern
  (`array/iterator.rs`: prototype object + native `next` + `\0`-prefixed
  state) is the established idiom for iterator objects.

## 1. Suspendable frames

Heap-allocate the frame as a new struct inside the bytecode module (it needs
`pub(super)` types `TryFrame`, `Op`):

- New file `crates/qjs-runtime/src/bytecode/vm_generator.rs`:
  - `GeneratorStart` retains the not-yet-run bytecode, `CallEnv`, received
    upvalues, with stack, and immutable function name. `GeneratorSnapshot`
    additionally owns `ip`, stack, locals, local/received upvalues, try and
    disposable stacks, pending completions, and suspension kind.
  - `pub(crate) enum GeneratorStatus { SuspendedStart, SuspendedYield,
    Executing, Completed }`
  - `pub(crate) struct GeneratorState { status, frame:
    Option<GeneratorFrame>, delegate: Option<Value>, function: Function,
    capture_names: Vec<String> }`, re-exported opaquely from
    `bytecode/mod.rs`.
  - `pub(crate) fn resume_generator_frame(frame: &mut GeneratorFrame,
    completion: ResumeCompletion) -> Result<StepResult, RuntimeError>` —
    builds a `Vm` borrowing `&frame.bytecode` with `std::mem::take`-en
    fields, applies the completion (`Next(v)`: push v; `Throw(v)`:
    `vm.throw_value(v)`; `Return(v)`: `vm.return_value(v)`), runs, then
    moves fields back.
- Change `Vm::run` (`vm.rs:115`) to return `Result<VmExit, RuntimeError>`
  where `enum VmExit { Return(Value), Suspend(SuspendKind, Value) }`.
  `eval_bytecode`/`eval_function_bytecode` map `Suspend` to an internal
  error ("yield outside generator"); only the generator/async drivers
  accept it.
- Generator objects: `Value::Object` whose `ObjectRef` (`value/object.rs`)
  gains one field `internal: Rc<RefCell<Option<InternalSlot>>>` with
  `enum InternalSlot { Generator(Rc<RefCell<GeneratorState>>) }` plus
  accessors. This follows the existing pattern of extra `Rc` fields
  (`to_string_tag`, `raw_json`) and avoids a new `Value` variant.

## 2. Bytecode ops + compiler

- `ir.rs Op` additions: `InitialYield`, `Yield`, `YieldStar`, `Await`
  (slice S5). `Op::NewFunction` gains `kind: FunctionKind`
  (`Normal | Generator | Async`); same field added to `Function`,
  `CompiledUserFunction` (`function/value.rs`), and
  `Stmt::FunctionDecl`/`Expr::Function` lowering sites
  (`compiler_expr.rs:262`, `compiler_values.rs:201`, `compiler.rs` hoisted
  decls).
- `compiler.rs::compile_function`: for generators emit `Op::InitialYield`
  after `compile_parameter_bindings` (spec: defaults/destructuring evaluate
  at call time, like QuickJS-NG's `OP_initial_yield`). `yield expr` ->
  evaluate expr, `Op::Yield` (result of the yield expression is whatever the
  resume pushes). `yield*` -> evaluate iterable, `Op::YieldStar`.
- AST/parser: add `Expr::Yield { argument, delegate, span }` to
  `qjs-ast/src/expression.rs`; `is_generator` on function nodes; parse
  `yield` in assignment position under an `in_generator` parser flag
  (`expression/assignment.rs`); delete the yield-only-body hack in
  `statement/functions.rs:90-192` (existing tests in `tests/functions.rs:848`,
  `tests/statements.rs:51`, `tests/sets.rs` are covered by real generators
  since destructuring/spread already use the iterator protocol).

## 3. Generator object model

New file `crates/qjs-runtime/src/generator.rs` (+ prototype install in
`builtins.rs`):

- `call_function`: if `function.kind == Generator`, build the slot/cell call
  frame, run a fresh `Vm` to
  `Suspend(InitialYield)`, capture into `GeneratorFrame`, wrap in an
  `ObjectRef` with prototype `%GeneratorPrototype%` (object with native
  `next`/`return`/`throw` + `Symbol.iterator` returning `this` — same
  install idiom as `array/iterator.rs`). The body has not run yet; its shared
  upvalue cells already provide the required binding identity.
- `NativeFunction::{GeneratorNext, GeneratorReturn, GeneratorThrow}`
  dispatched in `native.rs`. Each: read `InternalSlot::Generator`; if
  `Executing` -> TypeError (never `RefCell` double-borrow — take the frame
  out of the state, set `Executing`, run, put back); if `Completed` ->
  `{value: undefined, done: true}` (or rethrow/return per spec); if
  `SuspendedStart` and completion is `throw/return` -> complete without
  running the body.
- Resume drives `resume_generator`. `GeneratorOutcome::Yield(v)` ->
  `{value: v, done: false}`, status `SuspendedYield`. `StepResult::Done(v)`
  -> `{value: v, done: true}`, status `Completed`, frame dropped. Error ->
  `Completed` + propagate. After every step, run the
  no generic environment propagation is needed because snapshots retain the
  same local/received cells used by nested closures.
- `yield*`: on `Suspend(YieldStar, iterable)`, the driver (Rust, in
  `generator.rs`) gets the iterator and stores it in
  `GeneratorState.delegate`. While a delegate is set, `next/throw/return`
  forward to the inner iterator's methods; when inner `done` -> clear
  delegate, resume the frame with `Next(inner.value)`. Inner missing
  `throw` -> close inner, inject TypeError into frame; inner `return`
  missing -> resume frame's return path. Keep delegation iterative (loop,
  not recursion).

## 4. Async functions + jobs

Same machinery, different driver (`crates/qjs-runtime/src/async_function.rs`):

- No `InitialYield`; body runs synchronously until first `Op::Await`
  (suspends with the awaited value) or completion.
- `call_function` for `kind == Async`: create a promise object
  (`promise.rs::initialize_promise` is private — expose a
  `pub(crate) fn new_pending_promise(env)` and
  `pub(crate) fn perform_promise_then(promise, on_fulfilled, on_rejected,
  env)` from `promise.rs`). Run the frame; on `Suspend(Await, v)`: resolve
  `v` to a promise (`native_promise_resolve` logic), attach
  `NativeFunction::AsyncFunctionAwaitFulfilled/Rejected` handlers whose
  native context carries the state-holder `ObjectRef` and result promise.
  Handlers resume
  with `Next(value)`/`Throw(reason)` and loop to the next await. On
  `Return(v)` -> `resolve_promise(outer, v)`; on error ->
  `settle_promise(outer, rejected)`.
- Microtask drain already exists and is deterministic: handlers run inside
  `drain_promise_jobs` after script evaluation; no new queue needed.

## 5. Risks

1. Reentrancy: `gen.next()` called from inside the running body must hit the
   `Executing` status check before any `RefCell::borrow_mut` — panics on
   user input are forbidden. Use take/replace, never nested borrows.
2. Env staleness across suspensions: resolved captures are shared `Upvalue`
   cells retained by the suspended frame, so resume performs no generic
   name-based capture refresh/write-back. The remaining dynamic caller refresh
   path must exclude every `is_call_frame_binding` plus internal temporaries;
   otherwise a resume can replace the generator's own `globalThis`, `this`, or
   other frame-only state with the resuming caller's value.
3. Iterator close interplay: `for-of` abrupt exit emits `Op::IteratorClose`
   (`compiler_control.rs:181`), which calls the generator's `return` — that
   must run the frame's `finally` blocks via `return_value`, and a `yield`
   inside `finally` must re-suspend (status back to `SuspendedYield`);
   don't mark `Completed` until `StepResult::Done`.
4. Rust stack depth: each resume is one Rust re-entry (same as a call) — no
   regression, but keep `yield*` delegation iterative.
5. Rc cycles: generator object -> state -> env -> generator object leaks;
   consistent with the engine's existing Rc model, but documented.
6. `requires_scope_call_bindings`/global-name scans (`ir.rs:158+`):
   `NewFunction` kind changes must keep those scans intact.

## 6. Slice plan (one agent session each)

- S1 — Parser/AST: `Expr::Yield`, `is_generator` flags, `in_generator`
  context, remove the yield-only hack; runtime temporarily compiles
  `Expr::Yield` to a clear "generators not yet supported" error.
- S2 — VM suspension core + next(): `VmExit`, `Op::InitialYield`/`Op::Yield`,
  `FunctionKind` threading, `GeneratorFrame`/`GeneratorState`,
  `ObjectRef.internal`, generator creation in `call_function`,
  `GeneratorNext` only.
- S3 — return/throw + finally: `GeneratorReturn/GeneratorThrow`, abrupt
  completions through `try_stack`, yield-in-finally, `IteratorClose`
  integration, capture write-back per step.
- S4 — yield* delegation: `Op::YieldStar`, delegate forwarding, inner
  iterator throw/return protocol.
- S5 — async functions: parser `async`/`await`, `Op::Await`, async driver,
  `perform_promise_then` exposure.
- S6 — conformance + docs: enable generator/async Test262 buckets, compare
  parity, update `docs/architecture.md`.
