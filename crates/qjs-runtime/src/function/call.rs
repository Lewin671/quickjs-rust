use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    ArrayRef, Bytecode, Function, GLOBAL_THIS_BINDING, NativeFunction, ObjectRef, Property,
    RUNTIME_INTRINSIC_NAMES, RuntimeError, Value, bytecode::eval_function_bytecode,
    native::call_native_function, object_prototype, symbol,
};

use super::function_call_this;

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
        let function_env = function_env(
            &function,
            bytecode,
            callee,
            this_value,
            &argument_values,
            env,
        );
        let activation_captured_env = Rc::new(RefCell::new(function_env.env.clone()));
        let result = eval_function_bytecode(bytecode, function_env.env, activation_captured_env);
        propagate_function_captures(&function, &function_env.function_capture_names, &result);
        propagate_caller_bindings(env, &function_env.caller_binding_names, &result);
        return result.value;
    }

    Err(RuntimeError {
        thrown: None,
        message: "user function has no bytecode body".to_owned(),
    })
}

pub(crate) fn construct_function(
    target: Value,
    new_target: Value,
    argument_values: Vec<Value>,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    ensure_constructor(&target)?;
    ensure_constructor(&new_target)?;

    let prototype = crate::constructor_prototype(&new_target, env);
    let this_value = Value::Object(ObjectRef::with_prototype(HashMap::new(), prototype));
    let result = call_function(target, this_value.clone(), argument_values, env, true)?;

    match result {
        Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) | Value::Object(_) => {
            Ok(result)
        }
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
) -> FunctionCallEnv {
    let captured_env = function.captured_env.borrow();
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
        local_env.insert(name.clone(), callee);
    }
    local_env.insert(
        "this".to_owned(),
        function_call_this(Some(this_value), env, function.is_strict),
    );
    for (index, param) in function.params.positional.iter().enumerate() {
        let value = argument_values
            .get(index)
            .cloned()
            .unwrap_or(Value::Undefined);
        local_env.insert(param.clone(), value);
    }
    local_env.insert(
        "arguments".to_owned(),
        arguments_object(function, argument_values, env),
    );
    if let Some(rest) = &function.params.rest {
        let values = argument_values
            .iter()
            .skip(function.params.positional.len())
            .cloned()
            .collect();
        local_env.insert(rest.clone(), Value::Array(ArrayRef::new(values)));
    }
    FunctionCallEnv {
        env: local_env,
        function_capture_names,
        caller_binding_names,
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
    if function.is_strict || function.params.rest.is_some() {
        return None;
    }
    let parameter_name = function
        .params
        .positional
        .get(index)
        .map(String::as_str)
        .filter(|name| !name.is_empty())?;
    if function
        .params
        .positional
        .iter()
        .skip(index + 1)
        .any(|name| name == parameter_name)
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
    if let Some(value) = env.get(name) {
        local_env.insert(name.to_owned(), value.clone());
        if !caller_binding_names.iter().any(|existing| existing == name) {
            caller_binding_names.push(name.to_owned());
        }
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
