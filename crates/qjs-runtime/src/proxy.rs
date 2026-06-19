use std::{cell::RefCell, collections::HashMap, fmt, rc::Rc};

use crate::CallEnv;
use crate::{
    Function, NativeFunction, ObjectRef, Property, PropertyKey, RuntimeError, Value, call_function,
    has_property_key, is_truthy,
    private::{PrivateState, PrivateStorage},
    property_value, property_value_key_with_receiver, to_length_with_env,
};

#[derive(Clone)]
pub struct ProxyRef {
    inner: Rc<ProxyData>,
}

struct ProxyData {
    state: RefCell<Option<ProxyState>>,
    private_state: RefCell<PrivateState>,
    /// Whether the proxy exposes `[[Call]]`/`[[Construct]]`. ProxyCreate fixes
    /// these from the target's callability at creation, and they survive
    /// revocation (so `typeof` of a revoked function proxy stays `"function"`).
    callable: bool,
    constructor: bool,
}

struct ProxyState {
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
        let callable = target_is_callable(&target);
        let constructor = target_is_constructor(&target);
        Self {
            inner: Rc::new(ProxyData {
                state: RefCell::new(Some(ProxyState { target, handler })),
                private_state: RefCell::new(PrivateState::default()),
                callable,
                constructor,
            }),
        }
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.inner, &other.inner)
    }

    pub(crate) fn target(&self) -> Value {
        self.target_result().unwrap_or(Value::Undefined)
    }

    pub(crate) fn target_result(&self) -> Result<Value, RuntimeError> {
        self.inner
            .state
            .borrow()
            .as_ref()
            .map(|state| state.target.clone())
            .ok_or_else(revoked_proxy_error)
    }

    pub(crate) fn handler_result(&self) -> Result<Value, RuntimeError> {
        self.inner
            .state
            .borrow()
            .as_ref()
            .map(|state| state.handler.clone())
            .ok_or_else(revoked_proxy_error)
    }

    pub(crate) fn revoke(&self) {
        *self.inner.state.borrow_mut() = None;
    }

    /// Returns this Proxy exotic object's private-name storage, creating it on
    /// first use. Private fields are stored on the Proxy receiver itself, not
    /// on its target through the handler.
    pub(crate) fn private_storage(&self) -> PrivateStorage {
        self.inner
            .private_state
            .borrow_mut()
            .storage
            .get_or_insert_with(PrivateStorage::new)
            .clone()
    }
}

pub(crate) fn install_proxy(env: &mut CallEnv, global_this: &Value, _object_prototype: ObjectRef) {
    let proxy_function = Function::new_native(Some("Proxy"), 2, NativeFunction::Proxy, true);
    // `Proxy` is constructable but exposes no own `prototype`; the generic
    // native builder adds a (now non-configurable) one, so remove it directly.
    proxy_function.remove_own_property_unchecked("prototype");
    proxy_function.define_property(
        "revocable".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("revocable"),
            2,
            NativeFunction::ProxyRevocable,
            false,
        ))),
    );
    let proxy_value = Value::Function(proxy_function);
    env.insert_realm("Proxy".to_owned(), proxy_value.clone());
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

pub(crate) fn native_proxy_revocable(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let handler = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if !is_proxy_object_target(&target) || !is_proxy_object_target(&handler) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Proxy target and handler must be objects".to_owned(),
        });
    }

    let proxy = ProxyRef::new(target, handler);
    // The revocation function is anonymous: its `name` is the empty string
    // (ECMA-262 28.2.2.1, CreateBuiltinFunction with name "").
    let revoke = Function::new_native(Some(""), 0, NativeFunction::ProxyRevoke, false);
    revoke.define_property(
        "[[RevocableProxy]]".to_owned(),
        Property::non_enumerable(Value::Proxy(proxy.clone())),
    );

    Ok(Value::Object(ObjectRef::new(HashMap::from([
        ("proxy".to_owned(), Value::Proxy(proxy)),
        ("revoke".to_owned(), Value::Function(revoke)),
    ]))))
}

pub(crate) fn native_proxy_revoke(function: &Function) -> Result<Value, RuntimeError> {
    if let Some(Property {
        value: Value::Proxy(proxy),
        ..
    }) = function.own_property("[[RevocableProxy]]")
    {
        proxy.revoke();
        function.define_property(
            "[[RevocableProxy]]".to_owned(),
            Property::non_enumerable(Value::Undefined),
        );
    }
    Ok(Value::Undefined)
}

pub(crate) fn proxy_get(
    proxy: ProxyRef,
    key: &PropertyKey,
    receiver: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = proxy.target_result()?;
    let handler = proxy.handler_result()?;
    let Some(trap) = proxy_trap(handler.clone(), "get", env)? else {
        return property_value_key_with_receiver(target, key, receiver, env);
    };
    let result = call_function(
        trap,
        handler,
        vec![target.clone(), property_key_to_value(key), receiver],
        env,
        false,
    )?;
    // The trap result must agree with a non-configurable target property: a
    // non-writable data property's exact value, or undefined for an accessor
    // with no getter.
    if let Some(target_descriptor) = crate::object::own_property_descriptor_key(target, key)?
        && !target_descriptor.configurable
    {
        if target_descriptor.is_accessor() {
            if target_descriptor.get.is_none() && !matches!(result, Value::Undefined) {
                return Err(invariant_error(
                    "get trap returned a value for a non-configurable accessor without a getter",
                ));
            }
        } else if !target_descriptor.writable && !result.same_value(&target_descriptor.value) {
            return Err(invariant_error(
                "get trap returned a value differing from a non-configurable non-writable target property",
            ));
        }
    }
    Ok(result)
}

pub(crate) fn proxy_has(
    proxy: ProxyRef,
    key: &PropertyKey,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let target = proxy.target_result()?;
    let handler = proxy.handler_result()?;
    let Some(trap) = proxy_trap(handler.clone(), "has", env)? else {
        return has_property_key(target, env, key);
    };
    let result = call_function(
        trap,
        handler,
        vec![target.clone(), property_key_to_value(key)],
        env,
        false,
    )?;
    if !is_truthy(&result) {
        // A false result may not hide a property the target cannot drop: a
        // non-configurable own property, or any own property of a
        // non-extensible target.
        if let Some(target_descriptor) =
            crate::object::own_property_descriptor_key(target.clone(), key)?
        {
            if !target_descriptor.configurable {
                return Err(invariant_error(
                    "has trap returned false for a non-configurable target property",
                ));
            }
            if !crate::object::ordinary_value_is_extensible(&target) {
                return Err(invariant_error(
                    "has trap returned false for a property of a non-extensible target",
                ));
            }
        }
    }
    Ok(is_truthy(&result))
}

pub(crate) fn proxy_set(
    proxy: ProxyRef,
    key: &PropertyKey,
    value: Value,
    receiver: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let target = proxy.target_result()?;
    let handler = proxy.handler_result()?;
    let Some(trap) = proxy_trap(handler.clone(), "set", env)? else {
        return crate::reflect::ordinary_set(target, key, value, receiver, env);
    };
    let result = call_function(
        trap,
        handler,
        vec![
            target.clone(),
            property_key_to_value(key),
            value.clone(),
            receiver,
        ],
        env,
        false,
    )?;
    if is_truthy(&result) {
        // A successful set may not contradict a non-configurable target
        // property: a non-writable data property keeps its value, and an
        // accessor with no setter cannot be assigned.
        if let Some(target_descriptor) = crate::object::own_property_descriptor_key(target, key)?
            && !target_descriptor.configurable
        {
            if target_descriptor.is_accessor() {
                if target_descriptor.set.is_none() {
                    return Err(invariant_error(
                        "set trap succeeded on a non-configurable accessor without a setter",
                    ));
                }
            } else if !target_descriptor.writable && !value.same_value(&target_descriptor.value) {
                return Err(invariant_error(
                    "set trap changed a non-configurable non-writable target property",
                ));
            }
        }
    }
    Ok(is_truthy(&result))
}

pub(crate) fn proxy_delete_property(
    proxy: ProxyRef,
    key: &PropertyKey,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let target = proxy.target_result()?;
    let handler = proxy.handler_result()?;
    let Some(trap) = proxy_trap(handler.clone(), "deleteProperty", env)? else {
        if let Value::Proxy(inner) = target {
            return proxy_delete_property(inner, key, env);
        }
        return Ok(ordinary_delete_property(target, key));
    };
    let result = call_function(
        trap,
        handler,
        vec![target.clone(), property_key_to_value(key)],
        env,
        false,
    )?;
    if is_truthy(&result)
        && let Some(target_descriptor) =
            crate::object::own_property_descriptor_key(target.clone(), key)?
    {
        // A reported deletion may not drop a property the target keeps: a
        // non-configurable property, or any property of a non-extensible target.
        if !target_descriptor.configurable {
            return Err(invariant_error(
                "deleteProperty trap reported deleting a non-configurable target property",
            ));
        }
        if !crate::object::ordinary_value_is_extensible(&target) {
            return Err(invariant_error(
                "deleteProperty trap reported deleting a property of a non-extensible target",
            ));
        }
    }
    Ok(is_truthy(&result))
}

/// `[[DefineOwnProperty]]` for an exotic Proxy: invoke the `defineProperty`
/// trap with `(target, key, descriptorObject)`. When the trap is absent the
/// definition forwards to the target through `forward`. On a truthy trap
/// return, the target-consistency invariants are enforced.
pub(crate) fn proxy_define_property(
    proxy: ProxyRef,
    key: &PropertyKey,
    descriptor: &crate::object::PropertyDescriptor,
    env: &mut CallEnv,
    forward: impl FnOnce(Value, &mut CallEnv) -> Result<bool, RuntimeError>,
) -> Result<bool, RuntimeError> {
    let target = proxy.target_result()?;
    let handler = proxy.handler_result()?;
    let Some(trap) = proxy_trap(handler.clone(), "defineProperty", env)? else {
        return forward(target, env);
    };
    let descriptor_object = Value::Object(crate::object::property_descriptor_record_object(
        descriptor, env,
    ));
    let result = call_function(
        trap,
        handler,
        vec![
            target.clone(),
            property_key_to_value(key),
            descriptor_object,
        ],
        env,
        false,
    )?;
    if !is_truthy(&result) {
        return Ok(false);
    }

    // Target-consistency invariants (ECMA-262 10.5.6).
    let target_descriptor = crate::object::own_property_descriptor_key(target.clone(), key)?;
    let extensible_target = crate::object::ordinary_value_is_extensible(&target);
    let setting_config_false = descriptor.configurable_field() == Some(false);
    match target_descriptor {
        None => {
            if !extensible_target {
                return Err(invariant_error(
                    "defineProperty trap added a property to a non-extensible target",
                ));
            }
            if setting_config_false {
                return Err(invariant_error(
                    "defineProperty trap defined a non-configurable property absent on the target",
                ));
            }
        }
        Some(existing) => {
            if !descriptor.is_compatible_for_proxy_define(Some(&existing), extensible_target) {
                return Err(invariant_error(
                    "defineProperty trap result is incompatible with the existing target property",
                ));
            }
            if setting_config_false && existing.configurable {
                return Err(invariant_error(
                    "defineProperty trap reported a configurable target property as non-configurable",
                ));
            }
            if existing.configurable
                && !existing.accessor
                && setting_config_false
                && descriptor.writable_field() == Some(false)
            {
                // A configurable target property cannot be reported as a
                // non-configurable, non-writable data property.
                return Err(invariant_error(
                    "defineProperty trap reported a non-configurable non-writable data property on a configurable target",
                ));
            }
            if !existing.configurable
                && !existing.accessor
                && existing.writable
                && descriptor.writable_field() == Some(false)
            {
                // A non-configurable, writable data property on the target may
                // not be redefined as non-writable through the trap
                // (ECMA-262 10.5.6 [[DefineOwnProperty]]).
                return Err(invariant_error(
                    "defineProperty trap reported a non-writable redefinition of a non-configurable writable target property",
                ));
            }
        }
    }
    Ok(true)
}

/// `[[GetOwnProperty]]` for an exotic Proxy: invoke the
/// `getOwnPropertyDescriptor` trap with `(target, key)`. The trap must return
/// an object or undefined; the result is validated against the target.
pub(crate) fn proxy_get_own_property_descriptor(
    proxy: ProxyRef,
    key: &PropertyKey,
    env: &mut CallEnv,
    forward: impl FnOnce(Value, &mut CallEnv) -> Result<Option<Property>, RuntimeError>,
) -> Result<Option<Property>, RuntimeError> {
    let target = proxy.target_result()?;
    let handler = proxy.handler_result()?;
    let Some(trap) = proxy_trap(handler.clone(), "getOwnPropertyDescriptor", env)? else {
        if let Value::Proxy(inner) = target {
            return proxy_get_own_property_descriptor(inner, key, env, forward);
        }
        return forward(target, env);
    };
    let result = call_function(
        trap,
        handler,
        vec![target.clone(), property_key_to_value(key)],
        env,
        false,
    )?;

    let target_descriptor = crate::object::own_property_descriptor_key(target.clone(), key)?;
    let extensible_target = crate::object::ordinary_value_is_extensible(&target);

    let record = match &result {
        Value::Undefined => {
            // Trap returned undefined: the property must be absent or
            // configurable on an extensible target.
            match target_descriptor {
                None => return Ok(None),
                Some(target_descriptor) => {
                    if !target_descriptor.configurable {
                        return Err(invariant_error(
                            "getOwnPropertyDescriptor trap hid a non-configurable target property",
                        ));
                    }
                    if !extensible_target {
                        return Err(invariant_error(
                            "getOwnPropertyDescriptor trap hid a property of a non-extensible target",
                        ));
                    }
                    return Ok(None);
                }
            }
        }
        Value::Object(object) if !crate::symbol::is_symbol_primitive(object) => {
            crate::object::to_property_descriptor_record(result.clone(), env)?
        }
        Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) | Value::Proxy(_) => {
            crate::object::to_property_descriptor_record(result.clone(), env)?
        }
        _ => {
            return Err(invariant_error(
                "getOwnPropertyDescriptor trap must return an object or undefined",
            ));
        }
    };

    // Trap returned a descriptor object: check it is compatible with the
    // target, and that a non-configurable claim is backed by the target.
    if !record.is_compatible_for_proxy_define(target_descriptor.as_ref(), extensible_target) {
        return Err(invariant_error(
            "getOwnPropertyDescriptor trap result is incompatible with the target",
        ));
    }
    if record.configurable_field() == Some(false) {
        let backed = target_descriptor
            .as_ref()
            .is_some_and(|existing| !existing.configurable);
        if !backed {
            return Err(invariant_error(
                "getOwnPropertyDescriptor trap reported a non-configurable property not backed by the target",
            ));
        }
        if record.writable_field() == Some(false) {
            let writable_backed = target_descriptor
                .as_ref()
                .is_some_and(|existing| !existing.accessor && !existing.writable);
            if !writable_backed {
                return Err(invariant_error(
                    "getOwnPropertyDescriptor trap reported a non-configurable non-writable property not backed by the target",
                ));
            }
        }
    }
    Ok(Some(record.complete_for_get_own()))
}

/// `[[IsExtensible]]` for an exotic Proxy: invoke the `isExtensible` trap, then
/// enforce that the boolean result matches the target's own extensibility.
pub(crate) fn proxy_is_extensible(
    proxy: ProxyRef,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let target = proxy.target_result()?;
    let handler = proxy.handler_result()?;
    let Some(trap) = proxy_trap(handler.clone(), "isExtensible", env)? else {
        if let Value::Proxy(inner) = target {
            return proxy_is_extensible(inner, env);
        }
        return Ok(crate::object::ordinary_value_is_extensible(&target));
    };
    let result = call_function(trap, handler, vec![target.clone()], env, false)?;
    let trap_result = is_truthy(&result);
    // The invariant consults the target's own (proxy-aware) [[IsExtensible]].
    if trap_result != crate::object::value_is_extensible(&target, env)? {
        return Err(invariant_error(
            "isExtensible trap result disagrees with the target",
        ));
    }
    Ok(trap_result)
}

/// `[[PreventExtensions]]` for an exotic Proxy: invoke the `preventExtensions`
/// trap; a truthy result requires that the target is already non-extensible.
pub(crate) fn proxy_prevent_extensions(
    proxy: ProxyRef,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let target = proxy.target_result()?;
    let handler = proxy.handler_result()?;
    let Some(trap) = proxy_trap(handler.clone(), "preventExtensions", env)? else {
        if let Value::Proxy(inner) = target {
            return proxy_prevent_extensions(inner, env);
        }
        crate::object::ordinary_prevent_extensions(&target);
        return Ok(true);
    };
    let result = call_function(trap, handler, vec![target.clone()], env, false)?;
    if !is_truthy(&result) {
        return Ok(false);
    }
    // The invariant consults the target's own (proxy-aware) [[IsExtensible]].
    if crate::object::value_is_extensible(&target, env)? {
        return Err(invariant_error(
            "preventExtensions trap reported success while the target is still extensible",
        ));
    }
    Ok(true)
}

/// The ordinary `[[Prototype]]` of a value as a JavaScript value (object or
/// null), without dispatching through any Proxy trap.
fn ordinary_prototype_value(target: &Value, env: &CallEnv) -> Value {
    crate::value_prototype_slot(target.clone(), env)
        .map(|prototype| prototype.to_value())
        .unwrap_or(Value::Null)
}

/// `[[GetPrototypeOf]]` for an exotic Proxy: invoke the `getPrototypeOf` trap.
/// The trap must return an object or null, and when the target is
/// non-extensible the result must equal the target's own prototype.
pub(crate) fn proxy_get_prototype_of(
    proxy: ProxyRef,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = proxy.target_result()?;
    let handler = proxy.handler_result()?;
    let Some(trap) = proxy_trap(handler.clone(), "getPrototypeOf", env)? else {
        if let Value::Proxy(inner) = target {
            return proxy_get_prototype_of(inner, env);
        }
        return Ok(ordinary_prototype_value(&target, env));
    };
    let result = call_function(trap, handler, vec![target.clone()], env, false)?;
    match &result {
        Value::Null => {}
        Value::Object(object) if !crate::symbol::is_symbol_primitive(object) => {}
        Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) | Value::Proxy(_) => {}
        _ => {
            return Err(invariant_error(
                "getPrototypeOf trap must return an object or null",
            ));
        }
    }
    // The non-extensible invariant consults the target's own (proxy-aware)
    // [[IsExtensible]]/[[GetPrototypeOf]] so a proxy target's traps run.
    if !crate::object::value_is_extensible(&target, env)? {
        let target_prototype = target_prototype_value(&target, env)?;
        if !result.same_value(&target_prototype) {
            return Err(invariant_error(
                "getPrototypeOf trap result disagrees with a non-extensible target",
            ));
        }
    }
    Ok(result)
}

/// `[[SetPrototypeOf]]` for an exotic Proxy: invoke the `setPrototypeOf` trap.
/// A truthy result on a non-extensible target requires the requested prototype
/// to equal the target's current prototype. `prototype_value` is the requested
/// prototype as a value (an object or null).
pub(crate) fn proxy_set_prototype_of(
    proxy: ProxyRef,
    prototype_value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let target = proxy.target_result()?;
    let handler = proxy.handler_result()?;
    let Some(trap) = proxy_trap(handler.clone(), "setPrototypeOf", env)? else {
        if let Value::Proxy(inner) = target {
            return proxy_set_prototype_of(inner, prototype_value, env);
        }
        return crate::object::ordinary_set_prototype_of(&target, prototype_value, env);
    };
    let result = call_function(
        trap,
        handler,
        vec![target.clone(), prototype_value.clone()],
        env,
        false,
    )?;
    if !is_truthy(&result) {
        return Ok(false);
    }
    // The invariant consults the target's own [[IsExtensible]]/[[GetPrototypeOf]]
    // (proxy-aware), so an abrupt completion from a proxy target propagates.
    if !crate::object::value_is_extensible(&target, env)? {
        let target_prototype = target_prototype_value(&target, env)?;
        if !prototype_value.same_value(&target_prototype) {
            return Err(invariant_error(
                "setPrototypeOf trap changed the prototype of a non-extensible target",
            ));
        }
    }
    Ok(true)
}

/// The target's [[GetPrototypeOf]] result: a Proxy target consults its trap so
/// an abrupt completion or trap-driven value participates in invariant checks.
fn target_prototype_value(target: &Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    match target {
        Value::Proxy(inner) => proxy_get_prototype_of(inner.clone(), env),
        _ => Ok(ordinary_prototype_value(target, env)),
    }
}

/// `[[OwnPropertyKeys]]` for an exotic Proxy: invoke the `ownKeys` trap and
/// validate it against the target. When the trap is absent the ordinary target
/// keys (strings then symbols) are returned.
pub(crate) fn proxy_own_keys(
    proxy: ProxyRef,
    env: &mut CallEnv,
) -> Result<Vec<PropertyKey>, RuntimeError> {
    let target = proxy.target_result()?;
    let handler = proxy.handler_result()?;
    let Some(trap) = proxy_trap(handler.clone(), "ownKeys", env)? else {
        if let Value::Proxy(inner) = target {
            return proxy_own_keys(inner, env);
        }
        return Ok(ordinary_own_keys(&target));
    };
    let result = call_function(trap, handler, vec![target.clone()], env, false)?;

    // CreateListFromArrayLike, restricted to String and Symbol elements.
    let elements = create_list_from_array_like(result, env)?;
    let mut keys: Vec<PropertyKey> = Vec::with_capacity(elements.len());
    let mut seen_strings: Vec<String> = Vec::new();
    let mut seen_symbols: Vec<ObjectRef> = Vec::new();
    for element in elements {
        match element {
            Value::String(name) => {
                if seen_strings.contains(&name) {
                    return Err(invariant_error("ownKeys trap returned duplicate keys"));
                }
                seen_strings.push(name.clone().to_string());
                keys.push(PropertyKey::String(name.to_string()));
            }
            Value::Object(object) if crate::symbol::is_symbol_primitive(&object) => {
                if seen_symbols.iter().any(|seen| seen.ptr_eq(&object)) {
                    return Err(invariant_error("ownKeys trap returned duplicate keys"));
                }
                seen_symbols.push(object.clone());
                keys.push(PropertyKey::Symbol(object));
            }
            _ => {
                return Err(invariant_error(
                    "ownKeys trap result must contain only strings and symbols",
                ));
            }
        }
    }

    // Collect the target's own keys and split by configurability.
    let target_keys = ordinary_own_keys(&target);
    let extensible_target = crate::object::ordinary_value_is_extensible(&target);
    let mut non_configurable: Vec<PropertyKey> = Vec::new();
    let mut configurable_count = 0usize;
    for key in &target_keys {
        match crate::object::own_property_descriptor_key(target.clone(), key)? {
            Some(descriptor) if !descriptor.configurable => non_configurable.push(key.clone()),
            Some(_) => configurable_count += 1,
            None => {}
        }
    }

    // Every non-configurable target key must be present in the trap result.
    for key in &non_configurable {
        if !keys.iter().any(|present| property_keys_equal(present, key)) {
            return Err(invariant_error(
                "ownKeys trap omitted a non-configurable target key",
            ));
        }
    }
    // A non-extensible target requires the trap result to be exactly its keys.
    if !extensible_target {
        if keys.len() != non_configurable.len() + configurable_count {
            return Err(invariant_error(
                "ownKeys trap result disagrees with a non-extensible target",
            ));
        }
        for key in &target_keys {
            if !keys.iter().any(|present| property_keys_equal(present, key)) {
                return Err(invariant_error(
                    "ownKeys trap omitted a key of a non-extensible target",
                ));
            }
        }
    }
    Ok(keys)
}

fn create_list_from_array_like(
    value: Value,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    match value {
        Value::Array(array) => Ok(array.to_vec()),
        Value::Object(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) | Value::Proxy(_) => {
            let length = to_length_with_env(property_value(value.clone(), "length", env)?, env)?;
            (0..length)
                .map(|index| property_value(value.clone(), &index.to_string(), env))
                .collect()
        }
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Null
        | Value::Undefined => Err(invariant_error("ownKeys trap result must be an array-like")),
    }
}

/// The ordinary own keys of a value: string-keyed names followed by symbols,
/// forwarding through a Proxy to its target without dispatching traps.
fn ordinary_own_keys(target: &Value) -> Vec<PropertyKey> {
    crate::object::own_property_names(target.clone())
        .into_iter()
        .map(PropertyKey::String)
        .chain(
            crate::object::own_property_symbols(target.clone())
                .into_iter()
                .map(PropertyKey::Symbol),
        )
        .collect()
}

fn property_keys_equal(left: &PropertyKey, right: &PropertyKey) -> bool {
    match (left, right) {
        (PropertyKey::String(left), PropertyKey::String(right)) => left == right,
        (PropertyKey::Symbol(left), PropertyKey::Symbol(right)) => left.ptr_eq(right),
        _ => false,
    }
}

/// `[[Call]]` for an exotic Proxy: invoke the `apply` trap with
/// `(target, thisArgument, argumentsList)`, or forward to the target when the
/// trap is absent. The target must itself be callable.
pub(crate) fn proxy_apply(
    proxy: ProxyRef,
    this_value: Value,
    arguments: Vec<Value>,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = proxy.target_result()?;
    let handler = proxy.handler_result()?;
    let Some(trap) = proxy_trap(handler.clone(), "apply", env)? else {
        return crate::call_function(target, this_value, arguments, env, false);
    };
    let arguments_array = Value::Array(crate::ArrayRef::new(arguments));
    crate::call_function(
        trap,
        handler,
        vec![target, this_value, arguments_array],
        env,
        false,
    )
}

/// `[[Construct]]` for an exotic Proxy: invoke the `construct` trap with
/// `(target, argumentsList, newTarget)`, or forward to the target when the trap
/// is absent. The trap must return an object.
pub(crate) fn proxy_construct(
    proxy: ProxyRef,
    new_target: Value,
    arguments: Vec<Value>,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = proxy.target_result()?;
    let handler = proxy.handler_result()?;
    let Some(trap) = proxy_trap(handler.clone(), "construct", env)? else {
        return crate::construct_function(target, new_target, arguments, env);
    };
    let arguments_array = Value::Array(crate::ArrayRef::new(arguments));
    let result = crate::call_function(
        trap,
        handler,
        vec![target, arguments_array, new_target],
        env,
        false,
    )?;
    if is_proxy_object_target(&result) {
        Ok(result)
    } else {
        Err(RuntimeError {
            thrown: None,
            message: "TypeError: proxy [[Construct]] must return an object".to_owned(),
        })
    }
}

/// A Proxy is callable when its target was callable at creation. Recorded at
/// ProxyCreate so the answer is stable across revocation.
pub(crate) fn proxy_is_callable(proxy: &ProxyRef) -> bool {
    proxy.inner.callable
}

/// A Proxy is a constructor when its target was a constructor at creation.
pub(crate) fn proxy_is_constructor(proxy: &ProxyRef) -> bool {
    proxy.inner.constructor
}

/// Whether `target` exposes `[[Call]]` for ProxyCreate. A proxy target reports
/// its own recorded callability (so a revoked function proxy still counts).
fn target_is_callable(target: &Value) -> bool {
    match target {
        Value::Function(_) => true,
        Value::Proxy(inner) => inner.inner.callable,
        _ => false,
    }
}

/// Whether `target` exposes `[[Construct]]` for ProxyCreate.
fn target_is_constructor(target: &Value) -> bool {
    match target {
        Value::Function(function) => function.constructable,
        Value::Proxy(inner) => inner.inner.constructor,
        _ => false,
    }
}

pub(crate) fn proxy_target_is_array_result(proxy: &ProxyRef) -> Result<bool, RuntimeError> {
    match proxy.target_result()? {
        Value::Array(_) => Ok(true),
        Value::Proxy(inner) => proxy_target_is_array_result(&inner),
        _ => Ok(false),
    }
}

fn revoked_proxy_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: revoked proxy".to_owned(),
    }
}

fn invariant_error(detail: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("TypeError: proxy {detail}"),
    }
}

fn proxy_trap(
    handler: Value,
    name: &str,
    env: &mut CallEnv,
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
        (Value::Object(object), PropertyKey::String(key)) => {
            if crate::typed_array::is_typed_array_object(&object)
                && let crate::typed_array::IndexedDelete::Handled(success) =
                    crate::typed_array::delete_indexed_element(&object, key)
            {
                return success;
            }
            object.delete_own_property(key)
        }
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
        PropertyKey::String(key) => Value::String(key.clone().into()),
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
