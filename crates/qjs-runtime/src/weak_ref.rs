use std::collections::HashMap;

use crate::{CallEnv, Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, symbol};

const WEAK_REF_TARGET: &str = "\0WeakRefTarget";

pub(crate) fn install_weak_ref(
    env: &mut CallEnv,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    prototype.set_to_string_tag("WeakRef");
    symbol::define_well_known_to_string_tag(env, &prototype, "WeakRef");
    let function = Function::new_native(Some("WeakRef"), 1, NativeFunction::WeakRef, true);
    prototype.define_non_enumerable("constructor".to_owned(), Value::Function(function.clone()));
    prototype.define_non_enumerable(
        "deref".to_owned(),
        Value::Function(Function::new_native(
            Some("deref"),
            0,
            NativeFunction::WeakRefPrototypeDeref,
            false,
        )),
    );
    function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(prototype)),
    );

    let value = Value::Function(function);
    env.insert_realm("WeakRef".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("WeakRef".to_owned(), value);
    }
}

pub(crate) fn native_weak_ref(
    function: &Function,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Constructor WeakRef requires 'new'".to_owned(),
        });
    }
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !is_object_value(&target) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: WeakRef target must be an object".to_owned(),
        });
    }
    let object = ObjectRef::with_prototype_slot(
        HashMap::new(),
        crate::native_construct_prototype_slot(function, env)?,
    );
    object.set_to_string_tag("WeakRef");
    object.define_property(WEAK_REF_TARGET.to_owned(), Property::non_enumerable(target));
    Ok(Value::Object(object))
}

pub(crate) fn native_weak_ref_prototype_deref(this_value: Value) -> Result<Value, RuntimeError> {
    let object = weak_ref_object(&this_value)?;
    Ok(object
        .own_property(WEAK_REF_TARGET)
        .map(|property| property.value)
        .unwrap_or(Value::Undefined))
}

fn weak_ref_object(value: &Value) -> Result<ObjectRef, RuntimeError> {
    match value {
        Value::Object(object) if object.has_own_property(WEAK_REF_TARGET) => Ok(object.clone()),
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: WeakRef method called on incompatible receiver".to_owned(),
        }),
    }
}

fn is_object_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(object) if !symbol::is_symbol_primitive(object)
    ) || matches!(
        value,
        Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) | Value::Proxy(_)
    )
}
