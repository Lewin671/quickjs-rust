use qjs_ast::BinaryOp;

use crate::{ArrayRef, GLOBAL_THIS_BINDING, RuntimeError, Value, operations};

use super::util::stack_underflow;
use super::vm::Vm;
use super::vm_props::{enumerable_keys, fast_number_binary};

impl Vm<'_> {
    pub(super) fn pop(&mut self) -> Result<Value, RuntimeError> {
        self.stack.pop().ok_or_else(stack_underflow)
    }

    pub(super) fn load_local(&self, slot: usize) -> Result<Value, RuntimeError> {
        match self.locals.get(slot) {
            Some(Some(value)) => Ok(value.clone()),
            Some(None) => Err(RuntimeError {
                thrown: None,
                message: format!(
                    "ReferenceError: undefined identifier `{}`",
                    self.bytecode.locals[slot].name
                ),
            }),
            None => Err(RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            }),
        }
    }

    pub(super) fn load_local_or_undefined(&self, slot: usize) -> Result<Value, RuntimeError> {
        match self.locals.get(slot) {
            Some(Some(value)) => Ok(value.clone()),
            Some(None) => Ok(Value::Undefined),
            None => Err(RuntimeError {
                thrown: None,
                message: "bytecode local index out of bounds".to_owned(),
            }),
        }
    }

    pub(super) fn store_local(&mut self, slot: usize, value: Value) -> Result<(), RuntimeError> {
        let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
            thrown: None,
            message: "bytecode local index out of bounds".to_owned(),
        })?;
        *local = Some(value.clone());
        if self.sync_var_to_global_object
            && self
                .bytecode
                .locals
                .get(slot)
                .is_some_and(|local| local.hoisted)
            && let Some(Value::Object(global_object)) = self.globals.get(GLOBAL_THIS_BINDING)
            && let Some(name) = self
                .bytecode
                .locals
                .get(slot)
                .map(|local| local.name.clone())
        {
            global_object.set(name, value);
        }
        Ok(())
    }

    pub(super) fn clear_local(&mut self, slot: usize) -> Result<(), RuntimeError> {
        let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
            thrown: None,
            message: "bytecode local index out of bounds".to_owned(),
        })?;
        *local = None;
        Ok(())
    }

    pub(super) fn eval_binary(&mut self, op: BinaryOp) -> Result<Value, RuntimeError> {
        let right = self.pop()?;
        let left = self.pop()?;
        if let Some(value) = fast_number_binary(&left, op, &right) {
            return Ok(value);
        }
        operations::eval_binary(left, op, right, &mut self.globals)
    }

    pub(super) fn enumerate_keys(&mut self) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        self.stack.push(Value::Array(ArrayRef::new(enumerable_keys(
            value,
            &self.globals,
        )?)));
        Ok(())
    }
}
