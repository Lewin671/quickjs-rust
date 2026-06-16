use qjs_ast::{BinaryOp, UnaryOp};

use crate::{
    GLOBAL_THIS_BINDING, ObjectRef, Property, PropertyKey, RuntimeError, Value, array_prototype,
    bigint, boolean, call_function, function_delete_own_property,
    function_delete_own_symbol_property, function_own_property_descriptor,
    function_own_property_keys, function_own_property_names, function_prototype_chain_descriptor,
    inherited_primitive_prototype_descriptor, inherited_string_prototype_property, number,
    object::define_array_length_value, property_value, property_value_key, string, symbol,
    to_int32_number, to_uint32_number, value_prototype,
};

use super::vm::Vm;
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
        if !matches!(object, Value::Object(object) if symbol::is_symbol_primitive(object)) {
            return false;
        }
        let env = self.current_env();
        !property_set_uses_setter(object, key, &env)
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
                let property = object.property(key);
                data_property_value(property).or_else(|| {
                    crate::regexp::default_regexp_source_accessor_value(
                        object,
                        key,
                        &self.realm_env(),
                    )
                })
            }
            Value::Map(map) => data_property_value(map.object().property(key)),
            Value::Set(set) => data_property_value(set.object().property(key)),
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
            Value::Object(object) => data_property_value(object.symbol_property(symbol)),
            Value::Map(map) => data_property_value(map.object().symbol_property(symbol)),
            Value::Set(set) => data_property_value(set.object().symbol_property(symbol)),
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
        // A caller-scope binding carried in this frame's locals layer is written
        // there (and propagated back to the caller on return); only a true realm
        // global goes to the shared cell.
        if let Some(slot) = self.bytecode.local_slot(&name)
            && let Some(local) = self.locals.get_mut(slot)
            && local.is_some()
        {
            *local = Some(value.clone());
            self.env.insert(name.clone(), value.clone());
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
            self.write_through_captured(&name, value);
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
        if let Some(slot) = self.bytecode.local_slot(&name)
            && let Some(local) = self.locals.get_mut(slot)
            && local.is_some()
        {
            *local = Some(value.clone());
            self.env.insert(name.clone(), value.clone());
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
            self.write_through_captured(&name, value);
            return Ok(());
        }
        self.invalidate_array_prototype_cache(&name);
        if self.realm.borrow().contains_key(&name) {
            self.realm.borrow_mut().insert(name.clone(), value.clone());
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
        self.realm.borrow_mut().insert(name, value);
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
        Value::String(value) => Ok(string::string_property(&value, key)
            .or_else(|| inherited_string_prototype_property(env, key))
            .unwrap_or(Value::Undefined)),
        Value::Boolean(_) => {
            Ok(boolean::inherited_boolean_prototype_property(env, key).unwrap_or(Value::Undefined))
        }
        Value::Number(_) => {
            Ok(number::inherited_number_prototype_property(env, key).unwrap_or(Value::Undefined))
        }
        Value::BigInt(_) => {
            Ok(bigint::inherited_bigint_prototype_property(env, key).unwrap_or(Value::Undefined))
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

pub(crate) fn set_property(
    object: Value,
    key: String,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    match object {
        Value::Object(object) => {
            // Integer-indexed writes on a typed array route through the
            // per-kind numeric conversion and the backing buffer
            // (IntegerIndexedElementSet) before the ordinary property path.
            if crate::typed_array::is_typed_array_object(&object) {
                if let crate::typed_array::IndexedWrite::Handled =
                    crate::typed_array::set_indexed_element(&object, &key, value.clone(), env)?
                {
                    return Ok(true);
                }
            }
            let receiver = Value::Object(object.clone());
            ordinary_set_object(&object, receiver, key, value, env)
        }
        Value::Function(function) => {
            let receiver = Value::Function(function.clone());
            let inherited = function_property_for_set(&function, env, &key);
            match apply_set_step(inherited, receiver, value.clone(), env)? {
                SetStep::Done(ok) => Ok(ok),
                SetStep::WriteData => {
                    function.set_property(key, value);
                    Ok(true)
                }
            }
        }
        Value::Array(elements) => {
            if key == "length" {
                define_array_length_value(&elements, value, env)
            } else {
                let receiver = Value::Array(elements.clone());
                let property = elements.property(&key).or_else(|| {
                    elements
                        .prototype_override()
                        .unwrap_or_else(|| array_prototype(env))
                        .and_then(|prototype| prototype.property(&key))
                });
                match apply_set_step(property, receiver, value.clone(), env)? {
                    SetStep::Done(ok) => Ok(ok),
                    SetStep::WriteData => {
                        match key.parse::<usize>() {
                            Ok(index) => elements.set(index, value),
                            Err(_) => elements.set_property(key, value),
                        };
                        Ok(true)
                    }
                }
            }
        }
        Value::Map(map) => {
            let receiver = Value::Map(map.clone());
            ordinary_set_object(&map.object(), receiver, key, value, env)
        }
        Value::Set(set) => {
            let receiver = Value::Set(set.clone());
            ordinary_set_object(&set.object(), receiver, key, value, env)
        }
        Value::Proxy(proxy) => crate::reflect::ordinary_set(
            Value::Proxy(proxy.clone()),
            &PropertyKey::String(key),
            value,
            Value::Proxy(proxy),
            env,
        ),
        _ => Err(RuntimeError {
            thrown: None,
            message: "member assignment target is not an object".to_owned(),
        }),
    }
}

pub(super) fn set_property_key(
    object: Value,
    key: PropertyKey,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    match key {
        PropertyKey::String(key) => set_property(object, key, value, env),
        PropertyKey::Symbol(symbol) => set_symbol_property(object, symbol, value, env),
    }
}

fn set_symbol_property(
    object: Value,
    symbol: ObjectRef,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    match object {
        Value::Object(object) => {
            set_object_symbol_property(object.clone(), Value::Object(object), symbol, value, env)
        }
        Value::Map(map) => {
            set_object_symbol_property(map.object(), Value::Map(map), symbol, value, env)
        }
        Value::Set(set) => {
            set_object_symbol_property(set.object(), Value::Set(set), symbol, value, env)
        }
        Value::Proxy(proxy) => crate::reflect::ordinary_set(
            Value::Proxy(proxy.clone()),
            &PropertyKey::Symbol(symbol),
            value,
            Value::Proxy(proxy),
            env,
        ),
        Value::Function(function) => set_function_symbol_property(
            function.clone(),
            Value::Function(function),
            symbol,
            value,
            env,
        ),
        Value::Array(elements) => {
            set_array_symbol_property(elements.clone(), Value::Array(elements), symbol, value, env)
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "member assignment target is not an object".to_owned(),
        }),
    }
}

fn set_function_symbol_property(
    function: crate::Function,
    receiver: Value,
    symbol: ObjectRef,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let inherited = function.symbol_property(&symbol, env);
    match apply_set_step(inherited, receiver, value.clone(), env)? {
        SetStep::Done(ok) => return Ok(ok),
        SetStep::WriteData => {}
    }
    let descriptor = match function.own_symbol_property(&symbol) {
        Some(existing) => Property::data(
            value,
            existing.enumerable,
            existing.writable,
            existing.configurable,
        ),
        None if !function.is_extensible() => return Ok(false),
        None => Property::enumerable(value),
    };
    function.define_symbol_property(symbol, descriptor);
    Ok(true)
}

fn set_object_symbol_property(
    object: ObjectRef,
    receiver: Value,
    symbol: ObjectRef,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    match apply_set_step(
        object.symbol_property(&symbol),
        receiver,
        value.clone(),
        env,
    )? {
        SetStep::Done(ok) => return Ok(ok),
        SetStep::WriteData => {}
    }
    let descriptor = match object.own_symbol_property(&symbol) {
        Some(existing) => Property::data(
            value,
            existing.enumerable,
            existing.writable,
            existing.configurable,
        ),
        None if !object.is_extensible() => return Ok(false),
        None => Property::enumerable(value),
    };
    object.define_symbol_property(symbol, descriptor);
    Ok(true)
}

fn set_array_symbol_property(
    array: crate::ArrayRef,
    receiver: Value,
    symbol: ObjectRef,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let inherited = array.symbol_property(&symbol).or_else(|| {
        array
            .prototype_override()
            .unwrap_or_else(|| array_prototype(env))
            .and_then(|prototype| prototype.symbol_property(&symbol))
    });
    match apply_set_step(inherited, receiver, value.clone(), env)? {
        SetStep::Done(ok) => return Ok(ok),
        SetStep::WriteData => {}
    }
    let descriptor = match array.own_symbol_property(&symbol) {
        Some(existing) => Property::data(
            value,
            existing.enumerable,
            existing.writable,
            existing.configurable,
        ),
        None if !array.is_extensible() => return Ok(false),
        None => Property::enumerable(value),
    };
    array.define_symbol_property(symbol, descriptor);
    Ok(true)
}

/// Result of inspecting the resolved (own or inherited) property in the first
/// part of OrdinarySet. `Done(ok)` means the operation finished: a setter ran
/// (`true`), or it was rejected by a non-writable data property or a
/// getter-only accessor (`false`). `WriteData` means the caller should create
/// or overwrite an own data property and report success.
enum SetStep {
    Done(bool),
    WriteData,
}

/// Implements the property-inspection prelude of OrdinarySet for a resolved
/// own-or-inherited `property`. Returns whether `[[Set]]` succeeded, or signals
/// that a data write should follow.
fn apply_set_step(
    property: Option<Property>,
    receiver: Value,
    value: Value,
    env: &mut CallEnv,
) -> Result<SetStep, RuntimeError> {
    let Some(property) = property else {
        return Ok(SetStep::WriteData);
    };
    if property.is_accessor() {
        // Accessor property: succeed only when a setter exists.
        return match property.set {
            Some(setter) => {
                call_function(setter, receiver, vec![value], env, false)?;
                Ok(SetStep::Done(true))
            }
            None => Ok(SetStep::Done(false)),
        };
    }
    // Data property (own or inherited). A non-writable data property in the
    // chain rejects the write entirely; OrdinarySet otherwise falls through to
    // creating/overwriting an own data property.
    if !property.writable {
        return Ok(SetStep::Done(false));
    }
    Ok(SetStep::WriteData)
}

/// OrdinarySet for objects backed by an [`ObjectRef`] (plain objects, Map, Set
/// exotic wrappers). Honors own and inherited non-writable data properties and
/// accessors, returning the `[[Set]]` success boolean.
fn ordinary_set_object(
    object: &ObjectRef,
    receiver: Value,
    key: String,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    match apply_set_step(object.property(&key), receiver, value.clone(), env)? {
        SetStep::Done(ok) => Ok(ok),
        SetStep::WriteData => {
            // Creating an own property (the key is only inherited or absent)
            // requires an extensible receiver. An own writable data property is
            // overwritten in place regardless of extensibility.
            if object.own_property(&key).is_none() && !object.is_extensible() {
                return Ok(false);
            }
            object.set(key, value);
            Ok(true)
        }
    }
}

pub(super) fn property_set_uses_setter(object: &Value, key: &PropertyKey, env: &CallEnv) -> bool {
    property_for_set(object, key, env).is_some_and(|property| property.set.is_some())
}

fn property_for_set(object: &Value, key: &PropertyKey, env: &CallEnv) -> Option<Property> {
    let PropertyKey::String(key) = key else {
        return symbol_property_for_set(object, key, env);
    };
    match object {
        Value::Object(object) => object.property(key),
        Value::Function(function) => function_property_for_set(function, env, key),
        Value::Array(elements) => elements.property(key).or_else(|| {
            elements
                .prototype_override()
                .unwrap_or_else(|| array_prototype(env))
                .and_then(|prototype| prototype.property(key))
        }),
        Value::Map(map) => map.object().property(key),
        Value::Set(set) => set.object().property(key),
        _ => None,
    }
}

fn function_property_for_set(
    function: &crate::Function,
    env: &CallEnv,
    key: &str,
) -> Option<Property> {
    function_own_property_descriptor(function, key)
        .or_else(|| function_prototype_chain_descriptor(function, env, key))
}

fn symbol_property_for_set(object: &Value, key: &PropertyKey, env: &CallEnv) -> Option<Property> {
    let PropertyKey::Symbol(symbol) = key else {
        unreachable!("symbol property helper should only receive symbol keys");
    };
    match object {
        Value::Object(object) => object.symbol_property(symbol),
        Value::Function(function) => function.symbol_property(symbol, env),
        Value::Map(map) => map.object().symbol_property(symbol),
        Value::Set(set) => set.object().symbol_property(symbol),
        Value::Array(elements) => elements.symbol_property(symbol).or_else(|| {
            elements
                .prototype_override()
                .unwrap_or_else(|| array_prototype(env))
                .and_then(|prototype| prototype.symbol_property(symbol))
        }),
        _ => None,
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
                let (enumerable, all) = own_enumerable_and_all(&current);
                for key in enumerable {
                    if !seen.iter().any(|existing| existing == &key) {
                        keys.push(key);
                    }
                }
                for key in all {
                    if !seen.iter().any(|existing| existing == &key) {
                        seen.push(key);
                    }
                }
                value_prototype(current.clone(), env)
                    .map(Value::Object)
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
    Ok(keys.into_iter().map(Value::String).collect())
}

/// The (enumerable-own, all-own) string key lists for an ordinary value, used
/// per prototype-chain layer by [`enumerable_keys`].
fn own_enumerable_and_all(value: &Value) -> (Vec<String>, Vec<String>) {
    match value {
        Value::Object(object) => {
            if crate::typed_array::is_typed_array_object(object) {
                (
                    crate::typed_array::typed_array_own_property_keys(object),
                    crate::typed_array::typed_array_own_property_names(object),
                )
            } else {
                (object.own_property_keys(), object.own_property_names())
            }
        }
        Value::Array(elements) => {
            let mut keys: Vec<_> = (0..elements.len())
                .filter(|index| elements.has_index(*index))
                .map(|index| index.to_string())
                .collect();
            keys.extend(elements.property_keys());
            (keys.clone(), keys)
        }
        Value::Function(function) => (
            function_own_property_keys(function),
            function_own_property_names(function),
        ),
        Value::Map(map) => (
            map.object().own_property_keys(),
            map.object().own_property_names(),
        ),
        Value::Set(set) => (
            set.object().own_property_keys(),
            set.object().own_property_names(),
        ),
        _ => (Vec::new(), Vec::new()),
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
            |target, _env| crate::object::own_property_descriptor_key(target, &property_key),
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
        BinaryOp::Pow => Value::Number(left.powf(*right)),
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
