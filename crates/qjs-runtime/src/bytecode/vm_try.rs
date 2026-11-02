//! Abrupt completions unwind protected regions only when their destination exits
//! the region. Each running finalizer owns a separate completion, so nested
//! catches/finalizers and suspension cannot consume or overwrite an outer one.
use crate::{RuntimeError, Value, error_value};

use super::{ir::CatchScope, vm::Vm};

#[derive(Clone)]
struct CodeRegion {
    start: usize,
    end: usize,
}

impl CodeRegion {
    fn contains(&self, target: usize) -> bool {
        self.start <= target && target < self.end
    }
}

#[derive(Clone)]
pub(super) struct TryFrame {
    region: CodeRegion,
    catch: Option<usize>,
    finally: Option<usize>,
    catch_scope: Option<CatchScope>,
    cleanup_slots: Vec<usize>,
    catch_scope_active: bool,
    stack_depth: usize,
    /// Depth of the with-object stack when this try region was entered, so an
    /// exception unwinds any `with` scopes opened inside the protected region.
    with_depth: usize,
    class_depth: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) enum PendingAbrupt {
    Throw(Value),
    Return(Value),
    Jump(usize),
}

#[derive(Clone)]
pub(super) struct FinallyFrame {
    region: CodeRegion,
    pub(super) completion: Option<PendingAbrupt>,
}

impl<'a, F: crate::bytecode::vm::VmFrame<'a>> Vm<'a, '_, F> {
    pub(super) fn enter_try(
        &mut self,
        catch: Option<usize>,
        finally: Option<usize>,
        end: usize,
        catch_scope: Option<CatchScope>,
        cleanup_slots: Vec<usize>,
    ) {
        debug_assert!(end != usize::MAX, "unpatched protected region");
        let start = self.ip;
        let stack_depth = self.stack.len();
        let cold = self.cold_mut();
        let with_depth = cold.with_stack.len();
        let class_depth = cold.class_regions.len();
        cold.try_stack.push(TryFrame {
            region: CodeRegion { start, end },
            catch,
            finally,
            catch_scope,
            cleanup_slots,
            catch_scope_active: false,
            stack_depth,
            with_depth,
            class_depth,
        });
    }

    pub(super) fn exit_try(&mut self) -> Result<(), RuntimeError> {
        if let Some(frame) = self.cold_mut().try_stack.pop() {
            self.cleanup_catch_scope(frame.catch_scope_active, frame.catch_scope)?;
        }
        Ok(())
    }

    pub(super) fn throw_value(&mut self, value: Value) -> Result<(), RuntimeError> {
        // Fresh abrupt state is staged separately from active finalizers;
        // only routing outside a finalizer discards its saved completion.
        if !self.try_stack_is_empty() {
            let (stack_depth, with_depth, class_depth, catch, cleanup_slots) = {
                let frame = self
                    .cold_mut()
                    .try_stack
                    .last_mut()
                    .expect("non-empty try stack must have a final frame");
                let catch = frame.catch.take();
                if catch.is_some() {
                    frame.catch_scope_active = true;
                }
                let cleanup_slots = catch.is_some().then(|| frame.cleanup_slots.clone());
                (
                    frame.stack_depth,
                    frame.with_depth,
                    frame.class_depth,
                    catch,
                    cleanup_slots,
                )
            };
            self.stack.truncate(stack_depth);
            self.restore_class_depth(class_depth);
            if let Some(catch) = catch {
                self.cold_mut().with_stack.truncate(with_depth);
                self.cleanup_slots(
                    cleanup_slots
                        .as_deref()
                        .expect("catch frames must retain their cleanup slots"),
                )?;
                self.stack.push(value);
                self.unwind_finalizers(catch);
                self.ip = catch;
                return Ok(());
            }
        }

        if let Some(frame) = self.cold_mut().try_stack.pop() {
            self.stack.truncate(frame.stack_depth);
            self.cold_mut().with_stack.truncate(frame.with_depth);
            self.restore_class_depth(frame.class_depth);
            self.cleanup_slots(&frame.cleanup_slots)?;
            self.cleanup_catch_scope(frame.catch_scope_active, frame.catch_scope)?;
            if let Some(finally) = frame.finally {
                self.cold_mut().pending_abrupt = Some(PendingAbrupt::Throw(value));
                self.unwind_finalizers(finally);
                self.ip = finally;
            } else {
                self.throw_value(value)?;
            }
            return Ok(());
        }
        self.unwind_finalizers(usize::MAX);
        Err(RuntimeError {
            thrown: Some(Box::new(value.clone())),
            message: format!("throw statement executed: {}", error_value(value)),
        })
    }

    pub(super) fn return_value(&mut self, value: Value) -> Result<Option<Value>, RuntimeError> {
        while let Some(frame) = self
            .cold
            .as_deref_mut()
            .and_then(|cold| cold.try_stack.pop())
        {
            self.stack.truncate(frame.stack_depth);
            self.cold_mut().with_stack.truncate(frame.with_depth);
            self.restore_class_depth(frame.class_depth);
            self.cleanup_slots(&frame.cleanup_slots)?;
            self.cleanup_catch_scope(frame.catch_scope_active, frame.catch_scope)?;
            if let Some(finally) = frame.finally {
                self.cold_mut().pending_abrupt = Some(PendingAbrupt::Return(value));
                self.unwind_finalizers(finally);
                self.ip = finally;
                return Ok(None);
            }
        }
        self.unwind_finalizers(usize::MAX);
        Ok(Some(value))
    }

    /// Resume jumps through every exited region, retaining enclosing regions
    /// whose body/catch still contains the target (e.g. a loop inside a try).
    pub(super) fn abrupt_jump(&mut self, target: usize) -> Result<(), RuntimeError> {
        loop {
            let exits = self
                .cold
                .as_deref()
                .and_then(|cold| cold.try_stack.last())
                .is_some_and(|frame| !frame.region.contains(target));
            if !exits {
                break;
            }
            let frame = self.cold_mut().try_stack.pop().expect("exited region");
            self.cold_mut().with_stack.truncate(frame.with_depth);
            self.restore_class_depth(frame.class_depth);
            self.cleanup_slots(&frame.cleanup_slots)?;
            self.cleanup_catch_scope(frame.catch_scope_active, frame.catch_scope)?;
            if let Some(finally) = frame.finally {
                self.cold_mut().pending_abrupt = Some(PendingAbrupt::Jump(target));
                self.unwind_finalizers(finally);
                self.ip = finally;
                return Ok(());
            }
        }
        self.unwind_finalizers(target);
        self.ip = target;
        Ok(())
    }

    fn unwind_finalizers(&mut self, target: usize) {
        if let Some(cold) = self.cold.as_deref_mut() {
            while cold
                .finally_stack
                .last()
                .is_some_and(|frame| !frame.region.contains(target))
            {
                cold.finally_stack.pop();
            }
        }
    }

    pub(super) fn begin_finally(&mut self, end: usize) {
        debug_assert!(end != usize::MAX, "unpatched finalizer region");
        let completion = self.cold_mut().pending_abrupt.take();
        let start = self.ip;
        self.cold_mut().finally_stack.push(FinallyFrame {
            region: CodeRegion { start, end },
            completion,
        });
    }

    pub(super) fn end_finally(&mut self) -> Result<Option<Value>, RuntimeError> {
        let completion = self
            .cold_mut()
            .finally_stack
            .pop()
            .and_then(|frame| frame.completion);
        match completion {
            Some(PendingAbrupt::Throw(value)) => self.throw_value(value)?,
            Some(PendingAbrupt::Return(value)) => return self.return_value(value),
            Some(PendingAbrupt::Jump(target)) => self.abrupt_jump(target)?,
            None => {}
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
