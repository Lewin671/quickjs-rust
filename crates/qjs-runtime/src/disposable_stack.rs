use std::collections::HashMap;

use crate::{
    ArrayRef, CallEnv, Function, NativeFunction, ObjectRef, Property, PropertyKey, RuntimeError,
    Value, call_function, property_value_key, symbol,
};

const DISPOSABLE_STACK_DISPOSED: &str = "\0DisposableStackDisposed";
const DISPOSABLE_STACK_RESOURCES: &str = "\0DisposableStackResources";
const ASYNC_DISPOSABLE_STACK_DISPOSED: &str = "\0AsyncDisposableStackDisposed";
const RESOURCE_KIND: &str = "\0DisposableResourceKind";
const RESOURCE_VALUE: &str = "\0DisposableResourceValue";
const RESOURCE_METHOD: &str = "\0DisposableResourceMethod";

#[derive(Clone, Copy)]
enum DisposableResourceKind {
    Use,
    Adopt,
    Defer,
}

pub(crate) fn install_disposable_stack(
    env: &mut CallEnv,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    install_async_disposable_stack(env, global_this, object_prototype.clone());

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

    define_prototype_method(
        &prototype,
        "adopt",
        2,
        NativeFunction::DisposableStackPrototypeAdopt,
    );
    define_prototype_method(
        &prototype,
        "defer",
        1,
        NativeFunction::DisposableStackPrototypeDefer,
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
    define_prototype_method(
        &prototype,
        "move",
        0,
        NativeFunction::DisposableStackPrototypeMove,
    );
    define_prototype_method(
        &prototype,
        "use",
        1,
        NativeFunction::DisposableStackPrototypeUse,
    );

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

fn install_async_disposable_stack(
    env: &mut CallEnv,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    prototype.set_to_string_tag("AsyncDisposableStack");
    symbol::define_well_known_to_string_tag(env, &prototype, "AsyncDisposableStack");

    let function = Function::new_native(
        Some("AsyncDisposableStack"),
        0,
        NativeFunction::AsyncDisposableStack,
        true,
    );
    prototype.define_non_enumerable("constructor".to_owned(), Value::Function(function.clone()));
    prototype.define_property(
        "disposed".to_owned(),
        Property::accessor(
            Some(Value::Function(Function::new_native(
                Some("get disposed"),
                0,
                NativeFunction::AsyncDisposableStackPrototypeDisposed,
                false,
            ))),
            None,
            false,
            true,
        ),
    );
    define_prototype_method(
        &prototype,
        "adopt",
        2,
        NativeFunction::AsyncDisposableStackPrototypeAdopt,
    );
    define_prototype_method(
        &prototype,
        "defer",
        1,
        NativeFunction::AsyncDisposableStackPrototypeDefer,
    );
    let dispose_async = Function::new_native(
        Some("disposeAsync"),
        0,
        NativeFunction::AsyncDisposableStackPrototypeDisposeAsync,
        false,
    );
    let dispose_async_value = Value::Function(dispose_async);
    prototype.define_non_enumerable("disposeAsync".to_owned(), dispose_async_value.clone());
    if let Some(async_dispose_symbol) = symbol::async_dispose_symbol(env) {
        prototype.define_symbol_property(
            async_dispose_symbol,
            Property::non_enumerable(dispose_async_value),
        );
    }
    define_prototype_method(
        &prototype,
        "move",
        0,
        NativeFunction::AsyncDisposableStackPrototypeMove,
    );
    define_prototype_method(
        &prototype,
        "use",
        1,
        NativeFunction::AsyncDisposableStackPrototypeUse,
    );
    function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(prototype)),
    );

    let value = Value::Function(function);
    env.insert_realm("AsyncDisposableStack".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("AsyncDisposableStack".to_owned(), value);
    }
}

pub(crate) fn native_async_disposable_stack(
    function: &Function,
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Constructor AsyncDisposableStack requires 'new'".to_owned(),
        });
    }
    let object = ObjectRef::with_prototype_slot(
        HashMap::new(),
        crate::native_construct_prototype_slot(function, env)?,
    );
    object.set_to_string_tag("AsyncDisposableStack");
    object.define_property(
        ASYNC_DISPOSABLE_STACK_DISPOSED.to_owned(),
        Property::non_enumerable(Value::Boolean(false)),
    );
    object.define_property(
        DISPOSABLE_STACK_RESOURCES.to_owned(),
        Property::non_enumerable(Value::Array(ArrayRef::new(Vec::new()))),
    );
    Ok(Value::Object(object))
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
    object.define_property(
        DISPOSABLE_STACK_RESOURCES.to_owned(),
        Property::non_enumerable(Value::Array(ArrayRef::new(Vec::new()))),
    );
    Ok(Value::Object(object))
}

pub(crate) fn native_async_disposable_stack_prototype_adopt(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let object = async_disposable_stack_object(&this_value)?;
    ensure_async_pending(&object)?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let on_dispose = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if !is_callable(&on_dispose) {
        return Err(not_callable_error(
            "AsyncDisposableStack.prototype.adopt disposer",
        ));
    }
    push_resource(
        &object,
        DisposableResourceKind::Adopt,
        value.clone(),
        on_dispose,
    )?;
    Ok(value)
}

pub(crate) fn native_async_disposable_stack_prototype_defer(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let object = async_disposable_stack_object(&this_value)?;
    ensure_async_pending(&object)?;
    let on_dispose = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !is_callable(&on_dispose) {
        return Err(not_callable_error(
            "AsyncDisposableStack.prototype.defer disposer",
        ));
    }
    push_resource(
        &object,
        DisposableResourceKind::Defer,
        Value::Undefined,
        on_dispose,
    )?;
    Ok(Value::Undefined)
}

pub(crate) fn native_async_disposable_stack_prototype_disposed(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = async_disposable_stack_object(&this_value)?;
    Ok(object
        .own_property(ASYNC_DISPOSABLE_STACK_DISPOSED)
        .map(|property| property.value)
        .unwrap_or(Value::Boolean(false)))
}

pub(crate) fn native_async_disposable_stack_prototype_dispose_async(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = async_disposable_stack_object(&this_value)?;
    if !is_async_disposed(&object) {
        object.define_property(
            ASYNC_DISPOSABLE_STACK_DISPOSED.to_owned(),
            Property::non_enumerable(Value::Boolean(true)),
        );
        let resources = disposable_stack_resources(&object)?;
        while let Some(resource) = resources.pop() {
            dispose_resource(resource, env)?;
        }
    }
    fulfilled_promise(Value::Undefined, env)
}

pub(crate) fn native_async_disposable_stack_prototype_move(
    this_value: Value,
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    let object = async_disposable_stack_object(&this_value)?;
    ensure_async_pending(&object)?;
    let resources = disposable_stack_resources(&object)?;
    let moved_resources = resources.to_vec();
    resources.replace_with(Vec::new());
    object.define_property(
        ASYNC_DISPOSABLE_STACK_DISPOSED.to_owned(),
        Property::non_enumerable(Value::Boolean(true)),
    );

    let prototype = env
        .get("AsyncDisposableStack")
        .and_then(|constructor| crate::constructor_prototype_slot(&constructor, env));
    let new_stack = ObjectRef::with_prototype_slot(HashMap::new(), prototype);
    new_stack.set_to_string_tag("AsyncDisposableStack");
    new_stack.define_property(
        ASYNC_DISPOSABLE_STACK_DISPOSED.to_owned(),
        Property::non_enumerable(Value::Boolean(false)),
    );
    new_stack.define_property(
        DISPOSABLE_STACK_RESOURCES.to_owned(),
        Property::non_enumerable(Value::Array(ArrayRef::new(moved_resources))),
    );
    Ok(Value::Object(new_stack))
}

pub(crate) fn native_async_disposable_stack_prototype_use(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = async_disposable_stack_object(&this_value)?;
    ensure_async_pending(&object)?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(value, Value::Null | Value::Undefined) {
        return Ok(value);
    }
    if !is_object_like(&value) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: AsyncDisposableStack.prototype.use value must be an object"
                .to_owned(),
        });
    }

    let Some(async_dispose_symbol) = symbol::async_dispose_symbol(env) else {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Symbol.asyncDispose is not available".to_owned(),
        });
    };
    let async_dispose_method = property_value_key(
        value.clone(),
        &PropertyKey::Symbol(async_dispose_symbol),
        env,
    )?;
    let dispose_method = if matches!(async_dispose_method, Value::Null | Value::Undefined) {
        let Some(dispose_symbol) = symbol::dispose_symbol(env) else {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: Symbol.dispose is not available".to_owned(),
            });
        };
        property_value_key(value.clone(), &PropertyKey::Symbol(dispose_symbol), env)?
    } else {
        async_dispose_method
    };
    if matches!(dispose_method, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message:
                "TypeError: async disposable value is missing Symbol.asyncDispose or Symbol.dispose"
                    .to_owned(),
        });
    }
    if !is_callable(&dispose_method) {
        return Err(not_callable_error("Symbol.asyncDispose or Symbol.dispose"));
    }
    push_resource(
        &object,
        DisposableResourceKind::Use,
        value.clone(),
        dispose_method,
    )?;
    Ok(value)
}

pub(crate) fn native_disposable_stack_prototype_adopt(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let object = disposable_stack_object(&this_value)?;
    ensure_pending(&object)?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let on_dispose = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if !is_callable(&on_dispose) {
        return Err(not_callable_error(
            "DisposableStack.prototype.adopt disposer",
        ));
    }
    push_resource(
        &object,
        DisposableResourceKind::Adopt,
        value.clone(),
        on_dispose,
    )?;
    Ok(value)
}

pub(crate) fn native_disposable_stack_prototype_defer(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let object = disposable_stack_object(&this_value)?;
    ensure_pending(&object)?;
    let on_dispose = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !is_callable(&on_dispose) {
        return Err(not_callable_error(
            "DisposableStack.prototype.defer disposer",
        ));
    }
    push_resource(
        &object,
        DisposableResourceKind::Defer,
        Value::Undefined,
        on_dispose,
    )?;
    Ok(Value::Undefined)
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
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = disposable_stack_object(&this_value)?;
    if is_disposed(&object) {
        return Ok(Value::Undefined);
    }
    object.define_property(
        DISPOSABLE_STACK_DISPOSED.to_owned(),
        Property::non_enumerable(Value::Boolean(true)),
    );
    let resources = disposable_stack_resources(&object)?;
    let mut completion: Option<RuntimeError> = None;
    while let Some(resource) = resources.pop() {
        let error = dispose_resource(resource, env).err();
        if let Some(error) = error {
            completion = Some(match completion {
                Some(suppressed) => suppressed_error(error, suppressed, env)?,
                None => error,
            });
        }
    }
    if let Some(error) = completion {
        return Err(error);
    }
    Ok(Value::Undefined)
}

pub(crate) fn native_disposable_stack_prototype_move(
    this_value: Value,
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    let object = disposable_stack_object(&this_value)?;
    ensure_pending(&object)?;
    let resources = disposable_stack_resources(&object)?;
    let moved_resources = resources.to_vec();
    resources.replace_with(Vec::new());
    object.define_property(
        DISPOSABLE_STACK_DISPOSED.to_owned(),
        Property::non_enumerable(Value::Boolean(true)),
    );

    let prototype = env
        .get("DisposableStack")
        .and_then(|constructor| crate::constructor_prototype_slot(&constructor, env));
    let new_stack = ObjectRef::with_prototype_slot(HashMap::new(), prototype);
    new_stack.set_to_string_tag("DisposableStack");
    new_stack.define_property(
        DISPOSABLE_STACK_DISPOSED.to_owned(),
        Property::non_enumerable(Value::Boolean(false)),
    );
    new_stack.define_property(
        DISPOSABLE_STACK_RESOURCES.to_owned(),
        Property::non_enumerable(Value::Array(ArrayRef::new(moved_resources))),
    );
    Ok(Value::Object(new_stack))
}

pub(crate) fn native_disposable_stack_prototype_use(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = disposable_stack_object(&this_value)?;
    ensure_pending(&object)?;
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(value, Value::Null | Value::Undefined) {
        return Ok(value);
    }
    if !is_object_like(&value) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: DisposableStack.prototype.use value must be an object".to_owned(),
        });
    }
    let Some(dispose_symbol) = symbol::dispose_symbol(env) else {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Symbol.dispose is not available".to_owned(),
        });
    };
    let dispose_method =
        property_value_key(value.clone(), &PropertyKey::Symbol(dispose_symbol), env)?;
    if matches!(dispose_method, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: disposable value is missing Symbol.dispose".to_owned(),
        });
    }
    if !is_callable(&dispose_method) {
        return Err(not_callable_error("Symbol.dispose"));
    }
    push_resource(
        &object,
        DisposableResourceKind::Use,
        value.clone(),
        dispose_method,
    )?;
    Ok(value)
}

fn disposable_stack_object(value: &Value) -> Result<ObjectRef, RuntimeError> {
    let Value::Object(object) = value else {
        return Err(incompatible_receiver());
    };
    if object.has_own_property(DISPOSABLE_STACK_DISPOSED)
        && object.has_own_property(DISPOSABLE_STACK_RESOURCES)
    {
        Ok(object.clone())
    } else {
        Err(incompatible_receiver())
    }
}

fn async_disposable_stack_object(value: &Value) -> Result<ObjectRef, RuntimeError> {
    let Value::Object(object) = value else {
        return Err(incompatible_receiver());
    };
    if object.has_own_property(ASYNC_DISPOSABLE_STACK_DISPOSED)
        && object.has_own_property(DISPOSABLE_STACK_RESOURCES)
    {
        Ok(object.clone())
    } else {
        Err(incompatible_receiver())
    }
}

fn define_prototype_method(
    prototype: &ObjectRef,
    name: &str,
    length: usize,
    native: NativeFunction,
) {
    prototype.define_non_enumerable(
        name.to_owned(),
        Value::Function(Function::new_native(Some(name), length, native, false)),
    );
}

fn ensure_pending(object: &ObjectRef) -> Result<(), RuntimeError> {
    if is_disposed(object) {
        Err(RuntimeError {
            thrown: None,
            message: "ReferenceError: DisposableStack is already disposed".to_owned(),
        })
    } else {
        Ok(())
    }
}

fn ensure_async_pending(object: &ObjectRef) -> Result<(), RuntimeError> {
    if is_async_disposed(object) {
        Err(RuntimeError {
            thrown: None,
            message: "ReferenceError: AsyncDisposableStack is already disposed".to_owned(),
        })
    } else {
        Ok(())
    }
}

fn is_disposed(object: &ObjectRef) -> bool {
    matches!(
        object
            .own_property(DISPOSABLE_STACK_DISPOSED)
            .map(|property| property.value),
        Some(Value::Boolean(true))
    )
}

fn is_async_disposed(object: &ObjectRef) -> bool {
    matches!(
        object
            .own_property(ASYNC_DISPOSABLE_STACK_DISPOSED)
            .map(|property| property.value),
        Some(Value::Boolean(true))
    )
}

fn disposable_stack_resources(object: &ObjectRef) -> Result<ArrayRef, RuntimeError> {
    match object
        .own_property(DISPOSABLE_STACK_RESOURCES)
        .map(|property| property.value)
    {
        Some(Value::Array(resources)) => Ok(resources),
        _ => Err(incompatible_receiver()),
    }
}

fn push_resource(
    object: &ObjectRef,
    kind: DisposableResourceKind,
    value: Value,
    method: Value,
) -> Result<(), RuntimeError> {
    let resources = disposable_stack_resources(object)?;
    resources.set(resources.len(), resource_record(kind, value, method));
    Ok(())
}

fn resource_record(kind: DisposableResourceKind, value: Value, method: Value) -> Value {
    let record = ObjectRef::with_prototype(HashMap::new(), None);
    record.define_property(
        RESOURCE_KIND.to_owned(),
        Property::non_enumerable(Value::String(kind.as_str().to_owned().into())),
    );
    record.define_property(RESOURCE_VALUE.to_owned(), Property::non_enumerable(value));
    record.define_property(RESOURCE_METHOD.to_owned(), Property::non_enumerable(method));
    Value::Object(record)
}

fn dispose_resource(resource: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    let Value::Object(record) = resource else {
        return Ok(Value::Undefined);
    };
    let kind = match record
        .own_property(RESOURCE_KIND)
        .map(|property| property.value)
    {
        Some(Value::String(kind)) => kind,
        _ => return Ok(Value::Undefined),
    };
    let value = record
        .own_property(RESOURCE_VALUE)
        .map(|property| property.value)
        .unwrap_or(Value::Undefined);
    let method = record
        .own_property(RESOURCE_METHOD)
        .map(|property| property.value)
        .unwrap_or(Value::Undefined);
    match DisposableResourceKind::from_str(&kind) {
        Some(DisposableResourceKind::Use) => call_function(method, value, Vec::new(), env, false),
        Some(DisposableResourceKind::Adopt) => {
            call_function(method, Value::Undefined, vec![value], env, false)
        }
        Some(DisposableResourceKind::Defer) => {
            call_function(method, Value::Undefined, Vec::new(), env, false)
        }
        None => Ok(Value::Undefined),
    }
}

fn fulfilled_promise(value: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    let Some(constructor) = env.get("Promise") else {
        return Ok(value);
    };
    crate::promise::promise_resolve(&constructor, value, env)
}

fn suppressed_error(
    error: RuntimeError,
    suppressed: RuntimeError,
    env: &mut CallEnv,
) -> Result<RuntimeError, RuntimeError> {
    let error_value = crate::error::runtime_error_to_value(error, env);
    let suppressed_value = crate::error::runtime_error_to_value(suppressed, env);
    let thrown = crate::error::create_suppressed_error(error_value, suppressed_value, env)?;
    Ok(RuntimeError {
        thrown: Some(Box::new(thrown)),
        message: "throw statement executed: SuppressedError".to_owned(),
    })
}

fn is_callable(value: &Value) -> bool {
    match value {
        Value::Function(_) => true,
        Value::Proxy(proxy) => crate::proxy::proxy_is_callable(proxy),
        _ => false,
    }
}

fn is_object_like(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_)
            | Value::Array(_)
            | Value::Function(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Proxy(_)
    )
}

fn not_callable_error(label: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("TypeError: {label} must be callable"),
    }
}

fn incompatible_receiver() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: DisposableStack method called on incompatible receiver".to_owned(),
    }
}

impl DisposableResourceKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Use => "use",
            Self::Adopt => "adopt",
            Self::Defer => "defer",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "use" => Some(Self::Use),
            "adopt" => Some(Self::Adopt),
            "defer" => Some(Self::Defer),
            _ => None,
        }
    }
}
