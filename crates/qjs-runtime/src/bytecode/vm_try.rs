use crate::{RuntimeError, Value, error_value};

use super::{ir::CatchScope, vm::Vm};

#[derive(Clone)]
pub(super) struct TryFrame {
    catch: Option<usize>,
    finally: Option<usize>,
    catch_scope: Option<CatchScope>,
    cleanup_slots: Vec<usize>,
    catch_scope_active: bool,
    stack_depth: usize,
    /// Depth of the with-object stack when this try region was entered, so an
    /// exception unwinds any `with` scopes opened inside the protected region.
    with_depth: usize,
}

impl Vm<'_> {
    pub(super) fn enter_try(
        &mut self,
        catch: Option<usize>,
        finally: Option<usize>,
        catch_scope: Option<CatchScope>,
        cleanup_slots: Vec<usize>,
    ) {
        self.try_stack.push(TryFrame {
            catch,
            finally,
            catch_scope,
            cleanup_slots,
            catch_scope_active: false,
            stack_depth: self.stack.len(),
            with_depth: self.with_stack.len(),
        });
    }

    pub(super) fn exit_try(&mut self) -> Result<(), RuntimeError> {
        if let Some(frame) = self.try_stack.pop() {
            self.cleanup_catch_scope(frame.catch_scope_active, frame.catch_scope)?;
        }
        Ok(())
    }

    pub(super) fn throw_value(&mut self, value: Value) -> Result<(), RuntimeError> {
        // A fresh throw supersedes any abrupt completion deferred by an
        // in-flight finally (e.g. an inner `finally { throw }` overriding the
        // exception/break/return it interrupted), so the stale pending
        // completion must not be re-raised later by an enclosing EndFinally.
        self.pending_throw = None;
        self.pending_return = None;
        self.pending_jump = None;
        if let Some(frame) = self.try_stack.last_mut() {
            self.stack.truncate(frame.stack_depth);
            let with_depth = frame.with_depth;
            if let Some(catch) = frame.catch.take() {
                frame.catch_scope_active = true;
                let cleanup_slots = frame.cleanup_slots.clone();
                self.with_stack.truncate(with_depth);
                self.cleanup_slots(&cleanup_slots)?;
                self.stack.push(value);
                self.ip = catch;
                return Ok(());
            }
        }

        if let Some(frame) = self.try_stack.pop() {
            self.stack.truncate(frame.stack_depth);
            self.with_stack.truncate(frame.with_depth);
            self.cleanup_slots(&frame.cleanup_slots)?;
            self.cleanup_catch_scope(frame.catch_scope_active, frame.catch_scope)?;
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
            self.with_stack.truncate(frame.with_depth);
            self.cleanup_slots(&frame.cleanup_slots)?;
            self.cleanup_catch_scope(frame.catch_scope_active, frame.catch_scope)?;
            if let Some(finally) = frame.finally {
                self.pending_return = Some(value);
                self.ip = finally;
                return Ok(None);
            }
        }
        Ok(Some(value))
    }

    /// Routes a break/continue jump through finally blocks. Pops the current
    /// try frame and, if it has a finally clause, defers the jump until after
    /// the finally block executes. Otherwise jumps directly.
    /// Unlike throw/return, the operand stack is not truncated because the
    /// break/continue target expects its completion value on top.
    pub(super) fn abrupt_jump(&mut self, target: usize) -> Result<(), RuntimeError> {
        if let Some(frame) = self.try_stack.pop() {
            self.cleanup_slots(&frame.cleanup_slots)?;
            self.cleanup_catch_scope(frame.catch_scope_active, frame.catch_scope)?;
            if let Some(finally) = frame.finally {
                self.pending_jump = Some(target);
                self.ip = finally;
                return Ok(());
            }
        }
        self.ip = target;
        Ok(())
    }

    pub(super) fn end_finally(&mut self) -> Result<Option<Value>, RuntimeError> {
        if let Some(value) = self.pending_throw.take() {
            self.throw_value(value)?;
        } else if let Some(value) = self.pending_return.take() {
            return self.return_value(value);
        } else if let Some(target) = self.pending_jump.take() {
            self.ip = target;
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
            Some(CatchScope::Clear { slots }) => {
                for slot in slots {
                    self.clear_local(slot)?;
                }
                Ok(())
            }
            None => Ok(()),
        }
    }

    fn cleanup_slots(&mut self, slots: &[usize]) -> Result<(), RuntimeError> {
        for slot in slots {
            self.clear_local(*slot)?;
        }
        Ok(())
    }
}
