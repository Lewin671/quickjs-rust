use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    ArrayRef, Bytecode, DIRECT_EVAL_ARGUMENTS_BINDING, DIRECT_EVAL_FUNCTION_CONTEXT_BINDING,
    FIELD_INITIALIZER_EVAL_BINDING, Function, NEW_TARGET_BINDING, NativeFunction, ObjectRef,
    RuntimeError, Value, bytecode::eval_function_bytecode, function_prototype,
    native::call_native_function, object_prototype, private::PrivateEnvironment, symbol,
};

use super::{
    CallEnv, InstanceElementInitializer,
    arguments::arguments_object,
    captures::{
        caller_capture_matches_existing, caller_global_this_has_own_property,
        captured_global_this_has_own_property, is_call_frame_binding, propagate_function_captures,
        refresh_class_constructor_captures_from_caller, sync_global_var_captures,
    },
    function_call_this, is_internal_binding_name, parameter_binding_name,
    rest_parameter_binding_name,
};

const CROSS_REALM_OBJECT_PROTOTYPE: &str = "__quickjsRustRealmObjectPrototype";
const CROSS_REALM_REGEXP_PROTOTYPE: &str = "__quickjsRustRealmRegExpPrototype";

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
    let Value::Function(function) = callee.clone() else {
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
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: class constructor cannot be invoked without 'new'".to_owned(),
        });
    }
    if let Some(native) = function.native {
        return call_native_function(
            &function,
            native,
            this_value,
            argument_values,
            is_construct,
            env,
        );
    }
    if let Some(bytecode) = &function.bytecode {
        if function.is_class_constructor {
            refresh_class_constructor_captures_from_caller(&function, env);
        }
        if function.is_generator && function.is_async {
            let function_env = function_env(
                &function,
                bytecode,
                callee,
                this_value,
                &argument_values,
                env,
                is_construct,
            );
            return crate::async_generator::call_async_generator_function(
                &function,
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
                &function,
                bytecode,
                callee,
                this_value,
                &argument_values,
                env,
                is_construct,
            );
            let activation_captured_env = Rc::new(RefCell::new(function_env.env.snapshot_locals()));
            let capture_writeback = (!function_env.function_capture_names.is_empty()).then(|| {
                crate::bytecode::CaptureWriteback {
                    target: Rc::clone(&function.captured_env),
                    names: function_env.function_capture_names,
                    aliases: Vec::new(),
                    parent: None,
                }
            });
            return crate::generator::make_generator_object(
                &function,
                crate::bytecode::GeneratorStart {
                    bytecode: bytecode.clone(),
                    env: function_env.env,
                    captured_env: activation_captured_env,
                    with_stack: function.with_stack.clone(),
                    refresh_captured_slots_on_resume: true,
                    capture_writeback,
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
                &function,
                bytecode,
                callee,
                this_value,
                &argument_values,
                env,
                is_construct,
            );
            return Ok(crate::async_function::call_async_function(
                &function,
                function_env.env,
                function_env.function_capture_names,
                env,
            ));
        }
        // A base-class constructor initializes its instance fields right after
        // the receiver is created, before the constructor body runs. A derived
        // constructor defers this until `super(...)` binds `this`.
        if function.is_class_constructor && !function.is_derived_constructor && is_construct {
            initialize_instance_fields(&function, &this_value, env)?;
        }
        let function_env = function_env(
            &function,
            bytecode,
            callee,
            this_value,
            &argument_values,
            env,
            is_construct,
        );
        // The activation captured env is only ever read when the body creates a
        // nested closure or class (those ops snapshot it into the new function's
        // `captured_env`). A body that creates none never reads it, so skip
        // cloning the whole frame env into it on every leaf call.
        let activation_captured_env = if bytecode.creates_closures() {
            Rc::new(RefCell::new(function_env.env.snapshot_locals()))
        } else {
            Rc::new(RefCell::new(HashMap::new()))
        };
        let activation_writeback = (!function_env.function_capture_names.is_empty()).then(|| {
            crate::bytecode::CaptureWriteback {
                target: Rc::clone(&function.captured_env),
                names: function_env.function_capture_names.clone(),
                aliases: Vec::new(),
                parent: None,
            }
        });
        let result = eval_function_bytecode(
            bytecode,
            function_env.env,
            activation_captured_env,
            function.with_stack.clone(),
            activation_writeback,
        );
        propagate_function_captures(
            &function,
            bytecode,
            &function_env.function_capture_names,
            env,
            &result,
        );
        propagate_lexical_super_this(&function, bytecode, &result);
        propagate_caller_bindings(env, &function_env.caller_binding_names, &result);
        sync_global_var_captures(&function, bytecode, env);
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
    for (index, element) in elements.iter().enumerate() {
        match element {
            InstanceElementInitializer::PublicField(field) => {
                let value = match &field.initializer {
                    Some(thunk) => {
                        let value = call_function(
                            Value::Function(thunk.clone()),
                            this_value.clone(),
                            Vec::new(),
                            env,
                            false,
                        )?;
                        refresh_later_field_initializer_captures(&elements[index + 1..], thunk);
                        value
                    }
                    None => Value::Undefined,
                };
                crate::bytecode::install_field_value(this_value, field.key.clone(), value, env)?;
            }
            InstanceElementInitializer::PrivateElement(private) => {
                let initializer = private
                    .field_initializer
                    .as_ref()
                    .and_then(|field| field.initializer.as_ref());
                crate::bytecode::apply_instance_private_element(
                    function, this_value, private, env,
                )?;
                if let Some(thunk) = initializer {
                    refresh_later_field_initializer_captures(&elements[index + 1..], thunk);
                }
            }
        }
    }
    Ok(())
}

fn refresh_later_field_initializer_captures(
    elements: &[InstanceElementInitializer],
    source: &Function,
) {
    let source_env = source.captured_env.borrow();
    for element in elements {
        let target = match element {
            InstanceElementInitializer::PublicField(field) => field.initializer.as_ref(),
            InstanceElementInitializer::PrivateElement(private) => private
                .field_initializer
                .as_ref()
                .and_then(|field| field.initializer.as_ref()),
        };
        let Some(target) = target else { continue };
        let mut target_env = target.captured_env.borrow_mut();
        for (name, value) in source_env.iter() {
            if target_env.contains_key(name) {
                target_env.insert(name.clone(), value.clone());
            }
        }
    }
}

fn finish_derived_construct(
    result: crate::bytecode::FunctionBytecodeResult<'_>,
) -> Result<Value, RuntimeError> {
    let bound_this = result.frame_binding("this");
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
        Value::Function(function) if function.native == Some(NativeFunction::RegExp) => {
            CROSS_REALM_REGEXP_PROTOTYPE
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

struct FunctionCallEnv {
    env: CallEnv,
    function_capture_names: Vec<String>,
    caller_binding_names: Vec<String>,
}

fn function_env(
    function: &Function,
    bytecode: &Bytecode,
    callee: Value,
    this_value: Value,
    argument_values: &[Value],
    env: &CallEnv,
    is_construct: bool,
) -> FunctionCallEnv {
    let captured_env = function.captured_env.borrow();
    let lexical_this = captured_env.get("this").cloned();
    let lexical_field_initializer = captured_env.get(FIELD_INITIALIZER_EVAL_BINDING).cloned();
    let mut local_env = HashMap::with_capacity(
        captured_env.len() + function.params.binding_count() + argument_values.len() + 3,
    );
    let (function_capture_names, protected_capture_names) = insert_function_captures(
        &mut local_env,
        bytecode,
        &function.local_names,
        &captured_env,
    );
    drop(captured_env);
    refresh_written_global_captures_from_caller(
        &function.captured_env,
        &mut local_env,
        bytecode,
        &protected_capture_names,
        env,
    );
    let caller_shares_capture_source = env
        .captured_binding_source_env()
        .is_some_and(|source| Rc::ptr_eq(source, &function.captured_env));
    let mut caller_binding_names = Vec::new();
    insert_caller_bytecode_bindings(
        &mut local_env,
        &mut caller_binding_names,
        bytecode,
        &function.local_names,
        env,
        caller_shares_capture_source,
        &callee,
    );
    insert_caller_scope_bindings(
        &mut local_env,
        &mut caller_binding_names,
        &function.local_names,
        env,
    );
    if let Some(writeback) = &function.capture_writeback {
        if !function.is_field_initializer
            && !function.is_class_constructor
            && !function.lexical_this
            && !Rc::ptr_eq(&function.captured_env, &writeback.target)
        {
            refresh_writeback_captures_from_caller(&mut local_env, writeback, env);
        } else if let Some(parent) = writeback.parent.as_deref() {
            refresh_writeback_captures_from_caller(&mut local_env, parent, env);
        }
    }
    if function.has_name_binding
        && let Some(name) = &function.name
    {
        local_env.insert(name.clone(), callee.clone());
    }
    // A derived class constructor needs its own constructor value at hand so
    // that `super(...)` can initialize the instance fields once `this` exists.
    if function.is_class_constructor && function.is_derived_constructor && is_construct {
        local_env.insert(crate::ACTIVE_CONSTRUCTOR_BINDING.to_owned(), callee.clone());
    }
    if function.is_field_initializer {
        local_env.insert(
            FIELD_INITIALIZER_EVAL_BINDING.to_owned(),
            Value::Boolean(true),
        );
    } else if function.lexical_this
        && let Some(field_initializer) = lexical_field_initializer
    {
        local_env.insert(FIELD_INITIALIZER_EVAL_BINDING.to_owned(), field_initializer);
    }
    insert_super_bindings(&mut local_env, function, env, is_construct);
    // A derived-class constructor leaves `this` uninitialized (a TDZ): reading
    // `this` before `super(...)` is a ReferenceError, and `super(...)` binds
    // it. Every other function gets its `this` here.
    if function.is_derived_constructor && is_construct {
        local_env.remove("this");
    } else if function.lexical_this {
        let inherited_this = lexical_this.or_else(|| env.get_local("this")).or_else(|| {
            (!env.locals().contains_key(crate::SUPER_CONSTRUCTOR_BINDING))
                .then(|| env.get("this"))
                .flatten()
        });
        if let Some(this_value) = inherited_this {
            if matches!(
                &this_value,
                Value::Function(function) if function.is_uninitialized_lexical_marker()
            ) {
                local_env.remove("this");
            } else {
                local_env.insert("this".to_owned(), this_value);
            }
        }
    } else {
        local_env.insert(
            "this".to_owned(),
            function_call_this(Some(this_value), env, function.is_strict),
        );
    }
    for (index, element) in function.params.positional.iter().enumerate() {
        let value = argument_values
            .get(index)
            .cloned()
            .unwrap_or(Value::Undefined);
        local_env.insert(parameter_binding_name(&element.binding, index), value);
    }
    let has_own_arguments_object = !function.lexical_arguments && bytecode.needs_arguments_object();
    if has_own_arguments_object {
        local_env.insert(
            "arguments".to_owned(),
            arguments_object(function, argument_values, env),
        );
    }
    if let Some(rest) = &function.params.rest {
        let values = argument_values
            .iter()
            .skip(function.params.positional.len())
            .cloned()
            .collect();
        local_env.insert(
            rest_parameter_binding_name(rest),
            Value::Array(ArrayRef::new(values)),
        );
    }
    if has_own_arguments_object
        || function
            .params
            .names()
            .iter()
            .any(|name| name == "arguments")
    {
        local_env.insert(
            DIRECT_EVAL_ARGUMENTS_BINDING.to_owned(),
            Value::Boolean(true),
        );
    }
    if !function.lexical_this || function.is_field_initializer {
        local_env.insert(
            DIRECT_EVAL_FUNCTION_CONTEXT_BINDING.to_owned(),
            Value::Boolean(true),
        );
    }
    let mut frame_env = env.with_frame_locals(local_env);
    frame_env.set_private_environment(function_private_environment(function));
    if let Some(writeback) = &function.capture_writeback {
        frame_env.set_captured_binding_source_env(Rc::clone(&writeback.target));
    } else {
        frame_env.set_captured_binding_source_env(Rc::clone(&function.captured_env));
    }
    FunctionCallEnv {
        env: frame_env,
        function_capture_names,
        caller_binding_names,
    }
}

fn refresh_writeback_captures_from_caller(
    local_env: &mut HashMap<String, Value>,
    writeback: &crate::bytecode::CaptureWriteback,
    caller_env: &CallEnv,
) {
    for name in &writeback.names {
        if let Some(value) = caller_env
            .captured_binding_source_env()
            .and_then(|source| source.borrow().get(name).cloned())
            .or_else(|| caller_env.get(name))
        {
            local_env.insert(name.clone(), value);
        }
    }
    for (source_name, target_name) in &writeback.aliases {
        if let Some(value) = caller_env
            .get(target_name)
            .or_else(|| caller_env.get(source_name))
        {
            local_env.insert(source_name.clone(), value);
        }
    }
    if let Some(parent) = writeback.parent.as_deref() {
        refresh_writeback_captures_from_caller(local_env, parent, caller_env);
    }
}

fn refresh_written_global_captures_from_caller(
    captured_env: &Rc<RefCell<HashMap<String, Value>>>,
    local_env: &mut HashMap<String, Value>,
    bytecode: &Bytecode,
    protected_capture_names: &[String],
    caller_env: &CallEnv,
) {
    let written_names = bytecode.closure_written_binding_names();
    let names = local_env.keys().cloned().collect::<Vec<_>>();
    for name in names {
        if protected_capture_names
            .iter()
            .any(|protected| protected == &name)
            || !written_names.iter().any(|written| written == &name)
            || !captured_global_this_has_own_property(captured_env, &name)
            || !caller_global_this_has_own_property(caller_env, &name)
        {
            continue;
        }
        if let Some(value) = caller_env.get(&name) {
            local_env.insert(name, value);
        }
    }
}

/// Installs the per-frame `super` and `new.target` bindings. A method or
/// constructor uses its own `[[HomeObject]]`, parent constructor, and (when
/// constructing) `new.target`; an arrow inherits all three from the enclosing
/// frame's environment so `super` and `new.target` work lexically inside it.
fn insert_super_bindings(
    local_env: &mut HashMap<String, Value>,
    function: &Function,
    caller_env: &CallEnv,
    is_construct: bool,
) {
    use crate::{HOME_OBJECT_BINDING, NEW_TARGET_BINDING, SUPER_CONSTRUCTOR_BINDING};

    // Methods/constructors use their own home object and parent constructor;
    // arrows inherit both from the enclosing frame so `super` works lexically.
    if let Some(home) = function.home_object.borrow().clone() {
        local_env.insert(HOME_OBJECT_BINDING.to_owned(), home);
    } else if function.lexical_this
        && let Some(home) = caller_env.get(HOME_OBJECT_BINDING)
    {
        local_env.insert(HOME_OBJECT_BINDING.to_owned(), home);
    }

    if let Some(super_constructor) = function.super_constructor.borrow().clone() {
        local_env.insert(SUPER_CONSTRUCTOR_BINDING.to_owned(), super_constructor);
    } else if function.lexical_this
        && let Some(super_constructor) = caller_env.get(SUPER_CONSTRUCTOR_BINDING)
    {
        local_env.insert(SUPER_CONSTRUCTOR_BINDING.to_owned(), super_constructor);
    }

    // `new.target` reaches a constructor frame from `construct_function` (which
    // writes it into the call env). Arrows inherit it lexically; ordinary
    // calls see `new.target` undefined.
    if (is_construct || function.lexical_this)
        && let Some(new_target) = caller_env.get(NEW_TARGET_BINDING)
    {
        local_env.insert(NEW_TARGET_BINDING.to_owned(), new_target);
    } else if function.lexical_this
        && let Some(new_target) = function
            .captured_env
            .borrow()
            .get(NEW_TARGET_BINDING)
            .cloned()
    {
        local_env.insert(NEW_TARGET_BINDING.to_owned(), new_target);
    }
}

fn function_private_environment(function: &Function) -> Option<PrivateEnvironment> {
    if let Some(environment) = function.private_environment() {
        return Some(environment);
    }
    match function.home_object.borrow().clone() {
        Some(Value::Object(object)) => object.private_environment(),
        Some(Value::Function(function)) => function.private_environment(),
        _ => None,
    }
}

fn insert_function_captures(
    local_env: &mut HashMap<String, Value>,
    bytecode: &Bytecode,
    function_local_names: &[String],
    function_env: &HashMap<String, Value>,
) -> (Vec<String>, Vec<String>) {
    let mut writeback_names = Vec::new();
    let mut protected_names = Vec::new();
    let mut names = bytecode.closure_referenced_global_names();
    names.extend(bytecode.closure_written_binding_names());
    names.sort();
    names.dedup();
    for name in names {
        if function_local_names.iter().any(|local| local == &name) {
            continue;
        }
        insert_function_capture(
            local_env,
            &mut writeback_names,
            &mut protected_names,
            bytecode,
            function_env,
            &name,
        );
    }
    for name in bytecode.local_names() {
        if !function_local_names.iter().any(|local| local == name) {
            insert_function_capture(
                local_env,
                &mut writeback_names,
                &mut protected_names,
                bytecode,
                function_env,
                name,
            );
        }
    }
    (writeback_names, protected_names)
}

fn insert_function_capture(
    local_env: &mut HashMap<String, Value>,
    writeback_names: &mut Vec<String>,
    protected_names: &mut Vec<String>,
    bytecode: &Bytecode,
    function_env: &HashMap<String, Value>,
    name: &str,
) {
    if is_internal_binding_name(name) {
        return;
    }
    if let Some(value) = function_env.get(name) {
        local_env.insert(name.to_owned(), value.clone());
        let is_immutable_capture = bytecode
            .local_slot(name)
            .is_some_and(|slot| !bytecode.local_is_mutable(slot));
        let names = if is_immutable_capture {
            protected_names
        } else {
            writeback_names
        };
        if !names.iter().any(|existing| existing == name) {
            names.push(name.to_owned());
        }
    }
}

fn insert_caller_bytecode_bindings(
    local_env: &mut HashMap<String, Value>,
    caller_binding_names: &mut Vec<String>,
    bytecode: &Bytecode,
    function_local_names: &[String],
    env: &CallEnv,
    caller_shares_capture_source: bool,
    callee: &Value,
) {
    for name in bytecode.global_names() {
        let write_back_to_caller = !function_local_names.iter().any(|local| local == name);
        let existing_capture_matches_caller = caller_capture_matches_existing(
            local_env,
            env,
            name,
            caller_shares_capture_source,
            callee,
        );
        insert_caller_binding(
            local_env,
            caller_binding_names,
            env,
            name,
            write_back_to_caller,
            existing_capture_matches_caller
                && ((caller_shares_capture_source
                    && bytecode.sloppy_global_fallback_binding(name))
                    || (bytecode.local_slot(name).is_none()
                        && ((env.module_host().is_some() && env.realm_contains(name))
                            || (bytecode.writes_binding(name) && env.realm_contains(name))
                            || env
                                .captured_binding_source_env()
                                .is_some_and(|source| source.borrow().contains_key(name))))),
        );
    }
    for name in bytecode.sloppy_global_assignment_names() {
        if env.realm_contains(name) && !env.captures_binding(name) {
            insert_missing_caller_binding_name(caller_binding_names, name);
        }
    }
    for name in bytecode.local_names() {
        if !function_local_names.iter().any(|local| local == name) {
            let from_env = bytecode
                .local_slot(name)
                .is_some_and(|slot| bytecode.local_is_from_env(slot));
            let existing_capture_matches_caller = caller_capture_matches_existing(
                local_env,
                env,
                name,
                caller_shares_capture_source,
                callee,
            );
            let allow_live_env_override = from_env
                && existing_capture_matches_caller
                && (env.module_host().is_some()
                    || (bytecode.writes_binding(name)
                        && caller_global_this_has_own_property(env, name)));
            insert_caller_binding(
                local_env,
                caller_binding_names,
                env,
                name,
                true,
                allow_live_env_override,
            );
        }
    }
}

fn insert_caller_binding(
    local_env: &mut HashMap<String, Value>,
    caller_binding_names: &mut Vec<String>,
    env: &CallEnv,
    name: &str,
    write_back_to_caller: bool,
    allow_existing_override: bool,
) {
    if is_internal_binding_name(name) {
        return;
    }
    if local_env.contains_key(name) && !allow_existing_override {
        if write_back_to_caller && caller_global_this_has_own_property(env, name) {
            insert_missing_caller_binding_name(caller_binding_names, name);
        }
        return;
    }
    if let Some(value) = env.locals().get(name) {
        local_env.insert(name.to_owned(), value.clone());
        if write_back_to_caller {
            insert_missing_caller_binding_name(caller_binding_names, name);
        }
    } else if allow_existing_override && let Some(value) = env.get_realm(name) {
        local_env.insert(name.to_owned(), value);
    } else if allow_existing_override
        && let Some(source) = env.captured_binding_source_env()
        && let Some(value) = source.borrow().get(name).cloned()
    {
        local_env.insert(name.to_owned(), value);
    }
}

fn insert_missing_caller_binding_name(caller_binding_names: &mut Vec<String>, name: &str) {
    if !caller_binding_names.iter().any(|existing| existing == name) {
        caller_binding_names.push(name.to_owned());
    }
}

fn insert_caller_scope_bindings(
    local_env: &mut HashMap<String, Value>,
    caller_binding_names: &mut Vec<String>,
    function_local_names: &[String],
    env: &CallEnv,
) {
    // Iterate only the caller's frame locals; realm bindings are shared and need
    // no per-frame copy. The O(50)-per-key intrinsic scan is gone.
    let names: Vec<String> = env.locals().keys().cloned().collect();
    for name in names {
        if is_call_frame_binding(&name) || function_local_names.iter().any(|local| local == &name) {
            continue;
        }
        insert_caller_binding(local_env, caller_binding_names, env, &name, true, false);
    }
}

fn propagate_caller_bindings(
    env: &mut CallEnv,
    caller_binding_names: &[String],
    result: &crate::bytecode::FunctionBytecodeResult<'_>,
) {
    for name in caller_binding_names {
        if !is_call_frame_binding(name)
            && let Some(final_value) = result.binding(name)
        {
            if let Some(binding) = env.get_local_mut(name) {
                *binding = final_value;
            } else if env.realm_contains(name) {
                env.insert_realm(name.clone(), final_value);
            }
        }
    }
    // Sloppy-mode global creation: a new binding the callee introduced is
    // written to the shared realm so the caller (and every frame) sees it.
    for name in &result.sloppy_global_names {
        if !is_call_frame_binding(name)
            && !env.contains_key(name)
            && let Some(final_value) = result.binding(name)
        {
            env.insert_realm(name.clone(), final_value);
        }
    }
}

fn propagate_lexical_super_this(
    function: &Function,
    bytecode: &Bytecode,
    result: &crate::bytecode::FunctionBytecodeResult<'_>,
) {
    if !function.lexical_this || !bytecode.contains_super_call() {
        return;
    }
    let Some(this_value) = result.frame_binding("this") else {
        return;
    };
    if matches!(
        &this_value,
        Value::Function(function) if function.is_uninitialized_lexical_marker()
    ) {
        return;
    }
    function
        .captured_env
        .borrow_mut()
        .insert("this".to_owned(), this_value.clone());
    if let Some(writeback) = &function.capture_writeback {
        writeback
            .target
            .borrow_mut()
            .insert("this".to_owned(), this_value);
    }
}
