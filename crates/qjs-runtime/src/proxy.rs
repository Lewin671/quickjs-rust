use std::{collections::HashMap, fmt, rc::Rc};

use crate::{
    Function, NativeFunction, ObjectRef, PropertyKey, RuntimeError, Value, call_function,
    has_property_key, is_truthy, property_value, property_value_key_with_receiver,
};

#[derive(Clone)]
pub struct ProxyRef {
    inner: Rc<ProxyData>,
}

struct ProxyData {
    target: Value,
    handler: Value,
}

impl fmt::Debug for ProxyRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.debug_struct("ProxyRef").finish_non_exhaustive()
    }
}

impl ProxyRef {
    pub(crate) fn new(target: Value, handler: Value) -> Self {
        Self {
            inner: Rc::new(ProxyData { target, handler }),
        }
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }

    pub(crate) fn target(&self) -> Value {
        self.inner.target.clone()
    }

    pub(crate) fn handler(&self) -> Value {
        self.inner.handler.clone()
    }
}

pub(crate) fn install_proxy(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    _object_prototype: ObjectRef,
) {
    let proxy_function = Function::new_native(Some("Proxy"), 2, NativeFunction::Proxy, true);
    let proxy_value = Value::Function(proxy_function);
    env.insert("Proxy".to_owned(), proxy_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("Proxy".to_owned(), proxy_value);
    }
}

pub(crate) fn native_proxy(
    argument_values: &[Value],
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Proxy constructor requires new".to_owned(),
        });
    }
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let handler = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if !is_proxy_object_target(&target) || !is_proxy_object_target(&handler) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Proxy target and handler must be objects".to_owned(),
        });
    }
    Ok(Value::Proxy(ProxyRef::new(target, handler)))
}

pub(crate) fn proxy_get(
    proxy: ProxyRef,
    key: &PropertyKey,
    receiver: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = proxy.target();
    let Some(trap) = proxy_trap(proxy.handler(), "get", env)? else {
        return property_value_key_with_receiver(target, key, receiver, env);
    };
    call_function(
        trap,
        proxy.handler(),
        vec![target, property_key_to_value(key), receiver],
        env,
        false,
    )
}

pub(crate) fn proxy_has(
    proxy: ProxyRef,
    key: &PropertyKey,
    env: &mut HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    let target = proxy.target();
    let Some(trap) = proxy_trap(proxy.handler(), "has", env)? else {
        return has_property_key(target, env, key);
    };
    let result = call_function(
        trap,
        proxy.handler(),
        vec![target, property_key_to_value(key)],
        env,
        false,
    )?;
    Ok(is_truthy(&result))
}

pub(crate) fn proxy_delete_property(
    proxy: ProxyRef,
    key: &PropertyKey,
    env: &mut HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    let target = proxy.target();
    let Some(trap) = proxy_trap(proxy.handler(), "deleteProperty", env)? else {
        return Ok(ordinary_delete_property(target, key));
    };
    let result = call_function(
        trap,
        proxy.handler(),
        vec![target, property_key_to_value(key)],
        env,
        false,
    )?;
    Ok(is_truthy(&result))
}

pub(crate) fn proxy_target_is_array(proxy: &ProxyRef) -> bool {
    match proxy.target() {
        Value::Array(_) => true,
        Value::Proxy(inner) => proxy_target_is_array(&inner),
        _ => false,
    }
}

fn proxy_trap(
    handler: Value,
    name: &str,
    env: &mut HashMap<String, Value>,
) -> Result<Option<Value>, RuntimeError> {
    match property_value(handler, name, env)? {
        Value::Undefined | Value::Null => Ok(None),
        Value::Function(function) => Ok(Some(Value::Function(function))),
        _ => Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: Proxy {name} trap is not callable"),
        }),
    }
}

fn ordinary_delete_property(target: Value, key: &PropertyKey) -> bool {
    match (target, key) {
        (Value::Object(object), PropertyKey::String(key)) => object.delete_own_property(key),
        (Value::Object(object), PropertyKey::Symbol(symbol)) => {
            object.delete_own_symbol_property(symbol)
        }
        (Value::Array(array), PropertyKey::String(key)) => match key.parse::<usize>() {
            Ok(index) => array.delete_index(index),
            Err(_) => key != "length" && array.delete_property(key),
        },
        (Value::Array(array), PropertyKey::Symbol(symbol)) => {
            array.delete_own_symbol_property(symbol)
        }
        (Value::Function(function), PropertyKey::String(key)) => {
            crate::function_delete_own_property(&function, key)
        }
        (Value::Function(function), PropertyKey::Symbol(symbol)) => {
            crate::function_delete_own_symbol_property(&function, symbol)
        }
        (Value::Map(map), key) => ordinary_delete_property(Value::Object(map.object()), key),
        (Value::Set(set), key) => ordinary_delete_property(Value::Object(set.object()), key),
        (Value::Proxy(proxy), key) => match key {
            PropertyKey::String(key) => {
                ordinary_delete_property(proxy.target(), &PropertyKey::String(key.clone()))
            }
            PropertyKey::Symbol(symbol) => {
                ordinary_delete_property(proxy.target(), &PropertyKey::Symbol(symbol.clone()))
            }
        },
        _ => true,
    }
}

fn property_key_to_value(key: &PropertyKey) -> Value {
    match key {
        PropertyKey::String(key) => Value::String(key.clone()),
        PropertyKey::Symbol(symbol) => Value::Object(symbol.clone()),
    }
}

fn is_proxy_object_target(value: &Value) -> bool {
    match value {
        Value::Object(object) => !crate::symbol::is_symbol_primitive(object),
        Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) | Value::Proxy(_) => {
            true
        }
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => false,
    }
}
