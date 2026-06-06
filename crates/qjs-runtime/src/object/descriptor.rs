use std::collections::HashMap;

use crate::{
    ObjectRef, Property, PropertyKey, RuntimeError, Value, function_own_property_descriptor,
    function_own_symbol_property_descriptor, object_prototype, to_property_key_value,
};

use super::{
    boxed_primitive,
    descriptor_record::{
        PropertyDescriptor, property_descriptor_object, resolve_property_definition,
        to_property_descriptor_record,
    },
    enumeration::{enumerable_property_entries, own_property_names},
};

pub(crate) fn native_object_define_property(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let key = to_property_key_value(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let descriptor = to_property_descriptor_record(
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
        env,
    )?;

    if !define_property_descriptor_on_value_key(target.clone(), key, descriptor)? {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.defineProperty failed".to_owned(),
        });
    }
    Ok(target)
}

pub(crate) fn native_object_define_properties(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_define_property_target(&target)?;

    let descriptors = to_object_for_define_properties(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;

    for (key, descriptor_value) in enumerable_property_entries(descriptors, env)? {
        let descriptor = to_property_descriptor_record(descriptor_value, env)?;
        if !define_property_descriptor_on_value(target.clone(), key, descriptor)? {
            return Err(RuntimeError {
                thrown: None,
                message: "Object.defineProperties failed".to_owned(),
            });
        }
    }
    Ok(target)
}

fn to_object_for_define_properties(
    value: Value,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match value {
        value @ (Value::Array(_)
        | Value::Object(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Set(_)) => Ok(value),
        value @ (Value::String(_) | Value::Number(_) | Value::Boolean(_)) => {
            Ok(boxed_primitive(value, env).expect("primitive value should box"))
        }
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "property descriptors must be an object".to_owned(),
        }),
    }
}

pub(crate) fn native_object_get_own_property_descriptor(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let key = to_property_key_value(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let Some(property) = own_property_descriptor_key(target, &key)? else {
        return Ok(Value::Undefined);
    };
    Ok(Value::Object(property_descriptor_object(
        property,
        object_prototype(env),
    )))
}

pub(crate) fn native_object_get_own_property_descriptors(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.getOwnPropertyDescriptors target must not be null or undefined"
                .to_owned(),
        });
    }

    let prototype = object_prototype(env);
    let mut descriptors = HashMap::new();
    for key in own_property_names(target.clone()) {
        if let Some(property) = own_property_descriptor(target.clone(), &key)? {
            descriptors.insert(
                key,
                Value::Object(property_descriptor_object(property, prototype.clone())),
            );
        }
    }

    Ok(Value::Object(ObjectRef::with_prototype(
        descriptors,
        prototype,
    )))
}

pub(super) fn own_property_descriptor(
    value: Value,
    key: &str,
) -> Result<Option<Property>, RuntimeError> {
    own_property_descriptor_key(value, &PropertyKey::String(key.to_owned()))
}

pub(super) fn own_property_descriptor_key(
    value: Value,
    key: &PropertyKey,
) -> Result<Option<Property>, RuntimeError> {
    match value {
        Value::Object(object) => Ok(match key {
            PropertyKey::String(key) => object.own_property(key),
            PropertyKey::Symbol(symbol) => object.own_symbol_property(symbol),
        }),
        Value::Map(map) => Ok(match key {
            PropertyKey::String(key) => map.object().own_property(key),
            PropertyKey::Symbol(symbol) => map.object().own_symbol_property(symbol),
        }),
        Value::Set(set) => Ok(match key {
            PropertyKey::String(key) => set.object().own_property(key),
            PropertyKey::Symbol(symbol) => set.object().own_symbol_property(symbol),
        }),
        Value::Function(function) => Ok(match key {
            PropertyKey::String(key) => function_own_property_descriptor(&function, key),
            PropertyKey::Symbol(symbol) => {
                function_own_symbol_property_descriptor(&function, symbol)
            }
        }),
        Value::Array(elements) => Ok(match key {
            PropertyKey::String(key) => crate::array_own_property_descriptor(&elements, key),
            PropertyKey::Symbol(symbol) => elements.own_symbol_property(symbol),
        }),
        Value::String(value) => Ok(match key {
            PropertyKey::String(key) => crate::string::string_own_property_descriptor(&value, key),
            PropertyKey::Symbol(_) => None,
        }),
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined => Ok(None),
    }
}

fn define_property_descriptor_on_value(
    target: Value,
    key: String,
    descriptor: PropertyDescriptor,
) -> Result<bool, RuntimeError> {
    define_property_descriptor_on_value_key(target, PropertyKey::String(key), descriptor)
}

pub(crate) fn define_property_descriptor_on_value_key(
    target: Value,
    key: PropertyKey,
    descriptor: PropertyDescriptor,
) -> Result<bool, RuntimeError> {
    let key = match key {
        PropertyKey::String(key) => key,
        PropertyKey::Symbol(symbol) => {
            return define_symbol_property_descriptor_on_value(target, symbol, descriptor);
        }
    };
    match &target {
        Value::Object(object) => {
            let existing = object.own_property(&key);
            let defines_new_property = existing.is_none();
            let Some(property) = resolve_property_definition(existing, descriptor) else {
                return Ok(false);
            };
            if defines_new_property && !object.is_extensible() {
                return Ok(false);
            }
            object.define_property(key, property);
            Ok(true)
        }
        Value::Map(map) => {
            let object = map.object();
            let existing = object.own_property(&key);
            let defines_new_property = existing.is_none();
            let Some(property) = resolve_property_definition(existing, descriptor) else {
                return Ok(false);
            };
            if defines_new_property && !object.is_extensible() {
                return Ok(false);
            }
            object.define_property(key, property);
            Ok(true)
        }
        Value::Set(set) => {
            let object = set.object();
            let existing = object.own_property(&key);
            let defines_new_property = existing.is_none();
            let Some(property) = resolve_property_definition(existing, descriptor) else {
                return Ok(false);
            };
            if defines_new_property && !object.is_extensible() {
                return Ok(false);
            }
            object.define_property(key, property);
            Ok(true)
        }
        Value::Function(function) => {
            let existing = function_own_property_descriptor(function, &key);
            let defines_new_property = existing.is_none();
            let Some(property) = resolve_property_definition(existing, descriptor) else {
                return Ok(false);
            };
            if defines_new_property && !function.is_extensible() {
                return Ok(false);
            }
            function.properties.borrow_mut().insert(key, property);
            Ok(true)
        }
        Value::Array(elements) => {
            let existing = crate::array_own_property_descriptor(elements, &key);
            let defines_new_property = existing.is_none();
            let Some(property) = resolve_property_definition(existing, descriptor.clone()) else {
                return Ok(false);
            };
            if defines_new_property && !elements.is_extensible() {
                return Ok(false);
            }
            if key == "length" {
                return define_array_length_property(elements, descriptor);
            }
            if array_index_key(&key)
                .is_some_and(|index| index >= elements.len() && !elements.is_length_writable())
            {
                return Ok(false);
            } else {
                elements.define_property(key, property);
            }
            Ok(true)
        }
        _ => {
            ensure_define_property_target(&target)?;
            unreachable!("define property target validation should reject unsupported values")
        }
    }
}

fn define_array_length_property(
    elements: &crate::ArrayRef,
    descriptor: PropertyDescriptor,
) -> Result<bool, RuntimeError> {
    if let Some(value) = descriptor.value {
        let new_len = array_length_from_descriptor_value(value)?;
        let old_len = elements.len();
        if new_len < old_len {
            for index in (new_len..old_len).rev() {
                if crate::array_own_property_descriptor(elements, &index.to_string())
                    .is_some_and(|property| !property.configurable)
                {
                    elements.set_len(index + 1);
                    if descriptor.writable == Some(false) {
                        elements.set_length_writable(false);
                    }
                    return Ok(false);
                }
                elements.delete_index(index);
            }
        }
        elements.set_len(new_len);
        if elements.len() != new_len {
            return Ok(false);
        }
    }
    if let Some(writable) = descriptor.writable {
        elements.set_length_writable(writable);
    }
    Ok(true)
}

fn array_length_from_descriptor_value(value: Value) -> Result<usize, RuntimeError> {
    let number = crate::to_number(value)?;
    let length = crate::to_uint32_number(number);
    if f64::from(length) != number {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid array length".to_owned(),
        });
    }
    Ok(length as usize)
}

fn array_index_key(key: &str) -> Option<usize> {
    let index = key.parse::<usize>().ok()?;
    if index < u32::MAX as usize {
        Some(index)
    } else {
        None
    }
}

fn define_symbol_property_descriptor_on_value(
    target: Value,
    symbol: ObjectRef,
    descriptor: PropertyDescriptor,
) -> Result<bool, RuntimeError> {
    match &target {
        Value::Object(object) => {
            let existing = object.own_symbol_property(&symbol);
            let defines_new_property = existing.is_none();
            let Some(property) = resolve_property_definition(existing, descriptor) else {
                return Ok(false);
            };
            if defines_new_property && !object.is_extensible() {
                return Ok(false);
            }
            object.define_symbol_property(symbol, property);
            Ok(true)
        }
        Value::Map(map) => {
            let object = map.object();
            let existing = object.own_symbol_property(&symbol);
            let defines_new_property = existing.is_none();
            let Some(property) = resolve_property_definition(existing, descriptor) else {
                return Ok(false);
            };
            if defines_new_property && !object.is_extensible() {
                return Ok(false);
            }
            object.define_symbol_property(symbol, property);
            Ok(true)
        }
        Value::Set(set) => {
            let object = set.object();
            let existing = object.own_symbol_property(&symbol);
            let defines_new_property = existing.is_none();
            let Some(property) = resolve_property_definition(existing, descriptor) else {
                return Ok(false);
            };
            if defines_new_property && !object.is_extensible() {
                return Ok(false);
            }
            object.define_symbol_property(symbol, property);
            Ok(true)
        }
        Value::Function(function) => {
            let existing = function.own_symbol_property(&symbol);
            let defines_new_property = existing.is_none();
            let Some(property) = resolve_property_definition(existing, descriptor) else {
                return Ok(false);
            };
            if defines_new_property && !function.is_extensible() {
                return Ok(false);
            }
            function.define_symbol_property(symbol, property);
            Ok(true)
        }
        Value::Array(elements) => {
            let existing = elements.own_symbol_property(&symbol);
            let defines_new_property = existing.is_none();
            let Some(property) = resolve_property_definition(existing, descriptor) else {
                return Ok(false);
            };
            if defines_new_property && !elements.is_extensible() {
                return Ok(false);
            }
            elements.define_symbol_property(symbol, property);
            Ok(true)
        }
        _ => {
            ensure_define_property_target(&target)?;
            Ok(false)
        }
    }
}

pub(crate) fn define_property_on_value_key(
    target: Value,
    key: PropertyKey,
    descriptor: Property,
) -> Result<bool, RuntimeError> {
    let key = match key {
        PropertyKey::String(key) => key,
        PropertyKey::Symbol(symbol) => {
            return define_symbol_property_on_value(target, symbol, descriptor);
        }
    };
    match &target {
        Value::Object(object) => {
            if !object.has_own_property(&key) && !object.is_extensible() {
                return Ok(false);
            }
            if object
                .own_property(&key)
                .is_some_and(|property| !is_compatible_descriptor(&property, &descriptor))
            {
                return Ok(false);
            }
            object.define_property(key, descriptor);
            Ok(true)
        }
        Value::Map(map) => {
            let object = map.object();
            if !object.has_own_property(&key) && !object.is_extensible() {
                return Ok(false);
            }
            if object
                .own_property(&key)
                .is_some_and(|property| !is_compatible_descriptor(&property, &descriptor))
            {
                return Ok(false);
            }
            object.define_property(key, descriptor);
            Ok(true)
        }
        Value::Set(set) => {
            let object = set.object();
            if !object.has_own_property(&key) && !object.is_extensible() {
                return Ok(false);
            }
            if object
                .own_property(&key)
                .is_some_and(|property| !is_compatible_descriptor(&property, &descriptor))
            {
                return Ok(false);
            }
            object.define_property(key, descriptor);
            Ok(true)
        }
        Value::Function(function) => {
            let existing = function_own_property_descriptor(function, &key);
            if existing.is_none() && !function.is_extensible() {
                return Ok(false);
            }
            if existing.is_some_and(|property| !is_compatible_descriptor(&property, &descriptor)) {
                return Ok(false);
            }
            function.properties.borrow_mut().insert(key, descriptor);
            Ok(true)
        }
        Value::Array(elements) => {
            let existing = crate::array_own_property_descriptor(elements, &key);
            if existing.is_none() && !elements.is_extensible() {
                return Ok(false);
            }
            if existing.is_some_and(|property| !is_compatible_descriptor(&property, &descriptor)) {
                return Ok(false);
            }
            if key == "length" {
                if !matches!(descriptor.value, Value::Undefined) {
                    elements.set_len(crate::to_length(descriptor.value)?);
                }
                elements.set_length_writable(descriptor.writable);
            } else {
                elements.define_property(key, descriptor);
            }
            Ok(true)
        }
        _ => {
            ensure_define_property_target(&target)?;
            unreachable!("define property target validation should reject unsupported values")
        }
    }
}

fn define_symbol_property_on_value(
    target: Value,
    symbol: ObjectRef,
    descriptor: Property,
) -> Result<bool, RuntimeError> {
    match &target {
        Value::Object(object) => {
            if !object.has_own_symbol_property(&symbol) && !object.is_extensible() {
                return Ok(false);
            }
            if object
                .own_symbol_property(&symbol)
                .is_some_and(|property| !is_compatible_descriptor(&property, &descriptor))
            {
                return Ok(false);
            }
            object.define_symbol_property(symbol, descriptor);
            Ok(true)
        }
        Value::Map(map) => {
            let object = map.object();
            if !object.has_own_symbol_property(&symbol) && !object.is_extensible() {
                return Ok(false);
            }
            if object
                .own_symbol_property(&symbol)
                .is_some_and(|property| !is_compatible_descriptor(&property, &descriptor))
            {
                return Ok(false);
            }
            object.define_symbol_property(symbol, descriptor);
            Ok(true)
        }
        Value::Set(set) => {
            let object = set.object();
            if !object.has_own_symbol_property(&symbol) && !object.is_extensible() {
                return Ok(false);
            }
            if object
                .own_symbol_property(&symbol)
                .is_some_and(|property| !is_compatible_descriptor(&property, &descriptor))
            {
                return Ok(false);
            }
            object.define_symbol_property(symbol, descriptor);
            Ok(true)
        }
        Value::Function(function) => {
            if !function.has_own_symbol_property(&symbol) && !function.is_extensible() {
                return Ok(false);
            }
            if function
                .own_symbol_property(&symbol)
                .is_some_and(|property| !is_compatible_descriptor(&property, &descriptor))
            {
                return Ok(false);
            }
            function.define_symbol_property(symbol, descriptor);
            Ok(true)
        }
        Value::Array(elements) => {
            if !elements.has_own_symbol_property(&symbol) && !elements.is_extensible() {
                return Ok(false);
            }
            if elements
                .own_symbol_property(&symbol)
                .is_some_and(|property| !is_compatible_descriptor(&property, &descriptor))
            {
                return Ok(false);
            }
            elements.define_symbol_property(symbol, descriptor);
            Ok(true)
        }
        _ => {
            ensure_define_property_target(&target)?;
            Ok(false)
        }
    }
}

fn is_compatible_descriptor(existing: &Property, descriptor: &Property) -> bool {
    if existing.configurable {
        return true;
    }
    if descriptor.configurable {
        return false;
    }
    existing.writable || !descriptor.writable
}

fn ensure_define_property_target(target: &Value) -> Result<(), RuntimeError> {
    match target {
        Value::Object(_) | Value::Function(_) | Value::Array(_) | Value::Map(_) | Value::Set(_) => {
            Ok(())
        }
        Value::String(_) | Value::Number(_) | Value::Boolean(_) => Err(RuntimeError {
            thrown: None,
            message: "Object.defineProperty primitive targets are not implemented".to_owned(),
        }),
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "Object.defineProperty target must be an object".to_owned(),
        }),
    }
}
