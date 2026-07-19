use std::{collections::HashMap, rc::Rc};

use qjs_ast::{BindingPattern, FunctionParams};

use crate::{
    ArrayRef, Bytecode, DIRECT_EVAL_ARGUMENTS_BINDING, DIRECT_EVAL_FUNCTION_CONTEXT_BINDING,
    FIELD_INITIALIZER_EVAL_BINDING, Function, GLOBAL_THIS_BINDING, NEW_TARGET_BINDING,
    NativeFunction, ObjectRef, RuntimeError, Value,
    bytecode::{
        DirectCallSlots, eval_function_bytecode, eval_function_bytecode_with_direct_call_slots,
        try_eval_numeric_leaf,
    },
    function_prototype,
    native::call_native_function,
    object_prototype,
    private::PrivateEnvironment,
    symbol,
};

use super::{
    CROSS_REALM_TYPE_ERROR_PROTOTYPE, CallEnv, InstanceElementInitializer,
    arguments::arguments_object, function_call_this, parameter_binding_name,
    rest_parameter_binding_name,
};

const CROSS_REALM_OBJECT_PROTOTYPE: &str = "__quickjsRustRealmObjectPrototype";
const CROSS_REALM_BOOLEAN_PROTOTYPE: &str = "__quickjsRustRealmBooleanPrototype";
const CROSS_REALM_NUMBER_PROTOTYPE: &str = "__quickjsRustRealmNumberPrototype";
const CROSS_REALM_STRING_PROTOTYPE: &str = "__quickjsRustRealmStringPrototype";
const CROSS_REALM_DATE_PROTOTYPE: &str = "__quickjsRustRealmDatePrototype";
const CROSS_REALM_REGEXP_PROTOTYPE: &str = "__quickjsRustRealmRegExpPrototype";
const CROSS_REALM_ERROR_PROTOTYPE: &str = "__quickjsRustRealmErrorPrototype";
const CROSS_REALM_EVAL_ERROR_PROTOTYPE: &str = "__quickjsRustRealmEvalErrorPrototype";
const CROSS_REALM_RANGE_ERROR_PROTOTYPE: &str = "__quickjsRustRealmRangeErrorPrototype";
const CROSS_REALM_REFERENCE_ERROR_PROTOTYPE: &str = "__quickjsRustRealmReferenceErrorPrototype";
const CROSS_REALM_SYNTAX_ERROR_PROTOTYPE: &str = "__quickjsRustRealmSyntaxErrorPrototype";
const CROSS_REALM_URI_ERROR_PROTOTYPE: &str = "__quickjsRustRealmURIErrorPrototype";
const CROSS_REALM_AGGREGATE_ERROR_PROTOTYPE: &str = "__quickjsRustRealmAggregateErrorPrototype";
const CROSS_REALM_SUPPRESSED_ERROR_PROTOTYPE: &str = "__quickjsRustRealmSuppressedErrorPrototype";
const CROSS_REALM_ITERATOR_PROTOTYPE: &str = "__quickjsRustRealmIteratorPrototype";
const DYNAMIC_FUNCTION_REALM_GLOBAL: &str = "__quickjsRustDynamicFunctionRealm";

pub(crate) fn call_function(
    callee: Value,
    this_value: Value,
    argument_values: Vec<Value>,
    env: &mut CallEnv,
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    if let Value::Proxy(proxy) = &callee {
        if is_construct {
            return crate::proxy::proxy_construct(
                proxy.clone(),
                callee.clone(),
                argument_values,
                env,
            );
        }
        return crate::proxy::proxy_apply(proxy.clone(), this_value, argument_values, env);
    }
    let Value::Function(function) = &callee else {
        return Err(RuntimeError {
            thrown: None,
            message: "value is not callable".to_owned(),
        });
    };
    if let Some(bound) = &function.bound {
        if !is_construct
            && matches!(
                &bound.target,
                Value::Function(target)
                    if target.native == Some(NativeFunction::FunctionPrototypeCall)
            )
        {
            let mut combined_arguments = bound.arguments.clone();
            combined_arguments.extend(argument_values);
            let call_this = combined_arguments
                .first()
                .cloned()
                .unwrap_or(Value::Undefined);
            let call_arguments = combined_arguments.into_iter().skip(1).collect();
            return call_function(
                bound.this_value.clone(),
                call_this,
                call_arguments,
                env,
                false,
            );
        }
        let mut bound_arguments = bound.arguments.clone();
        bound_arguments.extend(argument_values);
        let bound_this = if is_construct {
            this_value
        } else {
            bound.this_value.clone()
        };
        return call_function(
            bound.target.clone(),
            bound_this,
            bound_arguments,
            env,
            is_construct,
        );
    }
    if function.is_class_constructor && !is_construct {
        return Err(class_constructor_call_error(function));
    }
    if let Some(native) = function.native {
        return call_native_function(
            function,
            native,
            this_value,
            argument_values,
            is_construct,
            env,
        );
    }
    if let Some(bytecode) = &function.bytecode {
        if function.is_generator && function.is_async {
            let function_env = function_env(
                function,
                bytecode,
                &callee,
                this_value,
                &argument_values,
                env,
                is_construct,
            );
            return crate::async_generator::call_async_generator_function(
                function,
                function_env.env,
                env,
            );
        }
        // Calling a generator function does not run its body, but it does run
        // the parameter prologue synchronously (per FunctionDeclarationInstantiation)
        // and then returns a generator object suspended at the start of the body.
        // A parameter-binding error therefore throws at the call, before the
        // generator object exists.
        if function.is_generator {
            let function_env = function_env(
                function,
                bytecode,
                &callee,
                this_value,
                &argument_values,
                env,
                is_construct,
            );
            return crate::generator::make_generator_object(
                function,
                crate::bytecode::GeneratorStart {
                    bytecode: bytecode.clone(),
                    env: function_env.env,
                    upvalues: function.upvalues.clone(),
                    with_stack: function.with_stack.clone(),
                    immutable_function_name: function
                        .immutable_name_binding
                        .then(|| function.name.clone())
                        .flatten()
                        .or_else(|| function.immutable_env_binding.clone()),
                },
                env,
            );
        }
        // Calling an async function does not run its body to completion: it
        // captures the call frame, builds the promise it returns, and drives the
        // body until the first `await` or completion. The returned promise is
        // resolved/rejected with the body's eventual outcome (including
        // parameter-binding errors, which reject rather than throw).
        if function.is_async {
            let function_env = function_env(
                function,
                bytecode,
                &callee,
                this_value,
                &argument_values,
                env,
                is_construct,
            );
            return Ok(crate::async_function::call_async_function(
                function,
                function_env.env,
                env,
            ));
        }
        // A base-class constructor initializes its instance fields right after
        // the receiver is created, before the constructor body runs. A derived
        // constructor defers this until `super(...)` binds `this`.
        if function.is_class_constructor && !function.is_derived_constructor && is_construct {
            initialize_instance_fields(function, &this_value, env)?;
        }
        let function_env = function_env(
            function,
            bytecode,
            &callee,
            this_value,
            &argument_values,
            env,
            is_construct,
        );
        let immutable_name_caller_value = immutable_name_caller_value(function, env);
        let FunctionCallEnv {
            env: call_env,
            direct_call_slots,
        } = function_env;
        let result = if let Some(direct_call_slots) = direct_call_slots {
            eval_function_bytecode_with_direct_call_slots(
                bytecode,
                call_env,
                true,
                direct_call_slots,
            )
        } else {
            eval_function_bytecode(
                bytecode,
                call_env,
                function.upvalues.clone(),
                function.with_stack.clone(),
                true,
            )
        };
        restore_immutable_name_caller_value(function, env, immutable_name_caller_value);
        // A derived constructor implicitly returns its (super-bound) `this`
        // when the body does not return an object, and it is a ReferenceError
        // to finish without having called `super(...)`.
        if function.is_derived_constructor && is_construct {
            return finish_derived_construct(result);
        }
        return result.value;
    }

    Err(RuntimeError {
        thrown: None,
        message: "user function has no bytecode body".to_owned(),
    })
}

/// Runs the already-guarded ordinary leaf shape directly from a VM frame.
///
/// The general VM call path builds a compatibility `CallEnv` for native,
/// dynamic, and write-back-capable callees. A direct leaf creates its own
/// slot-backed frame and cannot mutate caller compatibility bindings, so that
/// outer shell would be allocated and snapshotted without carrying data.
pub(crate) fn call_direct_leaf_function(
    callee: Value,
    this_value: Value,
    argument_values: &[Value],
    env: &CallEnv,
    module_host: Option<crate::module::ModuleHostRef>,
    #[cfg(feature = "agents")] agent_context: Option<crate::agent::AgentContextRef>,
) -> Result<Value, RuntimeError> {
    let Value::Function(function) = &callee else {
        unreachable!("direct leaf predicate only accepts functions");
    };
    let bytecode = function
        .bytecode
        .as_ref()
        .expect("direct leaf predicate requires bytecode");
    if let Some(value) = try_eval_numeric_leaf(
        bytecode,
        &function.params,
        argument_values,
        &function.upvalues,
    ) {
        return Ok(value);
    }
    let FunctionCallEnv {
        env: mut call_env,
        direct_call_slots,
    } = direct_leaf_function_env(function, bytecode, this_value, argument_values, env);
    if call_env.module_host().is_none()
        && let Some(host) = module_host
    {
        call_env.set_module_host(host);
    }
    #[cfg(feature = "agents")]
    if let Some(context) = agent_context {
        call_env.set_agent_context(context);
    }
    let direct_call_slots = direct_call_slots.expect("guarded direct leaf calls always seed slots");
    eval_function_bytecode_with_direct_call_slots(bytecode, call_env, true, direct_call_slots).value
}

pub(crate) fn is_direct_leaf_function(callee: &Value) -> bool {
    let Value::Function(function) = callee else {
        return false;
    };
    function
        .bytecode
        .as_ref()
        .is_some_and(|bytecode| can_seed_direct_leaf_call(function, bytecode))
}

fn class_constructor_call_error(function: &Function) -> RuntimeError {
    let message = "TypeError: class constructor cannot be invoked without 'new'".to_owned();
    if let Some(crate::Property {
        value: Value::Object(prototype),
        ..
    }) = function.own_property(CROSS_REALM_TYPE_ERROR_PROTOTYPE)
    {
        let error = ObjectRef::with_prototype(HashMap::new(), Some(prototype));
        return RuntimeError {
            thrown: Some(Box::new(Value::Object(error))),
            message,
        };
    }
    RuntimeError {
        thrown: None,
        message,
    }
}

fn immutable_name_caller_value(function: &Function, env: &CallEnv) -> Option<(String, Value)> {
    let name = function.name.as_ref()?;
    if !function.immutable_name_binding {
        return None;
    }
    env.get(name).map(|value| (name.clone(), value))
}

fn restore_immutable_name_caller_value(
    function: &Function,
    env: &mut CallEnv,
    saved: Option<(String, Value)>,
) {
    if !function.immutable_name_binding {
        return;
    }
    let Some((name, value)) = saved else {
        return;
    };
    env.set_local(&name, value.clone());
    if env.realm_contains(&name) {
        env.insert_realm(name, value);
    }
}

/// Runs a class constructor's instance-field initializers, in definition
/// order, installing each field on the receiver via CreateDataPropertyOrThrow.
/// Each initializer thunk evaluates with `this` = the receiver; a field with no
/// initializer installs `undefined`.
pub(crate) fn initialize_instance_fields(
    function: &Function,
    this_value: &Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let elements = function.instance_elements();
    for element in elements.iter() {
        match element {
            InstanceElementInitializer::PublicField(field) => {
                let value = match &field.initializer {
                    Some(thunk) => call_field_initializer(thunk, this_value.clone(), env)?,
                    None => Value::Undefined,
                };
                crate::bytecode::install_field_value(this_value, field.key.clone(), value, env)?;
            }
            InstanceElementInitializer::PrivateElement(private) => {
                crate::bytecode::apply_instance_private_element(
                    function, this_value, private, env,
                )?;
            }
        }
    }
    Ok(())
}

/// Evaluates a class field initializer with the receiver supplied by class
/// construction. Simple initializers have no observable compatibility
/// environment: their receiver and captures are already represented by VM
/// slots, and direct eval/closure/super cases are excluded by the leaf guard.
/// Run those thunks through the same allocation-light path as ordinary leaf
/// calls; semantic-heavy initializers retain the complete call machinery.
pub(crate) fn call_field_initializer(
    thunk: &Function,
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let callee = Value::Function(thunk.clone());
    if is_direct_leaf_function(&callee) {
        return call_direct_leaf_function(
            callee,
            this_value,
            &[],
            env,
            env.module_host(),
            #[cfg(feature = "agents")]
            env.agent_context(),
        );
    }
    call_function(callee, this_value, Vec::new(), env, false)
}

fn finish_derived_construct(
    result: crate::bytecode::FunctionBytecodeResult<'_>,
) -> Result<Value, RuntimeError> {
    let bound_this = result.frame_cell_binding("this");
    let value = result.value?;
    match value {
        // A Symbol (or BigInt) primitive is represented as an object reference
        // but is not an Object, so it does not override `this`; it is a TypeError
        // like any other primitive return.
        Value::Object(ref object) if crate::symbol::is_symbol_primitive(object) => {
            Err(primitive_derived_return_error())
        }
        Value::Array(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Object(_)
        | Value::Proxy(_) => Ok(value),
        Value::Undefined => match bound_this {
            Some(this_value) => Ok(this_value),
            None => Err(RuntimeError {
                thrown: None,
                message: "ReferenceError: must call super constructor before returning \
                          from derived constructor"
                    .to_owned(),
            }),
        },
        // A primitive explicit return from a derived constructor is a
        // TypeError per the spec.
        _ => Err(primitive_derived_return_error()),
    }
}

fn primitive_derived_return_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: derived constructor may only return an object or undefined".to_owned(),
    }
}

pub(crate) fn construct_function(
    target: Value,
    new_target: Value,
    argument_values: Vec<Value>,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    // A constructor Proxy dispatches through its `construct` trap. The trap (or
    // a forwarded target construction) produces the instance directly, so the
    // ordinary receiver-allocation path below is skipped.
    if let Value::Proxy(proxy) = &target {
        ensure_constructor(&new_target)?;
        return crate::proxy::proxy_construct(proxy.clone(), new_target, argument_values, env);
    }
    // A bound function's [[Construct]] prepends the bound arguments and forwards
    // to its target. Per step 4, when the bound function is itself the
    // `new.target` (`new B()` where `B = A.bind()`), `new.target` becomes the
    // target so the constructed instance and `new.target` observe the target,
    // not the bound wrapper. Reflect.construct with an explicit new.target keeps
    // it. Unwrapping here (before the receiver is allocated from
    // `new.target.prototype`) also handles chained binds.
    if let Value::Function(function) = &target
        && let Some(bound) = &function.bound
    {
        let real_target = bound.target.clone();
        let mut combined_arguments = bound.arguments.clone();
        combined_arguments.extend(argument_values);
        let forwarded_new_target = if target.same_value(&new_target) {
            real_target.clone()
        } else {
            new_target
        };
        return construct_function(real_target, forwarded_new_target, combined_arguments, env);
    }
    ensure_constructor(&target)?;
    ensure_constructor(&new_target)?;

    // Make `new.target` visible to the constructor frame (and, via `super(...)`,
    // to ancestor constructors) so subclass instances get the right prototype.
    let previous_new_target = env.insert(NEW_TARGET_BINDING.to_owned(), new_target.clone());

    // A derived constructor must create its `this` through `super(...)`, so it
    // receives no pre-built receiver. Some native constructors also need to run
    // argument validation before the user-observable `new.target.prototype`
    // access that allocates the receiver.
    let is_derived =
        matches!(&target, Value::Function(function) if function.is_derived_constructor);
    let constructs_receiver_lazily = native_constructs_receiver_lazily(&target);
    let this_value = if is_derived || constructs_receiver_lazily {
        Value::Undefined
    } else {
        let prototype = construct_prototype_slot(&target, &new_target, env)?;
        Value::Object(ObjectRef::with_prototype_slot(HashMap::new(), prototype))
    };

    let result = call_function(target, this_value.clone(), argument_values, env, true);

    match previous_new_target {
        Some(previous) => {
            env.insert(NEW_TARGET_BINDING.to_owned(), previous);
        }
        None => {
            env.remove(NEW_TARGET_BINDING);
        }
    }
    let result = result?;

    match result {
        Value::Array(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Object(_)
        | Value::Proxy(_) => Ok(result),
        // A derived constructor that returns no object must have called
        // `super(...)`, which bound `this` and is returned as the result.
        _ if is_derived || constructs_receiver_lazily => Ok(result),
        _ => Ok(this_value),
    }
}

fn native_constructs_receiver_lazily(target: &Value) -> bool {
    matches!(
        target,
        Value::Function(function)
            if matches!(
                function.native,
                Some(
                    NativeFunction::Promise
                        | NativeFunction::ArrayBuffer
                        | NativeFunction::DataView
                        | NativeFunction::SharedArrayBuffer
                        | NativeFunction::Uint8Array
                        | NativeFunction::Int8Array
                        | NativeFunction::Uint8ClampedArray
                        | NativeFunction::Uint16Array
                        | NativeFunction::Int16Array
                        | NativeFunction::Uint32Array
                        | NativeFunction::Int32Array
                        | NativeFunction::Float32Array
                        | NativeFunction::Float64Array
                        | NativeFunction::BigInt64Array
                        | NativeFunction::BigUint64Array
                )
            )
    )
}

fn construct_prototype_slot(
    target: &Value,
    new_target: &Value,
    env: &mut CallEnv,
) -> Result<Option<crate::Prototype>, RuntimeError> {
    let prototype = prototype_value_to_slot(
        crate::property_value(new_target.clone(), "prototype", env)?,
        env,
    );
    if prototype.is_none()
        && let Some(prototype) = cross_realm_construct_prototype_slot(target, new_target, env)?
    {
        return Ok(Some(prototype));
    }
    Ok(prototype.or_else(|| default_construct_prototype_slot(target, env)))
}

fn cross_realm_construct_prototype_slot(
    target: &Value,
    new_target: &Value,
    env: &CallEnv,
) -> Result<Option<crate::Prototype>, RuntimeError> {
    let marker = match target {
        Value::Function(function) if function.native == Some(NativeFunction::Boolean) => {
            CROSS_REALM_BOOLEAN_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::Number) => {
            CROSS_REALM_NUMBER_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::String) => {
            CROSS_REALM_STRING_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::Date) => {
            CROSS_REALM_DATE_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::Error) => {
            CROSS_REALM_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::EvalError) => {
            CROSS_REALM_EVAL_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::RangeError) => {
            CROSS_REALM_RANGE_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::ReferenceError) => {
            CROSS_REALM_REFERENCE_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::SyntaxError) => {
            CROSS_REALM_SYNTAX_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::TypeError) => {
            CROSS_REALM_TYPE_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::UriError) => {
            CROSS_REALM_URI_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::AggregateError) => {
            CROSS_REALM_AGGREGATE_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::SuppressedError) => {
            CROSS_REALM_SUPPRESSED_ERROR_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::RegExp) => {
            CROSS_REALM_REGEXP_PROTOTYPE
        }
        Value::Function(function) if function.native == Some(NativeFunction::Iterator) => {
            CROSS_REALM_ITERATOR_PROTOTYPE
        }
        Value::Function(_) => CROSS_REALM_OBJECT_PROTOTYPE,
        Value::Proxy(proxy) => {
            return cross_realm_construct_prototype_slot(&proxy.target_result()?, new_target, env);
        }
        _ => CROSS_REALM_OBJECT_PROTOTYPE,
    };
    marked_realm_prototype_slot(new_target, marker, env)
}

fn marked_realm_prototype_slot(
    new_target: &Value,
    marker: &str,
    env: &CallEnv,
) -> Result<Option<crate::Prototype>, RuntimeError> {
    match new_target {
        Value::Function(function) => Ok(function
            .own_property(marker)
            .and_then(|property| prototype_value_to_slot(property.value, env))),
        Value::Proxy(proxy) => marked_realm_prototype_slot(&proxy.target_result()?, marker, env),
        _ => Ok(None),
    }
}

fn prototype_value_to_slot(value: Value, env: &CallEnv) -> Option<crate::Prototype> {
    match value {
        Value::Object(prototype) if !symbol::is_symbol_primitive(&prototype) => {
            Some(crate::Prototype::Object(prototype))
        }
        Value::Function(prototype) => Some(crate::Prototype::Function(prototype)),
        Value::Array(array) => Some(crate::Prototype::Object(crate::array_as_object_prototype(
            &array, env,
        ))),
        Value::Proxy(prototype) => Some(crate::Prototype::Proxy(prototype)),
        _ => None,
    }
}

fn default_construct_prototype_slot(target: &Value, env: &CallEnv) -> Option<crate::Prototype> {
    match target {
        Value::Function(function) if function.native.is_some() => {
            function_prototype(function).map(crate::Prototype::Object)
        }
        Value::Function(_) => object_prototype(env).map(crate::Prototype::Object),
        Value::Proxy(proxy) => proxy
            .target_result()
            .ok()
            .and_then(|target| default_construct_prototype_slot(&target, env)),
        _ => object_prototype(env).map(crate::Prototype::Object),
    }
}

pub(crate) fn ensure_constructor(value: &Value) -> Result<(), RuntimeError> {
    match value {
        Value::Function(function) if function.constructable => Ok(()),
        Value::Proxy(proxy) if crate::proxy::proxy_is_constructor(proxy) => Ok(()),
        _ => Err(not_constructor_error()),
    }
}

fn not_constructor_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: value is not a constructor".to_owned(),
    }
}

struct FunctionCallEnv<'a> {
    env: CallEnv,
    direct_call_slots: Option<DirectCallSlots<'a>>,
}

fn direct_leaf_function_env<'a>(
    function: &'a Function,
    bytecode: &Bytecode,
    this_value: Value,
    argument_values: &'a [Value],
    env: &CallEnv,
) -> FunctionCallEnv<'a> {
    debug_assert!(can_seed_direct_leaf_call(function, bytecode));
    let mut frame_env = env.new_direct_leaf_function_frame();
    let direct_this_value = if bytecode.uses_lexical_this() {
        let this_env_storage = callee_this_realm_env(function, env);
        let this_env = this_env_storage.as_ref().unwrap_or(env);
        Some(function_call_this(
            Some(this_value),
            this_env,
            function.is_strict,
        ))
    } else {
        None
    };
    insert_marked_call_realm(function, &mut frame_env);
    if let Some(host) = function.module_host.clone() {
        frame_env.set_module_host(host);
    }
    frame_env.set_module_imports(function.module_imports.clone());
    frame_env.set_private_environment(function_private_environment(function));
    FunctionCallEnv {
        env: frame_env,
        direct_call_slots: Some(DirectCallSlots {
            this_value: direct_this_value,
            params: &function.params,
            arguments: argument_values,
            upvalues: &function.upvalues,
            realm_upvalue_slots: function.realm_upvalue_slots,
        }),
    }
}

fn function_env<'a>(
    function: &'a Function,
    bytecode: &Bytecode,
    callee: &Value,
    this_value: Value,
    argument_values: &'a [Value],
    env: &CallEnv,
    is_construct: bool,
) -> FunctionCallEnv<'a> {
    let use_direct_call_slots = can_seed_direct_leaf_call(function, bytecode);
    let lexical_this = received_upvalue_value(function, bytecode, "this");
    let lexical_field_initializer =
        received_upvalue_value(function, bytecode, FIELD_INITIALIZER_EVAL_BINDING);
    // Ordinary calls materialize `this`, each positional parameter, and a
    // small number of internal context bindings. Reserve that hot-path shape
    // up front instead of growing the frame binding vector several times.
    let mut frame_env = if use_direct_call_slots {
        env.new_direct_leaf_function_frame()
    } else {
        env.new_function_frame_with_capacity(function.params.positional.len().saturating_add(4))
    };
    let mut direct_this_value = None;
    if function.has_name_binding
        && let Some(name) = &function.name
        && !function_name_is_shadowed_by_body_var(function, bytecode, name)
    {
        frame_env.insert(name.clone(), callee.clone());
    }
    // A derived class constructor needs its own constructor value at hand so
    // that `super(...)` can initialize the instance fields once `this` exists.
    if function.is_class_constructor && function.is_derived_constructor && is_construct {
        frame_env.insert(crate::ACTIVE_CONSTRUCTOR_BINDING.to_owned(), callee.clone());
    }
    if function.is_field_initializer {
        frame_env.insert(
            FIELD_INITIALIZER_EVAL_BINDING.to_owned(),
            Value::Boolean(true),
        );
    } else if function.lexical_this
        && let Some(field_initializer) = lexical_field_initializer
    {
        frame_env.insert(FIELD_INITIALIZER_EVAL_BINDING.to_owned(), field_initializer);
    }
    insert_super_bindings(&mut frame_env, function, env, is_construct);
    // A derived-class constructor leaves `this` uninitialized (a TDZ): reading
    // `this` before `super(...)` is a ReferenceError, and `super(...)` binds
    // it. Every other function gets its `this` here.
    if function.is_derived_constructor && is_construct {
        frame_env.remove_frame_binding("this");
    } else if function.lexical_this {
        let caller_has_derived_this_tdz = env.has_local_binding(crate::SUPER_CONSTRUCTOR_BINDING)
            || env.has_local_binding(crate::ACTIVE_CONSTRUCTOR_BINDING);
        let inherited_this = if caller_has_derived_this_tdz {
            env.get_local("this")
        } else {
            lexical_this
                .or_else(|| env.get_local("this"))
                .or_else(|| env.get("this"))
        };
        if let Some(this_value) = inherited_this {
            if matches!(
                &this_value,
                Value::Function(function) if function.is_uninitialized_lexical_marker()
            ) {
                frame_env.remove_frame_binding("this");
            } else {
                frame_env.insert("this".to_owned(), this_value);
            }
        }
    } else {
        let call_this = if !use_direct_call_slots || bytecode.uses_lexical_this() {
            let this_env_storage = callee_this_realm_env(function, env);
            let this_env = this_env_storage.as_ref().unwrap_or(env);
            Some(function_call_this(
                Some(this_value),
                this_env,
                function.is_strict,
            ))
        } else {
            None
        };
        if use_direct_call_slots {
            direct_this_value = call_this;
        } else {
            frame_env.insert(
                "this".to_owned(),
                call_this.expect("general calls always bind this"),
            );
        }
    }
    if !use_direct_call_slots {
        for (index, element) in function.params.positional.iter().enumerate() {
            let value = argument_values
                .get(index)
                .cloned()
                .unwrap_or(Value::Undefined);
            frame_env.insert(parameter_binding_name(&element.binding, index), value);
        }
    }
    let parameter_shadows_arguments =
        !use_direct_call_slots && parameter_list_contains_name(&function.params, "arguments");
    let has_own_arguments_object = !function.lexical_arguments
        && !parameter_shadows_arguments
        && bytecode.needs_arguments_object();
    if has_own_arguments_object {
        frame_env.insert(
            "arguments".to_owned(),
            arguments_object(function, argument_values, &frame_env),
        );
    }
    if let Some(rest) = &function.params.rest {
        let values = argument_values
            .iter()
            .skip(function.params.positional.len())
            .cloned()
            .collect();
        frame_env.insert(
            rest_parameter_binding_name(rest),
            Value::Array(ArrayRef::new(values)),
        );
    }
    if has_own_arguments_object || parameter_shadows_arguments {
        frame_env.insert(
            DIRECT_EVAL_ARGUMENTS_BINDING.to_owned(),
            Value::Boolean(true),
        );
    }
    // The marker is only observable to direct eval in this frame or to a
    // lexically nested function (notably an arrow) that inherits its function
    // context. Leaf functions without eval can avoid allocating the internal
    // binding on every call.
    if (!function.lexical_this || function.is_field_initializer)
        && (bytecode.contains_direct_eval() || bytecode.creates_closures())
    {
        frame_env.insert(
            DIRECT_EVAL_FUNCTION_CONTEXT_BINDING.to_owned(),
            Value::Boolean(true),
        );
    }
    if let Some(name) = &function.immutable_env_binding {
        let captured_value = received_upvalue_value(function, bytecode, name)
            .or_else(|| function.immutable_env_value.as_ref().map(|cell| cell.get()));
        if let Some(value) = captured_value {
            frame_env.insert(name.clone(), value);
        }
    }
    insert_marked_call_realm(function, &mut frame_env);
    if function.immutable_name_binding
        && let Some(name) = &function.name
    {
        frame_env.set_immutable_function_name(name.clone());
    } else if let Some(name) = &function.immutable_env_binding {
        frame_env.set_immutable_function_name(name.clone());
    } else if function.lexical_this
        && let Some(name) = env.immutable_function_name()
    {
        frame_env.set_immutable_function_name(name.to_owned());
    }
    if let Some(host) = function.module_host.clone() {
        frame_env.set_module_host(host);
    }
    frame_env.set_module_imports(function.module_imports.clone());
    frame_env.set_private_environment(function_private_environment(function));
    if let Some(bindings) = &function.deopt_bindings {
        frame_env.set_deopt_bindings(bindings.clone());
    }
    let direct_call_slots = use_direct_call_slots.then(|| DirectCallSlots {
        this_value: direct_this_value,
        params: &function.params,
        arguments: argument_values,
        upvalues: &function.upvalues,
        realm_upvalue_slots: function.realm_upvalue_slots,
    });
    FunctionCallEnv {
        env: frame_env,
        direct_call_slots,
    }
}

fn can_seed_direct_leaf_call(function: &Function, bytecode: &Bytecode) -> bool {
    // Ordinary constructors use the same slot-backed parameter and receiver
    // model as ordinary calls. Constructor-only state such as `new.target`
    // remains in the small compatibility frame installed by function_env.
    !function.lexical_this
        && !function.lexical_arguments
        && !function.is_generator
        && !function.is_async
        && !function.is_class_constructor
        && !function.has_name_binding
        && !function.immutable_name_binding
        && (function.immutable_env_binding.is_none() || function.is_field_initializer)
        && function.deopt_bindings.is_none()
        && function.with_stack.is_empty()
        && function.params.is_simple()
        && !bytecode.needs_arguments_object()
        && !bytecode.contains_direct_eval()
        && !bytecode.contains_with()
        && !bytecode.contains_super_operation()
        && !bytecode.creates_closures()
}

fn parameter_list_contains_name(params: &FunctionParams, expected: &str) -> bool {
    params
        .positional
        .iter()
        .any(|element| binding_contains_name(&element.binding, expected))
        || params
            .rest
            .as_deref()
            .is_some_and(|binding| binding_contains_name(binding, expected))
}

fn binding_contains_name(binding: &BindingPattern, expected: &str) -> bool {
    match binding {
        BindingPattern::Identifier { name, .. } => name == expected,
        BindingPattern::Array { elements, rest, .. } => {
            elements
                .iter()
                .flatten()
                .any(|element| binding_contains_name(&element.binding, expected))
                || rest
                    .as_deref()
                    .is_some_and(|binding| binding_contains_name(binding, expected))
        }
        BindingPattern::Object {
            properties, rest, ..
        } => {
            properties
                .iter()
                .any(|property| binding_contains_name(&property.binding, expected))
                || rest
                    .as_deref()
                    .is_some_and(|binding| binding_contains_name(binding, expected))
        }
    }
}

fn received_upvalue_value(function: &Function, bytecode: &Bytecode, name: &str) -> Option<Value> {
    bytecode
        .received_upvalue_names()
        .zip(&function.upvalues)
        .find_map(|(candidate, upvalue)| (candidate == name).then(|| upvalue.get()))
}

fn insert_marked_call_realm(function: &Function, frame_env: &mut CallEnv) {
    let cached_global = function.dynamic_function_realm_global.as_ref();
    let has_override = function.has_dynamic_function_realm_override.get();
    if cached_global.is_none() && !has_override {
        return;
    }
    let own_global = has_override.then(|| {
        function
            .own_property(DYNAMIC_FUNCTION_REALM_GLOBAL)
            .and_then(|property| match property.value {
                Value::Object(global) => Some(global),
                _ => None,
            })
    });
    let Some(global) = own_global.flatten().or_else(|| cached_global.cloned()) else {
        return;
    };
    frame_env.insert(
        DYNAMIC_FUNCTION_REALM_GLOBAL.to_owned(),
        Value::Object(global.clone()),
    );
    frame_env.insert(
        GLOBAL_THIS_BINDING.to_owned(),
        Value::Object(global.clone()),
    );
    for name in global.own_property_names() {
        if let Some(property) = global.own_property(&name) {
            frame_env.insert(name, property.value);
        }
    }
}

fn function_name_is_shadowed_by_body_var(
    function: &Function,
    bytecode: &Bytecode,
    name: &str,
) -> bool {
    function.immutable_name_binding
        && bytecode
            .local_slot(name)
            .is_some_and(|slot| bytecode.local_is_body_hoist_only(slot))
}

fn callee_this_realm_env(function: &Function, caller_env: &CallEnv) -> Option<CallEnv> {
    if function.is_strict {
        return None;
    }
    let realm = function.realm.as_ref()?;
    if Rc::ptr_eq(realm, caller_env.realm()) {
        return None;
    }
    let captured_global = realm.global_this()?;
    if caller_env
        .global_this()
        .is_some_and(|caller_global| caller_global.same_value(&captured_global))
    {
        return None;
    }
    Some(CallEnv::new(Rc::clone(realm)))
}

/// Installs the per-frame `super` and `new.target` bindings. A method or
/// constructor uses its own `[[HomeObject]]`, parent constructor, and (when
/// constructing) `new.target`; an arrow inherits all three from the enclosing
/// frame's environment so `super` and `new.target` work lexically inside it.
fn insert_super_bindings(
    frame_env: &mut CallEnv,
    function: &Function,
    caller_env: &CallEnv,
    is_construct: bool,
) {
    use crate::{HOME_OBJECT_BINDING, NEW_TARGET_BINDING, SUPER_CONSTRUCTOR_BINDING};

    // Methods/constructors use their own home object and parent constructor;
    // arrows inherit both from the enclosing frame so `super` works lexically.
    if let Some(home) = function.home_object() {
        frame_env.insert(HOME_OBJECT_BINDING.to_owned(), home);
    } else if function.lexical_this
        && let Some(home) = caller_env.get_local(HOME_OBJECT_BINDING)
    {
        frame_env.insert(HOME_OBJECT_BINDING.to_owned(), home);
    }

    if let Some(super_constructor) = function.super_constructor() {
        frame_env.insert(SUPER_CONSTRUCTOR_BINDING.to_owned(), super_constructor);
    } else if function.lexical_this
        && let Some(super_constructor) = caller_env.get(SUPER_CONSTRUCTOR_BINDING)
    {
        frame_env.insert(SUPER_CONSTRUCTOR_BINDING.to_owned(), super_constructor);
    }

    // `new.target` reaches a constructor frame from `construct_function` (which
    // writes it into the call env). Arrows inherit it lexically; ordinary
    // calls see `new.target` undefined.
    if function.lexical_this {
        if let Some(new_target) = &function.lexical_new_target {
            frame_env.insert(NEW_TARGET_BINDING.to_owned(), new_target.get());
        }
    } else if is_construct && let Some(new_target) = caller_env.get(NEW_TARGET_BINDING) {
        frame_env.insert(NEW_TARGET_BINDING.to_owned(), new_target);
    }
}

fn function_private_environment(function: &Function) -> Option<PrivateEnvironment> {
    if let Some(environment) = function.private_environment() {
        return Some(environment);
    }
    match function.home_object() {
        Some(Value::Object(object)) => object.private_environment(),
        Some(Value::Function(function)) => function.private_environment(),
        _ => None,
    }
}
