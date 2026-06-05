use std::collections::HashMap;

use crate::{
    ArrayRef, Function, GLOBAL_THIS_BINDING, NativeFunction, ObjectRef, Property, RuntimeError,
    Value, call_function, property_value,
};

const PROMISE_FULFILL_REACTION: &str = "\0PromiseFulfillReaction";
const PROMISE_FINALLY_HANDLER: &str = "\0PromiseFinallyHandler";
const PROMISE_HANDLER: &str = "\0PromiseHandler";
const PROMISE_JOBS: &str = "\0PromiseJobs";
const PROMISE_REACTIONS: &str = "\0PromiseReactions";
const PROMISE_REACTION_ARGUMENT: &str = "\0PromiseReactionArgument";
const PROMISE_REACTION_CAPABILITY: &str = "\0PromiseReactionCapability";
const PROMISE_STATE: &str = "\0PromiseState";
const PROMISE_RESULT: &str = "\0PromiseResult";
const PROMISE_TARGET: &str = "\0PromiseTarget";
const PROMISE_PROTOTYPE: &str = "\0PromisePrototype";
const PROMISE_PENDING: &str = "pending";
const PROMISE_FULFILLED: &str = "fulfilled";
const PROMISE_REJECTED: &str = "rejected";

pub(crate) fn install_promise(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let promise_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    promise_prototype.set_to_string_tag("Promise");
    let promise_function = Function::new_native(Some("Promise"), 1, NativeFunction::Promise, true);
    let mut promise_then =
        Function::new_native(Some("then"), 2, NativeFunction::PromisePrototypeThen, false);
    promise_then.env.insert(
        PROMISE_PROTOTYPE.to_owned(),
        Value::Object(promise_prototype.clone()),
    );
    let mut promise_catch = Function::new_native(
        Some("catch"),
        1,
        NativeFunction::PromisePrototypeCatch,
        false,
    );
    promise_catch.env.insert(
        PROMISE_PROTOTYPE.to_owned(),
        Value::Object(promise_prototype.clone()),
    );
    let mut promise_finally = Function::new_native(
        Some("finally"),
        1,
        NativeFunction::PromisePrototypeFinally,
        false,
    );
    promise_finally.env.insert(
        PROMISE_PROTOTYPE.to_owned(),
        Value::Object(promise_prototype.clone()),
    );
    promise_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(promise_function.clone()),
    );
    promise_prototype.define_non_enumerable("catch".to_owned(), Value::Function(promise_catch));
    promise_prototype.define_non_enumerable("finally".to_owned(), Value::Function(promise_finally));
    promise_prototype.define_non_enumerable("then".to_owned(), Value::Function(promise_then));
    promise_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(promise_prototype.clone())),
    );
    let mut promise_resolve =
        Function::new_native(Some("resolve"), 1, NativeFunction::PromiseResolve, false);
    promise_resolve.env.insert(
        PROMISE_PROTOTYPE.to_owned(),
        Value::Object(promise_prototype.clone()),
    );
    promise_function.properties.borrow_mut().insert(
        "resolve".to_owned(),
        Property::non_enumerable(Value::Function(promise_resolve)),
    );

    let mut promise_reject =
        Function::new_native(Some("reject"), 1, NativeFunction::PromiseReject, false);
    promise_reject.env.insert(
        PROMISE_PROTOTYPE.to_owned(),
        Value::Object(promise_prototype),
    );
    promise_function.properties.borrow_mut().insert(
        "reject".to_owned(),
        Property::non_enumerable(Value::Function(promise_reject)),
    );

    let value = Value::Function(promise_function);
    env.insert("Promise".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Promise".to_owned(), value);
    }
}

pub(crate) fn native_promise(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Constructor Promise requires 'new'".to_owned(),
        });
    }
    let executor = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(executor, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Promise resolver must be callable".to_owned(),
        });
    }
    let object = match this_value {
        Value::Object(object) => object,
        _ => ObjectRef::with_prototype(HashMap::new(), crate::function_prototype(function)),
    };
    initialize_promise(&object);
    let promise = Value::Object(object.clone());
    let resolve = resolving_function(
        "resolve",
        NativeFunction::PromiseResolveFunction,
        promise.clone(),
    );
    let reject = resolving_function("reject", NativeFunction::PromiseRejectFunction, promise);
    if let Err(error) = call_function(
        executor,
        Value::Undefined,
        vec![resolve, reject.clone()],
        env,
        false,
    ) {
        settle_promise(
            &object,
            PROMISE_REJECTED,
            error.thrown.map_or(Value::Undefined, |value| *value),
            env,
        );
    }
    Ok(Value::Object(object))
}

pub(crate) fn native_promise_resolve(
    function: &Function,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if is_promise_value(&value) {
        return Ok(value);
    }
    let promise = promise_object_from_function(function);
    initialize_promise(&promise);
    settle_promise(&promise, PROMISE_FULFILLED, value, env);
    Ok(Value::Object(promise))
}

pub(crate) fn native_promise_reject(
    function: &Function,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let reason = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let promise = promise_object_from_function(function);
    initialize_promise(&promise);
    settle_promise(&promise, PROMISE_REJECTED, reason, env);
    Ok(Value::Object(promise))
}

pub(crate) fn native_promise_then(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Object(promise) = this_value else {
        return Err(not_a_promise_error());
    };
    if !is_promise_object(&promise) {
        return Err(not_a_promise_error());
    }
    let on_fulfilled = callable_or_undefined(argument_values.first());
    let on_rejected = callable_or_undefined(argument_values.get(1));
    let result_promise = promise_object_from_function(function);
    initialize_promise(&result_promise);
    let fulfill_reaction = promise_reaction(on_fulfilled, result_promise.clone(), true);
    let reject_reaction = promise_reaction(on_rejected, result_promise.clone(), false);

    match promise_state(&promise).as_deref() {
        Some(PROMISE_PENDING) => {
            add_promise_reaction(&promise, Value::Object(fulfill_reaction));
            add_promise_reaction(&promise, Value::Object(reject_reaction));
        }
        Some(PROMISE_FULFILLED) => {
            enqueue_promise_reaction_job(
                env,
                &fulfill_reaction,
                promise_result(&promise).unwrap_or(Value::Undefined),
            );
        }
        Some(PROMISE_REJECTED) => {
            enqueue_promise_reaction_job(
                env,
                &reject_reaction,
                promise_result(&promise).unwrap_or(Value::Undefined),
            );
        }
        _ => return Err(not_a_promise_error()),
    }

    Ok(Value::Object(result_promise))
}

pub(crate) fn native_promise_catch(
    _function: &Function,
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let on_rejected = argument_values.first().cloned().unwrap_or(Value::Undefined);
    call_promise_then(this_value, vec![Value::Undefined, on_rejected], env)
}

pub(crate) fn native_promise_finally(
    _function: &Function,
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let on_finally = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let (on_fulfilled, on_rejected) = if matches!(on_finally, Value::Function(_)) {
        (
            promise_finally_function(
                "thenFinally",
                NativeFunction::PromisePrototypeFinallyFulfilled,
                on_finally.clone(),
            ),
            promise_finally_function(
                "catchFinally",
                NativeFunction::PromisePrototypeFinallyRejected,
                on_finally,
            ),
        )
    } else {
        (on_finally.clone(), on_finally)
    };
    call_promise_then(this_value, vec![on_fulfilled, on_rejected], env)
}

pub(crate) fn native_promise_finally_fulfilled(
    function: &Function,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    call_finally_handler(function, env)?;
    Ok(value)
}

pub(crate) fn native_promise_finally_rejected(
    function: &Function,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let reason = argument_values.first().cloned().unwrap_or(Value::Undefined);
    call_finally_handler(function, env)?;
    Err(RuntimeError {
        thrown: Some(Box::new(reason)),
        message: "Promise finally rejected".to_owned(),
    })
}

pub(crate) fn native_promise_resolve_function(
    function: &Function,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let promise = promise_from_resolving_function(function)?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    settle_promise(&promise, PROMISE_FULFILLED, value, env);
    Ok(Value::Undefined)
}

pub(crate) fn native_promise_reject_function(
    function: &Function,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let promise = promise_from_resolving_function(function)?;
    let reason = argument_values.first().cloned().unwrap_or(Value::Undefined);
    settle_promise(&promise, PROMISE_REJECTED, reason, env);
    Ok(Value::Undefined)
}

pub(crate) fn is_promise_object(object: &ObjectRef) -> bool {
    object.own_property(PROMISE_STATE).is_some()
}

fn initialize_promise(object: &ObjectRef) {
    object.set_to_string_tag("Promise");
    if object.own_property(PROMISE_STATE).is_none() {
        object.define_non_enumerable(
            PROMISE_STATE.to_owned(),
            Value::String(PROMISE_PENDING.to_owned()),
        );
        object.define_non_enumerable(PROMISE_RESULT.to_owned(), Value::Undefined);
        object.define_non_enumerable(
            PROMISE_REACTIONS.to_owned(),
            Value::Array(ArrayRef::new(Vec::new())),
        );
    }
}

fn settle_promise(
    object: &ObjectRef,
    state: &str,
    result: Value,
    env: &mut HashMap<String, Value>,
) {
    if !matches!(
        object.own_property(PROMISE_STATE).map(|property| property.value),
        Some(Value::String(current)) if current == PROMISE_PENDING
    ) {
        return;
    }
    object.define_non_enumerable(PROMISE_STATE.to_owned(), Value::String(state.to_owned()));
    object.define_non_enumerable(PROMISE_RESULT.to_owned(), result.clone());
    let reactions = promise_reactions(object);
    object.define_non_enumerable(
        PROMISE_REACTIONS.to_owned(),
        Value::Array(ArrayRef::new(Vec::new())),
    );
    for reaction in reactions {
        let Value::Object(reaction) = reaction else {
            continue;
        };
        if reaction_is_fulfill(&reaction) == (state == PROMISE_FULFILLED) {
            enqueue_promise_reaction_job(env, &reaction, result.clone());
        }
    }
}

fn resolving_function(name: &str, native: NativeFunction, promise: Value) -> Value {
    let mut function = Function::new_native(Some(name), 1, native, false);
    function.env.insert(PROMISE_TARGET.to_owned(), promise);
    Value::Function(function)
}

fn promise_finally_function(name: &str, native: NativeFunction, handler: Value) -> Value {
    let mut function = Function::new_native(Some(name), 1, native, false);
    function
        .env
        .insert(PROMISE_FINALLY_HANDLER.to_owned(), handler);
    Value::Function(function)
}

fn call_finally_handler(
    function: &Function,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    let handler = function
        .env
        .get(PROMISE_FINALLY_HANDLER)
        .cloned()
        .unwrap_or(Value::Undefined);
    call_function(handler, Value::Undefined, Vec::new(), env, false)?;
    Ok(())
}

fn call_promise_then(
    this_value: Value,
    argument_values: Vec<Value>,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let then = property_value(this_value.clone(), "then", env)?;
    call_function(then, this_value, argument_values, env, false)
}

fn promise_from_resolving_function(function: &Function) -> Result<ObjectRef, RuntimeError> {
    match function.env.get(PROMISE_TARGET).cloned() {
        Some(Value::Object(object)) if is_promise_object(&object) => Ok(object),
        _ => Err(RuntimeError {
            thrown: None,
            message: "Promise resolving function is missing its promise".to_owned(),
        }),
    }
}

fn promise_object_from_function(function: &Function) -> ObjectRef {
    let prototype = match function.env.get(PROMISE_PROTOTYPE).cloned() {
        Some(Value::Object(prototype)) => Some(prototype),
        _ => crate::function_prototype(function),
    };
    ObjectRef::with_prototype(HashMap::new(), prototype)
}

fn is_promise_value(value: &Value) -> bool {
    matches!(value, Value::Object(object) if is_promise_object(object))
}

fn not_a_promise_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Promise.prototype.then called on incompatible receiver".to_owned(),
    }
}

fn callable_or_undefined(value: Option<&Value>) -> Value {
    match value {
        Some(Value::Function(function)) => Value::Function(function.clone()),
        _ => Value::Undefined,
    }
}

fn promise_reaction(handler: Value, capability: ObjectRef, fulfill: bool) -> ObjectRef {
    let reaction = ObjectRef::new(HashMap::new());
    reaction.define_non_enumerable(PROMISE_HANDLER.to_owned(), handler);
    reaction.define_non_enumerable(
        PROMISE_REACTION_CAPABILITY.to_owned(),
        Value::Object(capability),
    );
    reaction.define_non_enumerable(PROMISE_FULFILL_REACTION.to_owned(), Value::Boolean(fulfill));
    reaction
}

fn add_promise_reaction(promise: &ObjectRef, reaction: Value) {
    if let Some(Value::Array(reactions)) = promise
        .own_property(PROMISE_REACTIONS)
        .map(|property| property.value)
    {
        reactions.set(reactions.len(), reaction);
    }
}

fn promise_reactions(promise: &ObjectRef) -> Vec<Value> {
    match promise
        .own_property(PROMISE_REACTIONS)
        .map(|property| property.value)
    {
        Some(Value::Array(reactions)) => reactions.to_vec(),
        _ => Vec::new(),
    }
}

fn promise_state(promise: &ObjectRef) -> Option<String> {
    match promise
        .own_property(PROMISE_STATE)
        .map(|property| property.value)
    {
        Some(Value::String(state)) => Some(state),
        _ => None,
    }
}

fn promise_result(promise: &ObjectRef) -> Option<Value> {
    promise
        .own_property(PROMISE_RESULT)
        .map(|property| property.value)
}

fn reaction_is_fulfill(reaction: &ObjectRef) -> bool {
    matches!(
        reaction
            .own_property(PROMISE_FULFILL_REACTION)
            .map(|property| property.value),
        Some(Value::Boolean(true))
    )
}

fn enqueue_promise_reaction_job(
    env: &mut HashMap<String, Value>,
    reaction: &ObjectRef,
    argument: Value,
) {
    let job = ObjectRef::new(HashMap::new());
    if let Some(handler) = reaction
        .own_property(PROMISE_HANDLER)
        .map(|property| property.value)
    {
        job.define_non_enumerable(PROMISE_HANDLER.to_owned(), handler);
    }
    if let Some(capability) = reaction
        .own_property(PROMISE_REACTION_CAPABILITY)
        .map(|property| property.value)
    {
        job.define_non_enumerable(PROMISE_REACTION_CAPABILITY.to_owned(), capability);
    }
    job.define_non_enumerable(PROMISE_REACTION_ARGUMENT.to_owned(), argument);
    job.define_non_enumerable(
        PROMISE_FULFILL_REACTION.to_owned(),
        Value::Boolean(reaction_is_fulfill(reaction)),
    );
    let jobs = promise_jobs(env);
    jobs.set(jobs.len(), Value::Object(job));
}

pub(crate) fn drain_promise_jobs(env: &mut HashMap<String, Value>) -> Result<(), RuntimeError> {
    loop {
        let jobs = promise_jobs(env);
        let pending = jobs.to_vec();
        if pending.is_empty() {
            return Ok(());
        }
        jobs.replace_with(Vec::new());
        for job in pending {
            let Value::Object(job) = job else {
                continue;
            };
            run_promise_reaction_job(&job, env)?;
        }
    }
}

#[cfg(test)]
pub(crate) fn promise_debug_state_result(value: &Value) -> Option<(String, Value)> {
    let Value::Object(object) = value else {
        return None;
    };
    Some((promise_state(object)?, promise_result(object)?))
}

fn run_promise_reaction_job(
    job: &ObjectRef,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    let capability = match job
        .own_property(PROMISE_REACTION_CAPABILITY)
        .map(|property| property.value)
    {
        Some(Value::Object(promise)) if is_promise_object(&promise) => promise,
        _ => return Ok(()),
    };
    let argument = job
        .own_property(PROMISE_REACTION_ARGUMENT)
        .map_or(Value::Undefined, |property| property.value);
    let fulfill = matches!(
        job.own_property(PROMISE_FULFILL_REACTION)
            .map(|property| property.value),
        Some(Value::Boolean(true))
    );
    let handler = job
        .own_property(PROMISE_HANDLER)
        .map_or(Value::Undefined, |property| property.value);

    match handler {
        Value::Function(_) => {
            match call_function(handler, Value::Undefined, vec![argument], env, false) {
                Ok(value) => settle_promise(&capability, PROMISE_FULFILLED, value, env),
                Err(error) => settle_promise(
                    &capability,
                    PROMISE_REJECTED,
                    error.thrown.map_or(Value::Undefined, |value| *value),
                    env,
                ),
            }
        }
        _ if fulfill => settle_promise(&capability, PROMISE_FULFILLED, argument, env),
        _ => settle_promise(&capability, PROMISE_REJECTED, argument, env),
    }
    Ok(())
}

fn promise_jobs(env: &mut HashMap<String, Value>) -> ArrayRef {
    let global_this = match env.get(GLOBAL_THIS_BINDING).cloned() {
        Some(Value::Object(global_this)) => global_this,
        _ => {
            let global_this = ObjectRef::new(HashMap::new());
            env.insert(
                GLOBAL_THIS_BINDING.to_owned(),
                Value::Object(global_this.clone()),
            );
            global_this
        }
    };
    match global_this
        .own_property(PROMISE_JOBS)
        .map(|property| property.value)
    {
        Some(Value::Array(jobs)) => jobs,
        _ => {
            let jobs = ArrayRef::new(Vec::new());
            global_this.define_non_enumerable(PROMISE_JOBS.to_owned(), Value::Array(jobs.clone()));
            jobs
        }
    }
}
