//! Compiler provenance for initialized mutable captures. A value observed at
//! runtime never creates this proof; named/dynamic/import routes stay checked.
use super::{binding_captures::CaptureRoute, ir::Bytecode, ir_class::*};
use crate::binding::{BindingCell, BindingKind, WritePolicy};
use std::rc::Rc;

pub(super) fn finish(body: &mut Bytecode, params: Option<&qjs_ast::FunctionParams>) {
    if !body.creates_closures()
        || body.global_scope
        || body.is_direct_eval_code()
        || body.binding_plan.dynamic_scope_tree
        || !params.is_none_or(qjs_ast::FunctionParams::is_simple)
    {
        return;
    }
    let copy_target = body
        .binding_plan
        .arguments_body_copy
        .map(|(_, target)| target);
    for meta in Rc::make_mut(&mut body.binding_plan.bindings).0.iter_mut() {
        if !meta.deletable
            && meta.mutable
            && meta.strict_write == WritePolicy::Mutable
            && matches!(meta.kind, BindingKind::Var | BindingKind::Parameter)
            && copy_target != Some(meta.slot.0)
        {
            meta.strict_write = WritePolicy::InitializedMutable;
        }
    }
    body.binding_plan.initialized_mutable_cells = body
        .binding_plan
        .bindings
        .iter()
        .filter(|meta| {
            meta.strict_write == WritePolicy::InitializedMutable
                && meta.slot.0 < 128
                && body.binding_plan.cell_slots.contains(&meta.slot.0)
        })
        .fold(0, |mask, meta| mask | (1_u128 << meta.slot.0));
    propagate(body);
}

fn propagate(parent: &mut Bytecode) {
    // Incoming capabilities begin false and linking only adds static proofs.
    // No child needs relinking when this body supplies no provable source.
    if !parent
        .binding_plan
        .bindings
        .iter()
        .any(|m| m.strict_write == WritePolicy::InitializedMutable)
        && parent.binding_plan.received_writes == 0
    {
        return;
    }
    let local: Vec<bool> = parent
        .binding_plan
        .local_bindings
        .iter()
        .map(|id| {
            id.is_some_and(|id| {
                parent.binding_plan.bindings[id.0].strict_write == WritePolicy::InitializedMutable
            })
        })
        .collect();
    let received = parent.binding_plan.received_writes;
    let static_parent = !parent.binding_plan.dynamic_scope_tree;
    children(parent, |child| {
        let writes = child
            .binding_plan
            .received
            .iter()
            .take(128)
            .enumerate()
            .fold(0, |mask, (index, route)| {
                let proved = static_parent
                    && !child.binding_plan.dynamic_scope_tree
                    && match route {
                        CaptureRoute::ParentLocal(slot) => {
                            local.get(slot.0).copied().unwrap_or(false)
                        }
                        CaptureRoute::ParentUpvalue(index) => {
                            *index < 128 && received & (1_u128 << index) != 0
                        }
                        CaptureRoute::Named => false,
                    };
                mask | if proved { 1_u128 << index } else { 0 }
            });
        // A child already linked its own declarations while being compiled.
        // Only changed incoming proofs can affect its forwarded descendants.
        if writes != child.binding_plan.received_writes {
            let child = Rc::make_mut(child);
            child.binding_plan.received_writes = writes;
            let received_cells = child
                .received_upvalue_slots()
                .iter()
                .take(128)
                .enumerate()
                .filter(|&(index, &slot)| slot < 128 && writes & (1_u128 << index) != 0)
                .fold(0, |mask, (_, &slot)| mask | (1_u128 << slot));
            child.binding_plan.initialized_mutable_cells |= received_cells;
            propagate(child);
        }
    });
}

impl Bytecode {
    /// Activation-local links include own cells and positional received cells.
    /// A scope/name lookup never establishes this capability; initialization
    /// state merely validates the compiler's already selected route.
    pub(super) fn link_frame_write_capabilities(&self, cells: &[Option<BindingCell>]) -> u128 {
        let mut eligible = self.binding_plan.initialized_mutable_cells;
        let mut mask = 0;
        while eligible != 0 {
            let slot = eligible.trailing_zeros() as usize;
            eligible &= eligible - 1;
            let Some(cell) = cells.get(slot).and_then(Option::as_ref) else {
                continue;
            };
            let name = self.binding_plan.local_bindings[slot]
                .map_or(self.locals[slot].name.as_str(), |id| {
                    self.binding_plan.bindings[id.0].name.as_str()
                });
            if cell.validates_initialized_mutable(name) {
                mask |= 1_u128 << slot;
            }
        }
        mask
    }

    pub(crate) fn received_write_capabilities(&self) -> &u128 {
        &self.binding_plan.received_writes
    }

    /// Validate the shared compiler mask against this instance's actual cells.
    /// The function enables it only when every selected link validates.
    pub(crate) fn link_write_capabilities(&self, cells: &[BindingCell]) -> u128 {
        if self.binding_plan.received_writes == 0 {
            return 0;
        }
        self.received_upvalue_slots()
            .iter()
            .zip(cells)
            .take(128)
            .enumerate()
            .filter_map(|(index, (&slot, cell))| {
                if self.binding_plan.received_writes & (1_u128 << index) != 0
                    && cell.validates_initialized_mutable(&self.locals[slot].name)
                {
                    debug_assert!(cell.metadata().strict_write == WritePolicy::InitializedMutable);
                    Some(1_u128 << index)
                } else {
                    None
                }
            })
            .fold(0, |mask, bit| mask | bit)
    }
}

fn children(parent: &mut Bytecode, mut visit: impl FnMut(&mut Rc<Bytecode>)) {
    use super::ir::Op;
    for op in &mut parent.code {
        match op {
            Op::NewFunction { bytecode, .. } => visit(bytecode),
            Op::NewClass { definition } => {
                let def = Rc::make_mut(definition);
                visit(&mut def.constructor.bytecode);
                for element in &mut def.elements {
                    match element {
                        ClassElementDef::Method(d) => visit(&mut d.bytecode),
                        ClassElementDef::Field(d) => {
                            if let Some(d) = &mut d.initializer {
                                visit(&mut d.bytecode);
                            }
                        }
                        ClassElementDef::StaticBlock(d) => visit(&mut d.bytecode),
                        ClassElementDef::Private(d) => private(d, &mut visit),
                    }
                }
                for d in &mut def.private_elements {
                    private(d, &mut visit);
                }
            }
            _ => {}
        }
    }
}
fn private(def: &mut ClassPrivateElementDef, visit: &mut impl FnMut(&mut Rc<Bytecode>)) {
    match def {
        ClassPrivateElementDef::Field { initializer, .. } => {
            if let Some(d) = initializer {
                visit(&mut d.bytecode);
            }
        }
        ClassPrivateElementDef::Method { def, .. }
        | ClassPrivateElementDef::Getter { def, .. }
        | ClassPrivateElementDef::Setter { def, .. } => visit(&mut def.bytecode),
    }
}
