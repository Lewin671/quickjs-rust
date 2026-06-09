use std::collections::HashMap;

use crate::{
    PropertyKey, RuntimeError, Value, array_as_object_prototype, array_has_own_property,
    array_prototype, bigint, boolean, call_function, date, error, function_intrinsic_prototype,
    function_own_property_descriptor, function_prototype, number, property_value,
    property_value_key, regexp, string, symbol, to_property_key_value, value_prototype,
};

use super::descriptor::own_property_descriptor_key;

pub(crate) fn native_object_get_prototype_of(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match argument_values.first() {
        Some(Value::Object(object)) => {
            Ok(object.prototype().map(Value::Object).unwrap_or(Value::Null))
        }
        Some(Value::Map(map)) => Ok(map
            .object()
            .prototype()
            .map(Value::Object)
            .unwrap_or(Value::Null)),
        Some(Value::Set(set)) => Ok(set
            .object()
            .prototype()
            .map(Value::Object)
            .unwrap_or(Value::Null)),
        Some(Value::Array(elements)) => Ok(elements
            .prototype_override()
            .unwrap_or_else(|| array_prototype(env))
            .map(Value::Object)
            .unwrap_or(Value::Null)),
        Some(Value::Function(function)) => {
            Ok(error::native_error_constructor_parent(function, env)
                .or_else(|| {
                    function
                        .internal_prototype_override()
                        .unwrap_or_else(|| function_intrinsic_prototype(env))
                        .map(Value::Object)
                })
                .unwrap_or(Value::Null))
        }
        Some(Value::Boolean(_)) => Ok(constructor_prototype_value("Boolean", env)),
        Some(Value::BigInt(_)) => Ok(constructor_prototype_value("BigInt", env)),
        Some(Value::Number(_)) => Ok(constructor_prototype_value("Number", env)),
        Some(Value::String(_)) => Ok(constructor_prototype_value("String", env)),
        _ => Err(RuntimeError {
            thrown: None,
            message: "Object.getPrototypeOf target must be an object".to_owned(),
        }),
    }
}

pub(crate) fn native_object_set_prototype_of(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let prototype = match argument_values.get(1).cloned().unwrap_or(Value::Undefined) {
        Value::Object(prototype) if symbol::is_symbol_primitive(&prototype) => {
            return Err(RuntimeError {
                thrown: None,
                message: "Object.setPrototypeOf prototype must be an object or null".to_owned(),
            });
        }
        Value::Object(prototype) => Some(prototype),
        Value::Array(array) => Some(array_as_object_prototype(&array, env)),
        Value::Null => None,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "Object.setPrototypeOf prototype must be an object or null".to_owned(),
            });
        }
    };

    match &target {
        Value::Object(object) if symbol::is_symbol_primitive(object) => {}
        Value::Object(object) => object.set_prototype(prototype).map_err(|()| RuntimeError {
            thrown: None,
            message: "Object.setPrototypeOf failed".to_owned(),
        })?,
        Value::Map(map) => map
            .object()
            .set_prototype(prototype)
            .map_err(|()| RuntimeError {
                thrown: None,
                message: "Object.setPrototypeOf failed".to_owned(),
            })?,
        Value::Set(set) => set
            .object()
            .set_prototype(prototype)
            .map_err(|()| RuntimeError {
                thrown: None,
                message: "Object.setPrototypeOf failed".to_owned(),
            })?,
        Value::Proxy(proxy) => match proxy.target() {
            Value::Object(object) => {
                object.set_prototype(prototype).map_err(|()| RuntimeError {
                    thrown: None,
                    message: "Object.setPrototypeOf failed".to_owned(),
                })?
            }
            Value::Map(map) => {
                map.object()
                    .set_prototype(prototype)
                    .map_err(|()| RuntimeError {
                        thrown: None,
                        message: "Object.setPrototypeOf failed".to_owned(),
                    })?
            }
            Value::Set(set) => {
                set.object()
                    .set_prototype(prototype)
                    .map_err(|()| RuntimeError {
                        thrown: None,
                        message: "Object.setPrototypeOf failed".to_owned(),
                    })?
            }
            Value::Array(elements) => {
                elements
                    .set_prototype(prototype)
                    .map_err(|()| RuntimeError {
                        thrown: None,
                        message: "Object.setPrototypeOf failed".to_owned(),
                    })?
            }
            Value::Function(function) => {
                function
                    .set_internal_prototype(prototype)
                    .map_err(|()| RuntimeError {
                        thrown: None,
                        message: "Object.setPrototypeOf failed".to_owned(),
                    })?
            }
            _ => {}
        },
        Value::Array(elements) => elements
            .set_prototype(prototype)
            .map_err(|()| RuntimeError {
                thrown: None,
                message: "Object.setPrototypeOf failed".to_owned(),
            })?,
        Value::Function(function) => {
            function
                .set_internal_prototype(prototype)
                .map_err(|()| RuntimeError {
                    thrown: None,
                    message: "Object.setPrototypeOf failed".to_owned(),
                })?
        }
        Value::String(_) | Value::Number(_) | Value::BigInt(_) | Value::Boolean(_) => {}
        Value::Null | Value::Undefined => {
            return Err(RuntimeError {
                thrown: None,
                message: "Object.setPrototypeOf target must not be null or undefined".to_owned(),
            });
        }
    }
    Ok(target)
}

fn constructor_prototype_value(name: &str, env: &HashMap<String, Value>) -> Value {
    let Some(Value::Function(function)) = env.get(name) else {
        return Value::Null;
    };
    function_prototype(function)
        .map(Value::Object)
        .unwrap_or(Value::Null)
}

pub(crate) fn native_object_prototype_has_own_property(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let key = to_property_key_value(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    match (this_value, key) {
        (Value::Object(object), crate::PropertyKey::String(key)) => {
            Ok(Value::Boolean(object.has_own_property(&key)))
        }
        (Value::Object(object), crate::PropertyKey::Symbol(symbol)) => {
            Ok(Value::Boolean(object.has_own_symbol_property(&symbol)))
        }
        (Value::Map(map), crate::PropertyKey::String(key)) => {
            Ok(Value::Boolean(map.object().has_own_property(&key)))
        }
        (Value::Map(map), crate::PropertyKey::Symbol(symbol)) => Ok(Value::Boolean(
            map.object().has_own_symbol_property(&symbol),
        )),
        (Value::Set(set), crate::PropertyKey::String(key)) => {
            Ok(Value::Boolean(set.object().has_own_property(&key)))
        }
        (Value::Set(set), crate::PropertyKey::Symbol(symbol)) => Ok(Value::Boolean(
            set.object().has_own_symbol_property(&symbol),
        )),
        (Value::Proxy(proxy), key) => Ok(Value::Boolean(
            own_property_descriptor_key(proxy.target(), &key)?.is_some(),
        )),
        (Value::Function(function), crate::PropertyKey::String(key)) => Ok(Value::Boolean(
            function_own_property_descriptor(&function, &key).is_some(),
        )),
        (Value::Function(_), crate::PropertyKey::Symbol(_)) => Ok(Value::Boolean(false)),
        (Value::Array(elements), crate::PropertyKey::String(key)) => {
            Ok(Value::Boolean(array_has_own_property(&elements, &key)))
        }
        (Value::Array(array), crate::PropertyKey::Symbol(symbol)) => {
            Ok(Value::Boolean(array.has_own_symbol_property(&symbol)))
        }
        (Value::String(value), crate::PropertyKey::String(key)) => Ok(Value::Boolean(
            crate::string::string_has_own_property(&value, &key),
        )),
        (Value::String(_), crate::PropertyKey::Symbol(_)) => Ok(Value::Boolean(false)),
        (Value::Null, _) | (Value::Undefined, _) => Err(RuntimeError {
            thrown: None,
            message: "hasOwnProperty called on null or undefined".to_owned(),
        }),
        (Value::Number(_), _) | (Value::BigInt(_), _) | (Value::Boolean(_), _) => {
            Ok(Value::Boolean(false))
        }
    }
}

pub(crate) fn native_object_prototype_property_is_enumerable(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let key = to_property_key_value(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    match this_value {
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "propertyIsEnumerable called on null or undefined".to_owned(),
        }),
        value => Ok(Value::Boolean(
            own_property_descriptor_key(value, &key)?.is_some_and(|property| property.enumerable),
        )),
    }
}

pub(crate) fn native_object_prototype_is_prototype_of(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let Some(target_prototype) = value_prototype(target, env) else {
        return Ok(Value::Boolean(false));
    };
    let prototype = match this_value {
        Value::Object(prototype) => prototype,
        Value::Null | Value::Undefined => {
            return Err(RuntimeError {
                thrown: None,
                message: "isPrototypeOf called on non-object".to_owned(),
            });
        }
        Value::Function(_)
        | Value::Array(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Proxy(_)
        | Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_) => {
            return Ok(Value::Boolean(false));
        }
    };
    Ok(Value::Boolean(
        target_prototype.ptr_eq(&prototype) || target_prototype.has_prototype(&prototype),
    ))
}

pub(crate) fn native_object_prototype_to_string(
    this_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let tag = builtin_to_string_tag(this_value.clone());
    let tag = match symbol::to_string_tag_symbol(env) {
        Some(symbol) => match property_value_key(this_value, &PropertyKey::Symbol(symbol), env)? {
            Value::String(tag) => tag,
            _ => tag,
        },
        None => tag,
    };
    Ok(Value::String(format!("[object {tag}]")))
}

fn builtin_to_string_tag(value: Value) -> String {
    match value {
        Value::Undefined => "Undefined".to_owned(),
        Value::Null => "Null".to_owned(),
        Value::Array(_) => "Array".to_owned(),
        Value::Function(_) => "Function".to_owned(),
        Value::Map(_) | Value::Set(_) | Value::Proxy(_) => "Object".to_owned(),
        Value::String(_) => "String".to_owned(),
        Value::Number(_) => "Number".to_owned(),
        Value::BigInt(_) => "BigInt".to_owned(),
        Value::Boolean(_) => "Boolean".to_owned(),
        Value::Object(object) => {
            if boolean::is_boolean_object(&object) {
                "Boolean".to_owned()
            } else if bigint::is_bigint_object(&object) {
                "BigInt".to_owned()
            } else if number::is_number_object(&object) {
                "Number".to_owned()
            } else if string::is_string_object(&object) {
                "String".to_owned()
            } else if date::is_date_object(&object) {
                "Date".to_owned()
            } else if regexp::regexp_is_regexp(&Value::Object(object.clone())) {
                "RegExp".to_owned()
            } else if error::is_error_object(&object) {
                "Error".to_owned()
            } else if object.to_string_tag().as_deref() == Some("Arguments") {
                "Arguments".to_owned()
            } else {
                "Object".to_owned()
            }
        }
    }
}

pub(crate) fn native_object_prototype_to_locale_string(
    this_value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match this_value {
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "toLocaleString called on null or undefined".to_owned(),
        }),
        value => {
            let to_string = property_value(value.clone(), "toString", env)?;
            call_function(to_string, value, Vec::new(), env, false)
        }
    }
}

pub(crate) fn native_object_prototype_value_of(
    this_value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match this_value {
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "valueOf called on null or undefined".to_owned(),
        }),
        Value::Boolean(_) | Value::Number(_) | Value::String(_) => {
            Ok(super::boxed_primitive(this_value, env).expect("primitive value should box"))
        }
        _ => Ok(this_value),
    }
}
