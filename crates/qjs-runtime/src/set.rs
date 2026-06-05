use std::collections::HashMap;

use crate::{Function, NativeFunction, ObjectRef, Property, RuntimeError, SetRef, Value};

pub(crate) fn install_set(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let set_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    set_prototype.set_to_string_tag("Set");
    let set_function = Function::new_native(Some("Set"), 0, NativeFunction::Set, true);
    set_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(set_function.clone()),
    );
    set_prototype.define_property(
        "size".to_owned(),
        Property::accessor(
            Some(Value::Function(Function::new_native(
                Some("get size"),
                0,
                NativeFunction::SetPrototypeSize,
                false,
            ))),
            None,
            false,
            true,
        ),
    );
    define_set_prototype_function(&set_prototype, "add", 1, NativeFunction::SetPrototypeAdd);
    define_set_prototype_function(
        &set_prototype,
        "clear",
        0,
        NativeFunction::SetPrototypeClear,
    );
    define_set_prototype_function(
        &set_prototype,
        "delete",
        1,
        NativeFunction::SetPrototypeDelete,
    );
    define_set_prototype_function(&set_prototype, "has", 1, NativeFunction::SetPrototypeHas);
    set_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(set_prototype)),
    );

    let value = Value::Function(set_function);
    env.insert("Set".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Set".to_owned(), value);
    }
}

pub(crate) fn native_set(
    function: &Function,
    argument_values: &[Value],
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Constructor Set requires 'new'".to_owned(),
        });
    }
    if !matches!(argument_values.first(), None | Some(Value::Undefined)) {
        return Err(RuntimeError {
            thrown: None,
            message: "Set iterable constructor arguments are not implemented".to_owned(),
        });
    }
    Ok(Value::Set(SetRef::new(crate::function_prototype(function))))
}

pub(crate) fn native_set_prototype_size(this_value: Value) -> Result<Value, RuntimeError> {
    let set = this_set(this_value)?;
    Ok(Value::Number(set.len() as f64))
}

pub(crate) fn native_set_prototype_add(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let set = this_set(this_value.clone())?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    set.add(value);
    Ok(this_value)
}

pub(crate) fn native_set_prototype_clear(this_value: Value) -> Result<Value, RuntimeError> {
    let set = this_set(this_value)?;
    set.clear();
    Ok(Value::Undefined)
}

pub(crate) fn native_set_prototype_delete(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let set = this_set(this_value)?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(set.delete(&value)))
}

pub(crate) fn native_set_prototype_has(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let set = this_set(this_value)?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(set.has(&value)))
}

fn this_set(this_value: Value) -> Result<SetRef, RuntimeError> {
    match this_value {
        Value::Set(set) => Ok(set),
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: incompatible Set receiver".to_owned(),
        }),
    }
}

fn define_set_prototype_function(
    prototype: &ObjectRef,
    key: &str,
    length: usize,
    native: NativeFunction,
) {
    prototype.define_non_enumerable(
        key.to_owned(),
        Value::Function(Function::new_native(Some(key), length, native, false)),
    );
}
