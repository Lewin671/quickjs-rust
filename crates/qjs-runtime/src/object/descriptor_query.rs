use std::collections::HashMap;

use crate::{
    ObjectRef, Property, PropertyKey, RuntimeError, Value, object_prototype, to_property_key_value,
};

use super::{
    descriptor::own_property_descriptor_key,
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
    // An exotic Proxy consults its `getOwnPropertyDescriptor` trap; an absent
    // trap forwards to the ordinary descriptor lookup on the target.
    let property = if let Value::Proxy(proxy) = &target {
        crate::proxy::proxy_get_own_property_descriptor(
            proxy.clone(),
            &key,
            env,
            |target, _env| own_property_descriptor_key(target, &key),
        )?
    } else {
        own_property_descriptor_key(target, &key)?
    };
    let Some(property) = property else {
        return Ok(Value::Undefined);
    };
    Ok(Value::Object(property_descriptor_object(
        property,
        object_prototype(env),
    )))
}

pub(crate) fn native_object_get_own_property_descriptors(
    argument_values: &[Value],
    env: &mut CallEnv,
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
    let result = ObjectRef::with_prototype(HashMap::new(), prototype.clone());
    // Per spec: ownKeys = ? O.[[OwnPropertyKeys]](); then for each key in that
    // order, ? O.[[GetOwnProperty]](key). Both run a Proxy's traps. This covers
    // every own key (enumerable or not), unlike for-in enumeration.
    let keys: Vec<PropertyKey> = if let Value::Proxy(proxy) = &target {
        crate::proxy::proxy_own_keys(proxy.clone(), env)?
    } else {
        own_property_names(target.clone())
            .into_iter()
            .map(PropertyKey::String)
            .chain(
                own_property_symbols(target.clone())
                    .into_iter()
                    .map(PropertyKey::Symbol),
            )
            .collect()
    };
    for key in keys {
        let Some(property) =
            super::enumeration::observable_own_property_descriptor(target.clone(), &key, env)?
        else {
            continue;
        };
        let descriptor = Value::Object(property_descriptor_object(property, prototype.clone()));
        match key {
            PropertyKey::String(name) => {
                result.define_property(name, Property::enumerable(descriptor));
            }
            PropertyKey::Symbol(symbol) => {
                result.define_symbol_property(symbol, Property::enumerable(descriptor));
            }
        }
    }
    Ok(Value::Object(result))
}
