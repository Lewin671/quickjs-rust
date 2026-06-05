use qjs_ast::{
    ArrayAssignmentElement, AssignmentOp, AssignmentTarget, BinaryOp, Expr,
    ObjectAssignmentProperty, ObjectPropertyKey, UpdateOp,
};

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
                self.validate_strict_binding_name(name)?;
                if self.dynamic_scope_depth == 0
                    && let Some(slot) = self.local_slots.get(name).copied()
                {
                    self.compile_expr(value)?;
                    self.emit(Op::Dup);
                    self.emit(Op::StoreLocal(slot));
                } else {
                    self.emit(Op::ResolveName(name.clone()));
                    self.compile_expr(value)?;
                    self.emit(Op::Dup);
                    self.emit(Op::StoreName {
                        name: name.clone(),
                        strict: self.strict,
                    });
                }
                Ok(())
            }
            AssignmentTarget::Member {
                object, property, ..
            } => {
                if matches!(object.as_ref(), Expr::Identifier { name, .. } if name == "super") {
                    return self.compile_super_assign(property, value);
                }
                self.compile_expr(object)?;
                self.compile_member_key(property)?;
                self.compile_expr(value)?;
                self.emit(Op::SetProp {
                    strict: self.strict,
                });
                Ok(())
            }
            AssignmentTarget::Object { properties, .. } => {
                self.compile_object_destructuring_assign(properties, value)
            }
            AssignmentTarget::Array { elements, .. } => {
                self.compile_array_destructuring_assign(elements, value)
            }
        }
    }

    fn compile_super_assign(
        &mut self,
        property: &qjs_ast::MemberProperty,
        value: &Expr,
    ) -> Result<(), RuntimeError> {
        let key_slot = self.temp_local("super_key");
        self.compile_member_key(property)?;
        self.emit(Op::StoreLocal(key_slot));
        self.compile_expr(value)?;
        self.emit(Op::Pop);
        self.emit(Op::ThrowTypeError(
            "TypeError: cannot assign to super property".to_owned(),
        ));
        Ok(())
    }

    fn compile_array_destructuring_assign(
        &mut self,
        elements: &[Option<ArrayAssignmentElement>],
        value: &Expr,
    ) -> Result<(), RuntimeError> {
        let iterator_slot = self.temp_local("array_pattern_iterator");
        let value_slot = self.temp_local("array_pattern_value");
        let thrown_slot = self.temp_local("array_pattern_thrown");
        self.compile_expr(value)?;
        self.emit(Op::Dup);
        self.emit(Op::StoreLocal(iterator_slot));
        for element in elements.iter().flatten() {
            self.compile_iterator_next_value(iterator_slot)?;
            self.emit(Op::StoreLocal(value_slot));
            let enter = self.emit(Op::EnterTry {
                catch: None,
                finally: None,
                catch_scope: None,
            });
            if let Some(default) = &element.default {
                self.emit(Op::LoadLocal(value_slot));
                let use_existing = self.emit(Op::JumpIfNotUndefined(usize::MAX));
                self.emit(Op::Pop);
                self.compile_expr(default)?;
                self.emit(Op::StoreLocal(value_slot));
                let after_selection = self.emit(Op::Jump(usize::MAX));
                let existing_target = self.code.len();
                self.patch_jump(use_existing, existing_target);
                self.emit(Op::Pop);
                let after = self.code.len();
                self.patch_jump(after_selection, after);
            }
            self.compile_store_value(&element.target, value_slot)?;
            self.emit(Op::Pop);
            self.emit(Op::ExitTry);
            let normal_jump = self.emit(Op::Jump(usize::MAX));
            let catch_target = self.code.len();
            self.emit(Op::StoreLocal(thrown_slot));
            self.emit(Op::IteratorCloseForThrow(iterator_slot));
            self.emit(Op::LoadLocal(thrown_slot));
            self.emit(Op::Throw);
            let after = self.code.len();
            if let Op::EnterTry { catch, .. } = &mut self.code[enter] {
                *catch = Some(catch_target);
            }
            self.patch_jump(normal_jump, after);
        }
        Ok(())
    }

    fn compile_iterator_next_value(&mut self, iterator_slot: usize) -> Result<(), RuntimeError> {
        self.emit(Op::LoadLocal(iterator_slot));
        let next_slot = self.const_slot(Value::String("next".to_owned()));
        self.emit(Op::LoadConst(next_slot));
        self.emit(Op::CallMethod(0));
        let value_key = self.const_slot(Value::String("value".to_owned()));
        self.emit(Op::LoadConst(value_key));
        self.emit(Op::GetProp);
        Ok(())
    }

    fn compile_object_destructuring_assign(
        &mut self,
        properties: &[ObjectAssignmentProperty],
        value: &Expr,
    ) -> Result<(), RuntimeError> {
        let object_slot = self.temp_local("object_pattern_source");
        let value_slot = self.temp_local("object_pattern_value");
        self.compile_expr(value)?;
        self.emit(Op::Dup);
        self.emit(Op::StoreLocal(object_slot));
        for property in properties {
            self.emit(Op::LoadLocal(object_slot));
            match &property.key {
                ObjectPropertyKey::Literal(key) => {
                    let key_slot = self.const_slot(Value::String(key.clone()));
                    self.emit(Op::LoadConst(key_slot));
                }
                ObjectPropertyKey::Computed(expr) => self.compile_expr(expr)?,
            }
            self.emit(Op::GetProp);
            self.emit(Op::StoreLocal(value_slot));
            self.compile_store_value(&property.target, value_slot)?;
            self.emit(Op::Pop);
        }
        Ok(())
    }

    pub(super) fn compile_store_value(
        &mut self,
        target: &AssignmentTarget,
        value_slot: usize,
    ) -> Result<(), RuntimeError> {
        self.compile_store_value_with_mode(target, value_slot, false)
    }

    pub(super) fn compile_init_value(
        &mut self,
        target: &AssignmentTarget,
        value_slot: usize,
    ) -> Result<(), RuntimeError> {
        self.compile_store_value_with_mode(target, value_slot, true)
    }

    fn compile_store_value_with_mode(
        &mut self,
        target: &AssignmentTarget,
        value_slot: usize,
        initializing: bool,
    ) -> Result<(), RuntimeError> {
        match target {
            AssignmentTarget::Identifier { name, .. } => {
                if self.dynamic_scope_depth == 0
                    && let Some(slot) = self.local_slots.get(name).copied()
                {
                    self.emit(Op::LoadLocal(value_slot));
                    self.emit(Op::Dup);
                    self.emit(if initializing {
                        Op::InitLocal(slot)
                    } else {
                        Op::StoreLocal(slot)
                    });
                } else {
                    self.emit(Op::ResolveName(name.clone()));
                    self.emit(Op::LoadLocal(value_slot));
                    self.emit(Op::Dup);
                    self.emit(Op::StoreName {
                        name: name.clone(),
                        strict: self.strict,
                    });
                }
                Ok(())
            }
            AssignmentTarget::Member {
                object, property, ..
            } => {
                self.compile_expr(object)?;
                self.compile_member_key(property)?;
                self.emit(Op::LoadLocal(value_slot));
                self.emit(Op::SetProp {
                    strict: self.strict,
                });
                Ok(())
            }
            AssignmentTarget::Object { properties, .. } => {
                self.compile_object_pattern_from_slot(properties, value_slot, initializing)?;
                self.emit(Op::LoadLocal(value_slot));
                Ok(())
            }
            AssignmentTarget::Array { elements, .. } => {
                self.compile_array_pattern_from_slot(elements, value_slot, initializing)?;
                self.emit(Op::LoadLocal(value_slot));
                Ok(())
            }
        }
    }

    fn compile_array_pattern_from_slot(
        &mut self,
        elements: &[Option<ArrayAssignmentElement>],
        source_slot: usize,
        initializing: bool,
    ) -> Result<(), RuntimeError> {
        let value_slot = self.temp_local("array_pattern_value");
        for (index, element) in elements.iter().enumerate() {
            let Some(element) = element else {
                continue;
            };
            self.emit(Op::LoadLocal(source_slot));
            let index_slot = self.const_slot(Value::Number(index as f64));
            self.emit(Op::LoadConst(index_slot));
            self.emit(Op::GetProp);
            self.emit(Op::StoreLocal(value_slot));
            if let Some(default) = &element.default {
                self.emit(Op::LoadLocal(value_slot));
                let use_existing = self.emit(Op::JumpIfNotUndefined(usize::MAX));
                self.emit(Op::Pop);
                self.compile_expr(default)?;
                self.emit(Op::StoreLocal(value_slot));
                let after_selection = self.emit(Op::Jump(usize::MAX));
                let existing_target = self.code.len();
                self.patch_jump(use_existing, existing_target);
                self.emit(Op::Pop);
                let after = self.code.len();
                self.patch_jump(after_selection, after);
            }
            self.compile_store_value_with_mode(&element.target, value_slot, initializing)?;
            self.emit(Op::Pop);
        }
        Ok(())
    }

    fn compile_object_pattern_from_slot(
        &mut self,
        properties: &[ObjectAssignmentProperty],
        source_slot: usize,
        initializing: bool,
    ) -> Result<(), RuntimeError> {
        let value_slot = self.temp_local("object_pattern_value");
        for property in properties {
            self.emit(Op::LoadLocal(source_slot));
            match &property.key {
                ObjectPropertyKey::Literal(key) => {
                    let key_slot = self.const_slot(Value::String(key.clone()));
                    self.emit(Op::LoadConst(key_slot));
                }
                ObjectPropertyKey::Computed(expr) => self.compile_expr(expr)?,
            }
            self.emit(Op::GetProp);
            self.emit(Op::StoreLocal(value_slot));
            self.compile_store_value_with_mode(&property.target, value_slot, initializing)?;
            self.emit(Op::Pop);
        }
        Ok(())
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
        self.validate_strict_binding_name(name)?;
        match op {
            AssignmentOp::LogicalAndAssign => {
                self.emit_load_binding(name);
                let end_jump = self.emit(Op::JumpIfFalse(usize::MAX));
                self.emit(Op::Pop);
                self.compile_expr(value)?;
                self.emit(Op::Dup);
                self.emit_store_binding(name);
                let end = self.code.len();
                self.patch_jump(end_jump, end);
            }
            AssignmentOp::LogicalOrAssign => {
                self.emit_load_binding(name);
                let end_jump = self.emit(Op::JumpIfTrue(usize::MAX));
                self.emit(Op::Pop);
                self.compile_expr(value)?;
                self.emit(Op::Dup);
                self.emit_store_binding(name);
                let end = self.code.len();
                self.patch_jump(end_jump, end);
            }
            AssignmentOp::NullishAssign => {
                self.emit_load_binding(name);
                let end_jump = self.emit(Op::JumpIfNotNullish(usize::MAX));
                self.emit(Op::Pop);
                self.compile_expr(value)?;
                self.emit(Op::Dup);
                self.emit_store_binding(name);
                let end = self.code.len();
                self.patch_jump(end_jump, end);
            }
            _ => {
                self.emit_load_binding(name);
                self.compile_expr(value)?;
                self.emit(Op::Binary(assignment_binary_op(op)?));
                self.emit(Op::Dup);
                self.emit_store_binding(name);
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
        self.validate_strict_binding_name(name)?;
        self.emit_load_binding(name);
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
            self.emit_store_binding(name);
        } else {
            self.emit_store_binding(name);
        }
        Ok(())
    }

    fn emit_load_binding(&mut self, name: &str) {
        if self.dynamic_scope_depth == 0
            && let Some(slot) = self.local_slots.get(name).copied()
        {
            self.emit(Op::LoadLocal(slot));
        } else {
            self.emit(Op::LoadName(name.to_owned()));
        }
    }

    fn emit_store_binding(&mut self, name: &str) {
        if self.dynamic_scope_depth == 0
            && let Some(slot) = self.local_slots.get(name).copied()
        {
            self.emit(Op::StoreLocal(slot));
        } else {
            self.emit(Op::StoreName {
                name: name.to_owned(),
                strict: self.strict,
            });
        }
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
        self.emit(Op::CheckObjectCoercible);
        self.emit(Op::Pop);
        self.emit(Op::LoadLocal(key_slot));
        self.emit(Op::ToPropertyKey);
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
        self.emit(Op::CheckObjectCoercible);
        self.emit(Op::Pop);
        self.emit(Op::LoadLocal(key_slot));
        self.emit(Op::ToPropertyKey);
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
        self.emit(Op::SetProp {
            strict: self.strict,
        });
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
