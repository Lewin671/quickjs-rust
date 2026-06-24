use qjs_ast::{AssignmentOp, AssignmentTarget, Expr, Literal, UpdateOp};

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
            AssignmentTarget::Identifier {
                name,
                parenthesized,
                ..
            } => {
                // `f = <anon>` applies NamedEvaluation: an anonymous function or
                // class assigned to a plain identifier takes that identifier's
                // name. Member targets (`obj.x = <anon>`) never do.
                let slot = self.resolve_local_slot(name);
                if self.identifier_needs_with_resolution(slot) {
                    let object_slot = self.temp_local("with_assignment_object");
                    self.emit(Op::ResolveIdentWith {
                        name: name.clone(),
                        slot,
                        object_slot,
                    });
                    if *parenthesized {
                        self.compile_expr(value)?;
                    } else {
                        self.compile_named_expr(value, name)?;
                    }
                    self.emit(Op::Dup);
                    self.emit(Op::StoreResolvedIdentWith {
                        name: name.clone(),
                        slot,
                        object_slot,
                        is_strict: self.strict,
                    });
                    return Ok(());
                }
                let Some(slot) = slot else {
                    if *parenthesized {
                        self.compile_expr(value)?;
                    } else {
                        self.compile_named_expr(value, name)?;
                    }
                    self.emit(Op::Dup);
                    self.emit_store_unresolved_identifier(name, None);
                    return Ok(());
                };
                if *parenthesized {
                    self.compile_expr(value)?;
                } else {
                    self.compile_named_expr(value, name)?;
                }
                self.emit(Op::Dup);
                self.emit_store_unresolved_identifier(name, Some(slot));
                Ok(())
            }
            AssignmentTarget::Member {
                object,
                property: qjs_ast::MemberProperty::Private(name),
                ..
            } => {
                // `obj.#x = value` evaluates to `value`; SetPrivate leaves the
                // assigned value on the stack.
                self.compile_expr(object)?;
                self.compile_expr(value)?;
                self.emit(Op::SetPrivate(name.clone()));
                Ok(())
            }
            AssignmentTarget::Member {
                object, property, ..
            } if matches!(object.as_ref(), Expr::Super { .. }) => match property {
                qjs_ast::MemberProperty::Named(name) => {
                    self.compile_expr(value)?;
                    self.emit(Op::SuperSet {
                        key: name.clone(),
                        is_strict: self.strict,
                    });
                    Ok(())
                }
                qjs_ast::MemberProperty::Computed(expr) => {
                    self.emit(Op::SuperReference);
                    self.compile_expr(expr)?;
                    self.compile_expr(value)?;
                    self.emit(Op::SuperSetComputed {
                        is_strict: self.strict,
                    });
                    Ok(())
                }
                qjs_ast::MemberProperty::Private(name) => Err(RuntimeError {
                    thrown: None,
                    message: format!("SyntaxError: super.#{name} is not allowed"),
                }),
            },
            AssignmentTarget::Member {
                object, property, ..
            } => {
                let object_slot = self.temp_local("assign_object");
                let key_slot = self.temp_local("assign_key");
                self.compile_member_reference(object, property, object_slot, key_slot, false)?;
                self.compile_expr(value)?;
                let value_slot = self.temp_local("assign_value");
                self.emit(Op::StoreLocal(value_slot));
                self.emit_member_store(object_slot, key_slot, value_slot);
                Ok(())
            }
            AssignmentTarget::ArrayPattern { .. } | AssignmentTarget::ObjectPattern { .. } => {
                self.compile_expr(value)?;
                let rhs_slot = self.temp_local("destructuring_rhs");
                self.emit(Op::StoreLocal(rhs_slot));
                self.emit(Op::LoadLocal(rhs_slot));
                self.compile_assignment_pattern(target)?;
                // The assignment expression evaluates to the right-hand value.
                self.emit(Op::LoadLocal(rhs_slot));
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
        let AssignmentTarget::Identifier {
            name,
            parenthesized,
            ..
        } = target
        else {
            return self.compile_member_compound_assign(target, op, value);
        };
        let slot = self.resolve_local_slot(name);
        let resolved_with_object_slot = self.resolve_with_identifier_target(name, slot);
        match op {
            AssignmentOp::LogicalAndAssign => {
                self.emit_load_identifier(name, slot, resolved_with_object_slot);
                let end_jump = self.emit(Op::JumpIfFalse(usize::MAX));
                self.emit(Op::Pop);
                // `f &&= <anon>` names the anonymous value after the target
                // identifier (ES2023 §13.15.2); arithmetic compounds do not.
                if *parenthesized {
                    self.compile_expr(value)?;
                } else {
                    self.compile_named_expr(value, name)?;
                }
                self.emit(Op::Dup);
                self.emit_store_identifier(name, slot, resolved_with_object_slot);
                let end = self.code.len();
                self.patch_jump(end_jump, end);
            }
            AssignmentOp::LogicalOrAssign => {
                self.emit_load_identifier(name, slot, resolved_with_object_slot);
                let end_jump = self.emit(Op::JumpIfTrue(usize::MAX));
                self.emit(Op::Pop);
                if *parenthesized {
                    self.compile_expr(value)?;
                } else {
                    self.compile_named_expr(value, name)?;
                }
                self.emit(Op::Dup);
                self.emit_store_identifier(name, slot, resolved_with_object_slot);
                let end = self.code.len();
                self.patch_jump(end_jump, end);
            }
            AssignmentOp::NullishAssign => {
                self.emit_load_identifier(name, slot, resolved_with_object_slot);
                let end_jump = self.emit(Op::JumpIfNotNullish(usize::MAX));
                self.emit(Op::Pop);
                if *parenthesized {
                    self.compile_expr(value)?;
                } else {
                    self.compile_named_expr(value, name)?;
                }
                self.emit(Op::Dup);
                self.emit_store_identifier(name, slot, resolved_with_object_slot);
                let end = self.code.len();
                self.patch_jump(end_jump, end);
            }
            _ => {
                if op == AssignmentOp::AddAssign
                    && !self.inside_with()
                    && let Expr::Literal(Literal::String { value, .. }) = value
                {
                    if let Some(slot) = slot {
                        self.emit(Op::AppendStringLiteralLocal {
                            slot,
                            value: value.clone(),
                        });
                    } else {
                        self.emit(Op::AppendStringLiteralGlobal {
                            name: name.clone(),
                            value: value.clone(),
                            is_strict: self.strict,
                        });
                    }
                    return Ok(());
                }
                self.emit_load_identifier(name, slot, resolved_with_object_slot);
                self.compile_expr(value)?;
                self.emit(Op::Binary(assignment_binary_op(op)?));
                self.emit(Op::Dup);
                self.emit_store_identifier(name, slot, resolved_with_object_slot);
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
        let slot = self.resolve_local_slot(name);
        let resolved_with_object_slot = self.resolve_with_identifier_target(name, slot);
        self.emit_load_identifier(name, slot, resolved_with_object_slot);
        self.emit(Op::ToNumeric);
        if !prefix {
            self.emit(Op::Dup);
        }
        self.emit(Op::Update(op));
        if prefix {
            self.emit(Op::Dup);
            self.emit_store_identifier(name, slot, resolved_with_object_slot);
        } else {
            self.emit_store_identifier(name, slot, resolved_with_object_slot);
        }
        Ok(())
    }

    fn emit_load_identifier(
        &mut self,
        name: &str,
        slot: Option<usize>,
        resolved_with_object_slot: Option<usize>,
    ) {
        if let Some(object_slot) = resolved_with_object_slot {
            self.emit(Op::LoadResolvedIdentWith {
                name: name.to_owned(),
                slot,
                object_slot,
                is_strict: self.strict,
            });
        } else if self.identifier_needs_with_resolution(slot) {
            self.emit(Op::LoadIdentWith {
                name: name.to_owned(),
                slot,
                is_strict: self.strict,
            });
        } else if let Some(slot) = slot {
            self.emit(Op::LoadLocal(slot));
        } else {
            self.emit(Op::LoadGlobal(name.to_owned()));
        }
    }

    fn resolve_with_identifier_target(&mut self, name: &str, slot: Option<usize>) -> Option<usize> {
        if !self.identifier_needs_with_resolution(slot) {
            return None;
        }
        let object_slot = self.temp_local("with_assignment_object");
        self.emit(Op::ResolveIdentWith {
            name: name.to_owned(),
            slot,
            object_slot,
        });
        Some(object_slot)
    }

    pub(super) fn emit_store_identifier(
        &mut self,
        name: &str,
        slot: Option<usize>,
        resolved_with_object_slot: Option<usize>,
    ) {
        if let Some(object_slot) = resolved_with_object_slot {
            self.emit(Op::StoreResolvedIdentWith {
                name: name.to_owned(),
                slot,
                object_slot,
                is_strict: self.strict,
            });
        } else if self.identifier_needs_with_resolution(slot) {
            self.emit(Op::StoreIdentWith {
                name: name.to_owned(),
                slot,
                is_strict: self.strict,
            });
        } else {
            self.emit_store_unresolved_identifier(name, slot);
        }
    }

    pub(super) fn emit_store_unresolved_identifier(&mut self, name: &str, slot: Option<usize>) {
        if let Some(slot) = slot {
            self.emit(Op::AssignLocal(slot));
        } else if self.strict {
            self.emit(Op::StoreGlobalStrict(name.to_owned()));
        } else if self.global_scope && self.global_hoisted.contains(name) {
            self.emit(Op::StoreGlobalSloppy(name.to_owned()));
        } else {
            let slot = self.assignment_slot(name);
            self.emit(Op::StoreLocalOrGlobalSloppy {
                slot,
                name: name.to_owned(),
            });
        }
    }

    fn compile_member_compound_assign(
        &mut self,
        target: &AssignmentTarget,
        op: AssignmentOp,
        value: &Expr,
    ) -> Result<(), RuntimeError> {
        if let AssignmentTarget::Member {
            object,
            property: qjs_ast::MemberProperty::Private(name),
            ..
        } = target
        {
            return self.compile_private_member_compound_assign(object, name, op, value);
        }
        let AssignmentTarget::Member {
            object, property, ..
        } = target
        else {
            return Err(unsupported_target(target));
        };
        if matches!(object.as_ref(), Expr::Super { .. }) {
            return self.compile_super_member_compound_assign(property, op, value);
        }
        let object_slot = self.temp_local("assign_object");
        let key_slot = self.temp_local("assign_key");
        let value_slot = self.temp_local("assign_value");
        self.compile_member_reference(object, property, object_slot, key_slot, true)?;
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

    /// Compiles `obj.#x <op>= value` (including `&&=`, `||=`, `??=`). The object
    /// is read once into a temp, the private member read, combined, and written
    /// back; the expression evaluates to the stored value.
    fn compile_private_member_compound_assign(
        &mut self,
        object: &Expr,
        name: &str,
        op: AssignmentOp,
        value: &Expr,
    ) -> Result<(), RuntimeError> {
        let object_slot = self.temp_local("assign_object");
        let value_slot = self.temp_local("assign_value");
        self.compile_expr(object)?;
        self.emit(Op::StoreLocal(object_slot));
        self.emit(Op::LoadLocal(object_slot));
        self.emit(Op::GetPrivate(name.to_owned()));
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
                self.emit_private_member_store(object_slot, name, value_slot);
                let end = self.code.len();
                self.patch_jump(jump, end);
            }
            _ => {
                self.compile_expr(value)?;
                self.emit(Op::Binary(assignment_binary_op(op)?));
                self.emit(Op::StoreLocal(value_slot));
                self.emit_private_member_store(object_slot, name, value_slot);
            }
        }
        Ok(())
    }

    pub(super) fn emit_private_member_store(
        &mut self,
        object_slot: usize,
        name: &str,
        value_slot: usize,
    ) {
        self.emit(Op::LoadLocal(object_slot));
        self.emit(Op::LoadLocal(value_slot));
        self.emit(Op::SetPrivate(name.to_owned()));
    }

    /// Compiles `obj.#x++` / `++obj.#x` (and `--`). The expression evaluates to
    /// the numeric value before (postfix) or after (prefix) the update.
    fn compile_private_member_update(
        &mut self,
        object: &Expr,
        name: &str,
        op: UpdateOp,
        prefix: bool,
    ) -> Result<(), RuntimeError> {
        let object_slot = self.temp_local("update_object");
        let old_slot = self.temp_local("update_old");
        let new_slot = self.temp_local("update_new");
        self.compile_expr(object)?;
        self.emit(Op::StoreLocal(object_slot));
        self.emit(Op::LoadLocal(object_slot));
        self.emit(Op::GetPrivate(name.to_owned()));
        self.emit(Op::ToNumeric);
        self.emit(Op::StoreLocal(old_slot));
        self.emit(Op::LoadLocal(old_slot));
        self.emit(Op::Update(op));
        self.emit(Op::StoreLocal(new_slot));
        self.emit_private_member_store(object_slot, name, new_slot);
        self.emit(Op::Pop);
        self.emit(Op::LoadLocal(if prefix { new_slot } else { old_slot }));
        Ok(())
    }

    fn compile_member_update(
        &mut self,
        target: &AssignmentTarget,
        op: UpdateOp,
        prefix: bool,
    ) -> Result<(), RuntimeError> {
        if let AssignmentTarget::Member {
            object,
            property: qjs_ast::MemberProperty::Private(name),
            ..
        } = target
        {
            return self.compile_private_member_update(object, name, op, prefix);
        }
        let AssignmentTarget::Member {
            object, property, ..
        } = target
        else {
            return Err(unsupported_target(target));
        };
        if matches!(object.as_ref(), Expr::Super { .. }) {
            return self.compile_super_member_update(property, op, prefix);
        }
        let object_slot = self.temp_local("update_object");
        let key_slot = self.temp_local("update_key");
        let old_slot = self.temp_local("update_old");
        let new_slot = self.temp_local("update_new");
        self.compile_member_reference(object, property, object_slot, key_slot, true)?;
        self.emit(Op::LoadLocal(object_slot));
        self.emit(Op::LoadLocal(key_slot));
        self.emit(Op::GetProp);
        self.emit(Op::ToNumeric);
        self.emit(Op::StoreLocal(old_slot));
        self.emit(Op::LoadLocal(old_slot));
        self.emit(Op::Update(op));
        self.emit(Op::StoreLocal(new_slot));
        self.emit_member_store(object_slot, key_slot, new_slot);
        self.emit(Op::Pop);
        self.emit(Op::LoadLocal(if prefix { new_slot } else { old_slot }));
        Ok(())
    }

    pub(super) fn emit_member_store(
        &mut self,
        object_slot: usize,
        key_slot: usize,
        value_slot: usize,
    ) {
        self.emit(Op::LoadLocal(object_slot));
        self.emit(Op::LoadLocal(key_slot));
        self.emit(Op::LoadLocal(value_slot));
        self.emit(Op::SetProp {
            is_strict: self.strict,
        });
    }

    /// Compiles `super.x <op>= value` / `super[k] <op>= value` (including the
    /// short-circuiting `&&=`, `||=`, `??=`). For the computed form the key is
    /// evaluated exactly once and reused for the read and the write-back.
    fn compile_super_member_compound_assign(
        &mut self,
        property: &qjs_ast::MemberProperty,
        op: AssignmentOp,
        value: &Expr,
    ) -> Result<(), RuntimeError> {
        let is_strict = self.strict;
        // `key_slot` only used by the computed form; named uses the op's key.
        let key_slot = match property {
            qjs_ast::MemberProperty::Named(_) => None,
            qjs_ast::MemberProperty::Computed(expr) => {
                let receiver_slot = self.temp_local("super_receiver");
                let base_slot = self.temp_local("super_base");
                let slot = self.temp_local("assign_key");
                self.emit(Op::SuperReference);
                self.emit(Op::StoreLocal(base_slot));
                self.emit(Op::StoreLocal(receiver_slot));
                self.compile_expr(expr)?;
                self.emit(Op::StoreLocal(slot));
                Some((receiver_slot, base_slot, slot))
            }
            qjs_ast::MemberProperty::Private(name) => {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("SyntaxError: super.#{name} is not allowed"),
                });
            }
        };
        let emit_get = |compiler: &mut Self| match property {
            qjs_ast::MemberProperty::Named(name) => {
                compiler.emit(Op::SuperGet { key: name.clone() });
            }
            _ => {
                let (receiver_slot, base_slot, slot) = key_slot.unwrap();
                compiler.emit(Op::LoadLocal(receiver_slot));
                compiler.emit(Op::LoadLocal(base_slot));
                compiler.emit(Op::LoadLocal(slot));
                compiler.emit(Op::SuperGetComputed);
            }
        };
        let emit_set = |compiler: &mut Self, value_slot: usize| match property {
            qjs_ast::MemberProperty::Named(name) => {
                compiler.emit(Op::LoadLocal(value_slot));
                compiler.emit(Op::SuperSet {
                    key: name.clone(),
                    is_strict,
                });
            }
            _ => {
                let (receiver_slot, base_slot, slot) = key_slot.unwrap();
                compiler.emit(Op::LoadLocal(receiver_slot));
                compiler.emit(Op::LoadLocal(base_slot));
                compiler.emit(Op::LoadLocal(slot));
                compiler.emit(Op::LoadLocal(value_slot));
                compiler.emit(Op::SuperSetComputed { is_strict });
            }
        };
        let value_slot = self.temp_local("assign_value");
        emit_get(self);
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
                emit_set(self, value_slot);
                let end = self.code.len();
                self.patch_jump(jump, end);
            }
            _ => {
                self.compile_expr(value)?;
                self.emit(Op::Binary(assignment_binary_op(op)?));
                self.emit(Op::StoreLocal(value_slot));
                emit_set(self, value_slot);
            }
        }
        Ok(())
    }

    /// Compiles `super.x++` / `++super[k]` (and `--`). Evaluates to the numeric
    /// value before (postfix) or after (prefix) the update.
    fn compile_super_member_update(
        &mut self,
        property: &qjs_ast::MemberProperty,
        op: UpdateOp,
        prefix: bool,
    ) -> Result<(), RuntimeError> {
        let is_strict = self.strict;
        let key_slot = match property {
            qjs_ast::MemberProperty::Named(_) => None,
            qjs_ast::MemberProperty::Computed(expr) => {
                let receiver_slot = self.temp_local("super_receiver");
                let base_slot = self.temp_local("super_base");
                let slot = self.temp_local("update_key");
                self.emit(Op::SuperReference);
                self.emit(Op::StoreLocal(base_slot));
                self.emit(Op::StoreLocal(receiver_slot));
                self.compile_expr(expr)?;
                self.emit(Op::StoreLocal(slot));
                Some((receiver_slot, base_slot, slot))
            }
            qjs_ast::MemberProperty::Private(name) => {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("SyntaxError: super.#{name} is not allowed"),
                });
            }
        };
        let old_slot = self.temp_local("update_old");
        let new_slot = self.temp_local("update_new");
        match property {
            qjs_ast::MemberProperty::Named(name) => {
                self.emit(Op::SuperGet { key: name.clone() });
            }
            _ => {
                let (receiver_slot, base_slot, slot) = key_slot.unwrap();
                self.emit(Op::LoadLocal(receiver_slot));
                self.emit(Op::LoadLocal(base_slot));
                self.emit(Op::LoadLocal(slot));
                self.emit(Op::SuperGetComputed);
            }
        }
        self.emit(Op::ToNumeric);
        self.emit(Op::StoreLocal(old_slot));
        self.emit(Op::LoadLocal(old_slot));
        self.emit(Op::Update(op));
        self.emit(Op::StoreLocal(new_slot));
        match property {
            qjs_ast::MemberProperty::Named(name) => {
                self.emit(Op::LoadLocal(new_slot));
                self.emit(Op::SuperSet {
                    key: name.clone(),
                    is_strict,
                });
            }
            _ => {
                let (receiver_slot, base_slot, slot) = key_slot.unwrap();
                self.emit(Op::LoadLocal(receiver_slot));
                self.emit(Op::LoadLocal(base_slot));
                self.emit(Op::LoadLocal(slot));
                self.emit(Op::LoadLocal(new_slot));
                self.emit(Op::SuperSetComputed { is_strict });
            }
        }
        // SuperSet/SuperSetComputed leave the stored value on the stack.
        self.emit(Op::Pop);
        self.emit(Op::LoadLocal(if prefix { new_slot } else { old_slot }));
        Ok(())
    }

    fn compile_member_reference(
        &mut self,
        object: &Expr,
        property: &qjs_ast::MemberProperty,
        object_slot: usize,
        key_slot: usize,
        prepare_for_get: bool,
    ) -> Result<(), RuntimeError> {
        self.compile_expr(object)?;
        self.emit(Op::StoreLocal(object_slot));
        self.compile_member_key(property)?;
        if prepare_for_get {
            self.emit(Op::LoadLocal(object_slot));
            self.emit(Op::RequireObjectCoercible);
            self.emit(Op::Pop);
            self.emit(Op::ToPropertyKey);
        }
        self.emit(Op::StoreLocal(key_slot));
        Ok(())
    }

    pub(super) fn compile_typeof(&mut self, argument: &Expr) -> Result<(), RuntimeError> {
        match argument {
            Expr::Identifier { name, .. } => {
                let slot = self.resolve_local_slot(name);
                if self.identifier_needs_with_resolution(slot) {
                    self.emit(Op::TypeofIdentWith {
                        name: name.clone(),
                        slot,
                    });
                    return Ok(());
                } else if let Some(slot) = slot {
                    self.emit(Op::LoadLocal(slot));
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
