use crate::{
    ArrayRef, Function, Property, PropertyKey, RuntimeError, Value, array_prototype, call_function,
    construct_function, is_truthy,
    object::{
        PropertyDescriptor, array_length_from_descriptor_value, define_array_length_value,
        define_property_descriptor_on_value_key,
    },
    property_value, property_value_key,
    reflect::ordinary_set,
    symbol,
};

use super::array_like::array_like_values_with_env;
use crate::CallEnv;

pub(crate) fn native_array(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if let [Value::Number(length)] = argument_values {
        let length = array_length_from_descriptor_value(Value::Number(*length), env)?;
        return Ok(Value::Array(ArrayRef::new_sparse(
            vec![Value::Undefined; length],
            (0..length).collect(),
        )));
    }

    Ok(Value::Array(ArrayRef::new(argument_values.to_vec())))
}

pub(crate) fn native_array_is_array(
    argument_values: &[Value],
    env: &CallEnv,
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
    env: &mut CallEnv,
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

    let constructor = array_from_constructor(this_value);
    array_from_values(items, mapping.as_ref(), this_arg, constructor, env)
}

pub(crate) fn native_array_from_async() -> Result<Value, RuntimeError> {
    Err(RuntimeError {
        thrown: None,
        message: "TypeError: Array.fromAsync is not implemented".to_owned(),
    })
}

struct ArrayFromElements {
    values: Vec<Value>,
    construct_length: Option<usize>,
}

fn array_from_values(
    items: Value,
    mapping: Option<&Value>,
    this_arg: Value,
    constructor: Option<Value>,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if matches!(items, Value::Null | Value::Undefined) {
        let elements = array_like_values_with_env(items, "Array.from", env).map(|values| {
            ArrayFromElements {
                construct_length: Some(values.len()),
                values,
            }
        })?;
        return array_from_array_like_result(constructor, elements, env);
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
            array_from_array_like_result(
                constructor,
                ArrayFromElements {
                    construct_length: Some(values.len()),
                    values,
                },
                env,
            )
        }
        Value::Function(_) => {
            array_from_iterable_result(items, iterator_method, mapping, this_arg, constructor, env)
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "Array.from iterator method is not callable".to_owned(),
        }),
    }
}

fn array_from_array_like_result(
    constructor: Option<Value>,
    elements: ArrayFromElements,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let length = elements.values.len();
    let Some(constructor) = constructor else {
        return Ok(Value::Array(ArrayRef::new(elements.values)));
    };

    let arguments = elements
        .construct_length
        .map(|length| vec![Value::Number(length as f64)])
        .unwrap_or_default();
    let target = construct_function(constructor.clone(), constructor, arguments, env)?;
    for (index, value) in elements.values.into_iter().enumerate() {
        create_data_property_or_throw(target.clone(), index.to_string(), value, env)?;
    }
    set_array_from_length(target.clone(), length, env)?;
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
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let key = PropertyKey::String(key);
    let descriptor = PropertyDescriptor::data(value, true, true, true);
    if define_property_descriptor_on_value_key(target, key, descriptor, env)? {
        Ok(())
    } else {
        Err(create_data_property_error())
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
    env: &mut CallEnv,
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

fn array_from_iterable_result(
    items: Value,
    iterator_method: Value,
    mapping: Option<&Value>,
    this_arg: Value,
    constructor: Option<Value>,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = match constructor {
        Some(constructor) => construct_function(constructor.clone(), constructor, Vec::new(), env)?,
        None => Value::Array(ArrayRef::new(Vec::new())),
    };
    let iterator = call_function(iterator_method, items, Vec::new(), env, false)?;
    let next = property_value(iterator.clone(), "next", env)?;
    if !matches!(next, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.from iterator next method is not callable".to_owned(),
        });
    }

    let mut index = 0usize;
    loop {
        let step = call_function(next.clone(), iterator.clone(), Vec::new(), env, false)?;
        if !is_iterator_result_object(&step) {
            return Err(RuntimeError {
                thrown: None,
                message: "Array.from iterator result is not an object".to_owned(),
            });
        }
        if is_truthy(&property_value(step.clone(), "done", env)?) {
            set_array_from_length(target.clone(), index, env)?;
            break;
        }
        let value = property_value(step, "value", env)?;
        let value = match array_from_mapped_value(value, index, mapping, this_arg.clone(), env) {
            Ok(value) => value,
            Err(error) => {
                let _ = close_array_from_iterator(iterator, env);
                return Err(error);
            }
        };
        if let Err(error) =
            create_data_property_or_throw(target.clone(), index.to_string(), value, env)
        {
            let _ = close_array_from_iterator(iterator, env);
            return Err(error);
        }
        index += 1;
    }
    Ok(target)
}

fn close_array_from_iterator(iterator: Value, env: &mut CallEnv) -> Result<(), RuntimeError> {
    match property_value(iterator.clone(), "return", env)? {
        Value::Undefined | Value::Null => Ok(()),
        return_method @ Value::Function(_) => {
            let result = call_function(return_method, iterator, Vec::new(), env, false)?;
            if is_iterator_result_object(&result) {
                Ok(())
            } else {
                Err(RuntimeError {
                    thrown: None,
                    message: "Array.from iterator return result is not an object".to_owned(),
                })
            }
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "Array.from iterator return method is not callable".to_owned(),
        }),
    }
}

fn set_array_from_length(
    target: Value,
    length: usize,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    if let Value::Array(elements) = &target {
        if define_array_length_value(elements, Value::Number(length as f64), env)? {
            return Ok(());
        }
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Array.from cannot set result length".to_owned(),
        });
    }

    let key = PropertyKey::String("length".to_owned());
    let value = Value::Number(length as f64);
    if ordinary_set(target.clone(), &key, value, target, env)? {
        Ok(())
    } else {
        Err(RuntimeError {
            thrown: None,
            message: "TypeError: Array.from cannot set result length".to_owned(),
        })
    }
}

fn array_from_mapped_value(
    value: Value,
    index: usize,
    mapping: Option<&Value>,
    this_arg: Value,
    env: &mut CallEnv,
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

pub(crate) fn native_array_of(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let length = argument_values.len();
    let result = if matches!(&this_value, Value::Function(function) if function.constructable) {
        construct_function(
            this_value.clone(),
            this_value,
            vec![Value::Number(length as f64)],
            env,
        )?
    } else {
        Value::Array(ArrayRef::new(Vec::new()))
    };
    for (index, value) in argument_values.iter().enumerate() {
        create_array_of_data_property(&result, index.to_string(), value.clone(), env)?;
    }
    set_array_of_length(&result, Value::Number(length as f64), env)?;
    Ok(result)
}

fn create_array_of_data_property(
    target: &Value,
    key: String,
    value: Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let key = PropertyKey::String(key);
    let descriptor = PropertyDescriptor::data(value, true, true, true);
    if define_property_descriptor_on_value_key(target.clone(), key, descriptor, env)? {
        Ok(())
    } else {
        Err(RuntimeError {
            thrown: None,
            message: "TypeError: cannot create array property".to_owned(),
        })
    }
}

fn set_array_of_length(
    target: &Value,
    value: Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    match target {
        Value::Object(object) => set_object_array_of_length(
            target,
            object.own_property("length"),
            value,
            |property| {
                object.define_property("length".to_owned(), property);
            },
            env,
        ),
        Value::Function(function) => set_object_array_of_length(
            target,
            function.own_property("length"),
            value,
            |property| {
                function.define_property("length".to_owned(), property);
            },
            env,
        ),
        Value::Map(map) => {
            let object = map.object();
            set_object_array_of_length(
                target,
                object.own_property("length"),
                value,
                |property| {
                    object.define_property("length".to_owned(), property);
                },
                env,
            )
        }
        Value::Set(set) => {
            let object = set.object();
            set_object_array_of_length(
                target,
                object.own_property("length"),
                value,
                |property| {
                    object.define_property("length".to_owned(), property);
                },
                env,
            )
        }
        Value::Array(_) => Ok(()),
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: cannot set array length".to_owned(),
        }),
    }
}

fn set_object_array_of_length(
    target: &Value,
    existing: Option<Property>,
    value: Value,
    define: impl FnOnce(Property),
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    if let Some(existing) = existing {
        if existing.accessor {
            let Some(setter) = existing.set else {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: length property has no setter".to_owned(),
                });
            };
            call_function(setter, target.clone(), vec![value], env, false)?;
            return Ok(());
        }
        if !existing.writable {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: length property is not writable".to_owned(),
            });
        }
        define(Property::data(
            value,
            existing.enumerable,
            existing.writable,
            existing.configurable,
        ));
        return Ok(());
    }
    define(Property::data(value, false, true, true));
    Ok(())
}
