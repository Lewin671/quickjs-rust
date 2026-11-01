//! Identifier-reference evaluation intervals in the existing instruction stream.
//! Sites capture their lexical scope before RHS lowering can enter another scope.
//! The plan owns no values; bank entries exist only while an evaluation is live.
//! Runtime elision of those intervals is decided in `reference_admission`.
use super::ir::{Bytecode, Op};
use crate::binding::{BindingKind, ScopeId, ScopeKind};

#[derive(Clone, Copy, Debug)]
pub(super) struct ReferenceId(pub(super) usize);

#[derive(Clone, Debug)]
pub(super) struct ReferenceDraft {
    pub(super) name: String,
    pub(super) start: usize,
    pub(super) scope: ScopeId,
    pub(super) strict: bool,
    pub(super) read: Option<usize>,
    pub(super) store: Option<usize>,
    pub(super) end: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum ReferenceTarget {
    Received,
    Parameter,
    EvalVariable,
    Name,
    With,
}

#[derive(Clone, Debug)]
pub(super) struct ReferenceSite {
    pub(super) name: String,
    pub(super) start: usize,
    pub(super) scope: ScopeId,
    pub(super) strict: bool,
    pub(super) target: ReferenceTarget,
    pub(super) lifetime: usize,
    pub(super) append: bool,
    /// A global Name site with no read at its own start. Only such a site
    /// can observe its start resolvability later: a site that reads at its
    /// start throws there when unresolvable, before any RHS effect.
    pub(super) checked: bool,
}

#[derive(Clone, Debug)]
pub(super) struct ReferenceLifetime {
    pub(super) bank: usize,
    pub(super) start: usize,
    pub(super) end: usize,
    pub(super) temporary: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ReferenceEvent {
    pub(super) pc: usize,
    pub(super) starts: Vec<usize>,
    pub(super) read: Option<usize>,
    pub(super) store: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub(super) struct ReferencePlan {
    pub(super) sites: Vec<ReferenceSite>,
    pub(super) lifetimes: Vec<ReferenceLifetime>,
    pub(super) explicit: Vec<Option<usize>>,
    pub(super) events: Vec<ReferenceEvent>,
    pub(super) sloppy_eval: bool,
    pub(super) global_code: bool,
    /// Any with, eval-variable or parameter site, or explicit with reference:
    /// these always select through the VM's dynamic resolver.
    pub(super) dynamic: bool,
    /// Whether any site is `checked`.
    pub(super) has_checked: bool,
    /// Structural admission proofs (`reference_admission`); never values.
    pub(super) admission: std::cell::RefCell<Option<super::reference_admission::AdmissionCache>>,
}

impl ReferencePlan {
    pub(super) fn compile(bytecode: &Bytecode, drafts: Vec<ReferenceDraft>) -> Self {
        let mut plan = Self {
            sloppy_eval: bytecode.code.iter().any(|op| {
                matches!(
                    op,
                    Op::CallDirectEval {
                        is_strict: false,
                        ..
                    } | Op::CallDirectEvalSpread {
                        is_strict: false,
                        ..
                    }
                )
            }),
            global_code: bytecode.global_scope,
            ..Self::default()
        };
        let mut events = std::collections::BTreeMap::<usize, ReferenceEvent>::new();
        for draft in drafts {
            let store = draft.store.expect("finished reference site");
            let Some(target) = target_kind(bytecode, &draft, store) else {
                continue;
            };
            let index = plan.sites.len();
            let lifetime = plan.lifetimes.len();
            plan.lifetimes.push(ReferenceLifetime {
                bank: bytecode.locals.len() + index,
                start: draft.start,
                end: draft.end,
                temporary: None,
            });
            let append = matches!(
                bytecode.code[store],
                Op::AppendStringLiteralLocal { .. } | Op::AppendStringLiteralGlobal { .. }
            );
            events.entry(draft.start).or_default().starts.push(index);
            if let Some(read) = draft.read {
                events.entry(read).or_default().read = Some(index);
            }
            events.entry(store).or_default().store = Some(index);
            plan.sites.push(ReferenceSite {
                name: draft.name,
                start: draft.start,
                scope: draft.scope,
                strict: draft.strict,
                target,
                lifetime,
                append,
                checked: target == ReferenceTarget::Name && draft.read != Some(draft.start),
            });
        }
        // Existing explicit with references already have producing/consuming
        // opcodes. Derive their lifetime by the compiler's unique bank index,
        // not by matching an instruction template.
        let mut consumers =
            std::collections::BTreeMap::<usize, (Option<usize>, Option<usize>)>::new();
        for (pc, op) in bytecode.code.iter().enumerate() {
            match op {
                Op::LoadResolvedIdentWith { object_slot, .. } => {
                    consumers.entry(*object_slot).or_default().0 = Some(pc)
                }
                Op::StoreResolvedIdentWith { object_slot, .. } => {
                    consumers.entry(*object_slot).or_default().1 = Some(pc)
                }
                _ => {}
            }
        }
        for (pc, op) in bytecode.code.iter().enumerate() {
            let Op::ResolveIdentWith { object_slot, .. } = op else {
                continue;
            };
            let (read, store) = consumers[object_slot];
            let end = store.or(read).expect("explicit reference has a consumer") + 1;
            if plan.explicit.len() <= *object_slot {
                plan.explicit.resize(*object_slot + 1, None);
            }
            let lifetime = plan.lifetimes.len();
            plan.explicit[*object_slot] = Some(lifetime);
            plan.lifetimes.push(ReferenceLifetime {
                bank: *object_slot,
                start: pc,
                end,
                temporary: Some(*object_slot),
            });
        }
        plan.dynamic = !plan.explicit.is_empty()
            || plan.sites.iter().any(|site| {
                matches!(
                    site.target,
                    ReferenceTarget::With
                        | ReferenceTarget::EvalVariable
                        | ReferenceTarget::Parameter
                )
            });
        plan.has_checked = plan.sites.iter().any(|site| site.checked);
        plan.events = events
            .into_iter()
            .map(|(pc, mut event)| {
                event.pc = pc;
                event
            })
            .collect();
        plan
    }
}

fn target_kind(
    bytecode: &Bytecode,
    draft: &ReferenceDraft,
    store: usize,
) -> Option<ReferenceTarget> {
    let slot = match &bytecode.code[store] {
        Op::StoreIdentWith { .. } | Op::StoreResolvedIdentWith { .. } => {
            return Some(ReferenceTarget::With);
        }
        Op::StoreLocal(slot)
        | Op::AssignLocal(slot)
        | Op::AppendStringLiteralLocal { slot, .. } => Some(*slot),
        _ => None,
    };
    let Some(slot) = slot else {
        return Some(ReferenceTarget::Name);
    };
    let local = &bytecode.locals[slot];
    if local.is_received_upvalue() {
        return Some(ReferenceTarget::Received);
    }
    if bytecode.binding_plan.scopes[0].kind == ScopeKind::Eval && local.hoisted {
        return Some(ReferenceTarget::EvalVariable);
    }
    let parameter = local.parameter || bytecode.binding_plan.arguments_slot == Some(slot);
    let body = bytecode.binding_plan.scopes[draft.scope.0]
        .variable_target
        .0
        != 0;
    if parameter && body && bytecode.contains_direct_eval() {
        return Some(ReferenceTarget::Parameter);
    }
    let kind = bytecode.binding_plan.local_bindings[slot]
        .map(|id| bytecode.binding_plan.bindings[id.0].kind);
    debug_assert!(!matches!(kind, Some(BindingKind::Received)));
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lhs_scope_is_recorded_before_rhs_enters_class_name_scope() {
        let script =
            qjs_parser::parse_script("let x=1; x=class x { [(eval('0'),'m')](){} };").unwrap();
        let bytecode = crate::bytecode::compiler::compile_script(&script).unwrap();
        let site = bytecode
            .binding_plan
            .references
            .sites
            .iter()
            .find(|site| site.name == "x");
        // An owned lexical needs no saved reference at all.
        assert!(site.is_none());
        let script = qjs_parser::parse_script("x=class x { [(eval('0'),'m')](){} };").unwrap();
        let bytecode = crate::bytecode::compiler::compile_script(&script).unwrap();
        let site = bytecode
            .binding_plan
            .references
            .sites
            .iter()
            .find(|site| site.name == "x")
            .unwrap();
        assert_eq!(site.scope, ScopeId(0));
        assert_ne!(bytecode.binding_plan.pc_scopes[site.start], site.scope);
        let lifetime = &bytecode.binding_plan.references.lifetimes[site.lifetime];
        assert!(lifetime.bank >= bytecode.locals.len());
        assert!(lifetime.end > lifetime.start);
    }
}
