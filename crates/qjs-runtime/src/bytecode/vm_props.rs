use qjs_ast::{BinaryOp, UnaryOp};

use crate::{
    GLOBAL_THIS_BINDING, ObjectRef, Property, PropertyKey, RuntimeError, Value, array_prototype,
    function_delete_own_property, function_delete_own_symbol_property,
    function_own_property_descriptor, function_own_property_names,
    function_prototype_chain_descriptor, inherited_primitive_prototype_descriptor, property_value,
    property_value_key, property_value_key_with_receiver, string, symbol, to_int32_number,
    to_uint32_number,
};

use super::vm::Vm;
use super::vm_set::property_set_uses_setter;
use crate::CallEnv;

impl Vm<'_> {
    /// Whether the realm's current Array.prototype owns any indexed property.
    /// Returns `None` when no Array.prototype is reachable. The Array.prototype
    /// object is cached so the hot path skips the `Array`-binding lookup; the
    /// cache is dropped when the `Array` global is reassigned. The own-index
    /// count read is itself O(1).
    pub(super) fn array_prototype_has_index_property(&mut self) -> Option<bool> {
        if self.array_prototype_cache.is_none() {
            self.array_prototype_cache = Some(array_prototype(&self.realm_env())?);
        }
        self.array_prototype_cache
            .as_ref()
            .map(ObjectRef::has_own_index_property)
    }

    pub(super) fn symbol_primitive_set_fails(
        &self,
        object: &Value,
        key: &crate::PropertyKey,
    ) -> bool {
        let Value::Object(symbol_object) = object else {
            return false;
        };
        if !symbol::is_symbol_primitive(symbol_object) {
            return false;
        }
        let env = self.current_env();
        if property_set_uses_setter(object, key, &env) {
            return false;
        }
        // A Proxy anywhere in the prototype chain may carry a `set` trap that
        // `property_set_uses_setter` cannot inspect; the assignment must still
        // be attempted so the trap observes it. Only a plain data write onto the
        // Symbol primitive is the silent no-op (strict TypeError).
        !prototype_chain_has_proxy(symbol_object.prototype_slot())
    }

    pub(super) fn is_global_object(&self, value: &Value) -> bool {
        let Value::Object(object) = value else {
            return false;
        };
        matches!(
            self.realm.borrow().get(GLOBAL_THIS_BINDING),
            Some(Value::Object(global_object)) if object.ptr_eq(global_object)
        )
    }

    /// Drops the cached Array.prototype when the `Array` global binding itself is
    /// rewritten, so a later index store resolves the replacement constructor's
    /// prototype.
    pub(super) fn invalidate_array_prototype_cache(&mut self, name: &str) {
        if name == "Array" {
            self.array_prototype_cache = None;
        }
    }

    pub(super) fn delete_prop(&mut self, is_strict: bool) -> Result<(), RuntimeError> {
        let key_value = self.pop()?;
        let key = self.coerce_property_key(key_value)?;
        let object = self.pop()?;
        let mut env = self.current_env();
        let result = delete_property_key(object, &key, &mut env)?;
        self.apply_env(env);
        // Strict-mode `delete` of a non-configurable property is a TypeError.
        if is_strict && matches!(result, Value::Boolean(false)) {
            return Err(RuntimeError {
                thrown: None,
                message: "cannot delete non-configurable property in strict mode".to_owned(),
            });
        }
        self.stack.push(result);
        Ok(())
    }

    /// Resolves a property get through the prototype chain without cloning the
    /// realm env, returning `Some(value)` only when the descriptor is a plain
    /// data property (no getter). Returns `None` to signal that the generic
    /// clone-and-writeback path is required: any accessor descriptor, a Proxy
    /// target, or a primitive base that needs intrinsic lookups not covered
    /// here. Prototype intrinsics are read out of `&self.globals` directly,
    /// avoiding the full `current_env()` map copy on the dominant method-call
    /// and member-read patterns.
    pub(super) fn try_direct_get(&self, object: &Value, key: &PropertyKey) -> Option<Value> {
        match key {
            PropertyKey::String(name) => self.try_direct_get_string(object, name),
            PropertyKey::Symbol(symbol) => self.try_direct_get_symbol(object, symbol),
        }
    }

    fn try_direct_get_string(&self, object: &Value, key: &str) -> Option<Value> {
        match object {
            Value::Object(object) => {
                if crate::typed_array::is_typed_array_object(object) {
                    match crate::typed_array::indexed_element_value(object, key) {
                        crate::typed_array::IndexedRead::Present(value) => return Some(*value),
                        crate::typed_array::IndexedRead::Missing => {
                            return Some(Value::Undefined);
                        }
                        crate::typed_array::IndexedRead::NotIndexed => {}
                    }
                }
                if object.is_module_namespace_exotic() {
                    return None;
                }
                match ordinary_chain_property(object, key) {
                    Err(ProxyInChain) => None,
                    Ok(property) => data_property_value(property).or_else(|| {
                        crate::regexp::default_regexp_source_accessor_value(
                            object,
                            key,
                            &self.realm_env(),
                        )
                    }),
                }
            }
            Value::Map(map) => match ordinary_chain_property(&map.object(), key) {
                Err(ProxyInChain) => None,
                Ok(property) => data_property_value(property),
            },
            Value::Set(set) => match ordinary_chain_property(&set.object(), key) {
                Err(ProxyInChain) => None,
                Ok(property) => data_property_value(property),
            },
            Value::Array(elements) => {
                if key == "length" {
                    return Some(Value::Number(elements.len() as f64));
                }
                // Mirror the order in `property_value_key`: a present dense
                // element wins, then an own (possibly accessor) descriptor, then
                // the prototype chain. `data_property_value` bails on accessors.
                let descriptor = key
                    .parse::<usize>()
                    .ok()
                    .and_then(|index| elements.get(index).map(Property::enumerable))
                    .or_else(|| elements.property(key))
                    .or_else(|| {
                        elements
                            .prototype_override()
                            .unwrap_or_else(|| array_prototype(&self.realm_env()))
                            .and_then(|prototype| prototype.property(key))
                    });
                data_property_value(descriptor)
            }
            Value::Function(function) => {
                let descriptor = function_own_property_descriptor(function, key).or_else(|| {
                    function_prototype_chain_descriptor(function, &self.realm_env(), key)
                });
                match descriptor {
                    Some(property) => data_property_value(Some(property)),
                    // A function with no native-error parent (the only remaining
                    // generic branch) resolves to undefined; bail when a parent
                    // could contribute so the slow path stays authoritative.
                    None if function.native.is_none() => Some(Value::Undefined),
                    None => None,
                }
            }
            Value::String(value) => {
                if key == "length" {
                    return Some(Value::Number(string::string_code_units(value).len() as f64));
                }
                if let Some(value) = string::string_property(value, key) {
                    return Some(value);
                }
                data_property_value(inherited_primitive_prototype_descriptor(
                    &self.realm_env(),
                    "String",
                    key,
                ))
            }
            Value::Number(_) => data_property_value(inherited_primitive_prototype_descriptor(
                &self.realm_env(),
                "Number",
                key,
            )),
            Value::Boolean(_) => data_property_value(inherited_primitive_prototype_descriptor(
                &self.realm_env(),
                "Boolean",
                key,
            )),
            Value::BigInt(_) => data_property_value(inherited_primitive_prototype_descriptor(
                &self.realm_env(),
                "BigInt",
                key,
            )),
            // Proxy needs trap dispatch; Null/Undefined raise catchable errors.
            Value::Proxy(_) | Value::Null | Value::Undefined => None,
        }
    }

    fn try_direct_get_symbol(&self, object: &Value, symbol: &ObjectRef) -> Option<Value> {
        match object {
            Value::Object(object) => match ordinary_chain_symbol_property(object, symbol) {
                Err(ProxyInChain) => None,
                Ok(property) => data_property_value(property),
            },
            Value::Map(map) => match ordinary_chain_symbol_property(&map.object(), symbol) {
                Err(ProxyInChain) => None,
                Ok(property) => data_property_value(property),
            },
            Value::Set(set) => match ordinary_chain_symbol_property(&set.object(), symbol) {
                Err(ProxyInChain) => None,
                Ok(property) => data_property_value(property),
            },
            Value::Array(elements) => {
                let descriptor = elements.symbol_property(symbol).or_else(|| {
                    elements
                        .prototype_override()
                        .unwrap_or_else(|| array_prototype(&self.realm_env()))
                        .and_then(|prototype| prototype.symbol_property(symbol))
                });
                match descriptor {
                    Some(property) => data_property_value(Some(property)),
                    None => Some(Value::Undefined),
                }
            }
            // Function symbol lookup and the primitive wrappers go through the
            // slow path so their intrinsic resolution stays in one place.
            _ => None,
        }
    }

    pub(super) fn store_global_strict(
        &mut self,
        name: String,
        value: Value,
    ) -> Result<(), RuntimeError> {
        if self.env.has_module_import(&name) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        if self.env.is_immutable_lexical_binding(&name) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        // The inner name of a named function expression is immutable; a strict
        // assignment to it is a TypeError.
        if self.env.is_immutable_function_name(&name) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        // A caller-scope binding carried in this frame's locals layer is written
        // there (and propagated back to the caller on return); only a true realm
        // global goes to the shared cell.
        if let Some(slot) = self.bytecode.local_slot(&name)
            && self.locals.get(slot).is_some_and(Option::is_some)
        {
            if self.local_slot_targets_non_writable_global(slot, &name) {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("TypeError: Cannot assign to read only property '{name}'"),
                });
            }
            let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            })?;
            *local = Some(value.clone());
            self.env.insert(name.clone(), value.clone());
            if self.bytecode.global_scope
                && self.bytecode.local_is_body_hoist_only(slot)
                && !super::vm_bindings::is_compiler_temporary(&name)
            {
                if self.realm.borrow().contains_key(&name) {
                    self.realm.borrow_mut().insert(name.clone(), value.clone());
                }
                if let Some(Value::Object(global_this)) =
                    self.realm.borrow().get(GLOBAL_THIS_BINDING).cloned()
                    && global_this.has_own_property(&name)
                {
                    global_this.set(name.clone(), value.clone());
                }
            }
            self.write_through_captured(&name, value);
            return Ok(());
        }
        // Reject writes to non-writable global properties (e.g. NaN, Infinity,
        // undefined) before any env/realm write. In strict mode this is a
        // TypeError per the spec.
        if let Some(property) = self.global_this_own_property(&name) {
            if !property.writable {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("TypeError: Cannot assign to read only property '{name}'"),
                });
            }
        }
        if self.env.locals().contains_key(&name) {
            self.env.insert(name.clone(), value.clone());
            self.write_through_captured(&name, value.clone());
            if let Some(source) = self.env.captured_binding_source_env()
                && source.borrow().contains_key(&name)
            {
                source.borrow_mut().insert(name.clone(), value.clone());
            }
            if self.realm.borrow().contains_key(&name) {
                let should_update_realm = self.env.module_host().is_some()
                    || self.global_this_property(&name).is_none()
                    || self.bytecode.local_slot(&name).is_some_and(|slot| {
                        self.bytecode.global_scope
                            && self.bytecode.local_is_body_hoist_only(slot)
                            && !super::vm_bindings::is_compiler_temporary(&name)
                    })
                    || !self.capture_writeback_targets_name(&name);
                if should_update_realm {
                    self.realm.borrow_mut().insert(name.clone(), value.clone());
                }
            }
            if let Some(Value::Object(global_this)) =
                self.realm.borrow().get(GLOBAL_THIS_BINDING).cloned()
                && global_this.has_own_property(&name)
            {
                global_this.set(name, value);
            }
            return Ok(());
        }
        if !self.realm.borrow().contains_key(&name) && self.global_this_property(&name).is_none() {
            return Err(RuntimeError {
                thrown: None,
                message: format!("ReferenceError: undefined identifier `{name}`"),
            });
        }
        self.invalidate_array_prototype_cache(&name);
        self.realm.borrow_mut().insert(name.clone(), value.clone());
        self.write_through_captured(&name, value.clone());
        let global_this = match self.realm.borrow().get(GLOBAL_THIS_BINDING) {
            Some(Value::Object(global_this)) => Some(global_this.clone()),
            _ => None,
        };
        if let Some(global_this) = global_this
            && global_this.has_own_property(&name)
        {
            global_this.set(name, value);
        }
        Ok(())
    }

    pub(super) fn store_global_sloppy(
        &mut self,
        name: String,
        value: Value,
    ) -> Result<(), RuntimeError> {
        if self.env.has_module_import(&name) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        if self.env.is_immutable_lexical_binding(&name) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        // The inner name of a named function expression is immutable; a sloppy
        // assignment to it is a silent no-op, even from a nested function.
        if self.env.is_immutable_function_name(&name) {
            return Ok(());
        }
        if let Some(slot) = self.bytecode.local_slot(&name)
            && self.locals.get(slot).is_some_and(Option::is_some)
        {
            if self.local_slot_targets_non_writable_global(slot, &name) {
                return Ok(());
            }
            let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            })?;
            *local = Some(value.clone());
            self.env.insert(name.clone(), value.clone());
            if self.bytecode.global_scope
                && self.bytecode.local_is_body_hoist_only(slot)
                && !super::vm_bindings::is_compiler_temporary(&name)
            {
                if self.realm.borrow().contains_key(&name) {
                    self.realm.borrow_mut().insert(name.clone(), value.clone());
                }
                if let Some(Value::Object(global_this)) =
                    self.realm.borrow().get(GLOBAL_THIS_BINDING).cloned()
                    && global_this.has_own_property(&name)
                {
                    global_this.set(name.clone(), value.clone());
                }
            }
            self.write_through_captured(&name, value);
            return Ok(());
        }
        // Silently reject writes to non-writable global properties (e.g. NaN,
        // Infinity, undefined) in sloppy mode.
        if let Some(property) = self.global_this_own_property(&name) {
            if !property.writable {
                return Ok(());
            }
        }
        if self.env.locals().contains_key(&name) {
            self.env.insert(name.clone(), value.clone());
            self.write_through_captured(&name, value.clone());
            if let Some(source) = self.env.captured_binding_source_env()
                && source.borrow().contains_key(&name)
            {
                source.borrow_mut().insert(name.clone(), value.clone());
            }
            if self.realm.borrow().contains_key(&name) {
                let should_update_realm = self.env.module_host().is_some()
                    || self.global_this_property(&name).is_none()
                    || self.bytecode.local_slot(&name).is_some_and(|slot| {
                        self.bytecode.global_scope
                            && self.bytecode.local_is_body_hoist_only(slot)
                            && !super::vm_bindings::is_compiler_temporary(&name)
                    })
                    || !self.capture_writeback_targets_name(&name);
                if should_update_realm {
                    self.realm.borrow_mut().insert(name.clone(), value.clone());
                }
            }
            if let Some(Value::Object(global_this)) =
                self.realm.borrow().get(GLOBAL_THIS_BINDING).cloned()
                && global_this.has_own_property(&name)
            {
                global_this.set(name, value);
            }
            return Ok(());
        }
        self.invalidate_array_prototype_cache(&name);
        if self.realm.borrow().contains_key(&name) {
            self.realm.borrow_mut().insert(name.clone(), value.clone());
            self.write_through_captured(&name, value.clone());
            let global_this = match self.realm.borrow().get(GLOBAL_THIS_BINDING) {
                Some(Value::Object(global_this)) => Some(global_this.clone()),
                _ => None,
            };
            if let Some(global_this) = global_this
                && global_this.has_own_property(&name)
            {
                global_this.set(name, value);
            }
            return Ok(());
        }
        let global_this = match self.realm.borrow().get(GLOBAL_THIS_BINDING) {
            Some(Value::Object(global_this)) => Some(global_this.clone()),
            _ => None,
        };
        if let Some(global_this) = global_this {
            global_this.set(name.clone(), value.clone());
        }
        self.realm.borrow_mut().insert(name.clone(), value.clone());
        self.write_through_captured(&name, value);
        Ok(())
    }
}

/// Extracts a plain data value from a resolved descriptor. Returns `None` for
/// an accessor (a getter must run, which the slow path handles) so the caller
/// falls back to the clone-and-writeback get path.
fn data_property_value(property: Option<Property>) -> Option<Value> {
    match property {
        None => Some(Value::Undefined),
        Some(property) if property.get.is_some() || property.accessor => None,
        Some(property) => Some(property.value),
    }
}

/// Signals that an inline [[Prototype]]-chain walk reached a Proxy. The VM fast
/// paths cannot dispatch a Proxy's trap, so the caller defers to the slow path
/// where the proxy-aware `get`/`set` semantics live.
pub(super) struct ProxyInChain;

/// Walks `object`'s own property then its [[Prototype]] chain for `key`,
/// matching `ObjectRef::property` for an all-ordinary chain. Returns
/// `Err(ProxyInChain)` when a Proxy is reached so the caller defers to the
/// proxy-aware slow path.
pub(super) fn ordinary_chain_property(
    object: &ObjectRef,
    key: &str,
) -> Result<Option<Property>, ProxyInChain> {
    let mut current = object.clone();
    loop {
        if let Some(property) = current.own_property(key) {
            return Ok(Some(property));
        }
        match current.prototype_slot() {
            Some(crate::Prototype::Object(next)) => current = next,
            Some(crate::Prototype::Function(function)) => {
                return Ok(function.chain_property(key));
            }
            Some(crate::Prototype::Proxy(_)) => return Err(ProxyInChain),
            None => return Ok(None),
        }
    }
}

/// Whether any prototype reachable from `slot` is a Proxy, so that a set/get
/// over an ordinary chain must defer to the proxy-aware slow path.
pub(super) fn prototype_chain_has_proxy(slot: Option<crate::Prototype>) -> bool {
    let mut current = slot;
    loop {
        match current {
            Some(crate::Prototype::Proxy(_)) => return true,
            Some(crate::Prototype::Object(object)) => current = object.prototype_slot(),
            Some(crate::Prototype::Function(function)) => {
                current = function.internal_prototype_slot().flatten();
            }
            None => return false,
        }
    }
}

/// Symbol-keyed counterpart to [`ordinary_chain_property`].
pub(super) fn ordinary_chain_symbol_property(
    object: &ObjectRef,
    symbol: &ObjectRef,
) -> Result<Option<Property>, ProxyInChain> {
    let mut current = object.clone();
    loop {
        if let Some(property) = current.own_symbol_property(symbol) {
            return Ok(Some(property));
        }
        match current.prototype_slot() {
            Some(crate::Prototype::Object(next)) => current = next,
            Some(crate::Prototype::Function(function)) => {
                return Ok(function.chain_symbol_property(symbol));
            }
            Some(crate::Prototype::Proxy(_)) => return Err(ProxyInChain),
            None => return Ok(None),
        }
    }
}

pub(super) fn get_property(
    object: Value,
    key: &str,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    match object {
        Value::Array(elements) if key == "length" => Ok(Value::Number(elements.len() as f64)),
        Value::Array(elements) => property_value(Value::Array(elements), key, env),
        Value::Function(function) => property_value(Value::Function(function), key, env),
        Value::String(value) if key == "length" => {
            Ok(Value::Number(string::string_code_units(&value).len() as f64))
        }
        Value::String(value) => match string::string_property(&value, key) {
            Some(value) => Ok(value),
            None => {
                let receiver = Value::String(value);
                property_value_key_with_receiver(
                    receiver.clone(),
                    &PropertyKey::String(key.to_owned()),
                    receiver,
                    env,
                )
            }
        },
        object @ (Value::Boolean(_) | Value::Number(_) | Value::BigInt(_)) => {
            property_value_key_with_receiver(
                object.clone(),
                &PropertyKey::String(key.to_owned()),
                object,
                env,
            )
        }
        Value::Map(_) | Value::Set(_) | Value::Proxy(_) | Value::Object(_) => {
            property_value(object, key, env)
        }
        Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: Cannot read properties of undefined (reading '{key}')"),
        }),
        Value::Null => Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: Cannot read properties of null (reading '{key}')"),
        }),
    }
}

pub(super) fn get_property_key(
    object: Value,
    key: &PropertyKey,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    match key {
        PropertyKey::String(key) => get_property(object, key, env),
        PropertyKey::Symbol(_) => property_value_key(object, key, env),
    }
}

pub(super) fn delete_property_key(
    object: Value,
    key: &PropertyKey,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    match key {
        PropertyKey::String(key) => delete_property(object, key, env),
        PropertyKey::Symbol(symbol) => delete_symbol_property(object, symbol, env),
    }
}

fn delete_property(object: Value, key: &str, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    match object {
        Value::Object(object) => {
            if crate::typed_array::is_typed_array_object(&object)
                && let crate::typed_array::IndexedDelete::Handled(success) =
                    crate::typed_array::delete_indexed_element(&object, key)
            {
                return Ok(Value::Boolean(success));
            }
            Ok(Value::Boolean(object.delete_own_property(key)))
        }
        Value::Proxy(proxy) => Ok(Value::Boolean(crate::proxy::proxy_delete_property(
            proxy,
            &PropertyKey::String(key.to_owned()),
            env,
        )?)),
        Value::Map(map) => Ok(Value::Boolean(map.object().delete_own_property(key))),
        Value::Set(set) => Ok(Value::Boolean(set.object().delete_own_property(key))),
        Value::Array(elements) => Ok(Value::Boolean(match key.parse::<usize>() {
            Ok(index) => elements.delete_index(index),
            Err(_) => elements.delete_property(key),
        })),
        Value::Function(function) => {
            Ok(Value::Boolean(function_delete_own_property(&function, key)))
        }
        Value::String(s) => {
            // String objects have non-configurable indexed character properties.
            if let Ok(index) = key.parse::<usize>() {
                Ok(Value::Boolean(index >= s.len()))
            } else {
                Ok(Value::Boolean(true))
            }
        }
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "TypeError: cannot delete property of null or undefined".to_owned(),
        }),
        _ => Ok(Value::Boolean(true)),
    }
}

fn delete_symbol_property(
    object: Value,
    symbol: &ObjectRef,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    match object {
        Value::Object(object) => Ok(Value::Boolean(object.delete_own_symbol_property(symbol))),
        Value::Proxy(proxy) => Ok(Value::Boolean(crate::proxy::proxy_delete_property(
            proxy,
            &PropertyKey::Symbol(symbol.clone()),
            env,
        )?)),
        Value::Map(map) => Ok(Value::Boolean(
            map.object().delete_own_symbol_property(symbol),
        )),
        Value::Set(set) => Ok(Value::Boolean(
            set.object().delete_own_symbol_property(symbol),
        )),
        Value::Function(function) => Ok(Value::Boolean(function_delete_own_symbol_property(
            &function, symbol,
        ))),
        Value::Array(elements) => Ok(Value::Boolean(elements.delete_own_symbol_property(symbol))),
        Value::Null | Value::Undefined => Err(RuntimeError {
            thrown: None,
            message: "TypeError: cannot delete property of null or undefined".to_owned(),
        }),
        _ => Ok(Value::Boolean(true)),
    }
}

pub(super) fn enumerable_keys(value: Value, env: &mut CallEnv) -> Result<Vec<Value>, RuntimeError> {
    // EnumerateObjectProperties walks the prototype chain, collecting each
    // layer's enumerable own string keys (shadowing already-seen names). An
    // exotic Proxy in the chain is consulted through its ownKeys /
    // getOwnPropertyDescriptor / getPrototypeOf traps.
    let mut keys: Vec<String> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut current = value;
    loop {
        let prototype = match &current {
            Value::Proxy(proxy) => {
                proxy_enumerable_layer(proxy.clone(), &mut keys, &mut seen, env)?
            }
            Value::Object(_)
            | Value::Array(_)
            | Value::Function(_)
            | Value::Map(_)
            | Value::Set(_) => {
                for key in own_string_keys(&current) {
                    if !seen.iter().any(|existing| existing == &key) {
                        let property_key = PropertyKey::String(key.clone());
                        if let Some(property) = crate::object::own_property_descriptor_key(
                            current.clone(),
                            &property_key,
                            env,
                        )? {
                            if property.enumerable {
                                keys.push(key.clone());
                            }
                            seen.push(key);
                        }
                    }
                }
                crate::value_prototype_slot(current.clone(), env)
                    .map(|slot| slot.to_value())
                    .unwrap_or(Value::Null)
            }
            Value::Null | Value::Undefined => break,
            _ => {
                return Err(RuntimeError {
                    thrown: None,
                    message: "for-in target is not enumerable".to_owned(),
                });
            }
        };
        match prototype {
            Value::Null | Value::Undefined => break,
            next => current = next,
        }
    }
    Ok(keys.into_iter().map(|s| Value::String(s.into())).collect())
}

/// The ordinary own string key list for a prototype-chain layer. The caller
/// observes each key through `[[GetOwnProperty]]`, which matters for module
/// namespace TDZ bindings.
fn own_string_keys(value: &Value) -> Vec<String> {
    match value {
        Value::Object(object) => {
            if crate::typed_array::is_typed_array_object(object) {
                crate::typed_array::typed_array_own_property_names(object)
            } else {
                object.own_property_names()
            }
        }
        Value::Array(elements) => {
            let mut keys: Vec<_> = (0..elements.len())
                .filter(|index| elements.has_index(*index))
                .map(|index| index.to_string())
                .collect();
            keys.extend(elements.property_keys());
            keys
        }
        Value::Function(function) => function_own_property_names(function),
        Value::Map(map) => map.object().own_property_names(),
        Value::Set(set) => set.object().own_property_names(),
        _ => Vec::new(),
    }
}

/// Collects a Proxy layer's enumerable own string keys via its ownKeys and
/// getOwnPropertyDescriptor traps, returning the proxy's [[GetPrototypeOf]] so
/// the caller can continue up the chain.
fn proxy_enumerable_layer(
    proxy: crate::proxy::ProxyRef,
    keys: &mut Vec<String>,
    seen: &mut Vec<String>,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    for key in crate::proxy::proxy_own_keys(proxy.clone(), env)? {
        let PropertyKey::String(name) = key else {
            continue; // for-in ignores symbol keys.
        };
        let property_key = PropertyKey::String(name.clone());
        let descriptor = crate::proxy::proxy_get_own_property_descriptor(
            proxy.clone(),
            &property_key,
            env,
            |target, env| crate::object::own_property_descriptor_key(target, &property_key, env),
        )?;
        if let Some(property) = descriptor {
            if !seen.iter().any(|existing| existing == &name) {
                if property.enumerable {
                    keys.push(name.clone());
                }
                seen.push(name);
            }
        }
    }
    crate::proxy::proxy_get_prototype_of(proxy, env)
}

pub(super) fn fast_number_binary(left: &Value, op: BinaryOp, right: &Value) -> Option<Value> {
    let (Value::Number(left), Value::Number(right)) = (left, right) else {
        return None;
    };
    let value = match op {
        BinaryOp::Add => Value::Number(left + right),
        BinaryOp::Sub => Value::Number(left - right),
        BinaryOp::Mul => Value::Number(left * right),
        BinaryOp::Div => Value::Number(left / right),
        BinaryOp::Rem => Value::Number(left % right),
        BinaryOp::Pow => Value::Number(crate::operations::number_exponentiate(*left, *right)),
        BinaryOp::Shl => Value::Number(f64::from(
            to_int32_number(*left) << (to_uint32_number(*right) & 0x1f),
        )),
        BinaryOp::Shr => Value::Number(f64::from(
            to_int32_number(*left) >> (to_uint32_number(*right) & 0x1f),
        )),
        BinaryOp::UShr => Value::Number(f64::from(
            to_uint32_number(*left) >> (to_uint32_number(*right) & 0x1f),
        )),
        BinaryOp::BitwiseAnd => {
            Value::Number(f64::from(to_int32_number(*left) & to_int32_number(*right)))
        }
        BinaryOp::BitwiseXor => {
            Value::Number(f64::from(to_int32_number(*left) ^ to_int32_number(*right)))
        }
        BinaryOp::BitwiseOr => {
            Value::Number(f64::from(to_int32_number(*left) | to_int32_number(*right)))
        }
        BinaryOp::Eq => Value::Boolean(left == right),
        BinaryOp::Ne => Value::Boolean(left != right),
        BinaryOp::Lt => Value::Boolean(left < right),
        BinaryOp::Le => Value::Boolean(left <= right),
        BinaryOp::Gt => Value::Boolean(left > right),
        BinaryOp::Ge => Value::Boolean(left >= right),
        BinaryOp::StrictEq => Value::Boolean(left == right),
        BinaryOp::StrictNe => Value::Boolean(left != right),
        _ => return None,
    };
    Some(value)
}

pub(super) fn fast_number_unary(op: UnaryOp, argument: &Value) -> Option<Value> {
    let Value::Number(number) = argument else {
        return None;
    };
    let value = match op {
        UnaryOp::Plus => Value::Number(*number),
        UnaryOp::Minus => Value::Number(-*number),
        UnaryOp::BitwiseNot => Value::Number(f64::from(!to_int32_number(*number))),
        _ => return None,
    };
    Some(value)
}

/// Returns the value as an array index when it is a non-negative integer that
/// fits the array-index range (`< 2^32 - 1`). Used to take the dense-store fast
/// path for `array[i] = x` without round-tripping the index through a string.
pub(super) fn array_index_from_number(number: f64) -> Option<usize> {
    if number < 0.0 || number.fract() != 0.0 || number > (u32::MAX - 1) as f64 {
        return None;
    }
    Some(number as usize)
}
