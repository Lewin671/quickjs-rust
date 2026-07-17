use std::rc::Rc;

use num_bigint::BigInt;
use qjs_ast::{BinaryOp, UnaryOp, UpdateOp};

use crate::{
    ArrayRef, PreferredType, RuntimeError, Value, error, operations, to_number_with_env,
    to_primitive_with_hint,
};

use super::vm::Vm;
use super::vm_props::{enumerable_keys, fast_number_binary, fast_number_unary};

impl Vm<'_> {
    pub(super) fn eval_binary(&mut self, op: BinaryOp) -> Result<Value, RuntimeError> {
        let right = self.pop()?;
        let left = self.pop()?;
        if let Some(value) = fast_number_binary(&left, op, &right) {
            return Ok(value);
        }
        if let Some(value) = fast_primitive_string_binary(&left, op, &right) {
            return Ok(value);
        }
        if matches!(op, BinaryOp::StrictEq | BinaryOp::StrictNe)
            && let Some(equal) = fast_strict_eq(&left, &right)
        {
            return Ok(Value::Boolean(if op == BinaryOp::StrictEq {
                equal
            } else {
                !equal
            }));
        }
        if op == BinaryOp::Instanceof
            && matches!(
                &right,
                Value::Function(function) if function.native.is_some_and(error::is_native_error)
            )
        {
            let mut env = self.current_env();
            let result =
                operations::ordinary_has_instance(left, right, &mut env).map(Value::Boolean);
            self.apply_env(env);
            return result;
        }
        let mut env = self.current_env();
        let result = operations::eval_binary(left, op, right, &mut env);
        self.apply_env(env);
        result
    }

    pub(super) fn eval_unary(&mut self, op: UnaryOp) -> Result<Value, RuntimeError> {
        let value = self.pop()?;
        if let Some(value) = fast_number_unary(op, &value) {
            return Ok(value);
        }
        let mut env = self.current_env();
        let result = operations::eval_unary(op, value, &mut env);
        self.apply_env(env);
        result
    }

    pub(super) fn eval_to_numeric(&mut self) -> Result<Value, RuntimeError> {
        let value = self.pop()?;
        if matches!(value, Value::Number(_) | Value::BigInt(_)) {
            return Ok(value);
        }
        let mut env = self.current_env();
        let primitive = to_primitive_with_hint(value, PreferredType::Number, &mut env)?;
        let result = match primitive {
            Value::BigInt(_) => primitive,
            value => Value::Number(to_number_with_env(value, &mut env)?),
        };
        self.apply_env(env);
        Ok(result)
    }

    pub(super) fn eval_update(&mut self, op: UpdateOp) -> Result<Value, RuntimeError> {
        let value = self.pop()?;
        let value = match value {
            Value::Number(number) => {
                return Ok(match op {
                    UpdateOp::Increment => Value::Number(number + 1.0),
                    UpdateOp::Decrement => Value::Number(number - 1.0),
                });
            }
            Value::BigInt(value) => {
                let value = Rc::unwrap_or_clone(value);
                let one = BigInt::from(1);
                return Ok(match op {
                    UpdateOp::Increment => Value::bigint(value + one),
                    UpdateOp::Decrement => Value::bigint(value - one),
                });
            }
            value => value,
        };
        let mut env = self.current_env();
        let primitive = to_primitive_with_hint(value, PreferredType::Number, &mut env)?;
        let result = match primitive {
            Value::BigInt(value) => {
                let value = Rc::unwrap_or_clone(value);
                let one = BigInt::from(1);
                match op {
                    UpdateOp::Increment => Value::bigint(value + one),
                    UpdateOp::Decrement => Value::bigint(value - one),
                }
            }
            value => {
                let number = to_number_with_env(value, &mut env)?;
                match op {
                    UpdateOp::Increment => Value::Number(number + 1.0),
                    UpdateOp::Decrement => Value::Number(number - 1.0),
                }
            }
        };
        self.apply_env(env);
        Ok(result)
    }

    pub(super) fn enumerate_keys(&mut self) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let keys = enumerable_keys(value, &mut self.env)?;
        self.stack.push(Value::Array(ArrayRef::new(keys)));
        Ok(())
    }

    pub(super) fn for_in_key_is_enumerable(&mut self) -> Result<(), RuntimeError> {
        let key = self.pop()?;
        let target = self.pop()?;
        let Value::String(key) = key else {
            return Err(RuntimeError {
                thrown: None,
                message: "for-in key must be a string".to_owned(),
            });
        };
        let enumerable = match target {
            // A Proxy re-checks enumerability through its own (and prototype
            // chain's) traps; an absent own descriptor walks to the prototype.
            Value::Proxy(_) => self.proxy_key_is_enumerable(target, &key)?,
            Value::Function(function) => function
                .chain_property_with_env(&key, &self.env)
                .is_some_and(|property| property.enumerable),
            value => crate::property::own_or_inherited_descriptor(value, &key)
                .is_some_and(|property| property.enumerable),
        };
        self.stack.push(Value::Boolean(enumerable));
        Ok(())
    }

    /// Walks a Proxy's prototype chain looking for an own descriptor of `key`,
    /// consulting each exotic Proxy's traps; reports the first match's
    /// enumerability (a key that has vanished mid-iteration is not enumerable).
    fn proxy_key_is_enumerable(&mut self, target: Value, key: &str) -> Result<bool, RuntimeError> {
        let property_key = crate::PropertyKey::String(key.to_owned());
        let mut current = target;
        loop {
            match &current {
                Value::Proxy(proxy) => {
                    let descriptor = crate::proxy::proxy_get_own_property_descriptor(
                        proxy.clone(),
                        &property_key,
                        &mut self.env,
                        |t, env| crate::object::own_property_descriptor_key(t, &property_key, env),
                    )?;
                    if let Some(property) = descriptor {
                        return Ok(property.enumerable);
                    }
                    current = crate::proxy::proxy_get_prototype_of(proxy.clone(), &mut self.env)?;
                }
                Value::Null | Value::Undefined => return Ok(false),
                value => {
                    return Ok(
                        crate::property::own_or_inherited_descriptor(value.clone(), key)
                            .is_some_and(|property| property.enumerable),
                    );
                }
            }
        }
    }
}

fn fast_primitive_string_binary(left: &Value, op: BinaryOp, right: &Value) -> Option<Value> {
    if let (Value::String(left), Value::String(right)) = (left, right) {
        let value = match op {
            BinaryOp::Eq | BinaryOp::StrictEq => {
                Value::Boolean(crate::string::string_utf16_eq(left, right))
            }
            BinaryOp::Ne | BinaryOp::StrictNe => {
                Value::Boolean(!crate::string::string_utf16_eq(left, right))
            }
            BinaryOp::Lt | BinaryOp::Le | BinaryOp::Gt | BinaryOp::Ge => {
                let ordering = crate::string::string_code_units(left)
                    .cmp(&crate::string::string_code_units(right));
                Value::Boolean(match op {
                    BinaryOp::Lt => ordering.is_lt(),
                    BinaryOp::Le => ordering.is_le(),
                    BinaryOp::Gt => ordering.is_gt(),
                    BinaryOp::Ge => ordering.is_ge(),
                    _ => unreachable!("relational operator matched above"),
                })
            }
            BinaryOp::Add => {
                let mut result = String::with_capacity(left.len().checked_add(right.len())?);
                result.push_str(left);
                result.push_str(right);
                Value::String(result.into())
            }
            _ => return None,
        };
        return Some(value);
    }
    if op != BinaryOp::Add
        || (!matches!(left, Value::String(_)) && !matches!(right, Value::String(_)))
    {
        return None;
    }
    let left = primitive_js_string(left)?;
    let right = primitive_js_string(right)?;
    let mut result = String::with_capacity(left.len().checked_add(right.len())?);
    result.push_str(&left);
    result.push_str(&right);
    Some(Value::String(result.into()))
}

fn primitive_js_string(value: &Value) -> Option<String> {
    Some(match value {
        Value::Number(number) => crate::number::number_to_js_string(*number),
        Value::BigInt(value) => value.to_string(),
        Value::String(value) => value.to_string(),
        Value::Boolean(true) => "true".to_owned(),
        Value::Boolean(false) => "false".to_owned(),
        Value::Null => "null".to_owned(),
        Value::Undefined => "undefined".to_owned(),
        _ => return None,
    })
}

fn fast_strict_eq(left: &Value, right: &Value) -> Option<bool> {
    match (left, right) {
        (Value::String(left), Value::String(right)) => {
            Some(crate::string::string_utf16_eq(left, right))
        }
        (Value::Boolean(left), Value::Boolean(right)) => Some(left == right),
        (Value::Null, Value::Null) | (Value::Undefined, Value::Undefined) => Some(true),
        (Value::BigInt(left), Value::BigInt(right)) => Some(left == right),
        (Value::Number(_), Value::Number(_)) => None,
        (Value::Array(_), _)
        | (Value::Function(_), _)
        | (Value::Map(_), _)
        | (Value::Set(_), _)
        | (Value::Object(_), _)
        | (Value::Proxy(_), _)
        | (_, Value::Array(_))
        | (_, Value::Function(_))
        | (_, Value::Map(_))
        | (_, Value::Set(_))
        | (_, Value::Object(_))
        | (_, Value::Proxy(_)) => None,
        _ => Some(false),
    }
}
