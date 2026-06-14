use std::collections::HashMap;

use crate::{
    ArrayRef, CallEnv, Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, symbol,
};

const FINALIZATION_REGISTRY_CELLS: &str = "\0FinalizationRegistryCells";

pub(crate) fn install_finalization_registry(
    env: &mut CallEnv,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    prototype.set_to_string_tag("FinalizationRegistry");
    symbol::define_well_known_to_string_tag(env, &prototype, "FinalizationRegistry");
    let function = Function::new_native(
        Some("FinalizationRegistry"),
        1,
        NativeFunction::FinalizationRegistry,
        true,
    );
    prototype.define_non_enumerable("constructor".to_owned(), Value::Function(function.clone()));
    define_finalization_registry_prototype_function(
        &prototype,
        "register",
        2,
        NativeFunction::FinalizationRegistryPrototypeRegister,
    );
    define_finalization_registry_prototype_function(
        &prototype,
        "unregister",
        1,
        NativeFunction::FinalizationRegistryPrototypeUnregister,
    );
    function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(prototype)),
    );

    let value = Value::Function(function);
    env.insert_realm("FinalizationRegistry".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("FinalizationRegistry".to_owned(), value);
    }
}

pub(crate) fn native_finalization_registry(
    function: &Function,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Constructor FinalizationRegistry requires 'new'".to_owned(),
        });
    }
    let cleanup_callback = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !matches!(cleanup_callback, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: FinalizationRegistry cleanup callback must be callable".to_owned(),
        });
    }
    let object = ObjectRef::with_prototype_slot(
        HashMap::new(),
        crate::native_construct_prototype_slot(function, env)?,
    );
    object.set_to_string_tag("FinalizationRegistry");
    object.define_property(
        FINALIZATION_REGISTRY_CELLS.to_owned(),
        Property::non_enumerable(Value::Array(ArrayRef::new(Vec::new()))),
    );
    Ok(Value::Object(object))
}

pub(crate) fn native_finalization_registry_prototype_register(
    this_value: Value,
    argument_values: &[Value],
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    let object = finalization_registry_object(&this_value)?;
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !can_be_held_weakly(&target, env) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: FinalizationRegistry target cannot be held weakly".to_owned(),
        });
    }

    let holdings = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if target.same_value(&holdings) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: FinalizationRegistry holdings must differ from target".to_owned(),
        });
    }

    let unregister_token = argument_values.get(2).cloned().unwrap_or(Value::Undefined);
    if !matches!(unregister_token, Value::Undefined) && !can_be_held_weakly(&unregister_token, env)
    {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: FinalizationRegistry unregister token cannot be held weakly"
                .to_owned(),
        });
    }

    finalization_registry_register(object, target, holdings, unregister_token)?;
    Ok(Value::Undefined)
}

pub(crate) fn native_finalization_registry_prototype_unregister(
    this_value: Value,
    argument_values: &[Value],
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    let object = finalization_registry_object(&this_value)?;
    let unregister_token = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !can_be_held_weakly(&unregister_token, env) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: FinalizationRegistry unregister token cannot be held weakly"
                .to_owned(),
        });
    }
    Ok(Value::Boolean(finalization_registry_unregister(
        object,
        &unregister_token,
    )?))
}

fn finalization_registry_object(value: &Value) -> Result<ObjectRef, RuntimeError> {
    let Value::Object(object) = value else {
        return Err(incompatible_receiver());
    };
    match object
        .own_property(FINALIZATION_REGISTRY_CELLS)
        .map(|property| property.value)
    {
        Some(Value::Array(_)) => Ok(object.clone()),
        _ => Err(incompatible_receiver()),
    }
}

fn finalization_registry_cells(object: &ObjectRef) -> Result<ArrayRef, RuntimeError> {
    match object
        .own_property(FINALIZATION_REGISTRY_CELLS)
        .map(|property| property.value)
    {
        Some(Value::Array(cells)) => Ok(cells),
        _ => Err(RuntimeError {
            thrown: None,
            message: "FinalizationRegistry is missing internal state".to_owned(),
        }),
    }
}

fn finalization_registry_register(
    object: ObjectRef,
    target: Value,
    holdings: Value,
    unregister_token: Value,
) -> Result<(), RuntimeError> {
    let cells = finalization_registry_cells(&object)?;
    let mut values = cells.to_vec();
    values.push(Value::Array(ArrayRef::new(vec![
        target,
        holdings,
        unregister_token,
    ])));
    cells.replace_with(values);
    Ok(())
}

fn finalization_registry_unregister(
    object: ObjectRef,
    unregister_token: &Value,
) -> Result<bool, RuntimeError> {
    let cells = finalization_registry_cells(&object)?;
    let values = cells.to_vec();
    let original_len = values.len();
    let retained = values
        .into_iter()
        .filter(|cell| match cell {
            Value::Array(record) => !record
                .get(2)
                .is_some_and(|token| token.same_value(unregister_token)),
            _ => true,
        })
        .collect::<Vec<_>>();
    let removed = retained.len() != original_len;
    cells.replace_with(retained);
    Ok(removed)
}

fn can_be_held_weakly(value: &Value, env: &CallEnv) -> bool {
    match value {
        Value::Object(object) if symbol::is_symbol_primitive(object) => {
            !symbol::is_registered_symbol(object, env)
        }
        Value::Object(_)
        | Value::Array(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Proxy(_) => true,
        Value::Null
        | Value::Undefined
        | Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_) => false,
    }
}

fn incompatible_receiver() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: incompatible FinalizationRegistry receiver".to_owned(),
    }
}

fn define_finalization_registry_prototype_function(
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
