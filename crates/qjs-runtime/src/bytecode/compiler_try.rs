use qjs_ast::{BindingPattern, CatchClause, Stmt, VarKind};

use crate::RuntimeError;

use super::compiler::Compiler;
use super::compiler_lexical::catch_param_annex_b_blocked_names;
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
        self.with_lexical_scope(|compiler| compiler.compile_try_body(block, result_slot))?;
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
            let slots = self.with_lexical_scope(|compiler| {
                let slots = compiler.compile_catch_param(param)?;
                let blocked = catch_param_annex_b_blocked_names(Some(param));
                compiler.with_annex_b_blocked_function_names(&blocked, |compiler| {
                    compiler.compile_try_body(&handler.body, result_slot)
                })?;
                compiler.emit(Op::ExitTry);
                Ok(slots)
            })?;
            return Ok((target, Some(CatchScope::Clear { slots })));
        } else {
            self.emit(Op::Pop);
            self.compile_try_body(&handler.body, result_slot)?;
        }
        self.emit(Op::ExitTry);
        Ok((target, None))
    }

    /// Binds the thrown value on the stack to the catch parameter pattern,
    /// returning the lexical slots to clear when the handler exits.
    fn compile_catch_param(&mut self, param: &BindingPattern) -> Result<Vec<usize>, RuntimeError> {
        if let BindingPattern::Identifier { name, .. } = param {
            let slot = self.declare_lexical_slot(name, true);
            self.emit(Op::StoreLocal(slot));
            return Ok(vec![slot]);
        }
        self.compile_binding_initializer(param, VarKind::Let)?;
        Ok(self.current_lexical_slots_for_names(&param.names()))
    }

    fn compile_finally(&mut self, finalizer: &[Stmt]) -> Result<usize, RuntimeError> {
        let target = self.code.len();
        self.with_lexical_scope(|compiler| {
            for stmt in finalizer {
                compiler.compile_stmt(stmt)?;
                compiler.emit(Op::Pop);
            }
            Ok(())
        })?;
        self.emit(Op::EndFinally);
        Ok(target)
    }
}
