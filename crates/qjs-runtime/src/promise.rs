use std::collections::HashMap;

use crate::{
    ArrayRef, Function, NativeFunction, ObjectRef, Property, PropertyKey, RuntimeError, Value,
    call_function, ensure_constructor, property_value, property_value_key, symbol,
};

mod all;
pub(crate) mod all_settled;
pub(crate) mod any;
mod capability;
mod jobs;
mod perform;
mod race;
pub(crate) mod r#try;
pub(crate) mod with_resolvers;

pub(crate) use all::{native_promise_all, native_promise_all_resolve_element};
pub(crate) use capability::native_get_capabilities_executor;
pub(crate) use jobs::drain_promise_jobs;
use jobs::{enqueue_promise_reaction_job, enqueue_promise_thenable_job};
pub(crate) use race::native_promise_race;

const PROMISE_FULFILL_REACTION: &str = "\0PromiseFulfillReaction";
const PROMISE_FINALLY_HANDLER: &str = "\0PromiseFinallyHandler";
const PROMISE_HANDLER: &str = "\0PromiseHandler";
const PROMISE_ALL_INDEX: &str = "\0PromiseAllIndex";
const PROMISE_ALL_REMAINING: &str = "\0PromiseAllRemaining";
const PROMISE_ALL_VALUES: &str = "\0PromiseAllValues";
const PROMISE_ALL_CAPABILITY_RESOLVE: &str = "\0PromiseAllCapabilityResolve";
const PROMISE_ALL_ALREADY_CALLED: &str = "\0PromiseAllAlreadyCalled";
const PROMISE_AGGREGATE_ERROR: &str = "\0PromiseAggregateError";
const PROMISE_JOBS: &str = "\0PromiseJobs";
const PROMISE_REACTIONS: &str = "\0PromiseReactions";
const PROMISE_REACTION_ARGUMENT: &str = "\0PromiseReactionArgument";
const PROMISE_REACTION_CAPABILITY: &str = "\0PromiseReactionCapability";
const PROMISE_REACTION_RESOLVE: &str = "\0PromiseReactionResolve";
const PROMISE_REACTION_REJECT: &str = "\0PromiseReactionReject";
const PROMISE_STATE: &str = "\0PromiseState";
const PROMISE_RESULT: &str = "\0PromiseResult";
const PROMISE_TARGET: &str = "\0PromiseTarget";
const PROMISE_THEN: &str = "\0PromiseThen";
const PROMISE_THENABLE: &str = "\0PromiseThenable";
const PROMISE_THENABLE_CAPABILITY: &str = "\0PromiseThenableCapability";
const PROMISE_OBJECT_PROTOTYPE: &str = "\0PromiseObjectPrototype";
const PROMISE_PROTOTYPE: &str = "\0PromisePrototype";
const PROMISE_PENDING: &str = "pending";
const PROMISE_FULFILLED: &str = "fulfilled";
const PROMISE_REJECTED: &str = "rejected";

pub(crate) fn install_promise(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let promise_prototype =
        ObjectRef::with_prototype(HashMap::new(), Some(object_prototype.clone()));
    promise_prototype.set_to_string_tag("Promise");
    symbol::define_well_known_to_string_tag(env, &promise_prototype, "Promise");
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
        Property::fixed_non_enumerable(Value::Object(promise_prototype.clone())),
    );
    symbol::define_species_accessor(env, &promise_function);
    let promise_all = promise_static_function(
        "all",
        1,
        NativeFunction::PromiseAll,
        &promise_prototype,
        &object_prototype,
    );
    promise_function.properties.borrow_mut().insert(
        "all".to_owned(),
        Property::non_enumerable(Value::Function(promise_all)),
    );

    let mut promise_any = promise_static_function(
        "any",
        1,
        NativeFunction::PromiseAny,
        &promise_prototype,
        &object_prototype,
    );
    if let Some(aggregate_error) = env.get("AggregateError").cloned() {
        promise_any
            .env
            .insert(PROMISE_AGGREGATE_ERROR.to_owned(), aggregate_error);
    }
    define_promise_static(&promise_function, "any", promise_any);

    for (name, length, native) in [
        ("allSettled", 1, NativeFunction::PromiseAllSettled),
        ("race", 1, NativeFunction::PromiseRace),
        ("try", 1, NativeFunction::PromiseTry),
        ("withResolvers", 0, NativeFunction::PromiseWithResolvers),
        ("resolve", 1, NativeFunction::PromiseResolve),
        ("reject", 1, NativeFunction::PromiseReject),
    ] {
        define_promise_static(
            &promise_function,
            name,
            promise_static_function(name, length, native, &promise_prototype, &object_prototype),
        );
    }

    let value = Value::Function(promise_function);
    env.insert("Promise".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("Promise".to_owned(), value);
    }
}

fn promise_static_function(
    name: &str,
    length: usize,
    native: NativeFunction,
    promise_prototype: &ObjectRef,
    object_prototype: &ObjectRef,
) -> Function {
    let mut function = Function::new_native(Some(name), length, native, false);
    function.env.insert(
        PROMISE_OBJECT_PROTOTYPE.to_owned(),
        Value::Object(object_prototype.clone()),
    );
    function.env.insert(
        PROMISE_PROTOTYPE.to_owned(),
        Value::Object(promise_prototype.clone()),
    );
    function
}

fn define_promise_static(promise_function: &Function, name: &str, function: Function) {
    promise_function.properties.borrow_mut().insert(
        name.to_owned(),
        Property::non_enumerable(Value::Function(function)),
    );
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
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    // 1. Let C be the this value. 2. If Type(C) is not Object, throw TypeError.
    if !is_object_value(&this_value) {
        return Err(promise_receiver_not_object_error());
    }
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    promise_resolve(&this_value, value, env)
}

pub(crate) fn native_promise_reject(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if !is_object_value(&this_value) {
        return Err(promise_receiver_not_object_error());
    }
    let reason = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let capability = capability::new_promise_capability(&this_value, env)?;
    capability::capability_reject(&capability, reason, env)?;
    Ok(capability.promise)
}

/// `PromiseResolve(C, x)` (ES2023 27.2.4.7.1): returns `x` if it is already a
/// promise built by `C`, otherwise wraps it in a freshly resolved capability.
pub(crate) fn promise_resolve(
    c: &Value,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if is_promise_value(&value) {
        let constructor = property_value(value.clone(), "constructor", env)?;
        if constructor.same_value(c) {
            return Ok(value);
        }
    }
    let capability = capability::new_promise_capability(c, env)?;
    capability::capability_resolve(&capability, value, env)?;
    Ok(capability.promise)
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

    // C = SpeciesConstructor(promise, %Promise%); resultCapability = NewPromiseCapability(C).
    let species = species_constructor(&promise, function, env)?;
    let capability = capability::new_promise_capability(&species, env)?;

    let fulfill_reaction = promise_reaction_for_capability(on_fulfilled, &capability, true);
    let reject_reaction = promise_reaction_for_capability(on_rejected, &capability, false);

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

    Ok(capability.promise)
}

/// `SpeciesConstructor(O, defaultConstructor)` (ES2023 7.3.22) specialised to
/// promises: reads `O.constructor`, then `constructor[Symbol.species]`, falling
/// back to the realm `%Promise%` when either is undefined/null.
fn species_constructor(
    object: &ObjectRef,
    promise_function: &Function,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let default = default_promise_constructor(promise_function, env);
    let constructor = property_value(Value::Object(object.clone()), "constructor", env)?;
    if matches!(constructor, Value::Undefined) {
        return Ok(default);
    }
    if !is_object_value(&constructor) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: constructor property is not an object".to_owned(),
        });
    }
    let species = match symbol::species_symbol(env) {
        Some(symbol) => property_value_key(constructor, &PropertyKey::Symbol(symbol), env)?,
        None => return Ok(default),
    };
    if matches!(species, Value::Undefined | Value::Null) {
        return Ok(default);
    }
    ensure_constructor(&species)?;
    Ok(species)
}

/// Resolves the realm `%Promise%` constructor that serves as the default in
/// SpeciesConstructor and NewPromiseCapability fallbacks.
fn default_promise_constructor(promise_function: &Function, env: &HashMap<String, Value>) -> Value {
    if let Some(prototype) = promise_function
        .env
        .get(PROMISE_PROTOTYPE)
        .and_then(|value| match value {
            Value::Object(prototype) => Some(prototype),
            _ => None,
        })
        && let Some(Property {
            value: Value::Function(constructor),
            ..
        }) = prototype.own_property("constructor")
    {
        return Value::Function(constructor);
    }
    env.get("Promise").cloned().unwrap_or(Value::Undefined)
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
    resolve_promise(&promise, value, env);
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

/// Creates a fresh pending promise (a "promise capability" in spec terms) whose
/// `[[Prototype]]` is `%Promise.prototype%`. Used by the async-function driver
/// for the promise it returns to the caller.
pub(crate) fn new_pending_promise(env: &HashMap<String, Value>) -> ObjectRef {
    let promise = ObjectRef::with_prototype(HashMap::new(), promise_prototype_from_env(env));
    initialize_promise(&promise);
    promise
}

/// Resolves `%Promise.prototype%` from the realm: prefer the internal binding,
/// otherwise read it off the global `Promise` constructor's `prototype`.
fn promise_prototype_from_env(env: &HashMap<String, Value>) -> Option<ObjectRef> {
    if let Some(Value::Object(prototype)) = env.get(PROMISE_PROTOTYPE) {
        return Some(prototype.clone());
    }
    match env.get("Promise") {
        Some(Value::Function(promise)) => match promise.own_property("prototype") {
            Some(property) => match property.value {
                Value::Object(prototype) => Some(prototype),
                _ => None,
            },
            None => None,
        },
        _ => None,
    }
}

/// Resolves a promise capability with `value` (running the
/// thenable-unwrapping resolution algorithm). Used to settle an async
/// function's returned promise with the body's return value.
pub(crate) fn resolve_promise_capability(
    promise: &ObjectRef,
    value: Value,
    env: &mut HashMap<String, Value>,
) {
    resolve_promise(promise, value, env);
}

/// Rejects a promise capability with `reason`. Used to settle an async
/// function's returned promise when its body throws.
pub(crate) fn reject_promise_capability(
    promise: &ObjectRef,
    reason: Value,
    env: &mut HashMap<String, Value>,
) {
    settle_promise(promise, PROMISE_REJECTED, reason, env);
}

/// Implements the `await` plumbing: resolves `value` to a promise via
/// `PromiseResolve(%Promise%, value)`, then schedules `on_fulfilled` /
/// `on_rejected` as promise reactions whose handlers resume the suspended async
/// function through the job queue (spec 27.7.5.3 Await). The reactions feed a
/// throwaway capability promise; only their side effect (resuming the async
/// state) matters.
pub(crate) fn perform_await(
    value: Value,
    on_fulfilled: Value,
    on_rejected: Value,
    env: &mut HashMap<String, Value>,
) {
    // PromiseResolve(%Promise%, value): reuse an existing native promise,
    // otherwise wrap the value in a resolved promise.
    let promise = if is_promise_value(&value) {
        match value {
            Value::Object(promise) => promise,
            _ => unreachable!("is_promise_value guarantees an object"),
        }
    } else {
        let promise = new_pending_promise(env);
        resolve_promise(&promise, value, env);
        promise
    };

    // The reactions need a result capability to satisfy the reaction-job
    // contract; the async driver ignores its eventual settlement.
    let throwaway = new_pending_promise(env);
    let fulfill_reaction = promise_reaction(on_fulfilled, throwaway.clone(), true);
    let reject_reaction = promise_reaction(on_rejected, throwaway, false);

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
        _ => {}
    }
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

fn resolve_promise(object: &ObjectRef, value: Value, env: &mut HashMap<String, Value>) {
    if matches!(&value, Value::Object(value_object) if value_object.ptr_eq(object)) {
        let reason = crate::error::runtime_error_to_value(
            RuntimeError {
                thrown: None,
                message: "TypeError: Chaining cycle detected for promise".to_owned(),
            },
            env,
        );
        settle_promise(object, PROMISE_REJECTED, reason, env);
        return;
    }
    let then = match promise_thenable_then(value.clone(), env) {
        Ok(Some(then)) => then,
        Ok(None) => {
            settle_promise(object, PROMISE_FULFILLED, value, env);
            return;
        }
        Err(error) => {
            let reason = crate::error::runtime_error_to_value(error, env);
            settle_promise(object, PROMISE_REJECTED, reason, env);
            return;
        }
    };
    enqueue_promise_thenable_job(env, object.clone(), value, then);
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

/// Whether `value` is an Object in the ECMAScript sense (callable objects and
/// exotic collections included). Symbol primitives are boxed as objects
/// internally, so they are excluded explicitly. Used for the `Type(C) is Object`
/// guards in `Promise.resolve`/`Promise.reject`.
fn is_object_value(value: &Value) -> bool {
    match value {
        Value::Object(object) => !symbol::is_symbol_primitive(object),
        Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) | Value::Proxy(_) => {
            true
        }
        _ => false,
    }
}

fn promise_receiver_not_object_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Promise static method called on a non-object".to_owned(),
    }
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

/// Builds a reaction whose result capability may be an arbitrary (non-native)
/// promise: the resolve/reject functions are stored so the reaction job calls
/// them directly instead of settling a native promise object.
fn promise_reaction_for_capability(
    handler: Value,
    capability: &capability::PromiseCapability,
    fulfill: bool,
) -> ObjectRef {
    let reaction = ObjectRef::new(HashMap::new());
    reaction.define_non_enumerable(PROMISE_HANDLER.to_owned(), handler);
    reaction.define_non_enumerable(
        PROMISE_REACTION_RESOLVE.to_owned(),
        capability.resolve.clone(),
    );
    reaction.define_non_enumerable(
        PROMISE_REACTION_REJECT.to_owned(),
        capability.reject.clone(),
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

#[cfg(test)]
pub(crate) fn promise_debug_state_result(value: &Value) -> Option<(String, Value)> {
    let Value::Object(object) = value else {
        return None;
    };
    Some((promise_state(object)?, promise_result(object)?))
}

fn promise_thenable_then(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Option<Value>, RuntimeError> {
    if !is_thenable_candidate(&value) {
        return Ok(None);
    }
    let then = property_value(value, "then", env)?;
    if matches!(then, Value::Function(_)) {
        Ok(Some(then))
    } else {
        Ok(None)
    }
}

fn is_thenable_candidate(value: &Value) -> bool {
    matches!(
        value,
        Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Object(_) | Value::Set(_)
    )
}
