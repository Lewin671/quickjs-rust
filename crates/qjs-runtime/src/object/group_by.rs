use std::collections::HashMap;

use crate::CallEnv;
use crate::{
    ArrayRef, ObjectRef, Property, PropertyKey, RuntimeError, Value,
    array::iterable_values_with_env, to_property_key_value,
};

pub(crate) fn native_object_group_by(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let items = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let callback = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Object.groupBy callback must be callable".to_owned(),
        });
    }

    let result = ObjectRef::with_prototype(HashMap::new(), None, env.realm().atoms());
    for (index, value) in iterable_values_with_env(items, "Object.groupBy", env)?
        .into_iter()
        .enumerate()
    {
        let key = crate::call_function(
            callback.clone(),
            Value::Undefined,
            vec![value.clone(), Value::Number(index as f64)],
            env,
            false,
        )?;
        let key = to_property_key_value(key, env)?;
        append_group(env.realm(), &result, key, value);
    }

    Ok(Value::Object(result))
}

fn append_group(
    realm: &crate::function::Realm,
    result: &ObjectRef,
    key: PropertyKey,
    value: Value,
) {
    match key {
        PropertyKey::String(key) => {
            match result.own_property(&key).map(|property| property.value) {
                Some(Value::Array(group)) => group.set(group.len(), value),
                _ => result.define_property(
                    key,
                    Property::enumerable(group_array(realm, value, &result.domain())),
                ),
            }
        }
        PropertyKey::Symbol(symbol) => {
            match result
                .own_symbol_property(&symbol)
                .map(|property| property.value)
            {
                Some(Value::Array(group)) => group.set(group.len(), value),
                _ => result.define_symbol_property(
                    symbol,
                    Property::enumerable(group_array(realm, value, &result.domain())),
                ),
            }
        }
    }
}

fn group_array(
    realm: &crate::function::Realm,
    value: Value,
    _domain: &crate::property::atom::AtomDomain,
) -> Value {
    Value::Array(ArrayRef::new(realm, vec![value]))
}
