use std::collections::HashMap;

use crate::{RuntimeError, Value};

mod array;
mod function;
mod key;
mod prototype;

pub(crate) use array::{
    array_has_own_property, array_own_property_descriptor, array_own_property_keys,
    array_own_property_names,
};
pub(crate) use function::{
    function_delete_own_property, function_own_property_descriptor, function_own_property_keys,
    function_own_property_names,
};
pub(crate) use key::to_property_key;
pub(crate) use prototype::{
    array_prototype, array_prototype_property, constructor_prototype, function_intrinsic_prototype,
    function_prototype, function_prototype_property, inherited_object_prototype_property,
    inherited_string_prototype_property, object_prototype, string_prototype, value_prototype,
};

pub(crate) fn has_property(
    value: Value,
    env: &HashMap<String, Value>,
    key: &str,
) -> Result<bool, RuntimeError> {
    match value {
        Value::Object(object) => Ok(object.contains_property(key)),
        Value::Array(elements) => Ok(array_has_own_property(&elements, key)
            || array_prototype_property(&elements, env, key).is_some()),
        Value::Function(function) => Ok(function_own_property_descriptor(&function, key).is_some()
            || function_prototype_property(&function, env, key).is_some()),
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "property target must be an object".to_owned(),
        }),
    }
}
