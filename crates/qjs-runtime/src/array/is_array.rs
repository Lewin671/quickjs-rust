use crate::{RuntimeError, Value, array_prototype};

pub(crate) fn native_array_is_array(
    argument_values: &[Value],
    env: &crate::context::Context<'_>,
) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(is_array(argument_values.first(), env)?))
}

fn is_array(
    value: Option<&Value>,
    env: &crate::context::Context<'_>,
) -> Result<bool, RuntimeError> {
    match value {
        Some(Value::Array(_)) => Ok(true),
        Some(Value::Object(object)) => Ok(array_prototype(env)
            .as_ref()
            .is_some_and(|prototype| object.ptr_eq(prototype))),
        Some(Value::Proxy(proxy)) => crate::proxy::proxy_target_is_array_result(proxy),
        _ => Ok(false),
    }
}
