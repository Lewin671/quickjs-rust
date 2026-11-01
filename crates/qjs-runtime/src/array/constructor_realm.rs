use crate::{
    ArrayRef, Function, Prototype, RuntimeError, Value, array_prototype, property_value, symbol,
};

const CROSS_REALM_ARRAY_PROTOTYPE: &str = "__quickjsRustRealmArrayPrototype";

pub(super) fn array_with_prototype(array: ArrayRef, prototype: Option<Prototype>) -> ArrayRef {
    if let Some(prototype) = prototype {
        let _ = array.set_prototype_slot(Some(prototype));
    }
    array
}

pub(super) fn array_constructor_prototype_slot(
    function: &Function,
    is_construct: bool,
    env: &mut crate::context::Context<'_>,
) -> Result<Option<Prototype>, RuntimeError> {
    if !is_construct {
        return Ok(None);
    }
    let new_target = env
        .construct_target()
        .unwrap_or_else(|| Value::Function(function.clone()));
    let prototype = property_value(new_target.clone(), "prototype", env)?;
    if let Some(prototype) = array_prototype_slot_from_value(prototype, env) {
        return Ok(Some(prototype));
    }
    if let Some(prototype) = cross_realm_array_prototype_slot(new_target, env)? {
        return Ok(Some(prototype));
    }
    Ok(array_prototype(env).map(Prototype::Object))
}

fn array_prototype_slot_from_value(
    value: Value,
    env: &crate::context::Context<'_>,
) -> Option<Prototype> {
    match value {
        Value::Object(prototype) if !symbol::is_symbol_primitive(&prototype) => {
            Some(Prototype::Object(prototype))
        }
        Value::Function(prototype) => Some(Prototype::Function(prototype)),
        Value::Array(array) => Some(crate::array_as_prototype_slot(&array, env)),
        Value::Proxy(prototype) => Some(Prototype::Proxy(prototype)),
        _ => None,
    }
}

fn cross_realm_array_prototype_slot(
    new_target: Value,
    env: &crate::context::Context<'_>,
) -> Result<Option<Prototype>, RuntimeError> {
    match new_target {
        Value::Function(function) => Ok(function
            .own_property(CROSS_REALM_ARRAY_PROTOTYPE)
            .and_then(|property| array_prototype_slot_from_value(property.value, env))),
        Value::Proxy(proxy) => cross_realm_array_prototype_slot(proxy.target_result()?, env),
        _ => Ok(None),
    }
}
