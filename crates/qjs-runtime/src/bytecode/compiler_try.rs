use qjs_ast::{CatchClause, Stmt};

use crate::RuntimeError;

use super::compiler::Compiler;
use super::ir::{CatchScope, Op};

impl Compiler {
    pub(super) fn compile_try(
        &mut self,
        block: &[Stmt],
        handler: Option<&CatchClause>,
        finalizer: Option<&[Stmt]>,
    ) -> Result<(), RuntimeError> {
        let result_slot = self.temp_local("try_result");
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));

        let enter = self.emit(Op::EnterTry {
            catch: None,
            finally: None,
            catch_scope: None,
        });
        self.compile_try_body(block, result_slot)?;
        self.emit(Op::ExitTry);

        let normal_jump = self.emit(Op::Jump(usize::MAX));
        let (catch_target, catch_scope) = if let Some(handler) = handler {
            let (target, scope) = self.compile_catch(handler, result_slot)?;
            (Some(target), scope)
        } else {
            (None, None)
        };
        let finally_target = if let Some(finalizer) = finalizer {
            Some(self.compile_finally(finalizer)?)
        } else {
            None
        };
        let after = self.code.len();

        if let Op::EnterTry {
            catch,
            finally,
            catch_scope: scope,
        } = &mut self.code[enter]
        {
            *catch = catch_target;
            *finally = finally_target;
            *scope = catch_scope;
        }
        self.patch_jump(normal_jump, finally_target.unwrap_or(after));
        self.emit(Op::LoadLocal(result_slot));
        Ok(())
    }

    fn compile_try_body(&mut self, body: &[Stmt], result_slot: usize) -> Result<(), RuntimeError> {
        for stmt in body {
            self.compile_stmt(stmt)?;
            self.emit(Op::StoreLocal(result_slot));
        }
        Ok(())
    }

    fn compile_catch(
        &mut self,
        handler: &CatchClause,
        result_slot: usize,
    ) -> Result<(usize, Option<CatchScope>), RuntimeError> {
        let target = self.code.len();
        if let Some(param) = &handler.param {
            if self.strict && matches!(param.as_str(), "eval" | "arguments") {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("invalid strict catch binding `{param}`"),
                });
            }
            let existing_slot = self.local_slots.get(param).copied();
            let saved_slot = existing_slot.map(|slot| {
                let saved_slot = self.temp_local("catch_saved");
                self.emit(Op::LoadLocalOrUndefined(slot));
                self.emit(Op::StoreLocal(saved_slot));
                saved_slot
            });
            let slot = self.local_slot(param, false);
            self.emit(Op::StoreLocal(slot));
            self.compile_try_body(&handler.body, result_slot)?;
            self.emit(Op::ExitTry);
            let catch_scope = if let Some(saved_slot) = saved_slot {
                CatchScope::Restore { slot, saved_slot }
            } else {
                CatchScope::Clear { slot }
            };
            return Ok((target, Some(catch_scope)));
        } else {
            self.emit(Op::Pop);
            self.compile_try_body(&handler.body, result_slot)?;
        }
        self.emit(Op::ExitTry);
        Ok((target, None))
    }

    fn compile_finally(&mut self, finalizer: &[Stmt]) -> Result<usize, RuntimeError> {
        let target = self.code.len();
        let result_slot = self.current_result_slot();
        if let Some(result_slot) = result_slot {
            self.emit_load_undefined();
            self.emit(Op::StoreLocal(result_slot));
        }
        for stmt in finalizer {
            self.compile_stmt(stmt)?;
            if let Some(result_slot) = result_slot {
                self.emit(Op::StoreLocal(result_slot));
            } else {
                self.emit(Op::Pop);
            }
        }
        self.emit(Op::EndFinally);
        Ok(target)
    }
}
