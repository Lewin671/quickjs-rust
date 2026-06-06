use std::collections::HashMap;

use qjs_ast::{BinaryOp, UnaryOp};

use crate::{
    GLOBAL_THIS_BINDING, ObjectRef, Property, PropertyKey, RuntimeError, Value, array_prototype,
    boolean, call_function, function_delete_own_property, function_delete_own_symbol_property,
    function_own_property_keys, inherited_string_prototype_property, number, property_value,
    property_value_key, string, to_int32_number, to_length, to_uint32_number,
};

use super::vm::Vm;

impl Vm<'_> {
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
        self.globals.insert(name.clone(), value.clone());
        if let Some(Value::Object(global_this)) = self.globals.get(GLOBAL_THIS_BINDING)
            && global_this.has_own_property(&name)
        {
            global_this.set(name, value);
        }
        Ok(())
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
        Value::String(value) if key == "length" => Ok(Value::Number(value.chars().count() as f64)),
        Value::String(value) => Ok(string::string_property(&value, key)
            .or_else(|| inherited_string_prototype_property(env, key))
            .unwrap_or(Value::Undefined)),
        Value::Boolean(_) => {
            Ok(boolean::inherited_boolean_prototype_property(env, key).unwrap_or(Value::Undefined))
        }
        Value::Number(_) => {
            Ok(number::inherited_number_prototype_property(env, key).unwrap_or(Value::Undefined))
        }
        Value::Map(_) | Value::Set(_) => property_value(object, key, env),
        Value::Object(_) => property_value(object, key, env),
        _ => Err(RuntimeError {
            thrown: None,
            message: format!("unsupported property `{key}`"),
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
                function.properties.borrow().get(&key).cloned(),
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
                elements.set_len(to_length(value)?);
                Ok(true)
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
        Value::Array(_) => Ok(false),
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
        Value::Function(function) => function.properties.borrow().get(key).cloned(),
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
        _ => None,
    }
}

pub(super) fn delete_property_key(object: Value, key: &PropertyKey) -> Result<Value, RuntimeError> {
    match key {
        PropertyKey::String(key) => delete_property(object, key),
        PropertyKey::Symbol(symbol) => delete_symbol_property(object, symbol),
    }
}

fn delete_property(object: Value, key: &str) -> Result<Value, RuntimeError> {
    match object {
        Value::Object(object) => Ok(Value::Boolean(object.delete_own_property(key))),
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

fn delete_symbol_property(object: Value, symbol: &ObjectRef) -> Result<Value, RuntimeError> {
    match object {
        Value::Object(object) => Ok(Value::Boolean(object.delete_own_symbol_property(symbol))),
        Value::Map(map) => Ok(Value::Boolean(
            map.object().delete_own_symbol_property(symbol),
        )),
        Value::Set(set) => Ok(Value::Boolean(
            set.object().delete_own_symbol_property(symbol),
        )),
        Value::Function(function) => Ok(Value::Boolean(function_delete_own_symbol_property(
            &function, symbol,
        ))),
        Value::Array(_) => Ok(Value::Boolean(true)),
        _ => Err(RuntimeError {
            thrown: None,
            message: "delete target is not an object".to_owned(),
        }),
    }
}

pub(super) fn enumerable_keys(value: Value) -> Result<Vec<Value>, RuntimeError> {
    let keys = match value {
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
    Ok(keys.into_iter().map(Value::String).collect())
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
