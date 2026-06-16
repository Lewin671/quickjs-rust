use std::collections::HashMap;

use crate::{CallEnv, Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, symbol};

const DISPOSABLE_STACK_DISPOSED: &str = "\0DisposableStackDisposed";

pub(crate) fn install_disposable_stack(
    env: &mut CallEnv,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    prototype.set_to_string_tag("DisposableStack");
    symbol::define_well_known_to_string_tag(env, &prototype, "DisposableStack");

    let function = Function::new_native(
        Some("DisposableStack"),
        0,
        NativeFunction::DisposableStack,
        true,
    );
    prototype.define_non_enumerable("constructor".to_owned(), Value::Function(function.clone()));
    prototype.define_property(
        "disposed".to_owned(),
        Property::accessor(
            Some(Value::Function(Function::new_native(
                Some("get disposed"),
                0,
                NativeFunction::DisposableStackPrototypeDisposed,
                false,
            ))),
            None,
            false,
            true,
        ),
    );

    let dispose = Function::new_native(
        Some("dispose"),
        0,
        NativeFunction::DisposableStackPrototypeDispose,
        false,
    );
    let dispose_value = Value::Function(dispose);
    prototype.define_non_enumerable("dispose".to_owned(), dispose_value.clone());
    if let Some(dispose_symbol) = symbol::dispose_symbol(env) {
        prototype.define_symbol_property(dispose_symbol, Property::non_enumerable(dispose_value));
    }

    function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(prototype)),
    );

    let value = Value::Function(function);
    env.insert_realm("DisposableStack".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("DisposableStack".to_owned(), value);
    }
}

pub(crate) fn native_disposable_stack(
    function: &Function,
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Constructor DisposableStack requires 'new'".to_owned(),
        });
    }
    let object = ObjectRef::with_prototype_slot(
        HashMap::new(),
        crate::native_construct_prototype_slot(function, env)?,
    );
    object.set_to_string_tag("DisposableStack");
    object.define_property(
        DISPOSABLE_STACK_DISPOSED.to_owned(),
        Property::non_enumerable(Value::Boolean(false)),
    );
    Ok(Value::Object(object))
}

pub(crate) fn native_disposable_stack_prototype_disposed(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = disposable_stack_object(&this_value)?;
    Ok(object
        .own_property(DISPOSABLE_STACK_DISPOSED)
        .map(|property| property.value)
        .unwrap_or(Value::Boolean(false)))
}

pub(crate) fn native_disposable_stack_prototype_dispose(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = disposable_stack_object(&this_value)?;
    object.define_property(
        DISPOSABLE_STACK_DISPOSED.to_owned(),
        Property::non_enumerable(Value::Boolean(true)),
    );
    Ok(Value::Undefined)
}

fn disposable_stack_object(value: &Value) -> Result<ObjectRef, RuntimeError> {
    let Value::Object(object) = value else {
        return Err(incompatible_receiver());
    };
    if object.has_own_property(DISPOSABLE_STACK_DISPOSED) {
        Ok(object.clone())
    } else {
        Err(incompatible_receiver())
    }
}

fn incompatible_receiver() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: DisposableStack method called on incompatible receiver".to_owned(),
    }
}
