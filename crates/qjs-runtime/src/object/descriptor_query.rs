use std::collections::HashMap;

use crate::{
    ObjectRef, Property, PropertyKey, RuntimeError, Value, object_prototype, to_property_key_value,
};

use super::{
    descriptor::{own_property_descriptor, own_property_descriptor_key},
    descriptor_record::property_descriptor_object,
    enumeration::{own_property_names, own_property_symbols},
};
use crate::CallEnv;

pub(crate) fn native_object_get_own_property_descriptor(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "Object.getOwnPropertyDescriptor target must not be null or undefined"
                .to_owned(),
        });
    }
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
    env: &CallEnv,
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
    let result = ObjectRef::with_prototype(descriptors, prototype.clone());
    for symbol in own_property_symbols(target.clone()) {
        let key = PropertyKey::Symbol(symbol.clone());
        if let Some(property) = own_property_descriptor_key(target.clone(), &key)? {
            result.define_symbol_property(
                symbol,
                Property::enumerable(Value::Object(property_descriptor_object(
                    property,
                    prototype.clone(),
                ))),
            );
        }
    }

    Ok(Value::Object(result))
}
