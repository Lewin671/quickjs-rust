use crate::CallEnv;
use crate::{
    ArrayRef, Property, PropertyKey, RuntimeError, Value, construct_function, ensure_constructor,
    property_value, property_value_key, symbol,
};

const MAX_ARRAY_LENGTH: usize = u32::MAX as usize;

pub(super) fn array_species_create(
    receiver: Value,
    length: usize,
    method: &str,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !is_array_species_receiver(&receiver)? {
        return default_array_species_create(length);
    }
    if let Value::Array(array) = &receiver
        && species_is_intrinsic_array(array, env)
    {
        return default_array_species_create(length);
    }

    let mut constructor = property_value(receiver, "constructor", env)?;
    if is_cross_realm_array_constructor(constructor.clone(), env)? {
        constructor = Value::Undefined;
    } else if is_object_like(&constructor) {
        if let Some(species_symbol) = symbol::species_symbol(env) {
            constructor =
                property_value_key(constructor, &PropertyKey::Symbol(species_symbol), env)?;
        }
        if matches!(constructor, Value::Null) {
            constructor = Value::Undefined;
        }
    }
    if matches!(constructor, Value::Undefined) {
        return default_array_species_create(length);
    }
    ensure_constructor(&constructor).map_err(|_| RuntimeError {
        thrown: None,
        message: format!("TypeError: Array.prototype.{method} constructor is not a constructor"),
    })?;
    construct_function(
        constructor.clone(),
        constructor,
        vec![Value::Number(length as f64)],
        env,
    )
}

/// Whether ArraySpeciesCreate on `array` is observably `new Array(length)`
/// without running anything: no own property (so no own `constructor`), the
/// realm's Array.prototype, whose own `constructor` is the intrinsic Array,
/// whose own `@@species` is the intrinsic getter returning Array itself.
/// Constructing the intrinsic with a length makes the same fresh array the
/// default creation does. Anything else takes the observable lookups.
fn species_is_intrinsic_array(array: &ArrayRef, env: &CallEnv) -> bool {
    if !array.has_no_own_named_properties()
        || env.array_prototype_intrinsic_override().is_some()
        || env.has_dynamic_function_realm_global()
    {
        return false;
    }
    let Some(prototype) = env.realm().array_prototype() else {
        return false;
    };
    if !(array.uses_default_prototype() || array.uses_prototype_object(&prototype)) {
        return false;
    }
    let crate::value::OwnDataPropertyRead::Data(Value::Function(constructor)) =
        prototype.own_data_property_read("constructor")
    else {
        return false;
    };
    if constructor.native != Some(crate::NativeFunction::Array) || constructor.bound.is_some() {
        return false;
    }
    let Some(species) = symbol::species_symbol(env) else {
        return false;
    };
    constructor
        .own_symbol_property(&species)
        .is_some_and(|property| {
            matches!(
                property.getter(),
                Some(Value::Function(getter))
                    if getter.native == Some(crate::NativeFunction::SpeciesGetter)
            )
        })
}

fn default_array_species_create(length: usize) -> Result<Value, RuntimeError> {
    if length > MAX_ARRAY_LENGTH {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid array length".to_owned(),
        });
    }
    Ok(Value::Array(ArrayRef::new_with_length(length)))
}

pub(super) fn validate_array_species_constructor(
    receiver: Value,
    method: &str,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let Value::Array(array) = &receiver else {
        return Ok(());
    };
    if species_is_intrinsic_array(array, env) {
        return Ok(());
    }

    match property_value(receiver, "constructor", env)? {
        Value::Undefined | Value::Function(_) | Value::Object(_) => Ok(()),
        _ => Err(RuntimeError {
            thrown: None,
            message: format!(
                "TypeError: Array.prototype.{method} constructor is not a constructor"
            ),
        }),
    }
}

pub(super) fn create_data_property_or_throw(
    target: Value,
    key: String,
    value: Value,
    env: &mut crate::CallEnv,
) -> Result<(), RuntimeError> {
    if crate::object::define_property_on_value_key(
        target,
        PropertyKey::String(key),
        Property::data(value, true, true, true),
        env,
    )? {
        return Ok(());
    }
    Err(RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.concat cannot create result property".to_owned(),
    })
}

pub(super) fn set_array_like_length(
    target: Value,
    length: usize,
    env: &mut crate::CallEnv,
) -> Result<(), RuntimeError> {
    if crate::object::define_property_on_value_key(
        target,
        PropertyKey::String("length".to_owned()),
        Property::data(Value::Number(length as f64), false, true, false),
        env,
    )? {
        return Ok(());
    }
    Err(RuntimeError {
        thrown: None,
        message: "TypeError: Array.prototype.concat cannot set result length".to_owned(),
    })
}

fn is_array_species_receiver(value: &Value) -> Result<bool, RuntimeError> {
    if matches!(value, Value::Array(_)) {
        return Ok(true);
    }
    match value {
        Value::Proxy(proxy) => crate::proxy::proxy_target_is_array_result(proxy),
        _ => Ok(false),
    }
}

fn is_object_like(value: &Value) -> bool {
    if matches!(value, Value::Object(object) if symbol::is_symbol_primitive(object)) {
        return false;
    }
    matches!(
        value,
        Value::Object(_) | Value::Function(_) | Value::Array(_) | Value::Map(_) | Value::Set(_)
    ) || matches!(value, Value::Proxy(_))
}

fn is_cross_realm_array_constructor(
    constructor: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    if !is_object_like(&constructor) {
        return Ok(false);
    }
    Ok(matches!(
        property_value(constructor, "__quickjsRustCrossRealmArray", env)?,
        Value::Boolean(true)
    ))
}
