use qjs_ast::{AssignmentTarget, BinaryOp, Expr, ForInLeft, Stmt, SwitchCase, VarKind};

use crate::{RuntimeError, Value};

use super::compiler::Compiler;
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

        self.push_loop(result_slot);
        self.compile_stmt(body)?;
        self.emit(Op::StoreLocal(result_slot));
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
        self.emit(Op::LoadLocal(result_slot));
        let done = self.code.len();
        self.patch_loop_breaks(&context, done);
        self.patch_loop_continues(&context, update_start);
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
        self.push_breakable(result_slot);
        let mut case_starts = Vec::with_capacity(cases.len());
        for case in cases {
            case_starts.push(self.code.len());
            self.compile_switch_case(&case.consequent, result_slot)?;
        }
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
        self.emit(Op::LoadLocal(result_slot));
        let done = self.code.len();
        self.patch_loop_breaks(&context, done);
        Ok(())
    }

    fn compile_switch_case(
        &mut self,
        body: &[Stmt],
        result_slot: usize,
    ) -> Result<(), RuntimeError> {
        for stmt in body {
            self.compile_stmt(stmt)?;
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
                self.emit(Op::SetProp);
                self.emit(Op::Pop);
            }
        }
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
