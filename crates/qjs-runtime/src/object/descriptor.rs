use crate::{
    NativeFunction, ObjectRef, Property, PropertyKey, RuntimeError, Value, call_function,
    function_own_property_descriptor, function_own_symbol_property_descriptor,
    to_property_key_value,
};

use super::{
    boxed_primitive,
    descriptor_record::{
        PropertyDescriptor, resolve_property_definition, to_property_descriptor_record,
    },
    enumeration::enumerable_property_entries,
};
use crate::CallEnv;

pub(crate) fn native_object_define_property(
    argument_values: &[Value],
    env: &mut CallEnv,
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
    if !define_property_descriptor_on_value_key(target.clone(), key, descriptor, env)? {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.defineProperty failed".to_owned(),
        });
    }
    Ok(target)
}

pub(crate) fn native_object_define_properties(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_define_property_target(&target)?;
    let descriptors = to_object_for_define_properties(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    for (key, descriptor_value) in enumerable_property_entries(descriptors, env)? {
        let descriptor = to_property_descriptor_record(descriptor_value, env)?;
        if !define_property_descriptor_on_value(target.clone(), key, descriptor, env)? {
            return Err(RuntimeError {
                thrown: None,
                message: "Object.defineProperties failed".to_owned(),
            });
        }
    }
    Ok(target)
}

fn to_object_for_define_properties(value: Value, env: &CallEnv) -> Result<Value, RuntimeError> {
    match value {
        value @ (Value::Array(_)
        | Value::Object(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Proxy(_)) => Ok(value),
        value @ (Value::String(_) | Value::Number(_) | Value::BigInt(_) | Value::Boolean(_)) => {
            Ok(boxed_primitive(value, env).expect("primitive value should box"))
        }
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "property descriptors must be an object".to_owned(),
        }),
    }
}

pub(super) fn own_property_descriptor(
    value: Value,
    key: &str,
) -> Result<Option<Property>, RuntimeError> {
    own_property_descriptor_key(value, &PropertyKey::String(key.to_owned()))
}

pub(crate) fn own_property_descriptor_key(
    value: Value,
    key: &PropertyKey,
) -> Result<Option<Property>, RuntimeError> {
    match value {
        Value::Object(object)
            if matches!(key, PropertyKey::String(_))
                && crate::typed_array::is_typed_array_object(&object) =>
        {
            let PropertyKey::String(key) = key else {
                unreachable!("typed-array descriptor guard accepts only string keys");
            };
            Ok(crate::typed_array::typed_array_own_property_descriptor(
                &object, key,
            ))
        }
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
        Value::Proxy(proxy) => own_property_descriptor_key(proxy.target(), key),
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
        Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Ok(None),
    }
}

fn define_property_descriptor_on_value(
    target: Value,
    key: String,
    descriptor: PropertyDescriptor,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    define_property_descriptor_on_value_key(target, PropertyKey::String(key), descriptor, env)
}

pub(crate) fn define_property_descriptor_on_value_key(
    target: Value,
    key: PropertyKey,
    descriptor: PropertyDescriptor,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let key = match key {
        PropertyKey::String(key) => key,
        PropertyKey::Symbol(symbol) => {
            return define_symbol_property_descriptor_on_value(target, symbol, descriptor, env);
        }
    };
    match &target {
        Value::Object(object) => {
            let existing = object.own_property(&key);
            let defines_new_property = existing.is_none();
            let mapped_argument = mapped_argument_accessors(existing.as_ref());
            let original_descriptor = descriptor.clone();
            let descriptor = mapped_argument_descriptor(descriptor, mapped_argument.as_ref(), env)?;
            let Some(property) = resolve_property_definition(existing, descriptor) else {
                return Ok(false);
            };
            if defines_new_property && !object.is_extensible() {
                return Ok(false);
            }
            if let Some(mapped_argument) = mapped_argument
                && let Some(value) = original_descriptor.value
            {
                set_mapped_argument_value(&mapped_argument, value, env)?;
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
            function.define_property(key, property);
            Ok(true)
        }
        Value::Array(elements) => {
            if key == "length" {
                return define_array_length_property(elements, descriptor, env);
            }
            let existing = crate::array_own_property_descriptor(elements, &key);
            let defines_new_property = existing.is_none();
            let Some(property) = resolve_property_definition(existing, descriptor.clone()) else {
                return Ok(false);
            };
            if defines_new_property && !elements.is_extensible() {
                return Ok(false);
            }
            if array_index_key(&key)
                .is_some_and(|index| index >= elements.len() && !elements.is_length_writable())
            {
                return Ok(false);
            }
            elements.define_property(key, property);
            Ok(true)
        }
        Value::Proxy(proxy) => {
            let proxy_key = PropertyKey::String(key);
            let forward_descriptor = descriptor.clone();
            let forward_key = proxy_key.clone();
            crate::proxy::proxy_define_property(
                proxy.clone(),
                &proxy_key,
                &descriptor,
                env,
                move |target, env| {
                    define_property_descriptor_on_value_key(
                        target,
                        forward_key,
                        forward_descriptor,
                        env,
                    )
                },
            )
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
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let new_len = descriptor
        .value
        .clone()
        .map(|value| array_length_from_array_set_length_value(value, env))
        .transpose()?;
    if descriptor.configurable_field() == Some(true) || descriptor.enumerable_field() == Some(true)
    {
        return Ok(false);
    }
    if new_len.is_none() {
        let Some(property) =
            resolve_property_definition(Some(array_length_property(elements)), descriptor)
        else {
            return Ok(false);
        };
        elements.set_length_writable(property.writable);
        return Ok(true);
    }

    if let Some(new_len) = new_len {
        let old_len = elements.len();
        if !elements.is_length_writable() {
            if descriptor.writable == Some(true) || new_len != old_len {
                return Ok(false);
            }
            return Ok(true);
        }
        if new_len < old_len {
            if let Some(restored_len) = elements.delete_indices_from(new_len) {
                elements.set_len(restored_len);
                if descriptor.writable == Some(false) {
                    elements.set_length_writable(false);
                }
                return Ok(false);
            }
        }
        elements.set_len(new_len);
        if elements.len() != new_len {
            return Ok(false);
        }
    }
    if descriptor.writable == Some(true) && !elements.is_length_writable() {
        return Ok(false);
    }
    if let Some(writable) = descriptor.writable {
        elements.set_length_writable(writable);
    }
    Ok(true)
}

struct MappedArgumentAccessors {
    get: Value,
    set: Value,
}

fn mapped_argument_accessors(property: Option<&Property>) -> Option<MappedArgumentAccessors> {
    let property = property?;
    if !property.is_accessor() {
        return None;
    }
    let get = property.get.as_ref()?;
    let set = property.set.as_ref()?;
    if is_mapped_argument_accessor(get, NativeFunction::MappedArgumentGet)
        && is_mapped_argument_accessor(set, NativeFunction::MappedArgumentSet)
    {
        Some(MappedArgumentAccessors {
            get: get.clone(),
            set: set.clone(),
        })
    } else {
        None
    }
}

fn is_mapped_argument_accessor(value: &Value, native: NativeFunction) -> bool {
    let Value::Function(function) = value else {
        return false;
    };
    let Some(bound) = function.bound.as_ref() else {
        return false;
    };
    matches!(&bound.target, Value::Function(target) if target.native == Some(native))
}

fn mapped_argument_descriptor(
    mut descriptor: PropertyDescriptor,
    mapped_argument: Option<&MappedArgumentAccessors>,
    env: &mut CallEnv,
) -> Result<PropertyDescriptor, RuntimeError> {
    if let Some(mapped_argument) = mapped_argument
        && descriptor.value.is_none()
        && descriptor.writable == Some(false)
        && !descriptor.is_accessor_descriptor()
    {
        descriptor.value = Some(get_mapped_argument_value(mapped_argument, env)?);
    }
    Ok(descriptor)
}

fn get_mapped_argument_value(
    mapped_argument: &MappedArgumentAccessors,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    call_function(
        mapped_argument.get.clone(),
        Value::Undefined,
        Vec::new(),
        env,
        false,
    )
}

fn set_mapped_argument_value(
    mapped_argument: &MappedArgumentAccessors,
    value: Value,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    call_function(
        mapped_argument.set.clone(),
        Value::Undefined,
        vec![value],
        env,
        false,
    )?;
    Ok(())
}

pub(crate) fn define_array_length_value(
    elements: &crate::ArrayRef,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    define_array_length_property(elements, PropertyDescriptor::data_value(value), env)
}

fn array_length_property(elements: &crate::ArrayRef) -> Property {
    Property::data(
        Value::Number(elements.len() as f64),
        false,
        elements.is_length_writable(),
        false,
    )
}

fn array_length_from_array_set_length_value(
    value: Value,
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    let length = crate::to_uint32_number(crate::to_number_with_env(value.clone(), env)?);
    let number = crate::to_number_with_env(value, env)?;
    if f64::from(length) != number {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid array length".to_owned(),
        });
    }
    Ok(length as usize)
}

pub(crate) fn array_length_from_descriptor_value(
    value: Value,
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    let number = crate::to_number_with_env(value, env)?;
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
    key.parse::<usize>()
        .ok()
        .filter(|index| *index < u32::MAX as usize)
}

fn define_symbol_property_descriptor_on_value(
    target: Value,
    symbol: ObjectRef,
    descriptor: PropertyDescriptor,
    env: &mut CallEnv,
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
        Value::Proxy(proxy) => {
            let proxy_key = PropertyKey::Symbol(symbol.clone());
            let forward_descriptor = descriptor.clone();
            crate::proxy::proxy_define_property(
                proxy.clone(),
                &proxy_key,
                &descriptor,
                env,
                move |target, env| {
                    define_symbol_property_descriptor_on_value(
                        target,
                        symbol,
                        forward_descriptor,
                        env,
                    )
                },
            )
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
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let key = match key {
        PropertyKey::String(key) => key,
        PropertyKey::Symbol(symbol) => {
            return define_symbol_property_on_value(target, symbol, descriptor, env);
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
            function.define_property(key, descriptor);
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
                    elements.set_len(crate::to_length_with_env(descriptor.value, env)?);
                }
                elements.set_length_writable(descriptor.writable);
            } else {
                elements.define_property(key, descriptor);
            }
            Ok(true)
        }
        Value::Proxy(proxy) => {
            let proxy_key = PropertyKey::String(key);
            let proxy_descriptor = PropertyDescriptor::from_complete_property(descriptor.clone());
            let forward_key = proxy_key.clone();
            crate::proxy::proxy_define_property(
                proxy.clone(),
                &proxy_key,
                &proxy_descriptor,
                env,
                move |target, env| {
                    define_property_on_value_key(target, forward_key, descriptor, env)
                },
            )
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
    env: &mut CallEnv,
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
        Value::Proxy(proxy) => {
            let proxy_key = PropertyKey::Symbol(symbol.clone());
            let proxy_descriptor = PropertyDescriptor::from_complete_property(descriptor.clone());
            crate::proxy::proxy_define_property(
                proxy.clone(),
                &proxy_key,
                &proxy_descriptor,
                env,
                move |target, env| define_symbol_property_on_value(target, symbol, descriptor, env),
            )
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
        Value::Object(_)
        | Value::Function(_)
        | Value::Array(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Proxy(_) => Ok(()),
        Value::String(_) | Value::Number(_) | Value::BigInt(_) | Value::Boolean(_) => {
            Err(RuntimeError {
                thrown: None,
                message: "Object.defineProperty primitive targets are not implemented".to_owned(),
            })
        }
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "Object.defineProperty target must be an object".to_owned(),
        }),
    }
}
