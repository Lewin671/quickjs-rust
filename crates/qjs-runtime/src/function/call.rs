use std::{cell::RefCell, collections::HashMap, rc::Rc};

use qjs_ast::BindingPattern;

use crate::{
    ArrayRef, Bytecode, Function, GLOBAL_THIS_BINDING, NEW_TARGET_BINDING, NativeFunction,
    ObjectRef, Property, RUNTIME_INTRINSIC_NAMES, RuntimeError, Value,
    bytecode::eval_function_bytecode, native::call_native_function, object_prototype, symbol,
};

use super::{
    function_call_this, is_internal_binding_name, parameter_binding_name,
    rest_parameter_binding_name,
};

pub(crate) fn call_function(
    callee: Value,
    this_value: Value,
    argument_values: Vec<Value>,
    env: &mut HashMap<String, Value>,
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    let Value::Function(function) = callee.clone() else {
        return Err(RuntimeError {
            thrown: None,
            message: "value is not callable".to_owned(),
        });
    };
    if let Some(bound) = &function.bound {
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
        // Calling a generator function does not run its body: it captures the
        // call frame and returns a generator object in the SuspendedStart state.
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
            let activation_captured_env = Rc::new(RefCell::new(function_env.env.clone()));
            return Ok(crate::generator::make_generator_object(
                &function,
                crate::bytecode::GeneratorStart {
                    bytecode: bytecode.clone(),
                    env: function_env.env,
                    captured_env: activation_captured_env,
                },
                env,
            ));
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
        let activation_captured_env = Rc::new(RefCell::new(function_env.env.clone()));
        let result = eval_function_bytecode(bytecode, function_env.env, activation_captured_env);
        propagate_function_captures(&function, &function_env.function_capture_names, &result);
        propagate_caller_bindings(env, &function_env.caller_binding_names, &result);
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
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    // Private brands and private field values install alongside public fields,
    // before the constructor body runs, so the body may use `this.#x`.
    crate::bytecode::apply_instance_private_elements(function, this_value, env)?;
    let fields = function.instance_fields.borrow().clone();
    for field in fields {
        let value = match &field.initializer {
            Some(thunk) => call_function(
                Value::Function(thunk.clone()),
                this_value.clone(),
                Vec::new(),
                env,
                false,
            )?,
            None => Value::Undefined,
        };
        crate::bytecode::install_field_value(this_value, field.key.clone(), value)?;
    }
    Ok(())
}

fn finish_derived_construct(
    result: crate::bytecode::FunctionBytecodeResult<'_>,
) -> Result<Value, RuntimeError> {
    let bound_this = result.binding("this").cloned();
    let value = result.value?;
    match value {
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
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: derived constructor may only return an object or undefined"
                .to_owned(),
        }),
    }
}

pub(crate) fn construct_function(
    target: Value,
    new_target: Value,
    argument_values: Vec<Value>,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    ensure_constructor(&target)?;
    ensure_constructor(&new_target)?;

    // Make `new.target` visible to the constructor frame (and, via `super(...)`,
    // to ancestor constructors) so subclass instances get the right prototype.
    let previous_new_target = env.insert(NEW_TARGET_BINDING.to_owned(), new_target.clone());

    // A derived constructor must create its `this` through `super(...)`, so it
    // receives no pre-built receiver. Every other constructor gets an ordinary
    // object whose prototype comes from `new.target.prototype`.
    let is_derived =
        matches!(&target, Value::Function(function) if function.is_derived_constructor);
    let this_value = if is_derived {
        Value::Undefined
    } else {
        let prototype = crate::constructor_prototype_slot(&new_target, env);
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
        _ if is_derived => Ok(result),
        _ => Ok(this_value),
    }
}

pub(crate) fn ensure_constructor(value: &Value) -> Result<(), RuntimeError> {
    let Value::Function(function) = value else {
        return Err(not_constructor_error());
    };
    if !function.constructable {
        return Err(not_constructor_error());
    }
    Ok(())
}

fn not_constructor_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: value is not a constructor".to_owned(),
    }
}

struct FunctionCallEnv {
    env: HashMap<String, Value>,
    function_capture_names: Vec<String>,
    caller_binding_names: Vec<String>,
}

fn function_env(
    function: &Function,
    bytecode: &Bytecode,
    callee: Value,
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
    is_construct: bool,
) -> FunctionCallEnv {
    let captured_env = function.captured_env.borrow();
    let lexical_this = captured_env.get("this").cloned();
    let mut local_env = HashMap::with_capacity(
        RUNTIME_INTRINSIC_NAMES.len()
            + captured_env.len()
            + function.params.binding_count()
            + argument_values.len()
            + 3,
    );
    insert_runtime_intrinsics(&mut local_env, &captured_env, env);
    let function_capture_names = insert_function_captures(
        &mut local_env,
        bytecode,
        &function.local_names,
        &captured_env,
    );
    drop(captured_env);
    let mut caller_binding_names = Vec::new();
    insert_caller_bytecode_bindings(
        &mut local_env,
        &mut caller_binding_names,
        bytecode,
        &function.local_names,
        env,
    );
    insert_caller_scope_bindings(
        &mut local_env,
        &mut caller_binding_names,
        &function.local_names,
        env,
    );
    if let Some(global_this) = env.get(GLOBAL_THIS_BINDING).cloned() {
        local_env.insert(GLOBAL_THIS_BINDING.to_owned(), global_this);
    }
    if let Some(name) = &function.name {
        local_env.insert(name.clone(), callee.clone());
    }
    // A derived class constructor needs its own constructor value at hand so
    // that `super(...)` can initialize the instance fields once `this` exists.
    if function.is_class_constructor && function.is_derived_constructor && is_construct {
        local_env.insert(crate::ACTIVE_CONSTRUCTOR_BINDING.to_owned(), callee.clone());
    }
    insert_super_bindings(&mut local_env, function, env, is_construct);
    // A derived-class constructor leaves `this` uninitialized (a TDZ): reading
    // `this` before `super(...)` is a ReferenceError, and `super(...)` binds
    // it. Every other function gets its `this` here.
    if function.is_derived_constructor && is_construct {
        local_env.remove("this");
    } else {
        local_env.insert(
            "this".to_owned(),
            if function.lexical_this {
                lexical_this.unwrap_or(Value::Undefined)
            } else {
                function_call_this(Some(this_value), env, function.is_strict)
            },
        );
    }
    for (index, element) in function.params.positional.iter().enumerate() {
        let value = argument_values
            .get(index)
            .cloned()
            .unwrap_or(Value::Undefined);
        local_env.insert(parameter_binding_name(&element.binding, index), value);
    }
    if !function.lexical_arguments {
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
    FunctionCallEnv {
        env: local_env,
        function_capture_names,
        caller_binding_names,
    }
}

/// Installs the per-frame `super` and `new.target` bindings. A method or
/// constructor uses its own `[[HomeObject]]`, parent constructor, and (when
/// constructing) `new.target`; an arrow inherits all three from the enclosing
/// frame's environment so `super` and `new.target` work lexically inside it.
fn insert_super_bindings(
    local_env: &mut HashMap<String, Value>,
    function: &Function,
    caller_env: &HashMap<String, Value>,
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
        local_env.insert(HOME_OBJECT_BINDING.to_owned(), home.clone());
    }

    if let Some(super_constructor) = function.super_constructor.borrow().clone() {
        local_env.insert(SUPER_CONSTRUCTOR_BINDING.to_owned(), super_constructor);
    } else if function.lexical_this
        && let Some(super_constructor) = caller_env.get(SUPER_CONSTRUCTOR_BINDING)
    {
        local_env.insert(
            SUPER_CONSTRUCTOR_BINDING.to_owned(),
            super_constructor.clone(),
        );
    }

    // `new.target` reaches a constructor frame from `construct_function` (which
    // writes it into the call env). Arrows inherit it lexically; ordinary
    // calls see `new.target` undefined.
    if (is_construct || function.lexical_this)
        && let Some(new_target) = caller_env.get(NEW_TARGET_BINDING)
    {
        local_env.insert(NEW_TARGET_BINDING.to_owned(), new_target.clone());
    }
}

fn arguments_object(
    function: &Function,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Value {
    let object = ObjectRef::with_prototype(HashMap::new(), object_prototype(env));
    object.define_property(
        "length".to_owned(),
        Property::data(
            Value::Number(argument_values.len() as f64),
            false,
            true,
            true,
        ),
    );
    for (index, value) in argument_values.iter().cloned().enumerate() {
        if let Some(parameter_name) = mapped_argument_parameter(function, index) {
            object.define_property(
                index.to_string(),
                mapped_argument_property(parameter_name.to_owned(), value),
            );
        } else {
            object.define_property(index.to_string(), Property::enumerable(value));
        }
    }
    define_arguments_iterator(&object, env);
    object.set_to_string_tag("Arguments");
    Value::Object(object)
}

fn mapped_argument_parameter(function: &Function, index: usize) -> Option<&str> {
    if function.is_strict || !function.params.is_simple() {
        return None;
    }
    let element = function.params.positional.get(index)?;
    let BindingPattern::Identifier {
        name: parameter_name,
        ..
    } = &element.binding
    else {
        return None;
    };
    if parameter_name.is_empty() {
        return None;
    }
    if function
        .params
        .positional
        .iter()
        .skip(index + 1)
        .any(|element| {
            matches!(
                &element.binding,
                BindingPattern::Identifier { name, .. } if name == parameter_name
            )
        })
    {
        None
    } else {
        Some(parameter_name)
    }
}

fn mapped_argument_property(parameter_name: String, initial_value: Value) -> Property {
    let backing = ObjectRef::new(HashMap::from([("value".to_owned(), initial_value)]));
    Property::accessor(
        Some(mapped_argument_getter(
            parameter_name.clone(),
            backing.clone(),
        )),
        Some(mapped_argument_setter(parameter_name, backing)),
        true,
        true,
    )
}

fn mapped_argument_getter(parameter_name: String, backing: ObjectRef) -> Value {
    let target = Value::Function(Function::new_native(
        Some("[[MappedArgumentGet]]"),
        1,
        NativeFunction::MappedArgumentGet,
        false,
    ));
    Value::Function(Function::new_bound(
        target,
        Value::Undefined,
        vec![Value::String(parameter_name), Value::Object(backing)],
        1,
    ))
}

fn mapped_argument_setter(parameter_name: String, backing: ObjectRef) -> Value {
    let target = Value::Function(Function::new_native(
        Some("[[MappedArgumentSet]]"),
        1,
        NativeFunction::MappedArgumentSet,
        false,
    ));
    Value::Function(Function::new_bound(
        target,
        Value::Undefined,
        vec![Value::String(parameter_name), Value::Object(backing)],
        1,
    ))
}

fn define_arguments_iterator(object: &ObjectRef, env: &HashMap<String, Value>) {
    let Some(iterator) = symbol::iterator_symbol(env) else {
        return;
    };
    object.define_symbol_property(
        iterator,
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("[Symbol.iterator]"),
            0,
            NativeFunction::ArrayPrototypeValues,
            false,
        ))),
    );
}

pub(crate) fn native_mapped_argument_get(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Some(parameter_name) = mapped_argument_name(argument_values) else {
        return Ok(Value::Undefined);
    };
    Ok(env
        .get(parameter_name)
        .cloned()
        .or_else(|| {
            mapped_argument_backing(argument_values).and_then(|backing| backing.get("value"))
        })
        .unwrap_or(Value::Undefined))
}

pub(crate) fn native_mapped_argument_set(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Some(parameter_name) = mapped_argument_name(argument_values) else {
        return Ok(Value::Undefined);
    };
    let value = argument_values.get(2).cloned().unwrap_or(Value::Undefined);
    if let Some(binding) = env.get_mut(parameter_name) {
        *binding = value.clone();
    }
    if let Some(backing) = mapped_argument_backing(argument_values) {
        backing.set("value".to_owned(), value);
    }
    Ok(Value::Undefined)
}

fn mapped_argument_name(argument_values: &[Value]) -> Option<&str> {
    match argument_values.first() {
        Some(Value::String(name)) => Some(name),
        _ => None,
    }
}

fn mapped_argument_backing(argument_values: &[Value]) -> Option<ObjectRef> {
    match argument_values.get(1) {
        Some(Value::Object(object)) => Some(object.clone()),
        _ => None,
    }
}

fn insert_runtime_intrinsics(
    local_env: &mut HashMap<String, Value>,
    function_env: &HashMap<String, Value>,
    caller_env: &HashMap<String, Value>,
) {
    for name in RUNTIME_INTRINSIC_NAMES {
        if let Some(value) = caller_env.get(*name).or_else(|| function_env.get(*name)) {
            local_env.insert((*name).to_owned(), value.clone());
        }
    }
}

fn insert_function_captures(
    local_env: &mut HashMap<String, Value>,
    bytecode: &Bytecode,
    function_local_names: &[String],
    function_env: &HashMap<String, Value>,
) -> Vec<String> {
    let mut names = Vec::new();
    for name in bytecode.global_names() {
        insert_function_capture(local_env, &mut names, function_env, name);
    }
    for name in bytecode.local_names() {
        if function_local_names
            .binary_search_by(|local| local.as_str().cmp(name))
            .is_err()
        {
            insert_function_capture(local_env, &mut names, function_env, name);
        }
    }
    names
}

fn insert_function_capture(
    local_env: &mut HashMap<String, Value>,
    names: &mut Vec<String>,
    function_env: &HashMap<String, Value>,
    name: &str,
) {
    if is_internal_binding_name(name) {
        return;
    }
    if let Some(value) = function_env.get(name) {
        local_env.insert(name.to_owned(), value.clone());
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
    env: &HashMap<String, Value>,
) {
    for name in bytecode.global_names() {
        insert_caller_binding(local_env, caller_binding_names, env, name);
    }
    for name in bytecode.local_names() {
        if function_local_names
            .binary_search_by(|local| local.as_str().cmp(name))
            .is_err()
        {
            insert_caller_binding(local_env, caller_binding_names, env, name);
        }
    }
}

fn insert_caller_binding(
    local_env: &mut HashMap<String, Value>,
    caller_binding_names: &mut Vec<String>,
    env: &HashMap<String, Value>,
    name: &str,
) {
    if is_internal_binding_name(name) {
        return;
    }
    if let Some(value) = env.get(name) {
        local_env.insert(name.to_owned(), value.clone());
        insert_missing_caller_binding_name(caller_binding_names, name);
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
    env: &HashMap<String, Value>,
) {
    for name in env.keys() {
        if is_call_frame_binding(name)
            || RUNTIME_INTRINSIC_NAMES.contains(&name.as_str())
            || function_local_names
                .binary_search_by(|local| local.as_str().cmp(name))
                .is_ok()
        {
            continue;
        }
        insert_caller_binding(local_env, caller_binding_names, env, name);
    }
}

fn propagate_caller_bindings(
    env: &mut HashMap<String, Value>,
    caller_binding_names: &[String],
    result: &crate::bytecode::FunctionBytecodeResult<'_>,
) {
    for name in caller_binding_names {
        if !is_call_frame_binding(name)
            && let Some(value) = env.get_mut(name)
            && let Some(final_value) = result.binding(name)
        {
            *value = final_value.clone();
        }
    }
    for name in &result.sloppy_global_names {
        if !is_call_frame_binding(name)
            && !env.contains_key(name)
            && let Some(final_value) = result.binding(name)
        {
            env.insert(name.clone(), final_value.clone());
        }
    }
}

fn propagate_function_captures(
    function: &Function,
    function_capture_names: &[String],
    result: &crate::bytecode::FunctionBytecodeResult<'_>,
) {
    if function_capture_names.is_empty() {
        return;
    }
    let mut captured_env = function.captured_env.borrow_mut();
    for name in function_capture_names {
        if !is_call_frame_binding(name)
            && let Some(final_value) = result.binding(name)
        {
            captured_env.insert(name.clone(), final_value.clone());
        }
    }
}

fn is_call_frame_binding(name: &str) -> bool {
    matches!(name, GLOBAL_THIS_BINDING | "this" | "arguments")
}
