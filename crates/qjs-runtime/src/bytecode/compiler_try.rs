use qjs_ast::{BindingPattern, CatchClause, Stmt, VarKind};

use crate::RuntimeError;

use super::compiler::Compiler;
use super::compiler_lexical::{annex_b_blocked_names, catch_param_annex_b_blocked_names};
use super::ir::{CatchScope, Op};
use super::util::{stmts_contain_break_or_continue, stmts_write_completion_first};

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
    pub(super) fn finish_try(&mut self, enter: usize, region_end: usize) {
        if let Op::EnterTry { end, .. } = &mut self.code[enter] {
            *end = region_end;
        }
    }

    /// Compiles a block body in its own lexical scope. A block shares the
    /// enclosing completion register, so an empty block (or one of only
    /// declarations) leaves the previous statement's value in place.
    pub(super) fn compile_block_body(&mut self, body: &[Stmt]) -> Result<(), RuntimeError> {
        self.with_lexical_scope(|compiler| {
            let blocked = annex_b_blocked_names(body, compiler.strict);
            compiler.with_annex_b_blocked_function_names(&blocked, |compiler| {
                compiler.instantiate_block_declarations(body, &blocked)?;
                for stmt in body {
                    compiler.compile_stmt(stmt)?;
                }
                for slot in compiler.current_lexical_slots_for_names(&blocked) {
                    compiler.emit(Op::ClearLocal(slot));
                }
                Ok(())
            })
        })
    }

    /// Each entry creates fresh lexical locations, even if an abrupt jump
    /// skipped the previous instance's normal-exit cleanup. Emit this at the
    /// block entry, before any reference/closure or function initialization;
    /// clearing on continue itself would incorrectly precede finally handlers.
    fn instantiate_block_declarations(
        &mut self,
        body: &[Stmt],
        names: &[String],
    ) -> Result<(), RuntimeError> {
        self.predeclare_current_scope_lexicals(body);
        for stmt in body {
            if let Stmt::FunctionDecl { name, .. } = stmt {
                self.declare_lexical_slot(name, true);
            }
        }
        for slot in self.current_lexical_slots_for_names(names) {
            self.emit(Op::ClearLocal(slot));
        }
        self.compile_hoisted_function_decls(body)
    }

    /// Compiles a block that declares `using` resources: opens a disposal scope,
    /// runs the body inside an implicit try whose finally disposes the
    /// resources LIFO on every completion path.
    pub(super) fn compile_disposable_block(&mut self, body: &[Stmt]) -> Result<(), RuntimeError> {
        self.compile_disposal_region(block_has_await_using(body), |compiler| {
            compiler.compile_block_body(body)
        })
    }

    /// Compiles a statement list (a script or function body -- no new lexical
    /// scope) wrapped in a disposal try/finally so its top-level `using`
    /// resources are disposed when the body exits on any path.
    pub(super) fn compile_statements_with_disposal(
        &mut self,
        body: &[Stmt],
    ) -> Result<(), RuntimeError> {
        self.compile_disposal_region(block_has_await_using(body), |compiler| {
            body.iter().try_for_each(|stmt| compiler.compile_stmt(stmt))
        })
    }

    /// Opens a disposal scope and runs `body` inside the protected region
    /// whose finally disposes the scope's resources.
    pub(super) fn compile_disposal_region(
        &mut self,
        await_async: bool,
        body: impl FnOnce(&mut Self) -> Result<(), RuntimeError>,
    ) -> Result<(), RuntimeError> {
        self.emit(Op::EnterDisposableScope);
        let enter = self.emit(Op::EnterTry {
            catch: None,
            finally: None,
            end: usize::MAX,
            catch_scope: None,
            cleanup_slots: Vec::new(),
        });
        self.disposable_scope_depth += 1;
        let body_result = body(self);
        self.disposable_scope_depth -= 1;
        body_result?;
        self.emit(Op::ExitTry);
        let normal_jump = self.emit(Op::Jump(usize::MAX));
        let finally_target = self.compile_dispose_finally(await_async);
        if let Op::EnterTry { finally, .. } = &mut self.code[enter] {
            *finally = Some(finally_target);
        }
        self.patch_jump(normal_jump, finally_target);
        self.finish_try(enter, finally_target);
        Ok(())
    }

    /// Emits the disposal finally body and returns its entry IP.
    pub(super) fn compile_dispose_finally(&mut self, await_async: bool) -> usize {
        let target = self.code.len();
        let begin = self.emit(Op::BeginFinally(usize::MAX));
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
        self.emit(Op::EndFinally);
        let end = self.code.len();
        self.patch_finally_end(begin, end);
        target
    }

    pub(super) fn compile_try(
        &mut self,
        block: &[Stmt],
        handler: Option<&CatchClause>,
        finalizer: Option<&[Stmt]>,
    ) -> Result<(), RuntimeError> {
        // The try statement's completion is UpdateEmpty(B or C, undefined)
        // (ES2025 14.15.3), so the protected block and the catch block share
        // the enclosing register after resetting it. Only the finally block,
        // whose normal completion must not replace that value, gets its own.
        if !stmts_write_completion_first(block) {
            self.reset_completion();
        }
        let enter = self.emit(Op::EnterTry {
            catch: None,
            finally: None,
            end: usize::MAX,
            catch_scope: None,
            cleanup_slots: Vec::new(),
        });
        let cleanup_start = self.scope_plan.scopes.len();
        let mut cleanup_slots = Vec::new();
        if block_has_using(block) {
            self.compile_disposable_block(block)?;
        } else {
            self.with_lexical_scope(|compiler| compiler.compile_try_body(block))?;
            cleanup_slots = self.lexical_cleanup_slots_since(cleanup_start);
        }
        self.emit(Op::ExitTry);

        let normal_jump = self.emit(Op::Jump(usize::MAX));
        let (catch_target, catch_scope) = if let Some(handler) = handler {
            let (target, scope) = self.compile_catch(handler)?;
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
            cleanup_slots: slots,
            ..
        } = &mut self.code[enter]
        {
            *catch = catch_target;
            *finally = finally_target;
            *scope = catch_scope;
            *slots = cleanup_slots;
        }
        self.patch_jump(normal_jump, finally_target.unwrap_or(after));
        self.finish_try(enter, finally_target.unwrap_or(after));
        Ok(())
    }

    fn compile_try_body(&mut self, body: &[Stmt]) -> Result<(), RuntimeError> {
        let blocked = annex_b_blocked_names(body, self.strict);
        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
            compiler.instantiate_block_declarations(body, &blocked)?;
            for stmt in body {
                compiler.compile_stmt(stmt)?;
            }
            Ok(())
        })
    }

    fn lexical_cleanup_slots_since(&self, start: usize) -> Vec<usize> {
        self.scope_plan
            .declarations
            .iter()
            .filter(|(_, slot, scope)| {
                scope.0 >= start
                    && !self.locals[slot.0].is_received_upvalue()
                    && !self.locals[slot.0].compiler_temporary
            })
            .map(|(_, slot, _)| slot.0)
            .collect()
    }

    fn compile_catch(
        &mut self,
        handler: &CatchClause,
    ) -> Result<(usize, Option<CatchScope>), RuntimeError> {
        let target = self.code.len();
        // The catch block's completion replaces the protected block's, so
        // the shared register starts over from `undefined` once the thrown
        // value is bound.
        let reset = !stmts_write_completion_first(&handler.body);
        if let Some(param) = &handler.param {
            let slots = self.with_scope_kind(crate::binding::ScopeKind::Catch, |compiler| {
                let slots = compiler.compile_catch_param(param)?;
                if reset {
                    compiler.reset_completion();
                }
                let blocked = catch_param_annex_b_blocked_names(Some(param));
                compiler.with_annex_b_blocked_function_names(&blocked, |compiler| {
                    compiler.compile_block_body(&handler.body)
                })?;
                compiler.emit(Op::ExitTry);
                Ok(slots)
            })?;
            return Ok((target, Some(CatchScope::Clear { slots })));
        }
        self.emit(Op::Pop);
        if reset {
            self.reset_completion();
        }
        self.compile_block_body(&handler.body)?;
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
        // A finally block that completes normally yields the completion it
        // interrupted, so its statements must not touch the shared register.
        // Its own value matters only to a break/continue leaving it, which
        // carries it from a register of its own to the jump's target.
        let register = (self.completion.is_some() && stmts_contain_break_or_continue(finalizer))
            .then(|| self.temp_local("finally_result"));
        let target = self.code.len();
        let begin = self.emit(Op::BeginFinally(usize::MAX));
        self.with_completion(register, |compiler| {
            // Every entry (normal, thrown, returned or jumped) starts over.
            compiler.reset_completion();
            compiler.with_lexical_scope(|compiler| compiler.compile_try_body(finalizer))
        })?;
        self.emit(Op::EndFinally);
        let end = self.code.len();
        self.patch_finally_end(begin, end);
        Ok(target)
    }
}
