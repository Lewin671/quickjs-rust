//! References retain their selected environment across RHS evaluation. Cold
//! banks are live only for the compiler-recorded interval, including suspension.
use super::{
    ir::Op,
    reference_plan::{ReferenceEvent, ReferencePlan},
    vm::Vm,
};
use crate::{RuntimeError, Value, binding::ResolvedReference};

impl<'a, F: crate::bytecode::vm::VmFrame<'a>> Vm<'a, '_, F> {
    /// IP initially names the original operation after this checkpoint. A
    /// scope promotion restarts at the checkpoint before any observable event;
    /// a completed event restarts after its effects, never replaying selection.
    #[inline(never)]
    pub(super) fn run_binding_boundary(
        &mut self,
        program: &super::frame_program::FrameProgramView<'_>,
        event: Option<usize>,
        cleanup: bool,
        authority: u128,
    ) -> Result<Option<super::frame_stack::FrameExit>, RuntimeError> {
        let boundary_pc = self.current.ip - 1;
        let original_pc = self.current.ip;
        let plan = &program.bytecode.binding_plan;
        if let Some(active) = self.current.binding_scope
            && plan
                .pc_scopes
                .get(boundary_pc)
                .is_some_and(|scope| *scope != active)
        {
            crate::diagnostics::count!(binding_boundary_scope_changes);
            self.update_binding_scope(boundary_pc);
            if self.current.authoritative_slots != authority {
                self.current.ip = boundary_pc;
                return Ok(Some(super::frame_stack::FrameExit::RefreshBindings));
            }
        }
        if self.current.reference_hooks {
            let references = &plan.references;
            if cleanup && self.current.active_reference_count != 0 {
                crate::diagnostics::count!(reference_cleanup_boundaries);
                self.prune_references(references, boundary_pc);
            }
            if let Some(index) = event {
                let original =
                    program
                        .execution_code
                        .get(original_pc)
                        .ok_or_else(|| RuntimeError {
                            thrown: None,
                            message: "reference checkpoint has no operation".into(),
                        })?;
                self.current.ip = original_pc + 1;
                let result =
                    self.run_reference_event(references, &references.events[index], original);
                if matches!(self.handle_runtime_result(result)?, Some(false)) {
                    self.current.ip = original_pc;
                }
                if self.current.authoritative_slots != authority {
                    return Ok(Some(super::frame_stack::FrameExit::RefreshBindings));
                }
            }
        }
        Ok(None)
    }

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
        let frame = &mut *self.current;
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
            if plan.start_elidable(*index, &self.env) {
                crate::diagnostics::count!(reference_start_elisions);
                continue;
            }
            crate::diagnostics::count!(dynamic_reference_selections);
            let site = &plan.sites[*index];
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
                        &mut self.context(),
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
