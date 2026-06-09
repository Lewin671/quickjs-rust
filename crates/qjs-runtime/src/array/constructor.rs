use std::collections::HashMap;

use crate::{
    ArrayRef, Function, Property, PropertyKey, RuntimeError, Value, array_prototype, call_function,
    construct_function, is_truthy, property_value, property_value_key, symbol,
};

use super::array_like::array_like_values_with_env;

pub(crate) fn native_array(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    if let [Value::Number(length)] = argument_values {
        if !length.is_finite() || *length < 0.0 || length.fract() != 0.0 {
            return Err(RuntimeError {
                thrown: None,
                message: "RangeError: invalid array length".to_owned(),
            });
        }
        let length = *length as usize;
        return Ok(Value::Array(ArrayRef::new_sparse(
            vec![Value::Undefined; length],
            (0..length).collect(),
        )));
    }

    Ok(Value::Array(ArrayRef::new(argument_values.to_vec())))
}

pub(crate) fn native_array_is_array(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let is_array = match argument_values.first() {
        Some(Value::Array(_)) => true,
        Some(Value::Object(object)) => array_prototype(env)
            .as_ref()
            .is_some_and(|prototype| object.ptr_eq(prototype)),
        _ => false,
    };
    Ok(Value::Boolean(is_array))
}

pub(crate) fn native_array_from(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let items = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let map_fn = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let this_arg = argument_values.get(2).cloned().unwrap_or(Value::Undefined);
    let mapping = match map_fn {
        Value::Undefined => None,
        Value::Function(_) => Some(map_fn),
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Array.from map function is not callable".to_owned(),
            });
        }
    };

    let elements = array_from_values(items, mapping.as_ref(), this_arg, env)?;
    array_from_result(this_value, elements, env)
}

struct ArrayFromElements {
    values: Vec<Value>,
    construct_length: Option<usize>,
}

fn array_from_values(
    items: Value,
    mapping: Option<&Value>,
    this_arg: Value,
    env: &mut HashMap<String, Value>,
) -> Result<ArrayFromElements, RuntimeError> {
    if matches!(items, Value::Null | Value::Undefined) {
        return array_like_values_with_env(items, "Array.from", env).map(|values| {
            ArrayFromElements {
                construct_length: Some(values.len()),
                values,
            }
        });
    }

    let iterator_method = match symbol::iterator_symbol(env) {
        Some(iterator_symbol) => {
            property_value_key(items.clone(), &PropertyKey::Symbol(iterator_symbol), env)?
        }
        None => Value::Undefined,
    };

    match iterator_method {
        Value::Undefined | Value::Null => {
            let values = map_array_like_values(items, mapping, this_arg, env)?;
            Ok(ArrayFromElements {
                construct_length: Some(values.len()),
                values,
            })
        }
        Value::Function(_) => {
            let values =
                array_from_iterable_values(items, iterator_method, mapping, this_arg, env)?;
            Ok(ArrayFromElements {
                construct_length: None,
                values,
            })
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "Array.from iterator method is not callable".to_owned(),
        }),
    }
}

fn array_from_result(
    this_value: Value,
    elements: ArrayFromElements,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let length = elements.values.len();
    let Some(constructor) = array_from_constructor(this_value) else {
        return Ok(Value::Array(ArrayRef::new(elements.values)));
    };

    let arguments = elements
        .construct_length
        .map(|length| vec![Value::Number(length as f64)])
        .unwrap_or_default();
    let target = construct_function(constructor.clone(), constructor, arguments, env)?;
    for (index, value) in elements.values.into_iter().enumerate() {
        create_data_property_or_throw(target.clone(), index.to_string(), value)?;
    }
    create_data_property_or_throw(
        target.clone(),
        "length".to_owned(),
        Value::Number(length as f64),
    )?;
    Ok(target)
}

fn array_from_constructor(value: Value) -> Option<Value> {
    match &value {
        Value::Function(Function {
            constructable: true,
            ..
        }) => Some(value),
        _ => None,
    }
}

fn create_data_property_or_throw(
    target: Value,
    key: String,
    value: Value,
) -> Result<(), RuntimeError> {
    match target {
        Value::Object(object) => {
            if object
                .own_property(&key)
                .is_some_and(|property| !property.configurable)
                || (!object.has_own_property(&key) && !object.is_extensible())
            {
                return Err(create_data_property_error());
            }
            object.define_property(key, Property::enumerable(value));
            Ok(())
        }
        Value::Array(array) => {
            array.define_property(key, Property::enumerable(value));
            Ok(())
        }
        Value::Function(function) => {
            function.define_property(key, Property::enumerable(value));
            Ok(())
        }
        Value::Map(map) => {
            let object = map.object();
            if object
                .own_property(&key)
                .is_some_and(|property| !property.configurable)
                || (!object.has_own_property(&key) && !object.is_extensible())
            {
                return Err(create_data_property_error());
            }
            object.define_property(key, Property::enumerable(value));
            Ok(())
        }
        Value::Set(set) => {
            let object = set.object();
            if object
                .own_property(&key)
                .is_some_and(|property| !property.configurable)
                || (!object.has_own_property(&key) && !object.is_extensible())
            {
                return Err(create_data_property_error());
            }
            object.define_property(key, Property::enumerable(value));
            Ok(())
        }
        _ => Err(create_data_property_error()),
    }
}

fn create_data_property_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Array.from cannot create result property".to_owned(),
    }
}

fn map_array_like_values(
    items: Value,
    mapping: Option<&Value>,
    this_arg: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Vec<Value>, RuntimeError> {
    let values = array_like_values_with_env(items, "Array.from", env)?;
    let mut result = Vec::with_capacity(values.len());
    for (index, value) in values.into_iter().enumerate() {
        result.push(array_from_mapped_value(
            value,
            index,
            mapping,
            this_arg.clone(),
            env,
        )?);
    }
    Ok(result)
}

fn array_from_iterable_values(
    items: Value,
    iterator_method: Value,
    mapping: Option<&Value>,
    this_arg: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Vec<Value>, RuntimeError> {
    let iterator = call_function(iterator_method, items, Vec::new(), env, false)?;
    let next = property_value(iterator.clone(), "next", env)?;
    if !matches!(next, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.from iterator next method is not callable".to_owned(),
        });
    }

    let mut result = Vec::new();
    loop {
        let step = call_function(next.clone(), iterator.clone(), Vec::new(), env, false)?;
        if !is_iterator_result_object(&step) {
            return Err(RuntimeError {
                thrown: None,
                message: "Array.from iterator result is not an object".to_owned(),
            });
        }
        if is_truthy(&property_value(step.clone(), "done", env)?) {
            break;
        }
        let value = property_value(step, "value", env)?;
        let index = result.len();
        result.push(array_from_mapped_value(
            value,
            index,
            mapping,
            this_arg.clone(),
            env,
        )?);
    }
    Ok(result)
}

fn array_from_mapped_value(
    value: Value,
    index: usize,
    mapping: Option<&Value>,
    this_arg: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if let Some(callback) = mapping {
        call_function(
            callback.clone(),
            this_arg,
            vec![value, Value::Number(index as f64)],
            env,
            false,
        )
    } else {
        Ok(value)
    }
}

fn is_iterator_result_object(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_) | Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    )
}

pub(crate) fn native_array_of(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Array(ArrayRef::new(argument_values.to_vec())))
}
