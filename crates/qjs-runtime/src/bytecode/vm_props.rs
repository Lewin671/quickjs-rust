use std::collections::HashMap;

use qjs_ast::{BinaryOp, UnaryOp};

use crate::{
    GLOBAL_THIS_BINDING, ObjectRef, Property, PropertyKey, RuntimeError, Value, array_prototype,
    bigint, boolean, call_function, function_delete_own_property,
    function_delete_own_symbol_property, function_own_property_descriptor,
    function_own_property_keys, function_prototype_chain_descriptor,
    inherited_primitive_prototype_descriptor, inherited_string_prototype_property, number,
    object::define_array_length_value, property_value, property_value_key, string, to_int32_number,
    to_uint32_number, value_prototype,
};

use super::vm::Vm;

impl Vm<'_> {
    /// Whether the realm's current Array.prototype owns any indexed property.
    /// Returns `None` when no Array.prototype is reachable. The Array.prototype
    /// object is cached so the hot path skips the `Array`-binding lookup; the
    /// cache is dropped when the `Array` global is reassigned. The own-index
    /// count read is itself O(1).
    pub(super) fn array_prototype_has_index_property(&mut self) -> Option<bool> {
        if self.array_prototype_cache.is_none() {
            self.array_prototype_cache = Some(array_prototype(&self.globals)?);
        }
        self.array_prototype_cache
            .as_ref()
            .map(ObjectRef::has_own_index_property)
    }

    /// Drops the cached Array.prototype when the `Array` global binding itself is
    /// rewritten, so a later index store resolves the replacement constructor's
    /// prototype.
    pub(super) fn invalidate_array_prototype_cache(&mut self, name: &str) {
        if name == "Array" {
            self.array_prototype_cache = None;
        }
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
            Value::Object(object) => data_property_value(object.property(key)),
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
                            .unwrap_or_else(|| array_prototype(&self.globals))
                            .and_then(|prototype| prototype.property(key))
                    });
                data_property_value(descriptor)
            }
            Value::Function(function) => {
                let descriptor = function_own_property_descriptor(function, key)
                    .or_else(|| function_prototype_chain_descriptor(function, &self.globals, key));
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
                    &self.globals,
                    "String",
                    key,
                ))
            }
            Value::Number(_) => data_property_value(inherited_primitive_prototype_descriptor(
                &self.globals,
                "Number",
                key,
            )),
            Value::Boolean(_) => data_property_value(inherited_primitive_prototype_descriptor(
                &self.globals,
                "Boolean",
                key,
            )),
            Value::BigInt(_) => data_property_value(inherited_primitive_prototype_descriptor(
                &self.globals,
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
                        .unwrap_or_else(|| array_prototype(&self.globals))
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
        if !self.globals.contains_key(&name) {
            return Err(RuntimeError {
                thrown: None,
                message: format!("ReferenceError: undefined identifier `{name}`"),
            });
        }
        self.invalidate_array_prototype_cache(&name);
        self.globals.insert(name.clone(), value.clone());
        if let Some(Value::Object(global_this)) = self.globals.get(GLOBAL_THIS_BINDING)
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
        self.invalidate_array_prototype_cache(&name);
        if let Some(Value::Object(global_this)) = self.globals.get(GLOBAL_THIS_BINDING) {
            global_this.set(name.clone(), value.clone());
            self.globals.insert(name, value);
            return Ok(());
        }
        self.globals.insert(name, value);
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
    env: &mut HashMap<String, Value>,
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
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match key {
        PropertyKey::String(key) => get_property(object, key, env),
        PropertyKey::Symbol(_) => property_value_key(object, key, env),
    }
}

pub(super) fn set_property(
    object: Value,
    key: String,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    match object {
        Value::Object(object) => {
            if apply_property_setter(
                object.property(&key),
                Value::Object(object.clone()),
                value.clone(),
                env,
            )? {
                return Ok(false);
            }
            object.set(key, value);
            Ok(true)
        }
        Value::Function(function) => {
            if apply_property_setter(
                function_property_for_set(&function, env, &key),
                Value::Function(function.clone()),
                value.clone(),
                env,
            )? {
                return Ok(false);
            }
            function.set_property(key, value);
            Ok(true)
        }
        Value::Array(elements) => {
            if key == "length" {
                define_array_length_value(&elements, value, env)
            } else {
                let property = elements.property(&key).or_else(|| {
                    elements
                        .prototype_override()
                        .unwrap_or_else(|| array_prototype(env))
                        .and_then(|prototype| prototype.property(&key))
                });
                if apply_property_setter(
                    property,
                    Value::Array(elements.clone()),
                    value.clone(),
                    env,
                )? {
                    return Ok(false);
                }
                match key.parse::<usize>() {
                    Ok(index) => elements.set(index, value),
                    Err(_) => elements.set_property(key, value),
                };
                Ok(true)
            }
        }
        Value::Map(map) => {
            if apply_property_setter(
                map.object().property(&key),
                Value::Map(map.clone()),
                value.clone(),
                env,
            )? {
                return Ok(false);
            }
            map.object().set(key, value);
            Ok(true)
        }
        Value::Set(set) => {
            if apply_property_setter(
                set.object().property(&key),
                Value::Set(set.clone()),
                value.clone(),
                env,
            )? {
                return Ok(false);
            }
            set.object().set(key, value);
            Ok(true)
        }
        Value::Proxy(proxy) => set_property(proxy.target(), key, value, env),
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
    env: &mut HashMap<String, Value>,
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
    env: &mut HashMap<String, Value>,
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
    env: &mut HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    let inherited = function.symbol_property(&symbol, env);
    if apply_property_setter(inherited.clone(), receiver, value.clone(), env)? {
        return Ok(false);
    }
    let descriptor = match function.own_symbol_property(&symbol) {
        Some(existing) if !existing.writable => return Ok(false),
        Some(existing) => Property::data(
            value,
            existing.enumerable,
            existing.writable,
            existing.configurable,
        ),
        None if inherited.is_some_and(|property| !property.writable) => return Ok(false),
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
    env: &mut HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    if apply_property_setter(
        object.symbol_property(&symbol),
        receiver,
        value.clone(),
        env,
    )? {
        return Ok(false);
    }
    let descriptor = match object.own_symbol_property(&symbol) {
        Some(existing) if !existing.writable => return Ok(false),
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
    env: &mut HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    let inherited = array.symbol_property(&symbol).or_else(|| {
        array
            .prototype_override()
            .unwrap_or_else(|| array_prototype(env))
            .and_then(|prototype| prototype.symbol_property(&symbol))
    });
    if apply_property_setter(inherited.clone(), receiver, value.clone(), env)? {
        return Ok(false);
    }
    let descriptor = match array.own_symbol_property(&symbol) {
        Some(existing) if !existing.writable => return Ok(false),
        Some(existing) => Property::data(
            value,
            existing.enumerable,
            existing.writable,
            existing.configurable,
        ),
        None if inherited.is_some_and(|property| !property.writable) => return Ok(false),
        None if !array.is_extensible() => return Ok(false),
        None => Property::enumerable(value),
    };
    array.define_symbol_property(symbol, descriptor);
    Ok(true)
}

fn apply_property_setter(
    property: Option<Property>,
    receiver: Value,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<bool, RuntimeError> {
    let Some(property) = property else {
        return Ok(false);
    };
    if let Some(setter) = property.set {
        call_function(setter, receiver, vec![value], env, false)?;
        return Ok(true);
    }
    Ok(property.is_accessor())
}

pub(super) fn property_set_uses_setter(
    object: &Value,
    key: &PropertyKey,
    env: &HashMap<String, Value>,
) -> bool {
    property_for_set(object, key, env).is_some_and(|property| property.set.is_some())
}

fn property_for_set(
    object: &Value,
    key: &PropertyKey,
    env: &HashMap<String, Value>,
) -> Option<Property> {
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
    env: &HashMap<String, Value>,
    key: &str,
) -> Option<Property> {
    function.properties.borrow().get(key).cloned().or_else(|| {
        value_prototype(Value::Function(function.clone()), env)
            .and_then(|prototype| prototype.property(key))
    })
}

fn symbol_property_for_set(
    object: &Value,
    key: &PropertyKey,
    env: &HashMap<String, Value>,
) -> Option<Property> {
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
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match key {
        PropertyKey::String(key) => delete_property(object, key, env),
        PropertyKey::Symbol(symbol) => delete_symbol_property(object, symbol, env),
    }
}

fn delete_property(
    object: Value,
    key: &str,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match object {
        Value::Object(object) => Ok(Value::Boolean(object.delete_own_property(key))),
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
        _ => Err(RuntimeError {
            thrown: None,
            message: "delete target is not an object".to_owned(),
        }),
    }
}

fn delete_symbol_property(
    object: Value,
    symbol: &ObjectRef,
    env: &mut HashMap<String, Value>,
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
        _ => Err(RuntimeError {
            thrown: None,
            message: "delete target is not an object".to_owned(),
        }),
    }
}

pub(super) fn enumerable_keys(
    value: Value,
    env: &HashMap<String, Value>,
) -> Result<Vec<Value>, RuntimeError> {
    let mut keys = match value.clone() {
        Value::Object(object) => object.own_property_keys(),
        Value::Array(elements) => {
            let mut keys: Vec<_> = (0..elements.len())
                .filter(|index| elements.has_index(*index))
                .map(|index| index.to_string())
                .collect();
            keys.extend(elements.property_keys());
            keys
        }
        Value::Function(function) => function_own_property_keys(&function),
        Value::Map(map) => map.object().own_property_keys(),
        Value::Set(set) => set.object().own_property_keys(),
        Value::Null | Value::Undefined => Vec::new(),
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "for-in target is not enumerable".to_owned(),
            });
        }
    };
    append_prototype_enumerable_keys(&mut keys, value_prototype(value, env));
    Ok(keys.into_iter().map(Value::String).collect())
}

fn append_prototype_enumerable_keys(keys: &mut Vec<String>, mut prototype: Option<ObjectRef>) {
    while let Some(object) = prototype {
        for key in object.own_property_keys() {
            if !keys.iter().any(|existing| existing == &key) {
                keys.push(key);
            }
        }
        prototype = object.prototype();
    }
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
