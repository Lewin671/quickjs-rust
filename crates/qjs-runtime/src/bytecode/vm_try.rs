use crate::{RuntimeError, Value, error_value};

use super::vm::Vm;

#[derive(Clone)]
pub(super) struct TryFrame {
    catch: Option<usize>,
    finally: Option<usize>,
    stack_depth: usize,
}

impl Vm<'_> {
    pub(super) fn enter_try(&mut self, catch: Option<usize>, finally: Option<usize>) {
        self.try_stack.push(TryFrame {
            catch,
            finally,
            stack_depth: self.stack.len(),
        });
    }

    pub(super) fn exit_try(&mut self) {
        self.try_stack.pop();
    }

    pub(super) fn throw_value(&mut self, value: Value) -> Result<(), RuntimeError> {
        if let Some(frame) = self.try_stack.last_mut() {
            self.stack.truncate(frame.stack_depth);
            if let Some(catch) = frame.catch.take() {
                self.stack.push(value);
                self.ip = catch;
                return Ok(());
            }
        }

        if let Some(frame) = self.try_stack.pop() {
            self.stack.truncate(frame.stack_depth);
            if let Some(finally) = frame.finally {
                self.pending_throw = Some(value);
                self.ip = finally;
            } else {
                self.throw_value(value)?;
            }
            return Ok(());
        }
        Err(RuntimeError {
            thrown: Some(Box::new(value.clone())),
            message: format!("throw statement executed: {}", error_value(value)),
        })
    }

    pub(super) fn return_value(&mut self, value: Value) -> Result<Option<Value>, RuntimeError> {
        while let Some(frame) = self.try_stack.pop() {
            self.stack.truncate(frame.stack_depth);
            if let Some(finally) = frame.finally {
                self.pending_return = Some(value);
                self.ip = finally;
                return Ok(None);
            }
        }
        Ok(Some(value))
    }

    pub(super) fn end_finally(&mut self) -> Result<Option<Value>, RuntimeError> {
        if let Some(value) = self.pending_throw.take() {
            self.throw_value(value)?;
        } else if let Some(value) = self.pending_return.take() {
            return self.return_value(value);
        }
        Ok(None)
    }
}
