use crate::reflect::target::ensure_reflect_object_target;
use crate::{RuntimeError, Value};

pub(crate) fn native_reflect_own_keys(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.ownKeys")?;
    let keys = match target {
        Value::Object(object) => object.own_property_names(),
        Value::Map(map) => map.object().own_property_names(),
        Value::Set(set) => set.object().own_property_names(),
        Value::Array(elements) => crate::array_own_property_names(&elements),
        Value::Function(function) => crate::function_own_property_names(&function),
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => {
            unreachable!("target was validated before own key enumeration")
        }
    };

    Ok(Value::Array(crate::ArrayRef::new(
        keys.into_iter().map(Value::String).collect(),
    )))
}
