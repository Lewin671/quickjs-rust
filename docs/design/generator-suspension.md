# Suspension Invariants

What state and completion invariants survive a suspension? This describes the
code for generators, async functions, async generators and top-level `await`;
the original plan is
[design-generator-suspension-plan.md](../../tasks/archive/design-generator-suspension-plan.md).
Source paths are relative to `crates/qjs-runtime/src/`; tests are named
`tests/<file>.rs::<test>`.

## One mechanism, three drivers

`bytecode/vm_generator.rs` owns the state machine. A body suspends at
`Op::Yield`, `Op::Await` or `Op::YieldDelegate`; `drive` turns the
interpreter's `Completion` into a `GeneratorState` transition and a
`GeneratorOutcome`, which one of three drivers interprets:

- `generator.rs` maps it to an iterator result for `next`/`return`/`throw`.
- `async_function.rs` schedules promise reactions that resume the body from
  the job queue, and settles the result promise on return or error. A module
  body with top-level `await` uses the same driver (`drive_async_module`).
- `async_generator.rs` keeps a FIFO request queue; an `await` resumes through
  the job queue and a consumer-facing yield settles the front request.

The state lives in one place, the object's `generator_state` cell
(`value/object.rs`); an async function uses an internal context object. A
generator that a closure in its own cells refers to is an `Rc` cycle and is
never freed.

## What a suspended frame owns

`GeneratorSnapshot` owns the `Rc<Bytecode>`, `ip`, operand stack, `locals`,
`local_upvalues`, received `upvalues`, `CallEnv`, `with_stack`, the immutable
function name, the try stack, disposable scopes, the pending
throw/return/jump of an in-flight `finally`, and the suspension kind. Resume
builds a fresh `Vm` over the held bytecode, moves these back, and recomputes
the derived slot masks (`refresh_authoritative_slots`).

1. **Cell identity survives.** The snapshot holds the same `Upvalue` cells
   as the closures and declaring frames. Suspend and resume copy no binding
   value, run no refresh or write-back, and do not consult the resuming
   caller's environment ([env-model-rewrite.md](env-model-rewrite.md)). Tests:
   `generators.rs::suspended_generator_keeps_parameter_and_var_capture_cells`,
   `async_functions.rs::await_resume_observes_sibling_closure_capture_writes`.
2. **Try state survives.** Test:
   `frame_cold_state.rs::a_generator_carries_its_try_state_across_suspension`.
3. **Nothing scalar-replaced crosses a suspension.** Virtual-object lowering
   treats a suspension point as an escape (`EscapeReason::Suspension`,
   `bytecode/virtual_object.rs`), so a snapshot carries no virtual state.

## Start

Calling a generator or async generator runs the parameter prologue
synchronously (`start_suspended_at_body`) and stops at
`Op::FunctionPrologueEnd`: a parameter-binding error throws at the call,
before the object exists, and the body has not run. An async function is
stored as `SuspendedStart` and driven at once to its first `await` or
completion; every error, including a parameter-binding error, rejects the
returned promise. Tests:
`generators.rs::calling_a_generator_runs_the_parameter_prologue`,
`async_functions.rs::parameter_binding_error_rejects`.

## Re-entrancy

`resume_generator` takes the state out of the cell and leaves `Executing`
before any user code runs, and holds no `RefCell` borrow across the body. A
nested `next` on the same generator gets a `TypeError`, never a double-borrow
panic. Test: `generators.rs::reentrant_next_is_type_error`.

An async generator queues instead: `enqueue` appends the request and returns
if `draining` is set (`async_generators.rs::overlapping_next_calls_are_fifo`).

## Completion rules

- `next(v)` makes `v` the value of the suspended `yield`; the first is ignored.
- `throw(v)` raises `v` at the suspension point (`throw_value`), so the
  body's `catch` and `finally` run. `return(v)` injects a return completion
  (`return_value`): each enclosing `finally` runs, and with none the
  generator completes with `v` at once.
- A `finally` may override the injected completion. A suspension inside it
  keeps the pending completion in the snapshot. The state becomes
  `Completed` only when the body returns or an error leaves it, and an error
  always completes the generator (`finish`).
- Before the body starts, `return(v)` completes with `v` and `throw(v)`
  completes and rethrows; the body does not run. After completion `next`
  gives `{ value: undefined, done: true }`, `return(v)` gives
  `{ value: v, done: true }`, and `throw(v)` rethrows.
- Iterator close is `return`: an abrupt exit from `for-of` calls the
  generator's `return`, which runs its `finally` blocks by the rule above.

Tests: `generators.rs::return_mid_yield_runs_finally`,
`::finally_can_override_return_completion`, `::return_before_start`,
`::throw_before_start`, `::return_after_completion`,
`::throw_uncaught_propagates_and_completes`,
`::for_of_early_break_calls_return`.

## Delegation (`yield*`)

Delegation runs inside the body, in `Op::YieldDelegate`. The suspension kind
records that the frame stopped inside a delegation; on resume the request is
staged as a `resume_mode` that the re-entered operation forwards to the inner
iterator's `next`, `throw` or `return`.

- The inner result object reaches the consumer unchanged, not rebuilt.
- The value of the `yield*` expression is the inner iterator's return value.
- `throw` to an inner iterator without `throw` closes it and raises a
  `TypeError`; `return` to one without `return` runs the outer `finally`.

Tests: the `yield_delegation_*` tests in `generators.rs`.

## Async generator awaits

- `yield v` awaits `v` before the request settles with
  `{ value, done: false }`.
- `return(v)` at a plain `yield` awaits `v`, then resumes the body with the
  return completion. `ReturnAlreadyAwaited` marks a value that has been
  awaited so it is not unwrapped twice.
- An uncaught error rejects the front request and completes the generator.

Tests: `async_generators.rs::yield_awaits_its_operand`,
`::return_at_suspended_yield_thenable_is_not_unwrapped_twice`.
