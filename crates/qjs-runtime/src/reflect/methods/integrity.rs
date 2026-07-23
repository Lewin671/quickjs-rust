use crate::object::{ordinary_prevent_extensions, value_is_extensible};
use crate::reflect::target::ensure_reflect_object_target;
use crate::{CallEnv, RuntimeError, Value};

pub(crate) fn native_reflect_is_extensible(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.isExtensible")?;
    Ok(Value::Boolean(value_is_extensible(&target, env)?))
}

pub(crate) fn native_reflect_prevent_extensions(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_reflect_object_target(&target, "Reflect.preventExtensions")?;
    if let Value::Proxy(proxy) = &target {
        return Ok(Value::Boolean(crate::proxy::proxy_prevent_extensions(
            proxy.clone(),
            env,
        )?));
    }
    Ok(Value::Boolean(ordinary_prevent_extensions(&target)))
}
