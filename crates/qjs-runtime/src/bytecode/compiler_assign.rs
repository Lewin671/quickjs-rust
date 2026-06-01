use qjs_ast::{AssignmentOp, AssignmentTarget, BinaryOp, Expr, UpdateOp};

use crate::{RuntimeError, Value};

use super::compiler::Compiler;
use super::ir::Op;
use super::util::{assignment_binary_op, unsupported_target};

impl Compiler {
    pub(super) fn compile_assign(
        &mut self,
        target: &AssignmentTarget,
        value: &Expr,
    ) -> Result<(), RuntimeError> {
        match target {
            AssignmentTarget::Identifier { name, .. } => {
                let slot = self.local_slot(name, false);
                self.compile_expr(value)?;
                self.emit(Op::Dup);
                self.emit(Op::StoreLocal(slot));
                Ok(())
            }
            AssignmentTarget::Member {
                object, property, ..
            } => {
                self.compile_expr(object)?;
                self.compile_member_key(property)?;
                self.compile_expr(value)?;
                self.emit(Op::SetProp);
                Ok(())
            }
        }
    }

    pub(super) fn compile_compound_assign(
        &mut self,
        target: &AssignmentTarget,
        op: AssignmentOp,
        value: &Expr,
    ) -> Result<(), RuntimeError> {
        let AssignmentTarget::Identifier { name, .. } = target else {
            return Err(unsupported_target(target));
        };
        let slot = self.local_slot(name, false);
        match op {
            AssignmentOp::LogicalAndAssign => {
                self.emit(Op::LoadLocal(slot));
                let end_jump = self.emit(Op::JumpIfFalse(usize::MAX));
                self.emit(Op::Pop);
                self.compile_expr(value)?;
                self.emit(Op::Dup);
                self.emit(Op::StoreLocal(slot));
                let end = self.code.len();
                self.patch_jump(end_jump, end);
            }
            AssignmentOp::LogicalOrAssign => {
                self.emit(Op::LoadLocal(slot));
                let end_jump = self.emit(Op::JumpIfTrue(usize::MAX));
                self.emit(Op::Pop);
                self.compile_expr(value)?;
                self.emit(Op::Dup);
                self.emit(Op::StoreLocal(slot));
                let end = self.code.len();
                self.patch_jump(end_jump, end);
            }
            AssignmentOp::NullishAssign => {
                self.emit(Op::LoadLocal(slot));
                let end_jump = self.emit(Op::JumpIfNotNullish(usize::MAX));
                self.emit(Op::Pop);
                self.compile_expr(value)?;
                self.emit(Op::Dup);
                self.emit(Op::StoreLocal(slot));
                let end = self.code.len();
                self.patch_jump(end_jump, end);
            }
            _ => {
                self.emit(Op::LoadLocal(slot));
                self.compile_expr(value)?;
                self.emit(Op::Binary(assignment_binary_op(op)?));
                self.emit(Op::Dup);
                self.emit(Op::StoreLocal(slot));
            }
        }
        Ok(())
    }

    pub(super) fn compile_update(
        &mut self,
        target: &AssignmentTarget,
        op: UpdateOp,
        prefix: bool,
    ) -> Result<(), RuntimeError> {
        let AssignmentTarget::Identifier { name, .. } = target else {
            return Err(unsupported_target(target));
        };
        let slot = self.local_slot(name, false);
        self.emit(Op::LoadLocal(slot));
        if !prefix {
            self.emit(Op::Dup);
        }
        let one = self.const_slot(Value::Number(1.0));
        self.emit(Op::LoadConst(one));
        self.emit(Op::Binary(match op {
            UpdateOp::Increment => BinaryOp::Add,
            UpdateOp::Decrement => BinaryOp::Sub,
        }));
        if prefix {
            self.emit(Op::Dup);
            self.emit(Op::StoreLocal(slot));
        } else {
            self.emit(Op::StoreLocal(slot));
        }
        Ok(())
    }

    pub(super) fn compile_typeof(&mut self, argument: &Expr) -> Result<(), RuntimeError> {
        match argument {
            Expr::Identifier { name, .. } => {
                if let Some(slot) = self.local_slots.get(name) {
                    self.emit(Op::LoadLocal(*slot));
                } else {
                    let slot = self.const_slot(Value::String("undefined".to_owned()));
                    self.emit(Op::LoadConst(slot));
                    return Ok(());
                }
            }
            _ => self.compile_expr(argument)?,
        }
        self.emit(Op::Typeof);
        Ok(())
    }

    pub(super) fn emit_load_undefined(&mut self) {
        let slot = self.const_slot(Value::Undefined);
        self.emit(Op::LoadConst(slot));
    }
}
