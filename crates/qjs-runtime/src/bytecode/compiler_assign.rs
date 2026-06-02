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
            return self.compile_member_compound_assign(target, op, value);
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
            return self.compile_member_update(target, op, prefix);
        };
        let slot = self.local_slot(name, false);
        self.emit(Op::LoadLocal(slot));
        self.emit(Op::Unary(qjs_ast::UnaryOp::Plus));
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

    fn compile_member_compound_assign(
        &mut self,
        target: &AssignmentTarget,
        op: AssignmentOp,
        value: &Expr,
    ) -> Result<(), RuntimeError> {
        let AssignmentTarget::Member {
            object, property, ..
        } = target
        else {
            return Err(unsupported_target(target));
        };
        let object_slot = self.temp_local("assign_object");
        let key_slot = self.temp_local("assign_key");
        let value_slot = self.temp_local("assign_value");
        self.compile_expr(object)?;
        self.emit(Op::StoreLocal(object_slot));
        self.compile_member_key(property)?;
        self.emit(Op::StoreLocal(key_slot));
        self.emit(Op::LoadLocal(object_slot));
        self.emit(Op::LoadLocal(key_slot));
        self.emit(Op::GetProp);
        match op {
            AssignmentOp::LogicalAndAssign
            | AssignmentOp::LogicalOrAssign
            | AssignmentOp::NullishAssign => {
                let jump = match op {
                    AssignmentOp::LogicalAndAssign => self.emit(Op::JumpIfFalse(usize::MAX)),
                    AssignmentOp::LogicalOrAssign => self.emit(Op::JumpIfTrue(usize::MAX)),
                    AssignmentOp::NullishAssign => self.emit(Op::JumpIfNotNullish(usize::MAX)),
                    _ => unreachable!(),
                };
                self.emit(Op::Pop);
                self.compile_expr(value)?;
                self.emit(Op::StoreLocal(value_slot));
                self.emit_member_store(object_slot, key_slot, value_slot);
                let end = self.code.len();
                self.patch_jump(jump, end);
            }
            _ => {
                self.compile_expr(value)?;
                self.emit(Op::Binary(assignment_binary_op(op)?));
                self.emit(Op::StoreLocal(value_slot));
                self.emit_member_store(object_slot, key_slot, value_slot);
            }
        }
        Ok(())
    }

    fn compile_member_update(
        &mut self,
        target: &AssignmentTarget,
        op: UpdateOp,
        prefix: bool,
    ) -> Result<(), RuntimeError> {
        let AssignmentTarget::Member {
            object, property, ..
        } = target
        else {
            return Err(unsupported_target(target));
        };
        let object_slot = self.temp_local("update_object");
        let key_slot = self.temp_local("update_key");
        let old_slot = self.temp_local("update_old");
        let new_slot = self.temp_local("update_new");
        self.compile_expr(object)?;
        self.emit(Op::StoreLocal(object_slot));
        self.compile_member_key(property)?;
        self.emit(Op::StoreLocal(key_slot));
        self.emit(Op::LoadLocal(object_slot));
        self.emit(Op::LoadLocal(key_slot));
        self.emit(Op::GetProp);
        self.emit(Op::Unary(qjs_ast::UnaryOp::Plus));
        self.emit(Op::StoreLocal(old_slot));
        self.emit(Op::LoadLocal(old_slot));
        let one = self.const_slot(Value::Number(1.0));
        self.emit(Op::LoadConst(one));
        self.emit(Op::Binary(match op {
            UpdateOp::Increment => BinaryOp::Add,
            UpdateOp::Decrement => BinaryOp::Sub,
        }));
        self.emit(Op::StoreLocal(new_slot));
        self.emit_member_store(object_slot, key_slot, new_slot);
        self.emit(Op::Pop);
        self.emit(Op::LoadLocal(if prefix { new_slot } else { old_slot }));
        Ok(())
    }

    fn emit_member_store(&mut self, object_slot: usize, key_slot: usize, value_slot: usize) {
        self.emit(Op::LoadLocal(object_slot));
        self.emit(Op::LoadLocal(key_slot));
        self.emit(Op::LoadLocal(value_slot));
        self.emit(Op::SetProp);
    }

    pub(super) fn compile_typeof(&mut self, argument: &Expr) -> Result<(), RuntimeError> {
        match argument {
            Expr::Identifier { name, .. } => {
                if let Some(slot) = self.local_slots.get(name) {
                    self.emit(Op::LoadLocalOrUndefined(*slot));
                } else {
                    self.emit(Op::TypeofGlobal(name.clone()));
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
