//! Identifier-reference evaluation intervals in the existing instruction stream.
//! Sites capture their lexical scope before RHS lowering can enter another scope.
//! The plan owns no values; bank entries exist only while an evaluation is live.
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
        plan.events = events
            .into_iter()
            .map(|(pc, mut event)| {
                event.pc = pc;
                event
            })
            .collect();
        plan
    }

    pub(super) fn can_skip(&self, env: &crate::CallEnv) -> bool {
        self.explicit.is_empty() && self.sites.iter().all(|site| self.stable(site, env))
    }

    pub(super) fn can_skip_range(&self, start: usize, end: usize, env: &crate::CallEnv) -> bool {
        let first = self.sites.partition_point(|site| site.start < start);
        self.sites[first..]
            .iter()
            .take_while(|site| site.start <= end)
            .all(|site| self.stable(site, env))
            && self
                .lifetimes
                .iter()
                .filter(|life| life.temporary.is_some())
                .all(|life| life.start > end || life.end <= start)
    }

    pub(super) fn stable(&self, site: &ReferenceSite, env: &crate::CallEnv) -> bool {
        if matches!(
            site.target,
            ReferenceTarget::With | ReferenceTarget::EvalVariable | ReferenceTarget::Parameter
        ) {
            return false;
        }
        let global_variable = self.global_code || env.has_global_binding_root();
        if env.scope_cursor().is_some() || self.sloppy_eval && !global_variable {
            return false;
        }
        if site.target == ReferenceTarget::Received {
            return true;
        }
        if env.has_module_import(&site.name) {
            return true;
        }
        if env.can_cache_realm_globals() && env.is_global_lexical_binding(&site.name) {
            return true;
        }
        matches!(env.selected_global_this(), Some(crate::Value::Object(object))
            if object.own_property(&site.name).is_some_and(|property| !property.configurable))
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
