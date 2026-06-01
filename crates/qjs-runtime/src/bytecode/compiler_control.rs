use qjs_ast::{BinaryOp, Expr, SwitchCase};

use crate::RuntimeError;

use super::compiler::Compiler;
use super::ir::Op;

impl Compiler {
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
        body: &[qjs_ast::Stmt],
        result_slot: usize,
    ) -> Result<(), RuntimeError> {
        for stmt in body {
            self.compile_stmt(stmt)?;
            self.emit(Op::StoreLocal(result_slot));
        }
        Ok(())
    }
}
