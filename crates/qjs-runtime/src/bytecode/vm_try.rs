use crate::{RuntimeError, Value, error_value};

use super::{
    ir::CatchScope,
    vm::{Slot, Vm},
};

#[derive(Clone)]
pub(super) struct TryFrame {
    catch: Option<usize>,
    finally: Option<usize>,
    catch_scope: Option<CatchScope>,
    catch_scope_active: bool,
    stack_depth: usize,
}

impl Vm<'_> {
    pub(super) fn enter_try(
        &mut self,
        catch: Option<usize>,
        finally: Option<usize>,
        catch_scope: Option<CatchScope>,
    ) {
        self.try_stack.push(TryFrame {
            catch,
            finally,
            catch_scope,
            catch_scope_active: false,
            stack_depth: self.stack.len(),
        });
    }

    pub(super) fn exit_try(&mut self) -> Result<(), RuntimeError> {
        if let Some(frame) = self.try_stack.pop() {
            self.cleanup_catch_scope(frame.catch_scope_active, frame.catch_scope)?;
        }
        Ok(())
    }

    pub(super) fn throw_value(&mut self, value: Value) -> Result<(), RuntimeError> {
        self.pending_throw = None;
        self.pending_return = None;
        self.pending_jump = None;
        if let Some(frame) = self.try_stack.last_mut() {
            self.stack.truncate(frame.stack_depth);
            if let Some(catch) = frame.catch.take() {
                frame.catch_scope_active = true;
                self.stack.push(value);
                self.ip = catch;
                return Ok(());
            }
        }

        if let Some(frame) = self.try_stack.pop() {
            self.stack.truncate(frame.stack_depth);
            self.cleanup_catch_scope(frame.catch_scope_active, frame.catch_scope)?;
            if let Some(finally) = frame.finally {
                self.pending_throw = Some(value);
                self.pending_return = None;
                self.pending_jump = None;
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
        self.pending_throw = None;
        self.pending_return = None;
        self.pending_jump = None;
        while let Some(frame) = self.try_stack.pop() {
            self.stack.truncate(frame.stack_depth);
            self.cleanup_catch_scope(frame.catch_scope_active, frame.catch_scope)?;
            if let Some(finally) = frame.finally {
                self.pending_return = Some(value);
                self.pending_throw = None;
                self.pending_jump = None;
                self.ip = finally;
                return Ok(None);
            }
        }
        Ok(Some(value))
    }

    pub(super) fn jump_abrupt(&mut self, target: usize) -> Result<(), RuntimeError> {
        self.pending_jump = Some(target);
        self.pending_throw = None;
        self.pending_return = None;
        while let Some(frame) = self.try_stack.pop() {
            self.stack.truncate(frame.stack_depth);
            self.cleanup_catch_scope(frame.catch_scope_active, frame.catch_scope)?;
            if let Some(finally) = frame.finally {
                self.ip = finally;
                return Ok(());
            }
        }
        self.pending_jump = None;
        self.ip = target;
        Ok(())
    }

    pub(super) fn end_finally(&mut self) -> Result<Option<Value>, RuntimeError> {
        if let Some(value) = self.pending_throw.take() {
            self.throw_value(value)?;
        } else if let Some(value) = self.pending_return.take() {
            return self.return_value(value);
        } else if let Some(target) = self.pending_jump.take() {
            self.jump_abrupt(target)?;
        }
        Ok(None)
    }

    fn cleanup_catch_scope(
        &mut self,
        active: bool,
        scope: Option<CatchScope>,
    ) -> Result<(), RuntimeError> {
        if !active {
            return Ok(());
        }
        match scope {
            Some(CatchScope::Clear { slot }) => self.set_local_slot(slot, None),
            Some(CatchScope::Restore { slot, saved_slot }) => {
                let saved = self
                    .locals
                    .get(saved_slot)
                    .cloned()
                    .ok_or_else(|| RuntimeError {
                        thrown: None,
                        message: "bytecode local index out of bounds".to_owned(),
                    })?;
                self.set_local_slot(slot, saved)
            }
            None => Ok(()),
        }
    }

    fn set_local_slot(&mut self, slot: usize, value: Slot) -> Result<(), RuntimeError> {
        let local = self.locals.get_mut(slot).ok_or_else(|| RuntimeError {
            thrown: None,
            message: "bytecode local index out of bounds".to_owned(),
        })?;
        *local = value;
        Ok(())
    }
}
