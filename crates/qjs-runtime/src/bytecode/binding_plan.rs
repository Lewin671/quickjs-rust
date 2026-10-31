//! Immutable activation preparation, shared by every instance of a body.
//!
//! Plans contain slot indices and syntax facts, never values or cell identities.
//! Realm, module, mapped-arguments and dynamic-scope cells are resolved by the
//! activation that owns them. Nested code is already prepared at construction.

use qjs_ast::{BindingPattern, FunctionParams};

use super::ir::{Bytecode, Op};
use super::ir_class::{ClassElementDef, ClassPrivateElementDef};

#[derive(Clone, Copy, Debug)]
pub(super) enum InitialSlot {
    Empty,
    Undefined,
    Receiver,
    Arguments,
    ReceiverOrUndefined,
    IncomingOrUndefined,
    IncomingOrRealm,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ParameterInfo {
    pub(super) direct_seedable: bool,
    pub(super) rest_slot: Option<usize>,
    pub(super) mapped_slots: Box<[Option<usize>]>,
}

impl ParameterInfo {
    pub(super) fn compile(bytecode: &Bytecode, params: &FunctionParams) -> Self {
        Self {
            mapped_slots: bytecode
                .parameter_slots()
                .iter()
                .enumerate()
                .map(|(index, &slot)| {
                    (params.is_simple()
                        && !bytecode.locals[slot].name.is_empty()
                        && !bytecode.parameter_slots()[index + 1..].contains(&slot))
                    .then_some(slot)
                })
                .collect(),
            direct_seedable: params.rest.is_none()
                && params
                    .positional
                    .iter()
                    .all(|element| matches!(element.binding, BindingPattern::Identifier { .. })),
            rest_slot: params.rest.as_deref().map(|rest| {
                bytecode
                    .local_slot(&crate::function::rest_parameter_binding_name(rest))
                    .expect("compiler declares the rest argument slot")
            }),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct BindingPlan {
    pub(super) references: super::reference_plan::ReferencePlan,
    pub(super) self_name: Option<std::rc::Rc<crate::binding::FunctionNamePlan>>,
    pub(super) scopes: std::rc::Rc<crate::binding::ScopeDescriptors>,
    pub(super) pc_scopes: Box<[crate::binding::ScopeId]>,
    pub(super) bindings: std::rc::Rc<crate::binding::BindingMetadata>,
    pub(super) root_ordinals: Box<[Option<usize>]>,
    pub(super) local_bindings: Box<[Option<crate::binding::BindingId>]>,
    pub(super) realm_lexical_slots: Box<[bool]>,
    pub(super) has_realm_lexicals: bool,
    pub(super) exposure: crate::binding::EvalExposurePlan,
    pub(super) received: Box<[super::binding_captures::CaptureRoute]>,
    pub(super) received_indices: Box<[Option<usize>]>,
    pub(super) initial_slots: Vec<InitialSlot>,
    pub(super) cell_slots: Vec<usize>,
    pub(super) dynamic_cell_slots: Vec<usize>,
    pub(super) sloppy_global_slots: Vec<usize>,
    pub(super) module_live_candidates: Vec<usize>,
    pub(super) parameters: ParameterInfo,
    pub(super) arguments_slot: Option<usize>,
    pub(super) arguments_body_copy: Option<(usize, usize)>,
    pub(super) dynamic_scope_tree: bool,
    pub(super) needs_receiver: bool,
    pub(super) needs_lexical_owner: bool,
    pub(super) needs_lexical_arguments: bool,
    pub(super) reads_new_target: bool,
    pub(super) prologue_end: Option<usize>,
}

impl BindingPlan {
    pub(super) fn compile(bytecode: &Bytecode) -> Self {
        let initial_slots = bytecode
            .locals
            .iter()
            .map(|local| {
                if local.compiler_temporary {
                    InitialSlot::Undefined
                } else if local.hoisted && bytecode.global_scope {
                    InitialSlot::Empty
                } else if !local.from_env && crate::function::is_call_frame_binding(&local.name) {
                    if local.hoisted {
                        InitialSlot::ReceiverOrUndefined
                    } else {
                        InitialSlot::Receiver
                    }
                } else if local.from_env {
                    if local.hoisted {
                        InitialSlot::IncomingOrUndefined
                    } else if crate::function::is_call_frame_binding(&local.name) {
                        InitialSlot::Receiver
                    } else {
                        InitialSlot::IncomingOrRealm
                    }
                } else if local.hoisted {
                    InitialSlot::Undefined
                } else {
                    InitialSlot::Empty
                }
            })
            .collect();
        let mut cell_slots = super::upvalue_resolver::resolve_upvalues(bytecode).cell_slots;
        cell_slots.sort_unstable();
        cell_slots.dedup();
        let mut dynamic_cell_slots = cell_slots.clone();
        dynamic_cell_slots.extend(bytecode.locals.iter().enumerate().filter_map(
            |(slot, local)| {
                (!(local.compiler_temporary
                    || local.sloppy_global_fallback
                    || bytecode.global_scope && local.hoisted))
                    .then_some(slot)
            },
        ));
        dynamic_cell_slots.sort_unstable();
        dynamic_cell_slots.dedup();
        let dynamic_scope_tree = dynamic_tree(bytecode);
        Self {
            references: super::reference_plan::ReferencePlan::default(),
            self_name: None,
            scopes: std::rc::Rc::default(),
            pc_scopes: Box::default(),
            bindings: std::rc::Rc::default(),
            root_ordinals: Box::default(),
            local_bindings: Box::default(),
            realm_lexical_slots: vec![false; bytecode.locals.len()].into_boxed_slice(),
            has_realm_lexicals: false,
            exposure: crate::binding::EvalExposurePlan::default(),
            received: vec![
                super::binding_captures::CaptureRoute::Named;
                bytecode.received_upvalue_slots().len()
            ]
            .into_boxed_slice(),
            received_indices: {
                let mut indices = vec![None; bytecode.locals.len()];
                for (index, &slot) in bytecode.received_upvalue_slots().iter().enumerate() {
                    indices[slot] = Some(index);
                }
                indices.into_boxed_slice()
            },
            initial_slots,
            cell_slots,
            dynamic_cell_slots,
            sloppy_global_slots: bytecode
                .locals
                .iter()
                .enumerate()
                .filter_map(|(slot, local)| {
                    (!local.compiler_temporary && local.sloppy_global_fallback).then_some(slot)
                })
                .collect(),
            module_live_candidates: bytecode
                .locals
                .iter()
                .enumerate()
                .filter_map(|(slot, local)| {
                    (!local.compiler_temporary && bytecode.local_slot(&local.name) == Some(slot))
                        .then_some(slot)
                })
                .collect(),
            // Expression thunks (notably field initializers) have no parameter
            // AST. Their empty list still satisfies the existing leaf contract.
            arguments_slot: None,
            arguments_body_copy: None,
            parameters: ParameterInfo {
                direct_seedable: bytecode.parameter_slots().is_empty(),
                ..ParameterInfo::default()
            },
            dynamic_scope_tree,
            needs_lexical_arguments: false,
            needs_lexical_owner: bytecode.uses_lexical_this()
                || bytecode.contains_super_operation()
                || bytecode.contains_direct_eval()
                || bytecode.code.iter().any(|op| {
                    matches!(op, Op::LoadNewTarget)
                        || matches!(op, Op::NewFunction { lexical_this: true, bytecode, .. }
                        if bytecode.needs_lexical_owner())
                }),
            needs_receiver: bytecode.uses_lexical_this()
                || bytecode.contains_super_operation()
                || nested_receiver(bytecode)
                || dynamic_scope_tree,
            prologue_end: bytecode
                .code
                .iter()
                .position(|op| matches!(op, Op::FunctionPrologueEnd)),
            reads_new_target: bytecode
                .code
                .iter()
                .any(|op| matches!(op, Op::LoadNewTarget)),
        }
    }
}

/// Only these children execute with the enclosing receiver. Ordinary functions,
/// methods, field initializers and static blocks establish their own receiver;
/// their own plans account for any deferred keys or arrows nested inside them.
fn nested_receiver(bytecode: &Bytecode) -> bool {
    bytecode.code.iter().any(|op| match op {
        Op::NewFunction {
            lexical_this: true,
            bytecode,
            ..
        } => bytecode.needs_call_receiver(),
        _ => false,
    })
}

fn dynamic_tree(bytecode: &Bytecode) -> bool {
    dynamic_code(&bytecode.code)
}

pub(super) fn dynamic_code(code: &[Op]) -> bool {
    code.iter().any(|op| {
        matches!(
            op,
            Op::CallDirectEval { .. }
                | Op::CallDirectEvalSpread { .. }
                | Op::EnterWith
                | Op::ExitWith
        )
    }) || code.iter().any(|op| match op {
        Op::NewFunction { bytecode, .. } => bytecode.has_dynamic_scope_tree(),
        Op::NewClass { definition } => {
            definition.constructor.bytecode.has_dynamic_scope_tree()
                || definition.elements.iter().any(element_dynamic)
                || definition.private_elements.iter().any(private_dynamic)
        }
        _ => false,
    })
}

fn element_dynamic(element: &ClassElementDef) -> bool {
    match element {
        ClassElementDef::Method(def) => def.bytecode.has_dynamic_scope_tree(),
        ClassElementDef::Field(def) => def
            .initializer
            .as_ref()
            .is_some_and(|init| init.bytecode.has_dynamic_scope_tree()),
        ClassElementDef::Private(def) => private_dynamic(def),
        ClassElementDef::StaticBlock(def) => def.bytecode.has_dynamic_scope_tree(),
    }
}

fn private_dynamic(element: &ClassPrivateElementDef) -> bool {
    match element {
        ClassPrivateElementDef::Field { initializer, .. } => initializer
            .as_ref()
            .is_some_and(|init| init.bytecode.has_dynamic_scope_tree()),
        ClassPrivateElementDef::Method { def, .. }
        | ClassPrivateElementDef::Getter { def, .. }
        | ClassPrivateElementDef::Setter { def, .. } => def.bytecode.has_dynamic_scope_tree(),
    }
}

impl BindingPlan {
    pub(super) fn validate(&self, slots: usize, code_len: usize) {
        use crate::binding::{BindingKind, InitPolicy, WritePolicy};
        for (index, binding) in self.bindings.iter().enumerate() {
            debug_assert_eq!(binding.id.0, index);
            debug_assert!(binding.slot.0 < slots);
            debug_assert!(!binding.name.starts_with("\0\0"));
            debug_assert_eq!(
                binding.mutable,
                binding.strict_write == WritePolicy::Mutable
            );
            debug_assert!(
                !binding.deletable
                    || matches!(binding.kind, BindingKind::Var | BindingKind::Function)
            );
            debug_assert_eq!(
                binding.initialization == InitPolicy::Parameter,
                binding.kind == BindingKind::Parameter
            );
        }
        for (index, scope) in self.scopes.iter().enumerate() {
            debug_assert_eq!(scope.id.0, index);
            debug_assert!(scope.parent.is_none_or(|parent| parent.0 < index));
            debug_assert!(scope.variable_target.0 < self.scopes.len());
        }
        for site in &self.exposure.sites {
            debug_assert!(site.pc < code_len);
            debug_assert!(site.scopes.iter().all(|scope| scope.0 < self.scopes.len()));
        }
    }
}

#[cfg(test)]
mod tests;
