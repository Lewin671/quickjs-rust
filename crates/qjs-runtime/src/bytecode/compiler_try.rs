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

        let loop_depth = self.loop_stack_depth();
        self.push_try_result_slot(result_slot, loop_depth, false);

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

        self.pop_try_result_slot();

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
        // Reset the try result slot at catch entry: the catch clause starts a
        // fresh completion sequence, so values from the try body before the
        // throw must not leak through an abrupt exit (break/continue) from the
        // catch block. Per spec, UpdateEmpty on the catch clause's break with
        // empty value produces undefined, matching this reset.
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
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
        // The finally block needs its own result slot to track the statement
        // list completion value. When a break/continue exits from a finally
        // block, this slot provides the UpdateEmpty'd completion value.
        let finally_result_slot = self.temp_local("finally_result");

        let loop_depth = self.loop_stack_depth();
        self.push_try_result_slot(finally_result_slot, loop_depth, true);

        let target = self.code.len();
        // Initialize the finally result slot at the start of the finally block
        // (inside the target) so it's reset regardless of how finally is entered
        // (normal flow, exception, or abrupt completion from try/catch).
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(finally_result_slot));
        self.with_lexical_scope(|compiler| {
            for stmt in finalizer {
                compiler.compile_stmt(stmt)?;
                compiler.emit(Op::StoreLocal(finally_result_slot));
            }
            Ok(())
        })?;

        self.pop_try_result_slot();

        self.emit(Op::EndFinally);
        Ok(target)
    }
}
