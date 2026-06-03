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
        operations::eval_binary(left, op, right, &mut self.globals)
    }

    pub(super) fn eval_unary(&mut self, op: UnaryOp) -> Result<Value, RuntimeError> {
        let value = self.pop()?;
        if let Some(value) = fast_number_unary(op, &value) {
            return Ok(value);
        }
        operations::eval_unary(op, value)
    }

    pub(super) fn enumerate_keys(&mut self) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        self.stack
            .push(Value::Array(ArrayRef::new(enumerable_keys(value)?)));
        Ok(())
    }
}
