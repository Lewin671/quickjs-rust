//! Destructuring assignment pattern compilation.
//!
//! Mirrors the binding-pattern paths in `compiler_binding.rs`, but targets
//! assignment references (identifiers, members, and nested patterns) and
//! follows the spec evaluation order: member references are evaluated
//! before the corresponding value is read from the source.

use qjs_ast::{AssignmentTarget, AssignmentTargetElement, AssignmentTargetProperty};

use crate::{RuntimeError, Value};

use super::compiler::Compiler;
use super::compiler_binding::ArrayDestructuring;
use super::ir::Op;

/// A member-target reference evaluated ahead of the value read.
struct MemberReference {
    object_slot: usize,
    key_slot: usize,
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
                for property in properties {
                    self.compile_assignment_property(source_slot, property)?;
                }
                if let Some(rest) = rest {
                    let reference = self.prepare_member_reference(rest)?;
                    self.emit(Op::LoadLocal(source_slot));
                    self.emit(Op::ObjectRestExcluding {
                        excluded: properties
                            .iter()
                            .map(|property| property.key.clone())
                            .collect(),
                    });
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
    ) -> Result<(), RuntimeError> {
        let reference = self.prepare_member_reference(&property.target)?;
        self.emit(Op::LoadLocal(source_slot));
        let key = self.const_slot(Value::String(property.key.clone()));
        self.emit(Op::LoadConst(key));
        self.emit(Op::GetProp);
        self.compile_binding_default(
            property.default.as_ref(),
            assignment_target_inferred_name(&property.target),
        )?;
        self.store_pattern_value(&property.target, reference.as_ref())
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
        let key_slot = self.temp_local("pattern_target_key");
        self.compile_expr(object)?;
        self.emit(Op::StoreLocal(object_slot));
        self.compile_member_key(property)?;
        self.emit(Op::StoreLocal(key_slot));
        Ok(Some(MemberReference {
            object_slot,
            key_slot,
        }))
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
                self.emit_member_store(reference.object_slot, reference.key_slot, value_slot);
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
        if let Some(slot) = self.resolve_local_slot(name) {
            self.emit(Op::StoreLocal(slot));
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
}

/// The NamedEvaluation name for a destructuring-assignment default
/// (`[f = function(){}] = []`), or `None` for member or nested-pattern targets,
/// which never name their default value.
fn assignment_target_inferred_name(target: &AssignmentTarget) -> Option<&str> {
    match target {
        AssignmentTarget::Identifier { name, .. } => Some(name),
        AssignmentTarget::Member { .. }
        | AssignmentTarget::ArrayPattern { .. }
        | AssignmentTarget::ObjectPattern { .. } => None,
    }
}
