//! Async function evaluation (ES2023 27.7): suspend/resume on `await`, returning
//! a promise that settles with the body's completion.
//!
//! An async function reuses the generator suspend/resume machinery. Calling one
//! captures its call frame, creates the promise it returns, and drives the body
//! until the first `await` (compiled to the same `Op::Yield` suspension point a
//! generator uses) or completion. `await v` resolves `v` to a promise and
//! schedules reactions that resume the suspended body via the realm job queue,
//! so code after `await` always runs in a later microtask — never synchronously.
//!
//! The resumable state lives on an internal "async context" object reusing the
//! same `[[GeneratorState]]` cell generators use; the await reactions are native
//! functions that carry that object plus the result promise in their captured
//! environment and call back into the driver.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::CallEnv;
use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value,
    bytecode::{
        CaptureWriteback, GeneratorOutcome, GeneratorStart, GeneratorState, Resume,
        resume_generator,
    },
    function_constructor_as_prototype_slot, function_intrinsic_prototype_slot, promise, symbol,
};

/// Intrinsic binding for `%AsyncFunction.prototype%`, the object that sits
/// between an async function and `%Function.prototype%` in its prototype chain.
pub(crate) const ASYNC_FUNCTION_PROTOTYPE_BINDING: &str = "\0AsyncFunctionPrototype";

/// Internal slot names carried in an await-reaction native's environment.
const ASYNC_CONTEXT: &str = "\0AsyncContext";
const ASYNC_RESULT_PROMISE: &str = "\0AsyncResultPromise";

/// Installs `%AsyncFunction.prototype%`: an ordinary object whose `[[Prototype]]`
/// is `%Function.prototype%`, carrying the "AsyncFunction" `Symbol.toStringTag`.
/// The callable `%AsyncFunction%` constructor is reachable through async
/// function `.constructor`, but is not exposed as a global binding.
pub(crate) fn install_async_function(
    env: &mut CallEnv,
    _global_this: &Value,
    object_prototype: ObjectRef,
) {
    let async_function_prototype = ObjectRef::with_prototype_slot(
        HashMap::new(),
        function_intrinsic_prototype_slot(env).or(Some(crate::Prototype::Object(object_prototype))),
    );
    let async_function = Function::new_native(
        Some("AsyncFunction"),
        1,
        NativeFunction::AsyncFunction,
        true,
    );
    let _ = async_function.set_internal_prototype_slot(function_constructor_as_prototype_slot(env));
    async_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::data(
            Value::Object(async_function_prototype.clone()),
            false,
            false,
            false,
        ),
    );
    async_function_prototype.set_to_string_tag("AsyncFunction");
    symbol::define_well_known_to_string_tag(env, &async_function_prototype, "AsyncFunction");
    async_function_prototype.define_property(
        "constructor".to_owned(),
        Property::data(Value::Function(async_function), false, false, true),
    );
    env.insert_realm(
        ASYNC_FUNCTION_PROTOTYPE_BINDING.to_owned(),
        Value::Object(async_function_prototype),
    );
}

/// Returns `%AsyncFunction.prototype%` from the current environment.
pub(crate) fn async_function_prototype(env: &CallEnv) -> Option<ObjectRef> {
    match env.get(ASYNC_FUNCTION_PROTOTYPE_BINDING) {
        Some(Value::Object(object)) => Some(object.clone()),
        _ => None,
    }
}

/// Calls an async function: captures the call frame, creates the returned
/// promise, and drives the body to its first `await` or completion. Always
/// returns the promise (never the body's value directly); body errors — including
/// ones raised before the first `await`, such as parameter-binding failures —
/// reject the promise rather than throwing synchronously.
pub(crate) fn call_async_function(
    function: &Function,
    function_env: CallEnv,
    function_capture_names: Vec<String>,
    env: &mut CallEnv,
) -> Value {
    let bytecode = function
        .bytecode
        .clone()
        .expect("async function has a bytecode body");
    let captured = Rc::new(RefCell::new(function_env.snapshot_locals()));
    let mut capture_names = function_capture_names;
    {
        let captured = function.captured_env.borrow();
        for name in captured.keys() {
            if crate::function::is_internal_binding_name(name)
                || matches!(name.as_str(), "this" | "arguments")
            {
                continue;
            }
            if bytecode.local_slot(name).is_some()
                && !capture_names.iter().any(|existing| existing == name)
            {
                capture_names.push(name.clone());
            }
        }
    }
    let parent_writeback = function.capture_writeback.clone().map(Box::new);
    let syncs_cell_values = parent_writeback
        .as_deref()
        .is_some_and(CaptureWriteback::syncs_cell_values);
    let capture_writeback =
        (!capture_names.is_empty() || parent_writeback.is_some()).then(|| CaptureWriteback {
            target: Rc::clone(&function.captured_env),
            names: capture_names,
            aliases: Vec::new(),
            parent: parent_writeback,
            syncs_cell_values,
        });
    let context = ObjectRef::new(HashMap::new());
    *context.generator_state().borrow_mut() =
        Some(GeneratorState::SuspendedStart(Box::new(GeneratorStart {
            bytecode,
            env: function_env,
            captured_env: captured,
            upvalues: function.upvalues.clone(),
            with_stack: function.with_stack.clone(),
            immutable_function_name: function
                .immutable_name_binding
                .then(|| function.name.clone())
                .flatten()
                .or_else(|| function.immutable_env_binding.clone()),
            refresh_captured_slots_on_resume: true,
            capture_writeback,
        })));

    let result_promise = promise::new_pending_promise(env);
    drive(
        &context,
        &result_promise,
        Resume::Next(Value::Undefined),
        env,
    );
    Value::Object(result_promise)
}

/// Drives a module body with top-level `await` as an async evaluation: the
/// caller has staged the body in a `SuspendedStart` async context, and this
/// runs it to its first `await` or completion, returning the promise that
/// settles with the module's completion (16.2.1.5.3 AsyncModuleExecution). A
/// later `await` resumes through the job queue exactly as an async function
/// body does, so the caller drains the job queue to settle the module.
pub(crate) fn drive_async_module(context: &ObjectRef, env: &mut CallEnv) -> ObjectRef {
    let result_promise = promise::new_pending_promise(env);
    drive(
        context,
        &result_promise,
        Resume::Next(Value::Undefined),
        env,
    );
    result_promise
}

/// Resumes the suspended async body with `resume`, then settles or re-suspends:
/// a `Return` resolves the result promise, a thrown completion rejects it, and a
/// suspension (`await`) schedules reactions that re-enter this driver later.
fn drive(context: &ObjectRef, result_promise: &ObjectRef, resume: Resume, env: &mut CallEnv) {
    match resume_generator(context, resume, env) {
        Ok(GeneratorOutcome::Await(awaited)) => {
            schedule_await(context, result_promise, awaited, env);
        }
        // An async body suspends only at `await` (`Op::Await`); `yield` and
        // `yield*` are gated to generators. Treat a stray yield suspension
        // defensively as a normal await so a malformed body never deadlocks.
        Ok(GeneratorOutcome::Yield(awaited) | GeneratorOutcome::YieldDelegate(awaited)) => {
            schedule_await(context, result_promise, awaited, env);
        }
        Ok(GeneratorOutcome::Return(value) | GeneratorOutcome::ReturnAlreadyAwaited(value)) => {
            promise::resolve_promise_capability(result_promise, value, env);
        }
        Err(error) => {
            let reason = crate::error::runtime_error_to_value(error, env);
            promise::reject_promise_capability(result_promise, reason, env);
        }
    }
}

/// Schedules the `await` reactions: `PromiseResolve(awaited).then(onFulfilled,
/// onRejected)` where the handlers resume the async body via the job queue.
fn schedule_await(
    context: &ObjectRef,
    result_promise: &ObjectRef,
    awaited: Value,
    env: &mut CallEnv,
) {
    let on_fulfilled = await_reaction(
        NativeFunction::AsyncFunctionAwaitFulfilled,
        context,
        result_promise,
    );
    let on_rejected = await_reaction(
        NativeFunction::AsyncFunctionAwaitRejected,
        context,
        result_promise,
    );
    promise::perform_await(awaited, on_fulfilled, on_rejected, env);
}

/// Builds an await-resume reaction native carrying the async context and result
/// promise in its environment.
fn await_reaction(
    native: NativeFunction,
    context: &ObjectRef,
    result_promise: &ObjectRef,
) -> Value {
    let mut function = Function::new_native(None, 1, native, false);
    function.insert_env(ASYNC_CONTEXT.to_owned(), Value::Object(context.clone()));
    function.insert_env(
        ASYNC_RESULT_PROMISE.to_owned(),
        Value::Object(result_promise.clone()),
    );
    Value::Function(function)
}

/// Dispatches the await-resume reactions: delivers the fulfillment value (or an
/// injected throw at the await site) to the suspended body, then drives it.
pub(crate) fn call_async_await_native(
    function: &Function,
    native: NativeFunction,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    let (Some(Value::Object(context)), Some(Value::Object(result_promise))) = (
        function.env.get(ASYNC_CONTEXT),
        function.env.get(ASYNC_RESULT_PROMISE),
    ) else {
        return Ok(None);
    };
    let context = context.clone();
    let result_promise = result_promise.clone();
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let resume = match native {
        NativeFunction::AsyncFunctionAwaitFulfilled => Resume::Next(value),
        NativeFunction::AsyncFunctionAwaitRejected => Resume::Throw(value),
        _ => return Ok(None),
    };
    drive(&context, &result_promise, resume, env);
    Ok(Some(Value::Undefined))
}

/// Wires a freshly created async function into the async intrinsic chain: its
/// `[[Prototype]]` becomes `%AsyncFunction.prototype%`. Async functions have no
/// own `prototype` property (the non-constructable default wiring already
/// skipped it).
pub(crate) fn wire_async_function_intrinsics(function: &Function, env: &CallEnv) {
    if let Some(async_function_prototype) = async_function_prototype(env) {
        let _ = function
            .set_internal_prototype_slot(Some(crate::Prototype::Object(async_function_prototype)));
    }
}
