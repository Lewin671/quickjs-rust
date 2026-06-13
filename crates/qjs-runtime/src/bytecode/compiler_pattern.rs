//! Destructuring assignment pattern compilation.
//!
//! Mirrors the binding-pattern paths in `compiler_binding.rs`, but targets
//! assignment references (identifiers, members, and nested patterns) and
//! follows the spec evaluation order: member references are evaluated
//! before the corresponding value is read from the source.

use qjs_ast::{
    AssignmentTarget, AssignmentTargetElement, AssignmentTargetProperty,
    AssignmentTargetPropertyKey, MemberProperty,
};

use crate::{RuntimeError, Value};

use super::compiler::Compiler;
use super::compiler_binding::ArrayDestructuring;
use super::ir::{ObjectRestExclusion, Op};

/// A member-target reference evaluated ahead of the value read.
enum MemberReference {
    Ordinary { object_slot: usize, key_slot: usize },
    Private { object_slot: usize, name: String },
}

impl Compiler {
    /// Destructures the value on top of the stack into the assignment
    /// pattern targets, consuming the value.
    pub(super) fn compile_assignment_pattern(
        &mut self,
        target: &AssignmentTarget,
    ) -> Result<(), RuntimeError> {
        match target {
            AssignmentTarget::Identifier { .. } | AssignmentTarget::Member { .. } => {
                let reference = self.prepare_member_reference(target)?;
                self.store_pattern_value(target, reference.as_ref())
            }
            AssignmentTarget::ArrayPattern { elements, rest, .. } => {
                let destructuring = self.begin_array_destructuring();
                for element in elements {
                    let Some(element) = element else {
                        self.emit_iterator_step(&destructuring);
                        self.emit(Op::Pop);
                        continue;
                    };
                    self.compile_assignment_element(&destructuring, element)?;
                }
                if let Some(rest) = rest {
                    let reference = self.prepare_member_reference(rest)?;
                    self.emit_iterator_rest(&destructuring);
                    self.store_pattern_value(rest, reference.as_ref())?;
                }
                self.end_array_destructuring(&destructuring);
                Ok(())
            }
            AssignmentTarget::ObjectPattern {
                properties, rest, ..
            } => {
                self.emit(Op::RequireObjectCoercible);
                let source_slot = self.temp_local("object_pattern_source");
                self.emit(Op::StoreLocal(source_slot));
                let mut excluded = Vec::with_capacity(properties.len());
                for property in properties {
                    let exclusion = self.compile_assignment_property(source_slot, property)?;
                    excluded.push(exclusion);
                }
                if let Some(rest) = rest {
                    let reference = self.prepare_member_reference(rest)?;
                    self.emit(Op::LoadLocal(source_slot));
                    self.emit(Op::ObjectRestExcluding { excluded });
                    self.store_pattern_value(rest, reference.as_ref())?;
                }
                Ok(())
            }
        }
    }

    fn compile_assignment_element(
        &mut self,
        destructuring: &ArrayDestructuring,
        element: &AssignmentTargetElement,
    ) -> Result<(), RuntimeError> {
        let reference = self.prepare_member_reference(&element.target)?;
        self.emit_iterator_step(destructuring);
        self.compile_binding_default(
            element.default.as_ref(),
            assignment_target_inferred_name(&element.target),
        )?;
        self.store_pattern_value(&element.target, reference.as_ref())
    }

    fn compile_assignment_property(
        &mut self,
        source_slot: usize,
        property: &AssignmentTargetProperty,
    ) -> Result<ObjectRestExclusion, RuntimeError> {
        let exclusion = self.compile_assignment_property_key(&property.key)?;
        let reference = self.prepare_member_reference(&property.target)?;
        self.emit(Op::LoadLocal(source_slot));
        self.load_assignment_property_key(&property.key, &exclusion);
        self.emit(Op::GetProp);
        self.compile_binding_default(
            property.default.as_ref(),
            assignment_target_inferred_name(&property.target),
        )?;
        self.store_pattern_value(&property.target, reference.as_ref())?;
        Ok(exclusion)
    }

    fn compile_assignment_property_key(
        &mut self,
        key: &AssignmentTargetPropertyKey,
    ) -> Result<ObjectRestExclusion, RuntimeError> {
        match key {
            AssignmentTargetPropertyKey::Literal(key) => {
                Ok(ObjectRestExclusion::Literal(key.clone()))
            }
            AssignmentTargetPropertyKey::Computed(expr) => {
                self.compile_expr(expr)?;
                self.emit(Op::ToPropertyKey);
                let slot = self.temp_local("object_pattern_key");
                self.emit(Op::StoreLocal(slot));
                Ok(ObjectRestExclusion::Local(slot))
            }
        }
    }

    fn load_assignment_property_key(
        &mut self,
        key: &AssignmentTargetPropertyKey,
        exclusion: &ObjectRestExclusion,
    ) {
        match key {
            AssignmentTargetPropertyKey::Literal(key) => {
                let key = self.const_slot(Value::String(key.clone()));
                self.emit(Op::LoadConst(key));
            }
            AssignmentTargetPropertyKey::Computed(_) => {
                let ObjectRestExclusion::Local(slot) = exclusion else {
                    unreachable!("computed assignment key should record a local exclusion");
                };
                self.emit(Op::LoadLocal(*slot));
            }
        }
    }

    /// Evaluates a member target's object and key ahead of the value read,
    /// per the destructuring assignment evaluation order.
    fn prepare_member_reference(
        &mut self,
        target: &AssignmentTarget,
    ) -> Result<Option<MemberReference>, RuntimeError> {
        let AssignmentTarget::Member {
            object, property, ..
        } = target
        else {
            return Ok(None);
        };
        let object_slot = self.temp_local("pattern_target_object");
        self.compile_expr(object)?;
        self.emit(Op::StoreLocal(object_slot));
        match property {
            MemberProperty::Private(name) => Ok(Some(MemberReference::Private {
                object_slot,
                name: name.clone(),
            })),
            _ => {
                let key_slot = self.temp_local("pattern_target_key");
                self.compile_member_key(property)?;
                self.emit(Op::StoreLocal(key_slot));
                Ok(Some(MemberReference::Ordinary {
                    object_slot,
                    key_slot,
                }))
            }
        }
    }

    /// Stores the value on top of the stack into the target, consuming it.
    fn store_pattern_value(
        &mut self,
        target: &AssignmentTarget,
        reference: Option<&MemberReference>,
    ) -> Result<(), RuntimeError> {
        match target {
            AssignmentTarget::Identifier { name, .. } => {
                self.emit_pattern_identifier_store(name);
                Ok(())
            }
            AssignmentTarget::Member { .. } => {
                let reference = reference.expect("member reference should be prepared");
                let value_slot = self.temp_local("pattern_target_value");
                self.emit(Op::StoreLocal(value_slot));
                self.emit_pattern_member_store(reference, value_slot);
                self.emit(Op::Pop);
                Ok(())
            }
            AssignmentTarget::ArrayPattern { .. } | AssignmentTarget::ObjectPattern { .. } => {
                self.compile_assignment_pattern(target)
            }
        }
    }

    // (helper defined as a free function below)

    /// Stores the value on top of the stack into an identifier reference,
    /// consuming it.
    fn emit_pattern_identifier_store(&mut self, name: &str) {
        let slot = self.resolve_local_slot(name);
        if slot.is_some() || self.inside_with() {
            self.emit_store_identifier(name, slot);
        } else if self.strict || self.is_global_hoisted(name) {
            self.emit(Op::StoreGlobalStrict(name.to_owned()));
        } else {
            let slot = self.assignment_slot(name);
            self.emit(Op::StoreLocalOrGlobalSloppy {
                slot,
                name: name.to_owned(),
            });
        }
    }

    fn emit_pattern_member_store(&mut self, reference: &MemberReference, value_slot: usize) {
        match reference {
            MemberReference::Ordinary {
                object_slot,
                key_slot,
            } => self.emit_member_store(*object_slot, *key_slot, value_slot),
            MemberReference::Private { object_slot, name } => {
                self.emit_private_member_store(*object_slot, name, value_slot);
            }
        }
    }
}

/// The NamedEvaluation name for a destructuring-assignment default
/// (`[f = function(){}] = []`), or `None` for member or nested-pattern targets,
/// which never name their default value.
fn assignment_target_inferred_name(target: &AssignmentTarget) -> Option<&str> {
    match target {
        AssignmentTarget::Identifier {
            name,
            parenthesized: false,
            ..
        } => Some(name),
        AssignmentTarget::Identifier {
            parenthesized: true,
            ..
        } => None,
        AssignmentTarget::Member { .. }
        | AssignmentTarget::ArrayPattern { .. }
        | AssignmentTarget::ObjectPattern { .. } => None,
    }
}
