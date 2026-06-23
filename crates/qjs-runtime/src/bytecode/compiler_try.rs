use qjs_ast::{BindingPattern, CatchClause, Stmt, VarKind};

use crate::RuntimeError;

use super::compiler::Compiler;
use super::compiler_lexical::{annex_b_blocked_names, catch_param_annex_b_blocked_names};
use super::ir::{CatchScope, Op};
use super::util::stmt_updates_statement_list_completion;

/// Whether a block directly declares a `using`/`await using` resource (so its
/// scope needs an implicit disposal try/finally).
pub(super) fn block_has_using(body: &[Stmt]) -> bool {
    body.iter().any(|stmt| {
        matches!(
            stmt,
            Stmt::VarDecl {
                kind: VarKind::Using | VarKind::AwaitUsing,
                ..
            }
        )
    })
}

pub(super) fn block_has_await_using(body: &[Stmt]) -> bool {
    body.iter().any(|stmt| {
        matches!(
            stmt,
            Stmt::VarDecl {
                kind: VarKind::AwaitUsing,
                ..
            }
        )
    })
}

impl Compiler {
    /// Compiles a block body in its own lexical scope, leaving the block's
    /// completion value on the stack. Shared by plain and disposable blocks.
    pub(super) fn compile_block_body(&mut self, body: &[Stmt]) -> Result<(), RuntimeError> {
        self.with_lexical_scope(|compiler| {
            let blocked = annex_b_blocked_names(body, compiler.strict);
            compiler.with_annex_b_blocked_function_names(&blocked, |compiler| {
                compiler.predeclare_current_scope_lexicals(body);
                if body.is_empty() {
                    compiler.emit_load_undefined();
                    return Ok(());
                }
                compiler.compile_hoisted_function_decls(body)?;
                let result_slot = compiler.temp_local("block_result");
                compiler.emit_load_undefined();
                compiler.emit(Op::StoreLocal(result_slot));
                for stmt in body {
                    compiler.compile_stmt(stmt)?;
                    if stmt_updates_statement_list_completion(stmt) {
                        compiler.store_statement_list_completion(result_slot);
                    } else {
                        compiler.emit(Op::Pop);
                    }
                }
                for slot in compiler.current_lexical_slots_for_names(&blocked) {
                    compiler.emit(Op::ClearLocal(slot));
                }
                compiler.emit(Op::LoadLocal(result_slot));
                Ok(())
            })
        })
    }

    /// Compiles a block that declares `using` resources: opens a disposal scope,
    /// runs the body inside an implicit try whose finally disposes the
    /// resources LIFO on every completion path.
    pub(super) fn compile_disposable_block(&mut self, body: &[Stmt]) -> Result<(), RuntimeError> {
        self.emit(Op::EnterDisposableScope);
        let result_slot = self.temp_local("using_result");
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));

        let loop_depth = self.loop_stack_depth();
        self.push_try_result_slot(result_slot, loop_depth, false);

        let enter = self.emit(Op::EnterTry {
            catch: None,
            finally: None,
            catch_scope: None,
            cleanup_slots: Vec::new(),
        });
        self.disposable_scope_depth += 1;
        let body_result = self.compile_block_body(body);
        self.disposable_scope_depth -= 1;
        body_result?;
        self.emit(Op::StoreLocal(result_slot));
        self.emit(Op::ExitTry);
        let normal_jump = self.emit(Op::Jump(usize::MAX));

        self.pop_try_result_slot();

        let finally_target = self.compile_dispose_finally(block_has_await_using(body));
        if let Op::EnterTry { finally, .. } = &mut self.code[enter] {
            *finally = Some(finally_target);
        }
        self.patch_jump(normal_jump, finally_target);
        self.emit(Op::LoadLocal(result_slot));
        Ok(())
    }

    /// Compiles a statement list (a function/generator body — no new lexical
    /// scope) wrapped in a disposal try/finally so its top-level `using`
    /// resources are disposed when the body exits on any path.
    pub(super) fn compile_statements_with_disposal(
        &mut self,
        body: &[Stmt],
    ) -> Result<(), RuntimeError> {
        self.emit(Op::EnterDisposableScope);
        let result_slot = self.temp_local("body_using_result");
        let loop_depth = self.loop_stack_depth();
        self.push_try_result_slot(result_slot, loop_depth, false);
        let enter = self.emit(Op::EnterTry {
            catch: None,
            finally: None,
            catch_scope: None,
            cleanup_slots: Vec::new(),
        });
        self.disposable_scope_depth += 1;
        let mut body_result = Ok(());
        for stmt in body {
            if let Err(error) = self.compile_stmt(stmt) {
                body_result = Err(error);
                break;
            }
        }
        self.disposable_scope_depth -= 1;
        body_result?;
        self.emit(Op::ExitTry);
        let normal_jump = self.emit(Op::Jump(usize::MAX));
        self.pop_try_result_slot();
        let finally_target = self.compile_dispose_finally(block_has_await_using(body));
        if let Op::EnterTry { finally, .. } = &mut self.code[enter] {
            *finally = Some(finally_target);
        }
        self.patch_jump(normal_jump, finally_target);
        Ok(())
    }

    /// Emits the disposal finally body and returns its entry IP.
    pub(super) fn compile_dispose_finally(&mut self, await_async: bool) -> usize {
        let finally_result_slot = self.temp_local("dispose_result");
        let loop_depth = self.loop_stack_depth();
        self.push_try_result_slot(finally_result_slot, loop_depth, true);
        let target = self.code.len();
        self.emit(Op::DisposeScope { await_async });
        if await_async {
            let skip_await = self.emit(Op::JumpIfFalse(usize::MAX));
            self.emit(Op::Pop);
            self.emit(Op::Await);
            let done = self.emit(Op::Jump(usize::MAX));
            let skip_target = self.code.len();
            self.emit(Op::Pop);
            self.emit(Op::Pop);
            let done_target = self.code.len();
            self.patch_jump(skip_await, skip_target);
            self.patch_jump(done, done_target);
        }
        self.pop_try_result_slot();
        self.emit(Op::EndFinally);
        target
    }

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
            cleanup_slots: Vec::new(),
        });
        let cleanup_start = self.locals.len();
        let mut cleanup_slots = Vec::new();
        if block_has_using(block) {
            self.compile_disposable_block(block)?;
            self.emit(Op::StoreLocal(result_slot));
        } else {
            self.with_lexical_scope(|compiler| compiler.compile_try_body(block, result_slot))?;
            cleanup_slots = self.lexical_cleanup_slots_since(cleanup_start);
        }
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
            cleanup_slots: slots,
        } = &mut self.code[enter]
        {
            *catch = catch_target;
            *finally = finally_target;
            *scope = catch_scope;
            *slots = cleanup_slots;
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

    fn lexical_cleanup_slots_since(&self, start: usize) -> Vec<usize> {
        self.locals
            .iter()
            .enumerate()
            .skip(start)
            .filter(|(_, local)| {
                !local.name.starts_with("\0\0")
                    && !local.hoisted
                    && !local.parameter
                    && !local.sloppy_global_fallback
            })
            .map(|(slot, _)| slot)
            .collect()
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
                    compiler.compile_block_body(&handler.body)?;
                    compiler.emit(Op::StoreLocal(result_slot));
                    Ok(())
                })?;
                compiler.emit(Op::ExitTry);
                Ok(slots)
            })?;
            return Ok((target, Some(CatchScope::Clear { slots })));
        } else {
            self.emit(Op::Pop);
            self.compile_block_body(&handler.body)?;
            self.emit(Op::StoreLocal(result_slot));
        }
        self.emit(Op::ExitTry);
        Ok((target, None))
    }

    /// Binds the thrown value on the stack to the catch parameter pattern,
    /// returning the lexical slots to clear when the handler exits.
    fn compile_catch_param(&mut self, param: &BindingPattern) -> Result<Vec<usize>, RuntimeError> {
        if let BindingPattern::Identifier { name, .. } = param {
            let slot = self.declare_lexical_slot(name, true);
            self.locals[slot].catch_binding = true;
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
