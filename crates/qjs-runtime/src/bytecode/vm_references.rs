//! References retain their selected environment across RHS evaluation. Cold
//! banks are live only for the compiler-recorded interval, including suspension.
use super::{
    ir::Op,
    reference_plan::{ReferenceEvent, ReferencePlan},
    vm::Vm,
};
use crate::{RuntimeError, Value, binding::ResolvedReference};

impl Vm<'_> {
    pub(super) fn save_reference(
        &mut self,
        bank: usize,
        lifetime: usize,
        reference: ResolvedReference,
    ) {
        let cold = self.cold_mut();
        if cold.resolved_bindings.len() <= bank {
            cold.resolved_bindings.resize_with(bank + 1, || None);
        }
        cold.resolved_bindings[bank] = Some(reference);
        if !cold.active_reference_lifetimes.contains(&lifetime) {
            cold.active_reference_lifetimes.push(lifetime);
        }
        self.active_reference_count = self
            .cold()
            .expect("reference bank")
            .active_reference_lifetimes
            .len();
    }

    pub(super) fn prune_references(&mut self, plan: &ReferencePlan, pc: usize) {
        if self.active_reference_count == 0 {
            return;
        }
        let frame = &mut self.current;
        let cold = frame.cold.as_mut().expect("live reference bank");
        cold.active_reference_lifetimes.retain(|index| {
            let life = &plan.lifetimes[*index];
            let live = pc >= life.start
                && pc < life.end
                && cold
                    .resolved_bindings
                    .get(life.bank)
                    .is_some_and(Option::is_some);
            if !live {
                if let Some(bank) = cold.resolved_bindings.get_mut(life.bank) {
                    *bank = None;
                }
                if let Some(slot) = life.temporary {
                    frame.locals[slot] = Some(Value::Undefined);
                }
            }
            live
        });
        frame.active_reference_count = cold.active_reference_lifetimes.len();
    }

    /// `true` means this event executed the original load/store itself.
    pub(super) fn run_reference_event(
        &mut self,
        plan: &ReferencePlan,
        event: &ReferenceEvent,
        op: &Op,
    ) -> Result<bool, RuntimeError> {
        for index in &event.starts {
            let site = &plan.sites[*index];
            if plan.stable(site, &self.env) {
                continue;
            }
            let life = &plan.lifetimes[site.lifetime];
            if self
                .cold()
                .and_then(|cold| cold.resolved_bindings.get(life.bank))
                .is_some_and(Option::is_some)
            {
                continue;
            }
            let reference = self.select_binding_reference(&site.name, site.strict, site.scope)?;
            self.save_reference(life.bank, site.lifetime, reference);
        }
        if let Some(index) = event.store {
            let site = &plan.sites[index];
            let bank = plan.lifetimes[site.lifetime].bank;
            if let Some(reference) = self
                .cold
                .as_mut()
                .and_then(|cold| cold.resolved_bindings.get_mut(bank))
                .and_then(Option::take)
            {
                if site.append {
                    let (suffix, discard) = match op {
                        Op::AppendStringLiteralLocal { value, discard, .. }
                        | Op::AppendStringLiteralGlobal { value, discard, .. } => (value, discard),
                        _ => unreachable!("append reference opcode"),
                    };
                    let left = self.read_scope_binding(&reference)?;
                    let value = crate::operations::eval_binary(
                        left,
                        qjs_ast::BinaryOp::Add,
                        Value::String(suffix.clone().into()),
                        &mut self.current_env(),
                    )?;
                    self.assign_scope_binding(&reference, value.clone(), site.strict)?;
                    if !discard {
                        self.stack.push(value);
                    }
                } else {
                    let value = self.stack.pop().ok_or_else(|| RuntimeError {
                        thrown: None,
                        message: "reference store stack underflow".into(),
                    })?;
                    self.assign_scope_binding(&reference, value, site.strict)?;
                }
                return Ok(true);
            }
        }
        if let Some(index) = event.read {
            let site = &plan.sites[index];
            let bank = plan.lifetimes[site.lifetime].bank;
            if let Some(reference) = self
                .cold()
                .and_then(|cold| cold.resolved_bindings.get(bank))
                .and_then(Option::as_ref)
                .cloned()
            {
                let value = self.read_scope_binding(&reference)?;
                self.stack.push(value);
                return Ok(true);
            }
        }
        Ok(false)
    }
}
