use std::collections::HashMap;

use qjs_ast::BindingPattern;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, object_prototype, symbol,
};

use super::CallEnv;

pub(super) fn arguments_object(
    function: &Function,
    argument_values: &[Value],
    env: &CallEnv,
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
    if function.is_strict || !function.params.is_simple() {
        define_restricted_callee(&object);
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
        vec![Value::String(parameter_name.into()), Value::Object(backing)],
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
        vec![Value::String(parameter_name.into()), Value::Object(backing)],
        1,
    ))
}

fn define_restricted_callee(object: &ObjectRef) {
    let throw_type_error = Value::Function(Function::new_native(
        Some("ThrowTypeError"),
        0,
        NativeFunction::ThrowTypeError,
        false,
    ));
    object.define_property(
        "callee".to_owned(),
        Property::accessor(
            Some(throw_type_error.clone()),
            Some(throw_type_error),
            false,
            false,
        ),
    );
}

fn define_arguments_iterator(object: &ObjectRef, env: &CallEnv) {
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
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    let Some(parameter_name) = mapped_argument_name(argument_values) else {
        return Ok(Value::Undefined);
    };
    Ok(env
        .get(parameter_name)
        .or_else(|| {
            mapped_argument_backing(argument_values).and_then(|backing| backing.get("value"))
        })
        .unwrap_or(Value::Undefined))
}

pub(crate) fn native_mapped_argument_set(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let Some(parameter_name) = mapped_argument_name(argument_values) else {
        return Ok(Value::Undefined);
    };
    let value = argument_values.get(2).cloned().unwrap_or(Value::Undefined);
    if let Some(binding) = env.get_local_mut(parameter_name) {
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
