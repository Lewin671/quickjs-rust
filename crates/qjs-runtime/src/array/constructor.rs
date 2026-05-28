use crate::{ArrayRef, RuntimeError, Value};

pub(crate) fn native_array(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    if argument_values.len() == 1 && matches!(argument_values[0], Value::Number(_)) {
        return Err(RuntimeError {
            message: "Array length construction requires sparse array support".to_owned(),
        });
    }

    Ok(Value::Array(ArrayRef::new(argument_values.to_vec())))
}

pub(crate) fn native_array_is_array(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Array(_))
    )))
}
