use crate::{RuntimeError, Value, to_js_string};

pub(crate) fn native_array_prototype_join(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let separator = match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => ",".to_owned(),
        value => to_js_string(value)?,
    };
    Ok(Value::String(array_join(this_value, &separator)?))
}

pub(crate) fn native_array_prototype_to_string(this_value: Value) -> Result<Value, RuntimeError> {
    Ok(Value::String(array_join(this_value, ",")?))
}

fn array_join(value: Value, separator: &str) -> Result<String, RuntimeError> {
    let Value::Array(elements) = value else {
        return Err(RuntimeError {
            thrown: None,
            message: "Array.prototype.join called on non-array".to_owned(),
        });
    };

    let elements = elements.to_vec();
    let mut parts = Vec::with_capacity(elements.len());
    for element in elements {
        let part = match element {
            Value::Null | Value::Undefined => String::new(),
            Value::Array(_) => array_join(element, ",")?,
            value => to_js_string(value)?,
        };
        parts.push(part);
    }
    Ok(parts.join(separator))
}
