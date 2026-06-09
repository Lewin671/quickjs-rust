use qjs_ast::{AssignmentTarget, BinaryOp, Expr, ForInLeft, Stmt, SwitchCase, VarKind};

use crate::{RuntimeError, Value};

use super::compiler::{Compiler, for_in_left_lexical_name, switch_lexical_declared_names};
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
                kind: VarKind::Let | VarKind::Const,
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
                kind: VarKind::Let | VarKind::Const,
                ..
            }
        ) {
            return self
                .with_lexical_scope(|compiler| compiler.compile_for_of_scoped(left, right, body));
        }
        self.compile_for_of_scoped(left, right, body)
    }

    fn compile_for_in_scoped(
        &mut self,
        left: &ForInLeft,
        right: &Expr,
        body: &Stmt,
    ) -> Result<(), RuntimeError> {
        let result_slot = self.temp_local("for_in_result");
        let keys_slot = self.temp_local("for_in_keys");
        let index_slot = self.temp_local("for_in_index");
        let key_slot = self.temp_local("for_in_key");

        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
        self.compile_for_in_initializer(left)?;
        self.compile_expr(right)?;
        self.emit(Op::EnumerateKeys);
        self.emit(Op::StoreLocal(keys_slot));
        self.emit_number(0.0);
        self.emit(Op::StoreLocal(index_slot));

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
        self.compile_for_in_left(left, key_slot)?;

        let blocked = for_in_left_lexical_name(left).map_or_else(Vec::new, |name| vec![name]);
        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
            compiler.push_loop(result_slot);
            compiler.compile_stmt(body)?;
            compiler.emit(Op::StoreLocal(result_slot));
            Ok(())
        })?;
        let context = self.pop_loop();

        let update_start = self.code.len();
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
        let iterable_slot = self.temp_local("for_of_iterable");
        let iterator_slot = self.temp_local("for_of_iterator");
        let step_slot = self.temp_local("for_of_step");
        let value_slot = self.temp_local("for_of_value");

        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
        self.compile_expr(right)?;
        self.emit(Op::StoreLocal(iterable_slot));
        self.emit(Op::LoadLocal(iterable_slot));
        self.emit(Op::LoadGlobal("Symbol".to_owned()));
        self.emit_string("iterator");
        self.emit(Op::GetProp);
        self.emit(Op::CallMethod(0));
        self.emit(Op::StoreLocal(iterator_slot));

        let loop_start = self.code.len();
        self.emit(Op::LoadLocal(iterator_slot));
        self.emit_string("next");
        self.emit(Op::CallMethod(0));
        self.emit(Op::StoreLocal(step_slot));
        self.emit(Op::LoadLocal(step_slot));
        self.emit_string("done");
        self.emit(Op::GetProp);
        let exit_jump = self.emit(Op::JumpIfTrue(usize::MAX));
        self.emit(Op::Pop);

        self.emit(Op::LoadLocal(step_slot));
        self.emit_string("value");
        self.emit(Op::GetProp);
        self.emit(Op::StoreLocal(value_slot));
        self.compile_for_in_left(left, value_slot)?;

        let blocked = for_in_left_lexical_name(left).map_or_else(Vec::new, |name| vec![name]);
        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
            compiler.push_loop(result_slot);
            compiler.compile_stmt(body)?;
            compiler.emit(Op::StoreLocal(result_slot));
            Ok(())
        })?;
        let context = self.pop_loop();

        self.emit(Op::Jump(loop_start));

        let exit = self.code.len();
        self.patch_jump(exit_jump, exit);
        self.emit(Op::Pop);
        let cleanup_slots = self.current_lexical_slots_for_names(&blocked);
        self.emit_scoped_loop_completion(result_slot, &cleanup_slots, &context);
        self.patch_loop_continues(&context, loop_start);
        Ok(())
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
        let blocked = switch_lexical_declared_names(cases);
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
            ForInLeft::VarDecl { name, kind, .. } => {
                let slot = self.declare_var_kind_slot(name, *kind);
                if matches!(kind, VarKind::Let | VarKind::Const) {
                    self.emit(Op::ClearLocal(slot));
                }
                self.emit(Op::LoadLocal(key_slot));
                self.emit_store_var_binding(slot, name, *kind);
            }
            ForInLeft::Target(AssignmentTarget::Identifier { name, .. }) => {
                let slot = self.assignment_slot(name);
                self.emit(Op::LoadLocal(key_slot));
                self.emit(Op::StoreLocal(slot));
            }
            ForInLeft::Target(AssignmentTarget::Member {
                object, property, ..
            }) => {
                self.compile_expr(object)?;
                self.compile_member_key(property)?;
                self.emit(Op::LoadLocal(key_slot));
                self.emit(Op::SetProp {
                    is_strict: self.strict,
                });
                self.emit(Op::Pop);
            }
        }
        Ok(())
    }

    fn compile_for_in_initializer(&mut self, left: &ForInLeft) -> Result<(), RuntimeError> {
        let ForInLeft::VarDecl {
            name,
            kind,
            init: Some(init),
            ..
        } = left
        else {
            return Ok(());
        };
        let slot = self.declare_var_kind_slot(name, *kind);
        self.compile_expr(init)?;
        self.emit_store_var_binding(slot, name, *kind);
        Ok(())
    }

    fn emit_number(&mut self, value: f64) {
        let slot = self.const_slot(Value::Number(value));
        self.emit(Op::LoadConst(slot));
    }

    fn emit_string(&mut self, value: &str) {
        let slot = self.const_slot(Value::String(value.to_owned()));
        self.emit(Op::LoadConst(slot));
    }
}
