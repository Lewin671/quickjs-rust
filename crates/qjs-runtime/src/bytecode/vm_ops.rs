use qjs_ast::{BinaryOp, UnaryOp};

use crate::{ArrayRef, RuntimeError, Value, operations};

use super::vm::Vm;
use super::vm_props::{enumerable_keys, fast_number_binary, fast_number_unary};

impl Vm<'_> {
    pub(super) fn eval_binary(&mut self, op: BinaryOp) -> Result<Value, RuntimeError> {
        let right = self.pop()?;
        let left = self.pop()?;
        if let Some(value) = fast_number_binary(&left, op, &right) {
            return Ok(value);
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

    pub(super) fn enumerate_keys(&mut self) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let keys = enumerable_keys(value, &self.globals)?;
        self.stack.push(Value::Array(ArrayRef::new(keys)));
        Ok(())
    }
}
