use qjs_ast::BinaryOp;

use crate::{ArrayRef, RuntimeError, Value, operations};

use super::vm::Vm;
use super::vm_props::{enumerable_keys, fast_number_binary};

impl Vm<'_> {
    pub(super) fn eval_binary(&mut self, op: BinaryOp) -> Result<(), RuntimeError> {
        let right = self.pop()?;
        let left = self.pop()?;
        if let Some(value) = fast_number_binary(&left, op, &right) {
            self.stack.push(value);
            return Ok(());
        }
        self.stack
            .push(operations::eval_binary(left, op, right, &self.globals)?);
        Ok(())
    }

    pub(super) fn enumerate_keys(&mut self) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        self.stack
            .push(Value::Array(ArrayRef::new(enumerable_keys(value)?)));
        Ok(())
    }
}
