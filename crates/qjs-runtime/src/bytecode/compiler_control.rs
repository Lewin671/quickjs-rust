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
        let result_slot = self.temp_local("for_in_result");
        let keys_slot = self.temp_local("for_in_keys");
        let index_slot = self.temp_local("for_in_index");
        let key_slot = self.temp_local("for_in_key");
        self.prepare_for_in_left(left);
        let cleanup_slots = self.lexical_for_in_left_slots(left);

        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
        self.emit_clear_locals(&cleanup_slots);
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
        let cleanup_start = self.code.len();
        self.emit_clear_locals(&cleanup_slots);
        self.patch_loop_breaks(&context, cleanup_start);
        self.patch_loop_continues(&context, update_start);
        Ok(())
    }

    pub(super) fn compile_for_of(
        &mut self,
        left: &ForInLeft,
        right: &Expr,
        body: &Stmt,
    ) -> Result<(), RuntimeError> {
        let result_slot = self.temp_local("for_of_result");
        let values_slot = self.temp_local("for_of_values");
        let index_slot = self.temp_local("for_of_index");
        let value_slot = self.temp_local("for_of_value");
        self.prepare_for_in_left(left);
        let cleanup_slots = self.lexical_for_in_left_slots(left);

        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
        self.emit_clear_locals(&cleanup_slots);
        self.compile_expr(right)?;
        self.emit(Op::ForOfValues);
        self.emit(Op::StoreLocal(values_slot));
        self.emit_number(0.0);
        self.emit(Op::StoreLocal(index_slot));

        let loop_start = self.code.len();
        self.emit(Op::LoadLocal(index_slot));
        self.emit(Op::LoadLocal(values_slot));
        self.emit_string("length");
        self.emit(Op::GetProp);
        self.emit(Op::Binary(BinaryOp::Lt));
        let exit_jump = self.emit(Op::JumpIfFalse(usize::MAX));
        self.emit(Op::Pop);

        self.emit(Op::LoadLocal(values_slot));
        self.emit(Op::LoadLocal(index_slot));
        self.emit(Op::GetProp);
        self.emit(Op::StoreLocal(value_slot));
        self.compile_for_in_left(left, value_slot)?;

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
        let cleanup_start = self.code.len();
        self.emit_clear_locals(&cleanup_slots);
        self.patch_loop_breaks(&context, cleanup_start);
        self.patch_loop_continues(&context, update_start);
        Ok(())
    }

    pub(super) fn compile_switch(
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
                let slot = self.local_slot(name, *kind == VarKind::Var);
                self.emit(Op::LoadLocal(key_slot));
                self.emit(Op::StoreLocal(slot));
            }
            ForInLeft::Binding { target, .. } => {
                self.compile_store_value(target, key_slot)?;
                self.emit(Op::Pop);
            }
            ForInLeft::Target(AssignmentTarget::Identifier { name, .. }) => {
                let slot = self.local_slot(name, false);
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
                    strict: self.strict,
                });
                self.emit(Op::Pop);
            }
            ForInLeft::Target(
                target @ (AssignmentTarget::Object { .. } | AssignmentTarget::Array { .. }),
            ) => {
                self.compile_store_value(target, key_slot)?;
                self.emit(Op::Pop);
            }
        }
        Ok(())
    }

    fn prepare_for_in_left(&mut self, left: &ForInLeft) {
        match left {
            ForInLeft::VarDecl { name, kind, .. } => {
                self.local_slot(name, *kind == VarKind::Var);
            }
            ForInLeft::Binding { target, kind, .. } => {
                self.ensure_target_local_slots(target, *kind == VarKind::Var);
            }
            ForInLeft::Target(_) => {}
        }
    }

    fn lexical_for_in_left_slots(&mut self, left: &ForInLeft) -> Vec<usize> {
        let mut slots = Vec::new();
        match left {
            ForInLeft::VarDecl { name, kind, .. } if *kind != VarKind::Var => {
                slots.push(self.local_slot(name, false));
            }
            ForInLeft::Binding { target, kind, .. } if *kind != VarKind::Var => {
                self.collect_target_local_slots(target, false, &mut slots);
            }
            _ => {}
        }
        slots.sort_unstable();
        slots.dedup();
        slots
    }

    fn ensure_target_local_slots(&mut self, target: &AssignmentTarget, hoisted: bool) {
        match target {
            AssignmentTarget::Identifier { name, .. } => {
                self.local_slot(name, hoisted);
            }
            AssignmentTarget::Array { elements, .. } => {
                for element in elements.iter().flatten() {
                    self.ensure_target_local_slots(&element.target, hoisted);
                }
            }
            AssignmentTarget::Object { properties, .. } => {
                for property in properties {
                    self.ensure_target_local_slots(&property.target, hoisted);
                }
            }
            AssignmentTarget::Member { .. } => {}
        }
    }

    fn collect_target_local_slots(
        &mut self,
        target: &AssignmentTarget,
        hoisted: bool,
        slots: &mut Vec<usize>,
    ) {
        match target {
            AssignmentTarget::Identifier { name, .. } => {
                slots.push(self.local_slot(name, hoisted));
            }
            AssignmentTarget::Array { elements, .. } => {
                for element in elements.iter().flatten() {
                    self.collect_target_local_slots(&element.target, hoisted, slots);
                }
            }
            AssignmentTarget::Object { properties, .. } => {
                for property in properties {
                    self.collect_target_local_slots(&property.target, hoisted, slots);
                }
            }
            AssignmentTarget::Member { .. } => {}
        }
    }

    fn emit_clear_locals(&mut self, slots: &[usize]) {
        for slot in slots {
            self.emit(Op::ClearLocal(*slot));
        }
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
