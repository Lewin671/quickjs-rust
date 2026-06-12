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
                let one = BigInt::from(1);
                return Ok(match op {
                    UpdateOp::Increment => Value::BigInt(value + one),
                    UpdateOp::Decrement => Value::BigInt(value - one),
                });
            }
            value => value,
        };
        let mut env = self.current_env();
        let primitive = to_primitive_with_hint(value, PreferredType::Number, &mut env)?;
        let result = match primitive {
            Value::BigInt(value) => {
                let one = BigInt::from(1);
                match op {
                    UpdateOp::Increment => Value::BigInt(value + one),
                    UpdateOp::Decrement => Value::BigInt(value - one),
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
        let keys = enumerable_keys(value, &self.env)?;
        self.stack.push(Value::Array(ArrayRef::new(keys)));
        Ok(())
    }
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
        | (_, Value::Array(_))
        | (_, Value::Function(_))
        | (_, Value::Map(_))
        | (_, Value::Set(_))
        | (_, Value::Object(_)) => None,
        _ => Some(false),
    }
}
