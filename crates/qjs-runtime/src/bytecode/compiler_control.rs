use qjs_ast::{
    AssignmentTarget, BinaryOp, BindingPattern, Expr, ForInLeft, Stmt, SwitchCase, VarKind,
};

use crate::{RuntimeError, Value};

use super::compiler::{Compiler, LoopIterator};
use super::compiler_lexical::{
    for_in_left_lexical_names, is_lexical_for_in_left, switch_annex_b_blocked_names,
    switch_lexical_declared_bindings,
};
use super::ir::Op;

impl Compiler {
    pub(super) fn compile_for_in(
        &mut self,
        left: &ForInLeft,
        right: &Expr,
        body: &Stmt,
    ) -> Result<(), RuntimeError> {
        if matches!(
            left,
            ForInLeft::VarDecl {
                kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
                ..
            }
        ) {
            return self
                .with_lexical_scope(|compiler| compiler.compile_for_in_scoped(left, right, body));
        }
        self.compile_for_in_scoped(left, right, body)
    }

    pub(super) fn compile_for_of(
        &mut self,
        left: &ForInLeft,
        right: &Expr,
        body: &Stmt,
    ) -> Result<(), RuntimeError> {
        if matches!(
            left,
            ForInLeft::VarDecl {
                kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
                ..
            }
        ) {
            return self
                .with_lexical_scope(|compiler| compiler.compile_for_of_scoped(left, right, body));
        }
        self.compile_for_of_scoped(left, right, body)
    }

    pub(super) fn compile_for_await_of(
        &mut self,
        left: &ForInLeft,
        right: &Expr,
        body: &Stmt,
    ) -> Result<(), RuntimeError> {
        if matches!(
            left,
            ForInLeft::VarDecl {
                kind: VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing,
                ..
            }
        ) {
            return self.with_lexical_scope(|compiler| {
                compiler.compile_for_await_of_scoped(left, right, body)
            });
        }
        self.compile_for_await_of_scoped(left, right, body)
    }

    /// Compiles `for await (x of y)`: gets the async iterator, and on each pass
    /// calls `next()`, awaits the result promise, and validates/destructures the
    /// iterator result. Closing on break/throw reuses the sync close (calling
    /// `return()`); awaiting the close result is a known residual edge.
    fn compile_for_await_of_scoped(
        &mut self,
        left: &ForInLeft,
        right: &Expr,
        body: &Stmt,
    ) -> Result<(), RuntimeError> {
        let result_slot = self.temp_local("for_await_result");
        let iterator_slot = self.temp_local("for_await_iterator");
        let next_slot = self.temp_local("for_await_next");
        let done_slot = self.temp_local("for_await_done");
        let value_slot = self.temp_local("for_await_value");
        let iterator = LoopIterator {
            iterator_slot,
            done_slot,
        };

        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
        self.declare_for_in_head_tdz(left);
        self.compile_expr(right)?;
        self.emit(Op::GetAsyncIterator);
        self.emit(Op::StoreLocal(iterator_slot));
        self.emit(Op::LoadLocal(iterator_slot));
        self.emit_string("next");
        self.emit(Op::GetProp);
        self.emit(Op::StoreLocal(next_slot));
        let false_slot = self.const_slot(Value::Boolean(false));
        self.emit(Op::LoadConst(false_slot));
        self.emit(Op::StoreLocal(done_slot));
        let has_iteration_scope = is_lexical_for_in_left(left);
        if has_iteration_scope {
            self.emit(Op::PushCapturedEnv);
        }
        let enter = self.emit(Op::EnterTry {
            catch: None,
            finally: None,
            catch_scope: None,
        });

        let loop_start = self.code.len();
        // result = await iterator.next(); validate; extract value/done.
        self.emit(Op::LoadLocal(iterator_slot));
        self.emit(Op::LoadLocal(next_slot));
        self.emit(Op::CallResolved(0));
        self.emit(Op::Await);
        self.emit(Op::AsyncIteratorComplete { done_slot });
        self.emit(Op::LoadLocal(done_slot));
        let exit_jump = self.emit(Op::JumpIfTrue(usize::MAX));
        self.emit(Op::Pop);
        self.emit(Op::StoreLocal(value_slot));

        let blocked = for_in_left_lexical_names(left);
        let iteration_slots = self.for_in_left_iteration_slots(left);
        if !iteration_slots.is_empty() {
            self.emit(Op::FreshIterationScope(iteration_slots.clone()));
        }
        self.compile_for_of_iteration(left, value_slot, body, result_slot, iterator, &blocked)?;
        let context = self.pop_loop();

        self.emit(Op::Jump(loop_start));

        let exit = self.code.len();
        self.patch_jump(exit_jump, exit);
        self.emit(Op::Pop);
        self.emit(Op::Pop);
        self.emit(Op::ExitTry);
        let cleanup_slots = self.current_lexical_slots_for_names(&blocked);
        self.emit_for_of_loop_completion(result_slot, iterator, &cleanup_slots, &context, enter);
        self.patch_loop_continues(&context, loop_start);
        Ok(())
    }

    fn compile_for_in_scoped(
        &mut self,
        left: &ForInLeft,
        right: &Expr,
        body: &Stmt,
    ) -> Result<(), RuntimeError> {
        let result_slot = self.temp_local("for_in_result");
        let target_slot = self.temp_local("for_in_target");
        let keys_slot = self.temp_local("for_in_keys");
        let index_slot = self.temp_local("for_in_index");
        let key_slot = self.temp_local("for_in_key");

        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
        self.compile_for_in_initializer(left)?;
        self.declare_for_in_head_tdz(left);
        self.compile_expr(right)?;
        self.emit(Op::StoreLocal(target_slot));
        self.emit(Op::LoadLocal(target_slot));
        self.emit(Op::EnumerateKeys);
        self.emit(Op::StoreLocal(keys_slot));
        self.emit_number(0.0);
        self.emit(Op::StoreLocal(index_slot));
        let blocked = for_in_left_lexical_names(left);
        let has_iteration_scope = is_lexical_for_in_left(left);
        if has_iteration_scope {
            self.emit(Op::PushCapturedEnv);
        }
        let iteration_slots = self.for_in_left_iteration_slots(left);

        let loop_start = self.code.len();
        self.emit(Op::LoadLocal(index_slot));
        self.emit(Op::LoadLocal(keys_slot));
        self.emit_string("length");
        self.emit(Op::GetProp);
        self.emit(Op::Binary(BinaryOp::Lt));
        let exit_jump = self.emit(Op::JumpIfFalse(usize::MAX));
        self.emit(Op::Pop);

        self.emit(Op::LoadLocal(keys_slot));
        self.emit(Op::LoadLocal(index_slot));
        self.emit(Op::GetProp);
        self.emit(Op::StoreLocal(key_slot));
        self.emit(Op::LoadLocal(target_slot));
        self.emit(Op::LoadLocal(key_slot));
        self.emit(Op::ForInKeyIsEnumerable);
        let skip_key_jump = self.emit(Op::JumpIfFalse(usize::MAX));
        self.emit(Op::Pop);
        if !iteration_slots.is_empty() {
            self.emit(Op::FreshIterationScope(iteration_slots.clone()));
        }
        self.compile_for_in_left(left, key_slot)?;

        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
            compiler.push_loop(result_slot);
            if has_iteration_scope {
                compiler.mark_loop_captured_env_scope();
            }
            compiler.compile_stmt(body)?;
            compiler.emit(Op::StoreLocal(result_slot));
            Ok(())
        })?;
        let context = self.pop_loop();

        let body_done_jump = self.emit(Op::Jump(usize::MAX));
        let skip_key = self.code.len();
        self.emit(Op::Pop);
        let update_start = self.code.len();
        self.patch_jump(body_done_jump, update_start);
        self.patch_jump(skip_key_jump, skip_key);
        self.emit(Op::LoadLocal(index_slot));
        self.emit_number(1.0);
        self.emit(Op::Binary(BinaryOp::Add));
        self.emit(Op::StoreLocal(index_slot));
        self.emit(Op::Jump(loop_start));

        let exit = self.code.len();
        self.patch_jump(exit_jump, exit);
        self.emit(Op::Pop);
        let cleanup_slots = self.current_lexical_slots_for_names(&blocked);
        self.emit_scoped_loop_completion(result_slot, &cleanup_slots, &context);
        self.patch_loop_continues(&context, update_start);
        Ok(())
    }

    fn compile_for_of_scoped(
        &mut self,
        left: &ForInLeft,
        right: &Expr,
        body: &Stmt,
    ) -> Result<(), RuntimeError> {
        let result_slot = self.temp_local("for_of_result");
        let iterator_slot = self.temp_local("for_of_iterator");
        let next_slot = self.temp_local("for_of_next");
        let done_slot = self.temp_local("for_of_done");
        let value_slot = self.temp_local("for_of_value");
        let iterator = LoopIterator {
            iterator_slot,
            done_slot,
        };

        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
        self.declare_for_in_head_tdz(left);
        self.compile_expr(right)?;
        self.emit(Op::GetIterator);
        self.emit(Op::StoreLocal(iterator_slot));
        self.emit(Op::LoadLocal(iterator_slot));
        self.emit_string("next");
        self.emit(Op::GetProp);
        self.emit(Op::StoreLocal(next_slot));
        let false_slot = self.const_slot(Value::Boolean(false));
        self.emit(Op::LoadConst(false_slot));
        self.emit(Op::StoreLocal(done_slot));
        let has_iteration_scope = is_lexical_for_in_left(left);
        if has_iteration_scope {
            self.emit(Op::PushCapturedEnv);
        }
        let enter = self.emit(Op::EnterTry {
            catch: None,
            finally: None,
            catch_scope: None,
        });

        let loop_start = self.code.len();
        self.emit(Op::LoadLocal(iterator_slot));
        self.emit(Op::LoadLocal(next_slot));
        self.emit(Op::IteratorStep { done_slot });
        self.emit(Op::LoadLocal(done_slot));
        let exit_jump = self.emit(Op::JumpIfTrue(usize::MAX));
        self.emit(Op::Pop);
        self.emit(Op::StoreLocal(value_slot));

        let blocked = for_in_left_lexical_names(left);
        let iteration_slots = self.for_in_left_iteration_slots(left);
        if !iteration_slots.is_empty() {
            self.emit(Op::FreshIterationScope(iteration_slots.clone()));
        }
        self.compile_for_of_iteration(left, value_slot, body, result_slot, iterator, &blocked)?;
        let context = self.pop_loop();

        self.emit(Op::Jump(loop_start));

        let exit = self.code.len();
        self.patch_jump(exit_jump, exit);
        self.emit(Op::Pop);
        self.emit(Op::Pop);
        self.emit(Op::ExitTry);
        let cleanup_slots = self.current_lexical_slots_for_names(&blocked);
        self.emit_for_of_loop_completion(result_slot, iterator, &cleanup_slots, &context, enter);
        self.patch_loop_continues(&context, loop_start);
        Ok(())
    }

    fn compile_for_of_iteration(
        &mut self,
        left: &ForInLeft,
        value_slot: usize,
        body: &Stmt,
        result_slot: usize,
        iterator: LoopIterator,
        blocked: &[String],
    ) -> Result<(), RuntimeError> {
        self.with_annex_b_blocked_function_names(blocked, |compiler| {
            compiler.push_loop_with_iterator(result_slot, iterator);
            if is_lexical_for_in_left(left) {
                compiler.mark_loop_captured_env_scope();
            }
            if for_in_left_has_disposal(left) {
                compiler.compile_for_of_iteration_with_disposal(
                    left,
                    value_slot,
                    body,
                    result_slot,
                )?;
            } else {
                compiler.compile_for_in_left(left, value_slot)?;
                compiler.compile_stmt(body)?;
                compiler.emit(Op::StoreLocal(result_slot));
            }
            Ok(())
        })
    }

    fn compile_for_of_iteration_with_disposal(
        &mut self,
        left: &ForInLeft,
        value_slot: usize,
        body: &Stmt,
        result_slot: usize,
    ) -> Result<(), RuntimeError> {
        self.emit(Op::EnterDisposableScope);
        let loop_depth = self.loop_stack_depth();
        self.push_try_result_slot(result_slot, loop_depth, false);
        let enter = self.emit(Op::EnterTry {
            catch: None,
            finally: None,
            catch_scope: None,
        });
        self.disposable_scope_depth += 1;
        let body_result = (|| {
            self.compile_for_in_left(left, value_slot)?;
            self.compile_stmt(body)?;
            self.emit(Op::StoreLocal(result_slot));
            Ok(())
        })();
        self.disposable_scope_depth -= 1;
        body_result?;
        self.emit(Op::ExitTry);
        let normal_jump = self.emit(Op::Jump(usize::MAX));
        self.pop_try_result_slot();
        let finally_target = self.compile_dispose_finally(for_in_left_has_await_using(left));
        if let Op::EnterTry { finally, .. } = &mut self.code[enter] {
            *finally = Some(finally_target);
        }
        self.patch_jump(normal_jump, finally_target);
        Ok(())
    }

    /// Emits the for-of exit paths. The normal path arrives with the
    /// iterator exhausted and the protected region exited. Breaks exit the
    /// region and close the iterator with close errors propagating; thrown
    /// errors close it with close errors swallowed before rethrowing.
    fn emit_for_of_loop_completion(
        &mut self,
        result_slot: usize,
        iterator: LoopIterator,
        cleanup_slots: &[usize],
        context: &super::compiler::LoopContext,
        enter: usize,
    ) {
        for slot in cleanup_slots {
            self.emit(Op::ClearLocal(*slot));
        }
        if context.captured_env_scope {
            self.emit(Op::PopCapturedEnv);
        }
        self.emit(Op::LoadLocal(result_slot));
        let normal_done = self.emit(Op::Jump(usize::MAX));

        let break_cleanup = self.code.len();
        self.emit(Op::ExitTry);
        self.emit_close_unless_done(iterator.iterator_slot, iterator.done_slot, false);
        for slot in cleanup_slots {
            self.emit(Op::ClearLocal(*slot));
        }
        if context.captured_env_scope {
            self.emit(Op::PopCapturedEnv);
        }
        let break_done = self.emit(Op::Jump(usize::MAX));

        let catch_target = self.code.len();
        self.emit_close_unless_done(iterator.iterator_slot, iterator.done_slot, true);
        if context.captured_env_scope {
            self.emit(Op::PopCapturedEnv);
        }
        self.emit(Op::Throw);

        let done = self.code.len();
        self.patch_jump(normal_done, done);
        self.patch_jump(break_done, done);
        self.patch_loop_breaks(context, break_cleanup);
        if let Op::EnterTry { catch, .. } = &mut self.code[enter] {
            *catch = Some(catch_target);
        }
    }

    pub(super) fn compile_switch(
        &mut self,
        discriminant: &Expr,
        cases: &[SwitchCase],
    ) -> Result<(), RuntimeError> {
        self.with_lexical_scope(|compiler| compiler.compile_switch_scoped(discriminant, cases))
    }

    fn compile_switch_scoped(
        &mut self,
        discriminant: &Expr,
        cases: &[SwitchCase],
    ) -> Result<(), RuntimeError> {
        let discriminant_slot = self.temp_local("switch_discriminant");
        let result_slot = self.temp_local("switch_result");
        self.compile_expr(discriminant)?;
        self.emit(Op::StoreLocal(discriminant_slot));
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));

        for (name, mutable) in switch_lexical_declared_bindings(cases, self.strict) {
            self.declare_lexical_slot(&name, mutable);
        }
        let mut default_index = None;
        let mut case_jumps = Vec::with_capacity(cases.len());
        for (index, case) in cases.iter().enumerate() {
            case_jumps.push(None);
            if let Some(test) = &case.test {
                self.emit(Op::LoadLocal(discriminant_slot));
                self.compile_expr(test)?;
                self.emit(Op::Binary(BinaryOp::StrictEq));
                let next_test = self.emit(Op::JumpIfFalse(usize::MAX));
                self.emit(Op::Pop);
                case_jumps[index] = Some(self.emit(Op::Jump(usize::MAX)));
                let next_test_target = self.code.len();
                self.patch_jump(next_test, next_test_target);
                self.emit(Op::Pop);
            } else {
                default_index = Some(index);
            }
        }

        let no_match_jump = self.emit(Op::Jump(usize::MAX));
        let blocked = switch_annex_b_blocked_names(cases, self.strict);
        self.push_breakable(result_slot);
        let mut case_starts = Vec::with_capacity(cases.len());
        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
            for case in cases {
                case_starts.push(compiler.code.len());
                compiler.compile_switch_case(&case.consequent, result_slot)?;
            }
            Ok(())
        })?;
        let context = self.pop_loop();
        let normal_exit = self.code.len();
        self.patch_jump(
            no_match_jump,
            default_index
                .and_then(|index| case_starts.get(index).copied())
                .unwrap_or(normal_exit),
        );
        for (index, jump) in case_jumps.into_iter().enumerate() {
            if let Some(jump) = jump {
                self.patch_jump(jump, case_starts[index]);
            }
        }
        let cleanup_slots = self.current_lexical_slots_for_names(&blocked);
        self.emit_scoped_loop_completion(result_slot, &cleanup_slots, &context);
        Ok(())
    }

    fn compile_switch_case(
        &mut self,
        body: &[Stmt],
        result_slot: usize,
    ) -> Result<(), RuntimeError> {
        for stmt in body {
            if let Stmt::FunctionDecl { .. } = stmt {
                self.compile_function_decl(stmt)?;
            } else {
                self.compile_stmt(stmt)?;
            }
            self.emit(Op::StoreLocal(result_slot));
        }
        Ok(())
    }

    fn compile_for_in_left(
        &mut self,
        left: &ForInLeft,
        key_slot: usize,
    ) -> Result<(), RuntimeError> {
        match left {
            ForInLeft::VarDecl { binding, kind, .. } => {
                // Per-iteration bindings: lexical declarations are cleared
                // before re-initialization each round.
                for name in binding.names() {
                    let slot = self.declare_var_kind_slot(&name, *kind);
                    if matches!(
                        kind,
                        VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing
                    ) {
                        self.emit(Op::ClearLocal(slot));
                    }
                }
                self.emit(Op::LoadLocal(key_slot));
                if self.disposable_scope_depth > 0 {
                    match kind {
                        VarKind::Using => {
                            self.emit(Op::RegisterDisposable);
                        }
                        VarKind::AwaitUsing => {
                            self.emit(Op::RegisterAsyncDisposable);
                        }
                        _ => {}
                    }
                }
                self.compile_binding_initializer(binding, *kind)?;
            }
            ForInLeft::Target(AssignmentTarget::Identifier { name, .. }) => {
                self.emit(Op::LoadLocal(key_slot));
                let slot = self.resolve_local_slot(name);
                if slot.is_some() || self.inside_with() {
                    self.emit_store_identifier(name, slot, None);
                } else {
                    self.emit_store_unresolved_identifier(name, None);
                }
            }
            ForInLeft::Target(AssignmentTarget::Member {
                object, property, ..
            }) => {
                self.compile_expr(object)?;
                match property {
                    qjs_ast::MemberProperty::Private(name) => {
                        self.emit(Op::LoadLocal(key_slot));
                        self.emit(Op::SetPrivate(name.clone()));
                    }
                    _ => {
                        self.compile_member_key(property)?;
                        self.emit(Op::LoadLocal(key_slot));
                        self.emit(Op::SetProp {
                            is_strict: self.strict,
                        });
                    }
                }
                self.emit(Op::Pop);
            }
            ForInLeft::Target(
                target @ (AssignmentTarget::ArrayPattern { .. }
                | AssignmentTarget::ObjectPattern { .. }),
            ) => {
                self.emit(Op::LoadLocal(key_slot));
                self.compile_assignment_pattern(target)?;
            }
        }
        Ok(())
    }

    fn for_in_left_iteration_slots(&mut self, left: &ForInLeft) -> Vec<usize> {
        let ForInLeft::VarDecl {
            binding,
            kind: kind @ (VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing),
            ..
        } = left
        else {
            return Vec::new();
        };
        binding
            .names()
            .into_iter()
            .map(|name| self.declare_var_kind_slot(&name, *kind))
            .collect()
    }

    fn declare_for_in_head_tdz(&mut self, left: &ForInLeft) {
        let ForInLeft::VarDecl {
            binding,
            kind: kind @ (VarKind::Let | VarKind::Const | VarKind::Using | VarKind::AwaitUsing),
            ..
        } = left
        else {
            return;
        };
        for name in binding.names() {
            let slot = self.declare_var_kind_slot(&name, *kind);
            self.emit(Op::ClearLocal(slot));
        }
    }

    fn compile_for_in_initializer(&mut self, left: &ForInLeft) -> Result<(), RuntimeError> {
        let ForInLeft::VarDecl {
            binding: BindingPattern::Identifier { name, .. },
            kind,
            init: Some(init),
            ..
        } = left
        else {
            return Ok(());
        };
        let slot = self.declare_var_kind_slot(name, *kind);
        self.compile_expr(init)?;
        self.emit_store_var_initializer(slot, name, *kind);
        Ok(())
    }

    fn emit_number(&mut self, value: f64) {
        let slot = self.const_slot(Value::Number(value));
        self.emit(Op::LoadConst(slot));
    }

    fn emit_string(&mut self, value: &str) {
        let slot = self.const_slot(Value::String(value.to_owned().into()));
        self.emit(Op::LoadConst(slot));
    }

    pub(super) fn compile_with(&mut self, object: &Expr, body: &Stmt) -> Result<(), RuntimeError> {
        // The completion value of a `with` is its body's completion value.
        self.compile_expr(object)?;
        self.emit(Op::EnterWith);
        self.with_depth += 1;
        self.reset_current_loop_completion_to_undefined();
        let result = self.compile_stmt(body);
        self.with_depth -= 1;
        result?;
        self.emit(Op::ExitWith);
        Ok(())
    }

    /// Emits an `Op::ExitWith` for each `with` scope opened above `target_depth`,
    /// keeping the VM's with-object stack balanced when control jumps out of one
    /// or more `with` bodies. Does not change the compiler's own `with_depth`,
    /// since the surrounding scope keeps compiling after the jump.
    pub(super) fn emit_with_exits_above(&mut self, target_depth: usize) {
        for _ in target_depth..self.with_depth {
            self.emit(Op::ExitWith);
        }
    }
}

fn for_in_left_has_await_using(left: &ForInLeft) -> bool {
    matches!(
        left,
        ForInLeft::VarDecl {
            kind: VarKind::AwaitUsing,
            ..
        }
    )
}

fn for_in_left_has_disposal(left: &ForInLeft) -> bool {
    matches!(
        left,
        ForInLeft::VarDecl {
            kind: VarKind::Using | VarKind::AwaitUsing,
            ..
        }
    )
}
