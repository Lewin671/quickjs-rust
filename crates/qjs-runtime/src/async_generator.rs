//! Async generators and `for await ... of` (ES2023 27.6, 14.7.5.6).
//!
//! Calling an `async function*` returns an async generator object. Its
//! `next`/`return`/`throw` methods each return a promise of an iterator result.
//! The body reuses the generator suspend/resume machinery (`vm_generator`): a
//! suspension is tagged either as an `await` (resumed by a promise reaction) or
//! as a consumer-facing `yield` (resumed by the next/return/throw request).
//!
//! Per spec each async generator keeps a FIFO queue of pending requests
//! (AsyncGeneratorEnqueue). Overlapping `next()` calls are served in order: only
//! the front request drives the body at a time, and resolving it dequeues and
//! drains the next. `yield v` awaits `v` first (an implicit await), then
//! suspends delivering `{ value, done: false }`; `await` suspends on the job
//! queue like an async function; a return completion resolves the request with
//! `{ value, done: true }`; an exception rejects the pending request and
//! completes the generator.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::CallEnv;
use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value,
    bytecode::{
        GeneratorOutcome, GeneratorStart, GeneratorState, Resume, is_suspended_at_plain_yield,
        resume_generator,
    },
    call_function, function_constructor_as_prototype_slot, function_intrinsic_prototype_slot,
    is_truthy, object_prototype, promise, property_value, symbol,
};

/// Intrinsic binding for `%AsyncGeneratorFunction.prototype%`, the object that
/// sits between an async generator function and `%Function.prototype%`.
pub(crate) const ASYNC_GENERATOR_FUNCTION_PROTOTYPE_BINDING: &str =
    "\0AsyncGeneratorFunctionPrototype";
/// Intrinsic binding for `%AsyncIteratorPrototype%`, the parent shared by async
/// iterator instance prototypes.
pub(crate) const ASYNC_ITERATOR_PROTOTYPE_BINDING: &str = "\0AsyncIteratorPrototype";
/// Intrinsic binding for `%AsyncGeneratorPrototype%`, the object async generator
/// instances inherit from.
pub(crate) const ASYNC_GENERATOR_PROTOTYPE_BINDING: &str = "\0AsyncGeneratorPrototype";

/// Internal slot names carried in a reaction native's environment.
const ASYNC_GEN: &str = "\0AsyncGenObject";
/// The sync iterator wrapped by a CreateAsyncFromSyncIterator object.
const SYNC_ITERATOR: &str = "\0SyncIterator";
const SYNC_ITERATOR_NEXT: &str = "\0SyncIteratorNext";
/// The wrapper promise capability and the `done` flag for an async-from-sync
/// value await reaction.
const WRAP_PROMISE: &str = "\0WrapPromise";
const WRAP_DONE: &str = "\0WrapDone";

/// How a pending async-generator request was requested.
#[derive(Clone)]
enum RequestKind {
    Next(Value),
    Return(Value),
    Throw(Value),
}

/// One queued request: the resume to run and the promise capability to settle.
struct AsyncGeneratorRequest {
    kind: RequestKind,
    capability: ObjectRef,
}

/// The async generator object's internal state: its request queue plus a
/// draining flag so a reaction that fires while the queue is being served does
/// not re-enter the drain loop (re-entrancy guard).
pub(crate) struct AsyncGeneratorInternal {
    queue: Vec<AsyncGeneratorRequest>,
    draining: bool,
}

/// Installs `%AsyncGeneratorPrototype%` (with `next`/`return`/`throw`,
/// `Symbol.asyncIterator`, and the "AsyncGenerator" toStringTag) and
/// `%AsyncGeneratorFunction.prototype%`, recording both under intrinsic
/// bindings. `%AsyncGeneratorFunction%` is created only for prototype-chain
/// consistency and is not exposed as a global binding.
pub(crate) fn install_async_generator(
    env: &mut CallEnv,
    _global_this: &Value,
    object_prototype: ObjectRef,
) {
    let async_iterator_prototype =
        ObjectRef::with_prototype(HashMap::new(), Some(object_prototype.clone()));
    if let Some(async_iterator) = symbol::async_iterator_symbol(env) {
        async_iterator_prototype.define_symbol_property(
            async_iterator,
            Property::non_enumerable(Value::Function(Function::new_native(
                Some("[Symbol.asyncIterator]"),
                0,
                NativeFunction::AsyncGeneratorPrototypeAsyncIterator,
                false,
            ))),
        );
    }
    if let Some(async_dispose) = symbol::async_dispose_symbol(env) {
        async_iterator_prototype.define_symbol_property(
            async_dispose,
            Property::non_enumerable(Value::Function(Function::new_native(
                Some("[Symbol.asyncDispose]"),
                0,
                NativeFunction::AsyncIteratorPrototypeAsyncDispose,
                false,
            ))),
        );
    }

    let async_generator_prototype =
        ObjectRef::with_prototype(HashMap::new(), Some(async_iterator_prototype.clone()));
    for (name, native) in [
        ("next", NativeFunction::AsyncGeneratorPrototypeNext),
        ("return", NativeFunction::AsyncGeneratorPrototypeReturn),
        ("throw", NativeFunction::AsyncGeneratorPrototypeThrow),
    ] {
        async_generator_prototype.define_non_enumerable(
            name.to_owned(),
            Value::Function(Function::new_native(Some(name), 1, native, false)),
        );
    }
    if let Some(async_iterator) = symbol::async_iterator_symbol(env) {
        async_generator_prototype.define_symbol_property(
            async_iterator,
            Property::non_enumerable(Value::Function(Function::new_native(
                Some("[Symbol.asyncIterator]"),
                0,
                NativeFunction::AsyncGeneratorPrototypeAsyncIterator,
                false,
            ))),
        );
    }
    async_generator_prototype.set_to_string_tag("AsyncGenerator");
    symbol::define_well_known_to_string_tag(env, &async_generator_prototype, "AsyncGenerator");

    let async_generator_function_prototype = ObjectRef::with_prototype_slot(
        HashMap::new(),
        function_intrinsic_prototype_slot(env).or(Some(crate::Prototype::Object(object_prototype))),
    );
    let async_generator_function = Function::new_native(
        Some("AsyncGeneratorFunction"),
        1,
        NativeFunction::AsyncGeneratorFunction,
        true,
    );
    let _ = async_generator_function
        .set_internal_prototype_slot(function_constructor_as_prototype_slot(env));
    async_generator_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::data(
            Value::Object(async_generator_function_prototype.clone()),
            false,
            false,
            false,
        ),
    );
    async_generator_function_prototype.define_property(
        "prototype".to_owned(),
        Property::data(
            Value::Object(async_generator_prototype.clone()),
            false,
            false,
            true,
        ),
    );
    async_generator_function_prototype.set_to_string_tag("AsyncGeneratorFunction");
    symbol::define_well_known_to_string_tag(
        env,
        &async_generator_function_prototype,
        "AsyncGeneratorFunction",
    );
    async_generator_function_prototype.define_property(
        "constructor".to_owned(),
        Property::data(
            Value::Function(async_generator_function.clone()),
            false,
            false,
            true,
        ),
    );
    async_generator_prototype.define_property(
        "constructor".to_owned(),
        Property::data(
            Value::Object(async_generator_function_prototype.clone()),
            false,
            false,
            true,
        ),
    );

    env.insert_realm(
        ASYNC_ITERATOR_PROTOTYPE_BINDING.to_owned(),
        Value::Object(async_iterator_prototype),
    );
    env.insert_realm(
        ASYNC_GENERATOR_PROTOTYPE_BINDING.to_owned(),
        Value::Object(async_generator_prototype),
    );
    env.insert_realm(
        ASYNC_GENERATOR_FUNCTION_PROTOTYPE_BINDING.to_owned(),
        Value::Object(async_generator_function_prototype),
    );
}

/// Returns `%AsyncGeneratorFunction.prototype%` from the current environment.
pub(crate) fn async_generator_function_prototype(env: &CallEnv) -> Option<ObjectRef> {
    match env.get(ASYNC_GENERATOR_FUNCTION_PROTOTYPE_BINDING) {
        Some(Value::Object(object)) => Some(object.clone()),
        _ => None,
    }
}

/// Returns `%AsyncGeneratorPrototype%` from the current environment.
pub(crate) fn async_generator_prototype(env: &CallEnv) -> Option<ObjectRef> {
    match env.get(ASYNC_GENERATOR_PROTOTYPE_BINDING) {
        Some(Value::Object(object)) => Some(object.clone()),
        _ => None,
    }
}

/// Builds the async generator object returned by calling an `async function*`:
/// an ordinary object whose [[Prototype]] is the function's own `prototype`
/// (when an object) or `%AsyncGeneratorPrototype%`, plus an empty request queue.
/// The parameter prologue runs synchronously here, so a binding error throws at
/// the call before the object exists; the object then carries the body-start
/// state for the first resume.
pub(crate) fn make_async_generator_object(
    function: &Function,
    start: GeneratorStart,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let state = crate::bytecode::start_suspended_at_body(start, env)?;
    let prototype = async_generator_object_prototype(function, env);
    let generator = ObjectRef::with_prototype(HashMap::new(), prototype);
    *generator.generator_state().borrow_mut() = Some(state);
    *generator.async_generator_state().borrow_mut() = Some(AsyncGeneratorInternal {
        queue: Vec::new(),
        draining: false,
    });
    Ok(Value::Object(generator))
}

fn async_generator_object_prototype(function: &Function, env: &CallEnv) -> Option<ObjectRef> {
    if let Some(Value::Object(prototype)) = function
        .own_property("prototype")
        .map(|property| property.value)
        && !symbol::is_symbol_primitive(&prototype)
    {
        return Some(prototype);
    }
    async_generator_prototype(env)
}

/// Dispatches the `%AsyncGeneratorPrototype%` methods. Each enqueues a request
/// and returns a promise of the eventual iterator result.
pub(crate) fn call_async_generator_native(
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    if matches!(native, NativeFunction::AsyncGeneratorPrototypeAsyncIterator) {
        return Ok(Some(this_value));
    }
    if matches!(native, NativeFunction::AsyncIteratorPrototypeAsyncDispose) {
        return Ok(Some(promise::async_iterator_async_dispose(
            this_value, env,
        )?));
    }
    if matches!(native, NativeFunction::AsyncDisposeReturnUndefined) {
        return Ok(Some(Value::Undefined));
    }
    let kind = match native {
        NativeFunction::AsyncGeneratorPrototypeNext => {
            RequestKind::Next(argument_values.first().cloned().unwrap_or(Value::Undefined))
        }
        NativeFunction::AsyncGeneratorPrototypeReturn => {
            RequestKind::Return(argument_values.first().cloned().unwrap_or(Value::Undefined))
        }
        NativeFunction::AsyncGeneratorPrototypeThrow => {
            RequestKind::Throw(argument_values.first().cloned().unwrap_or(Value::Undefined))
        }
        _ => return Ok(None),
    };

    let capability = promise::new_pending_promise(env);
    let Value::Object(generator) = &this_value else {
        let reason = not_an_async_generator_value(env);
        promise::reject_promise_capability(&capability, reason, env);
        return Ok(Some(Value::Object(capability)));
    };
    if generator.async_generator_state().borrow().is_none() {
        let reason = not_an_async_generator_value(env);
        promise::reject_promise_capability(&capability, reason, env);
        return Ok(Some(Value::Object(capability)));
    }

    enqueue(generator, kind, capability.clone(), env);
    Ok(Some(Value::Object(capability)))
}

/// AsyncGeneratorEnqueue: appends a request and, unless the queue is already
/// being drained, starts draining.
fn enqueue(generator: &ObjectRef, kind: RequestKind, capability: ObjectRef, env: &mut CallEnv) {
    {
        let mut slot = generator.async_generator_state().borrow_mut();
        let Some(state) = slot.as_mut() else { return };
        state.queue.push(AsyncGeneratorRequest { kind, capability });
        if state.draining {
            return;
        }
        state.draining = true;
    }
    drain(generator, env);
}

/// Serves the front request: resumes the body once and dispatches on the
/// outcome. Each path either re-enters `drain` synchronously (when the request
/// settles without an await) or schedules a reaction that re-enters it later.
fn drain(generator: &ObjectRef, env: &mut CallEnv) {
    loop {
        // If the generator has completed, settle any queued requests directly.
        if matches!(
            generator.generator_state().borrow().as_ref(),
            Some(GeneratorState::Completed) | None
        ) {
            if !settle_completed_front(generator, env) {
                set_draining(generator, false);
                return;
            }
            continue;
        }
        let Some(resume) = front_resume(generator) else {
            set_draining(generator, false);
            return;
        };
        if matches!(resume, Resume::Return(_))
            && is_suspended_at_plain_yield(generator)
            && let Resume::Return(value) = resume
        {
            schedule_return_resume_await(generator, value, env);
            return;
        }
        match resume_generator(generator, resume, env) {
            Ok(GeneratorOutcome::Await(awaited)) => {
                schedule_internal_await(generator, awaited, env);
                return;
            }
            Ok(GeneratorOutcome::Yield(value)) => {
                // `yield v` awaits `v` first (implicit), then suspends
                // delivering `{ value, done: false }`.
                schedule_yield_await(generator, value, env);
                return;
            }
            Ok(GeneratorOutcome::YieldDelegate(result)) => {
                // `yield*` suspended on an inner result object; deliver it to
                // the consumer (its value/done already shaped by the inner
                // iterator). Awaiting the inner result happens inside the
                // delegation; surface the result object directly.
                resolve_front_with_result(generator, result, env);
                continue;
            }
            Ok(GeneratorOutcome::Return(value)) => {
                if front_is_return_request(generator) {
                    schedule_return_await(generator, value, env);
                    return;
                }
                resolve_front(generator, value, true, env);
                continue;
            }
            Ok(GeneratorOutcome::ReturnAlreadyAwaited(value)) => {
                resolve_front(generator, value, true, env);
                continue;
            }
            Err(error) => {
                let reason = crate::error::runtime_error_to_value(error, env);
                reject_front(generator, reason, env);
                continue;
            }
        }
    }
}

/// Resolves and dequeues every request that should observe a completed
/// generator: a queued `next`/`return` yields `{ value: undefined, done: true }`
/// (or `{ value, done: true }` for a `return(v)`), while a `throw` rejects.
/// Returns whether a request was served (so the caller keeps draining).
fn settle_completed_front(generator: &ObjectRef, env: &mut CallEnv) -> bool {
    let kind = {
        let slot = generator.async_generator_state().borrow();
        let Some(state) = slot.as_ref() else {
            return false;
        };
        let Some(request) = state.queue.first() else {
            return false;
        };
        request.kind.clone()
    };
    match kind {
        RequestKind::Next(_) => {
            if let Some(request) = dequeue_front(generator) {
                settle_resolve(&request.capability, Value::Undefined, true, env);
            }
        }
        RequestKind::Return(value) => {
            schedule_return_await(generator, value, env);
            return false;
        }
        RequestKind::Throw(value) => {
            if let Some(request) = dequeue_front(generator) {
                promise::reject_promise_capability(&request.capability, value, env);
            }
        }
    }
    true
}

/// The resume kind for the front request, leaving it on the queue (it is
/// dequeued only when it settles).
fn front_resume(generator: &ObjectRef) -> Option<Resume> {
    let slot = generator.async_generator_state().borrow();
    let state = slot.as_ref()?;
    let request = state.queue.first()?;
    Some(match &request.kind {
        RequestKind::Next(value) => Resume::Next(value.clone()),
        RequestKind::Return(value) => Resume::Return(value.clone()),
        RequestKind::Throw(value) => Resume::Throw(value.clone()),
    })
}

fn front_is_return_request(generator: &ObjectRef) -> bool {
    let slot = generator.async_generator_state().borrow();
    let Some(state) = slot.as_ref() else {
        return false;
    };
    state
        .queue
        .first()
        .is_some_and(|request| matches!(request.kind, RequestKind::Return(_)))
}

fn set_draining(generator: &ObjectRef, value: bool) {
    if let Some(state) = generator.async_generator_state().borrow_mut().as_mut() {
        state.draining = value;
    }
}

/// Resolves the front request with `{ value, done }`, dequeues it.
fn resolve_front(generator: &ObjectRef, value: Value, done: bool, env: &mut CallEnv) {
    if let Some(request) = dequeue_front(generator) {
        settle_resolve(&request.capability, value, done, env);
    }
}

/// Resolves the front request with an already-built iterator result object.
fn resolve_front_with_result(generator: &ObjectRef, result: Value, env: &mut CallEnv) {
    if let Some(request) = dequeue_front(generator) {
        promise::resolve_promise_capability(&request.capability, result, env);
    }
}

fn reject_front(generator: &ObjectRef, reason: Value, env: &mut CallEnv) {
    if let Some(request) = dequeue_front(generator) {
        promise::reject_promise_capability(&request.capability, reason, env);
    }
}

fn dequeue_front(generator: &ObjectRef) -> Option<AsyncGeneratorRequest> {
    let mut slot = generator.async_generator_state().borrow_mut();
    let state = slot.as_mut()?;
    if state.queue.is_empty() {
        None
    } else {
        Some(state.queue.remove(0))
    }
}

/// Settles a capability with an iterator result `{ value, done }`.
fn settle_resolve(capability: &ObjectRef, value: Value, done: bool, env: &mut CallEnv) {
    let result = iterator_result(value, done, env);
    promise::resolve_promise_capability(capability, result, env);
}

/// Schedules an internal `await` (from `Op::Await` in the body): on fulfillment
/// the body resumes with the value; on rejection it resumes with a throw. The
/// front request stays in flight.
fn schedule_internal_await(generator: &ObjectRef, awaited: Value, env: &mut CallEnv) {
    let on_fulfilled = reaction(NativeFunction::AsyncGeneratorAwaitFulfilled, generator);
    let on_rejected = reaction(NativeFunction::AsyncGeneratorAwaitRejected, generator);
    promise::perform_await(awaited, on_fulfilled, on_rejected, env);
}

/// Schedules the implicit `await` of a `yield`'s operand. On fulfillment the
/// front request resolves with `{ value, done: false }` and draining continues;
/// on rejection the body resumes with a throw at the yield site.
fn schedule_yield_await(generator: &ObjectRef, value: Value, env: &mut CallEnv) {
    let on_fulfilled = reaction(NativeFunction::AsyncGeneratorYieldFulfilled, generator);
    let on_rejected = reaction(NativeFunction::AsyncGeneratorYieldRejected, generator);
    promise::perform_await(value, on_fulfilled, on_rejected, env);
}

/// Schedules the await that precedes resuming a suspended async generator for a
/// `.return(value)` request. A rejected await is thrown at the suspended `yield`
/// site so the generator body can catch it.
fn schedule_return_resume_await(generator: &ObjectRef, value: Value, env: &mut CallEnv) {
    let on_fulfilled = reaction(
        NativeFunction::AsyncGeneratorReturnResumeFulfilled,
        generator,
    );
    let on_rejected = reaction(
        NativeFunction::AsyncGeneratorReturnResumeRejected,
        generator,
    );
    promise::perform_await(value, on_fulfilled, on_rejected, env);
}

/// Schedules the `AsyncGeneratorResolve(..., done = true)` unwrapping step.
/// The front request remains queued until the fulfillment/rejection reaction
/// settles it, preserving request order while `return(value)` awaits `value`.
fn schedule_return_await(generator: &ObjectRef, value: Value, env: &mut CallEnv) {
    let on_fulfilled = reaction(NativeFunction::AsyncGeneratorReturnFulfilled, generator);
    let on_rejected = reaction(NativeFunction::AsyncGeneratorReturnRejected, generator);
    promise::perform_await(value, on_fulfilled, on_rejected, env);
}

/// Builds a reaction native carrying the async generator object.
fn reaction(native: NativeFunction, generator: &ObjectRef) -> Value {
    let mut function = Function::new_native(None, 1, native, false);
    function.insert_env(ASYNC_GEN.to_owned(), Value::Object(generator.clone()));
    Value::Function(function)
}

/// Dispatches the async generator reaction natives that resume body suspensions
/// or settle requests after an implicit yield await.
pub(crate) fn call_async_generator_reaction(
    function: &Function,
    native: NativeFunction,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    match native {
        NativeFunction::AsyncGeneratorAwaitFulfilled
        | NativeFunction::AsyncGeneratorAwaitRejected => {
            let Some(Value::Object(generator)) = function.env.get(ASYNC_GEN) else {
                return Ok(None);
            };
            let generator = generator.clone();
            let resume = if matches!(native, NativeFunction::AsyncGeneratorAwaitFulfilled) {
                Resume::Next(value)
            } else {
                Resume::Throw(value)
            };
            resume_body(&generator, resume, env);
            Ok(Some(Value::Undefined))
        }
        NativeFunction::AsyncGeneratorYieldFulfilled => {
            let Some(Value::Object(generator)) = function.env.get(ASYNC_GEN) else {
                return Ok(None);
            };
            let generator = generator.clone();
            // The implicit yield await fulfilled: resolve the consumer with the
            // resolved value and continue draining the next request.
            resolve_front(&generator, value, false, env);
            set_draining(&generator, true);
            drain(&generator, env);
            Ok(Some(Value::Undefined))
        }
        NativeFunction::AsyncGeneratorYieldRejected => {
            let Some(Value::Object(generator)) = function.env.get(ASYNC_GEN) else {
                return Ok(None);
            };
            let generator = generator.clone();
            // The implicit yield await rejected: throw at the yield site.
            resume_body(&generator, Resume::Throw(value), env);
            Ok(Some(Value::Undefined))
        }
        NativeFunction::AsyncGeneratorReturnResumeFulfilled => {
            let Some(Value::Object(generator)) = function.env.get(ASYNC_GEN) else {
                return Ok(None);
            };
            let generator = generator.clone();
            resume_body(&generator, Resume::ReturnAlreadyAwaited(value), env);
            Ok(Some(Value::Undefined))
        }
        NativeFunction::AsyncGeneratorReturnResumeRejected => {
            let Some(Value::Object(generator)) = function.env.get(ASYNC_GEN) else {
                return Ok(None);
            };
            let generator = generator.clone();
            resume_body(&generator, Resume::Throw(value), env);
            Ok(Some(Value::Undefined))
        }
        NativeFunction::AsyncGeneratorReturnFulfilled => {
            let Some(Value::Object(generator)) = function.env.get(ASYNC_GEN) else {
                return Ok(None);
            };
            let generator = generator.clone();
            resolve_front(&generator, value, true, env);
            set_draining(&generator, true);
            drain(&generator, env);
            Ok(Some(Value::Undefined))
        }
        NativeFunction::AsyncGeneratorReturnRejected => {
            let Some(Value::Object(generator)) = function.env.get(ASYNC_GEN) else {
                return Ok(None);
            };
            let generator = generator.clone();
            reject_front(&generator, value, env);
            set_draining(&generator, true);
            drain(&generator, env);
            Ok(Some(Value::Undefined))
        }
        NativeFunction::AsyncFromSyncIteratorValueFulfilled => {
            async_from_sync_value_fulfilled(function, value, env);
            Ok(Some(Value::Undefined))
        }
        NativeFunction::AsyncFromSyncIteratorValueRejected => {
            async_from_sync_value_rejected(function, value, env);
            Ok(Some(Value::Undefined))
        }
        NativeFunction::AsyncFromSyncIteratorNext
        | NativeFunction::AsyncFromSyncIteratorReturn
        | NativeFunction::AsyncFromSyncIteratorThrow => Ok(Some(async_from_sync_method(
            function,
            native,
            argument_values,
            env,
        ))),
        _ => Ok(None),
    }
}

/// Resumes the body for an in-flight request (after an internal await fired),
/// re-entering the drain loop. Used by the await reactions.
fn resume_body(generator: &ObjectRef, resume: Resume, env: &mut CallEnv) {
    // Mark draining so a reaction enqueued during the body run does not start a
    // second drain loop.
    set_draining(generator, true);
    match resume_generator(generator, resume, env) {
        Ok(GeneratorOutcome::Await(awaited)) => {
            schedule_internal_await(generator, awaited, env);
        }
        Ok(GeneratorOutcome::Yield(value)) => {
            schedule_yield_await(generator, value, env);
        }
        Ok(GeneratorOutcome::YieldDelegate(result)) => {
            resolve_front_with_result(generator, result, env);
            drain(generator, env);
        }
        Ok(GeneratorOutcome::Return(value)) => {
            if front_is_return_request(generator) {
                schedule_return_await(generator, value, env);
            } else {
                resolve_front(generator, value, true, env);
                drain(generator, env);
            }
        }
        Ok(GeneratorOutcome::ReturnAlreadyAwaited(value)) => {
            resolve_front(generator, value, true, env);
            drain(generator, env);
        }
        Err(error) => {
            let reason = crate::error::runtime_error_to_value(error, env);
            reject_front(generator, reason, env);
            drain(generator, env);
        }
    }
}

/// Wires a freshly created async generator function into the async-generator
/// intrinsic chain: its `[[Prototype]]` becomes
/// `%AsyncGeneratorFunction.prototype%` and its own `prototype` property's
/// `[[Prototype]]` becomes `%AsyncGeneratorPrototype%`.
pub(crate) fn wire_async_generator_function_intrinsics(function: &Function, env: &CallEnv) {
    if let Some(prototype) = async_generator_function_prototype(env) {
        let _ = function.set_internal_prototype_slot(Some(crate::Prototype::Object(prototype)));
    }
    if let Some(async_generator_prototype) = async_generator_prototype(env) {
        let prototype = ObjectRef::with_prototype(HashMap::new(), Some(async_generator_prototype));
        function.define_property(
            "prototype".to_owned(),
            Property::data(Value::Object(prototype), false, true, false),
        );
    }
}

fn iterator_result(value: Value, done: bool, env: &CallEnv) -> Value {
    let object = ObjectRef::with_prototype(HashMap::new(), object_prototype(env));
    object.define_property("value".to_owned(), Property::enumerable(value));
    object.define_property(
        "done".to_owned(),
        Property::enumerable(Value::Boolean(done)),
    );
    Value::Object(object)
}

fn not_an_async_generator_value(env: &mut CallEnv) -> Value {
    // A real TypeError object, not a bare string, so a rejection handler's
    // `e instanceof TypeError` check holds.
    crate::error::runtime_error_to_value(
        RuntimeError {
            thrown: None,
            message: "TypeError: method called on a non-async-generator object".to_owned(),
        },
        env,
    )
}

// ---------------------------------------------------------------------------
// for-await-of support: GetIterator(obj, async) and the async-from-sync wrapper.
// ---------------------------------------------------------------------------

/// GetIterator(obj, async): looks up `Symbol.asyncIterator`; if absent, gets the
/// sync iterator via `Symbol.iterator` and wraps it in a
/// CreateAsyncFromSyncIterator object. Returns the async iterator object.
pub(crate) fn get_async_iterator(value: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    if matches!(value, Value::Undefined | Value::Null) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: value is not async iterable".to_owned(),
        });
    }
    if let Some(async_iterator_symbol) = symbol::async_iterator_symbol(env) {
        let method = crate::property_value_key(
            value.clone(),
            &crate::PropertyKey::Symbol(async_iterator_symbol),
            env,
        )?;
        if matches!(method, Value::Function(_)) {
            let iterator = call_function(method, value, Vec::new(), env, false)?;
            if !matches!(
                iterator,
                Value::Object(_) | Value::Array(_) | Value::Function(_)
            ) {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: async iterator method must return an object".to_owned(),
                });
            }
            return Ok(iterator);
        }
        if !matches!(method, Value::Undefined | Value::Null) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: Symbol.asyncIterator is not a function".to_owned(),
            });
        }
    }
    // No async iterator: get the sync iterator and wrap it.
    let sync_iterator = crate::bytecode::sync_iterator_for_value(value, env)?;
    create_async_from_sync_iterator(sync_iterator, env)
}

/// CreateAsyncFromSyncIterator: builds a wrapper object whose `next`/`return`/
/// `throw` forward to the sync iterator and await each result value.
pub(crate) fn create_async_from_sync_iterator(
    sync_iterator: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let next = property_value(sync_iterator.clone(), "next", env)?;
    let wrapper = ObjectRef::with_prototype(HashMap::new(), object_prototype(env));
    for (name, native) in [
        ("next", NativeFunction::AsyncFromSyncIteratorNext),
        ("return", NativeFunction::AsyncFromSyncIteratorReturn),
        ("throw", NativeFunction::AsyncFromSyncIteratorThrow),
    ] {
        let mut method = Function::new_native(Some(name), 1, native, false);
        method.insert_env(SYNC_ITERATOR.to_owned(), sync_iterator.clone());
        if matches!(native, NativeFunction::AsyncFromSyncIteratorNext) {
            method.insert_env(SYNC_ITERATOR_NEXT.to_owned(), next.clone());
        }
        wrapper.define_non_enumerable(name.to_owned(), Value::Function(method));
    }
    Ok(Value::Object(wrapper))
}

/// Builds a `TypeError` value with the given message, suitable for rejecting a
/// promise capability (a real error object, not a bare string).
fn type_error_value(message: &str, env: &mut CallEnv) -> Value {
    crate::error::runtime_error_to_value(
        RuntimeError {
            thrown: None,
            message: format!("TypeError: {message}"),
        },
        env,
    )
}

/// `IteratorClose(syncIterator, NormalCompletion)` specialised to the
/// async-from-sync `throw`-without-throw path: calls the sync iterator's
/// `return` (if any) and discards its value. Returns `Some(reason)` carrying the
/// rejection value when the `return` lookup or call is abrupt, or when the
/// `return` result is not an object; `None` when the iterator closed cleanly.
fn close_sync_iterator(sync_iterator: &Value, env: &mut CallEnv) -> Option<Value> {
    let return_method = match property_value(sync_iterator.clone(), "return", env) {
        Ok(value) => value,
        Err(error) => return Some(crate::error::runtime_error_to_value(error, env)),
    };
    if matches!(return_method, Value::Undefined | Value::Null) {
        return None;
    }
    let result = match call_function(return_method, sync_iterator.clone(), Vec::new(), env, false) {
        Ok(value) => value,
        Err(error) => return Some(crate::error::runtime_error_to_value(error, env)),
    };
    if !matches!(result, Value::Object(_) | Value::Array(_)) {
        return Some(type_error_value(
            "iterator return result is not an object",
            env,
        ));
    }
    None
}

/// The `next`/`return`/`throw` of a CreateAsyncFromSyncIterator wrapper: invokes
/// the matching sync-iterator method, then awaits the result's `value` and
/// resolves a wrapper promise with `{ value: awaited, done }`. A `return`/`throw`
/// with no underlying method resolves/rejects directly per spec.
fn async_from_sync_method(
    function: &Function,
    native: NativeFunction,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Value {
    let argument = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let capability = promise::new_pending_promise(env);
    let Some(sync_iterator) = function.env.get(SYNC_ITERATOR).cloned() else {
        let reason = not_an_async_generator_value(env);
        promise::reject_promise_capability(&capability, reason, env);
        return Value::Object(capability);
    };

    let method_name = match native {
        NativeFunction::AsyncFromSyncIteratorNext => "next",
        NativeFunction::AsyncFromSyncIteratorReturn => "return",
        NativeFunction::AsyncFromSyncIteratorThrow => "throw",
        _ => "next",
    };

    let method = if matches!(native, NativeFunction::AsyncFromSyncIteratorNext) {
        function
            .env
            .get(SYNC_ITERATOR_NEXT)
            .cloned()
            .unwrap_or(Value::Undefined)
    } else {
        match property_value(sync_iterator.clone(), method_name, env) {
            Ok(method) => method,
            Err(error) => {
                promise::reject_promise_capability(
                    &capability,
                    crate::error::runtime_error_to_value(error, env),
                    env,
                );
                return Value::Object(capability);
            }
        }
    };

    if matches!(native, NativeFunction::AsyncFromSyncIteratorReturn)
        && matches!(method, Value::Undefined | Value::Null)
    {
        // No `return`: resolve with `{ value: argument, done: true }`.
        let result = iterator_result(argument, true, env);
        promise::resolve_promise_capability(&capability, result, env);
        return Value::Object(capability);
    }
    if matches!(native, NativeFunction::AsyncFromSyncIteratorThrow)
        && matches!(method, Value::Undefined | Value::Null)
    {
        // No `throw` method (GetMethod treats null as absent): close the sync
        // iterator, then reject with a TypeError. An abrupt close (a poisoned
        // `return` getter, or a `return` result that is not an object) rejects
        // with that completion instead (spec 27.1.6.2.3, step 7).
        if let Some(reason) = close_sync_iterator(&sync_iterator, env) {
            promise::reject_promise_capability(&capability, reason, env);
            return Value::Object(capability);
        }
        promise::reject_promise_capability(
            &capability,
            type_error_value("The iterator does not provide a throw method", env),
            env,
        );
        return Value::Object(capability);
    }

    let sync_arguments = argument_values.to_vec();
    let outcome = call_function(method, sync_iterator, sync_arguments, env, false);
    let result = match outcome {
        Ok(result) => result,
        Err(error) => {
            promise::reject_promise_capability(
                &capability,
                crate::error::runtime_error_to_value(error, env),
                env,
            );
            return Value::Object(capability);
        }
    };
    if !matches!(result, Value::Object(_) | Value::Array(_)) {
        promise::reject_promise_capability(
            &capability,
            type_error_value("iterator result is not an object", env),
            env,
        );
        return Value::Object(capability);
    }
    let done = match property_value(result.clone(), "done", env) {
        Ok(done) => is_truthy(&done),
        Err(error) => {
            promise::reject_promise_capability(
                &capability,
                crate::error::runtime_error_to_value(error, env),
                env,
            );
            return Value::Object(capability);
        }
    };
    let value = match property_value(result, "value", env) {
        Ok(value) => value,
        Err(error) => {
            promise::reject_promise_capability(
                &capability,
                crate::error::runtime_error_to_value(error, env),
                env,
            );
            return Value::Object(capability);
        }
    };

    // AsyncFromSyncIteratorContinuation performs PromiseResolve before await.
    // If that throws, the wrapper promise is rejected immediately rather than
    // after an additional await-rejection job.
    let promise_constructor = env.get("Promise").unwrap_or(Value::Undefined);
    let value_wrapper = match promise::promise_resolve(&promise_constructor, value, env) {
        Ok(Value::Object(promise)) => promise,
        Ok(_) => return Value::Object(capability),
        Err(error) => {
            promise::reject_promise_capability(
                &capability,
                crate::error::runtime_error_to_value(error, env),
                env,
            );
            return Value::Object(capability);
        }
    };

    // Await the value, then resolve the wrapper promise with `{ value, done }`.
    let on_fulfilled = value_await_reaction(
        NativeFunction::AsyncFromSyncIteratorValueFulfilled,
        &capability,
        done,
    );
    let on_rejected = value_await_reaction(
        NativeFunction::AsyncFromSyncIteratorValueRejected,
        &capability,
        done,
    );
    promise::perform_await_on_promise(value_wrapper, on_fulfilled, on_rejected, env);
    Value::Object(capability)
}

/// Builds an async-from-sync value-await reaction carrying the wrapper promise
/// and the recorded `done` flag.
fn value_await_reaction(native: NativeFunction, capability: &ObjectRef, done: bool) -> Value {
    let mut function = Function::new_native(None, 1, native, false);
    function.insert_env(WRAP_PROMISE.to_owned(), Value::Object(capability.clone()));
    function.insert_env(WRAP_DONE.to_owned(), Value::Boolean(done));
    Value::Function(function)
}

fn async_from_sync_value_rejected(function: &Function, reason: Value, env: &mut CallEnv) {
    let Some(Value::Object(capability)) = function.env.get(WRAP_PROMISE) else {
        return;
    };
    let capability = capability.clone();
    promise::reject_promise_capability(&capability, reason, env);
}

fn async_from_sync_value_fulfilled(function: &Function, value: Value, env: &mut CallEnv) {
    let (Some(Value::Object(capability)), Some(Value::Boolean(done))) =
        (function.env.get(WRAP_PROMISE), function.env.get(WRAP_DONE))
    else {
        return;
    };
    let capability = capability.clone();
    let done = *done;
    let result = iterator_result(value, done, env);
    promise::resolve_promise_capability(&capability, result, env);
}

/// Wraps the captured-frame/Rc plumbing the caller path uses so it stays out of
/// `call.rs`. Mirrors the generator path but builds an async generator object.
pub(crate) fn call_async_generator_function(
    function: &Function,
    function_env: CallEnv,
    function_capture_names: Vec<String>,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let bytecode = function
        .bytecode
        .clone()
        .expect("async generator has a bytecode body");
    let captured = Rc::new(RefCell::new(function_env.snapshot_locals()));
    // Mirror `call_async_function`: a body that writes an outer-captured binding
    // after an `await`/`yield` resumes in a later microtask whose caller env is
    // not the defining frame, so the write only reaches the cell its outer
    // closures read through this `CaptureWriteback`. Without it, `let c`-style
    // outer bindings assigned past the first suspension stay stale (the entire
    // async-generator `for await`/destructuring counter-update test cluster).
    let mut capture_names = function_capture_names;
    {
        let captured_env = function.captured_env.borrow();
        for name in captured_env.keys() {
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
    let capture_writeback = (!capture_names.is_empty() || parent_writeback.is_some()).then(|| {
        crate::bytecode::CaptureWriteback {
            target: Rc::clone(&function.captured_env),
            names: capture_names,
            aliases: Vec::new(),
            parent: parent_writeback,
        }
    });
    make_async_generator_object(
        function,
        GeneratorStart {
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
        },
        env,
    )
}
