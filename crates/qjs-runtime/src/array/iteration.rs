use std::collections::HashMap;

use crate::{ArrayRef, RuntimeError, Value, call_function};

pub(crate) fn native_array_prototype_map(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Array(elements) = this_value.clone() else {
        return Err(RuntimeError {
            message: "Array.prototype.map called on non-array".to_owned(),
        });
    };
    let callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(callback, Value::Function(_)) {
        return Err(RuntimeError {
            message: "Array.prototype.map callback is not callable".to_owned(),
        });
    }

    let callback_this = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let source = elements.to_vec();
    let mut mapped = Vec::with_capacity(source.len());
    for (index, value) in source.into_iter().enumerate() {
        mapped.push(call_function(
            callback.clone(),
            callback_this.clone(),
            vec![
                value,
                Value::Number(index as f64),
                Value::Array(elements.clone()),
            ],
            env,
            false,
        )?);
    }

    Ok(Value::Array(ArrayRef::new(mapped)))
}
