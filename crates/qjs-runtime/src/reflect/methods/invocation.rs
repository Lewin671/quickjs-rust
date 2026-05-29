use std::collections::HashMap;

use crate::{RuntimeError, Value};

pub(crate) fn native_reflect_apply(
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(target, Value::Function(_)) {
        return Err(RuntimeError {
            message: "Reflect.apply target is not callable".to_owned(),
        });
    }

    let this_value = crate::function::function_call_this(argument_values.get(1).cloned(), env);
    let arguments = match argument_values.get(2).cloned().unwrap_or(Value::Undefined) {
        Value::Array(elements) => elements.to_vec(),
        value => {
            return Err(RuntimeError {
                message: format!("Reflect.apply argument list is not array-like: {value:?}"),
            });
        }
    };

    crate::call_function(target, this_value, arguments, env, false)
}
