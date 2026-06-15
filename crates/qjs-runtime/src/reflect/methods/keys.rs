use crate::reflect::target::ensure_reflect_object_target;
use crate::{CallEnv, PropertyKey, RuntimeError, Value};

pub(crate) fn native_reflect_own_keys(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.ownKeys")?;

    // An exotic Proxy consults its `ownKeys` trap, returning a validated mix of
    // string and symbol keys in trap order.
    if let Value::Proxy(proxy) = &target {
        let keys = crate::proxy::proxy_own_keys(proxy.clone(), env)?;
        return Ok(Value::Array(crate::ArrayRef::new(
            keys.into_iter()
                .map(|key| match key {
                    PropertyKey::String(name) => Value::String(name),
                    PropertyKey::Symbol(symbol) => Value::Object(symbol),
                })
                .collect(),
        )));
    }

    let names = match target.clone() {
        Value::Object(object) if crate::typed_array::is_typed_array_object(&object) => {
            crate::typed_array::typed_array_own_property_names(&object)
        }
        Value::Object(object) => object.own_property_names(),
        Value::Map(map) => map.object().own_property_names(),
        Value::Set(set) => set.object().own_property_names(),
        Value::Array(elements) => crate::array_own_property_names(&elements),
        Value::Function(function) => crate::function_own_property_names(&function),
        Value::Proxy(_)
        | Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => {
            unreachable!("target was validated before own key enumeration")
        }
    };
    let symbols = match target {
        Value::Object(object) => object.own_property_symbols(),
        Value::Map(map) => map.object().own_property_symbols(),
        Value::Set(set) => set.object().own_property_symbols(),
        Value::Array(elements) => elements.own_property_symbols(),
        Value::Function(function) => crate::function_own_property_symbols(&function),
        Value::Proxy(_)
        | Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => {
            unreachable!("target was validated before own key enumeration")
        }
    };

    Ok(Value::Array(crate::ArrayRef::new(
        names
            .into_iter()
            .map(Value::String)
            .chain(symbols.into_iter().map(Value::Object))
            .collect(),
    )))
}
