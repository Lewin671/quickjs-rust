use std::collections::HashMap;

use crate::{Function, NativeFunction, ObjectRef, RuntimeError, Value, object};

pub(crate) fn install_reflect(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let reflect_object = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    define_reflect_function(
        &reflect_object,
        "getPrototypeOf",
        1,
        NativeFunction::ReflectGetPrototypeOf,
    );
    define_reflect_function(
        &reflect_object,
        "setPrototypeOf",
        2,
        NativeFunction::ReflectSetPrototypeOf,
    );

    let reflect_value = Value::Object(reflect_object);
    env.insert("Reflect".to_owned(), reflect_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Reflect".to_owned(), reflect_value);
    }
}

fn define_reflect_function(object: &ObjectRef, key: &str, length: usize, native: NativeFunction) {
    object.define_non_enumerable(
        key.to_owned(),
        Value::Function(Function::new_native(Some(key), length, native, false)),
    );
}

pub(crate) fn native_reflect_get_prototype_of(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    object::native_object_get_prototype_of(argument_values, env)
}

pub(crate) fn native_reflect_set_prototype_of(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let prototype = match argument_values.get(1).cloned().unwrap_or(Value::Undefined) {
        Value::Object(prototype) => Some(prototype),
        Value::Null => None,
        _ => {
            return Err(RuntimeError {
                message: "Reflect.setPrototypeOf prototype must be an object or null".to_owned(),
            });
        }
    };

    let success = match target {
        Value::Object(object) => object.set_prototype(prototype).is_ok(),
        Value::Array(elements) => elements.set_prototype(prototype).is_ok(),
        Value::Function(function) => function.set_internal_prototype(prototype).is_ok(),
        Value::String(_)
        | Value::Number(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => {
            return Err(RuntimeError {
                message: "Reflect.setPrototypeOf target must be an object".to_owned(),
            });
        }
    };

    Ok(Value::Boolean(success))
}
