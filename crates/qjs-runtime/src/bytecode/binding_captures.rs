//! Link capture inputs once, while parent and child declaration plans are live.
//!
//! Routes follow the child's received-slot order, including unresolved names;
//! activation setup must never compress away a missing route and shift cells.

use super::ir::{Bytecode, ClassElementDef, ClassPrivateElementDef, Local, Op};
use crate::binding::LocalSlot;
use crate::value::name_hash::NameMap;
use std::rc::Rc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CaptureRoute {
    ParentLocal(LocalSlot),
    ParentUpvalue(usize),
    Named,
}

pub(super) fn link_children(code: &mut [Op], locals: &[Local], global_scope: bool) {
    // Match Bytecode::local_slot: lexical captures take precedence below,
    // otherwise the first same-named local supplies the existing fallback.
    let mut slots = NameMap::default();
    for (slot, local) in locals.iter().enumerate() {
        if !(local.compiler_temporary || global_scope && local.hoisted && !local.parameter) {
            slots.entry(local.name.as_str()).or_insert(slot);
        }
    }
    let received: NameMap<_, _> = locals
        .iter()
        .enumerate()
        .filter(|(_, local)| local.is_received_upvalue())
        .enumerate()
        .map(|(received, (slot, _))| (slot, received))
        .collect();
    for op in code {
        match op {
            Op::NewFunction {
                bytecode,
                lexical_captures,
                ..
            } => {
                link_child(bytecode, lexical_captures, locals, &slots, &received);
            }
            Op::NewClass { definition } => {
                let def = Rc::make_mut(definition);
                link_child(
                    &mut def.constructor.bytecode,
                    &def.constructor.lexical_captures,
                    locals,
                    &slots,
                    &received,
                );
                for key in &mut def.computed_keys {
                    if let super::ir::ClassComputedKeyDef::Deferred {
                        bytecode,
                        lexical_captures,
                        ..
                    } = key
                    {
                        link_child(bytecode, lexical_captures, locals, &slots, &received);
                    }
                }
                for element in &mut def.elements {
                    match element {
                        ClassElementDef::Method(def) => link_child(
                            &mut def.bytecode,
                            &def.lexical_captures,
                            locals,
                            &slots,
                            &received,
                        ),
                        ClassElementDef::Field(def) => {
                            if let Some(init) = &mut def.initializer {
                                link_child(
                                    &mut init.bytecode,
                                    &init.lexical_captures,
                                    locals,
                                    &slots,
                                    &received,
                                );
                            }
                        }
                        ClassElementDef::StaticBlock(def) => link_child(
                            &mut def.bytecode,
                            &def.lexical_captures,
                            locals,
                            &slots,
                            &received,
                        ),
                        ClassElementDef::Private(def) => {
                            link_private(def, locals, &slots, &received)
                        }
                    }
                }
                for def in &mut def.private_elements {
                    link_private(def, locals, &slots, &received);
                }
            }
            _ => {}
        }
    }
}

fn link_private(
    def: &mut ClassPrivateElementDef,
    locals: &[Local],
    slots: &NameMap<&str, usize>,
    received: &NameMap<usize, usize>,
) {
    match def {
        ClassPrivateElementDef::Field { initializer, .. } => {
            if let Some(init) = initializer {
                link_child(
                    &mut init.bytecode,
                    &init.lexical_captures,
                    locals,
                    slots,
                    received,
                );
            }
        }
        ClassPrivateElementDef::Method { def, .. }
        | ClassPrivateElementDef::Getter { def, .. }
        | ClassPrivateElementDef::Setter { def, .. } => link_child(
            &mut def.bytecode,
            &def.lexical_captures,
            locals,
            slots,
            received,
        ),
    }
}

fn link_child(
    child: &mut Rc<Bytecode>,
    captures: &[(String, usize)],
    locals: &[Local],
    slots: &NameMap<&str, usize>,
    received: &NameMap<usize, usize>,
) {
    let child = Rc::make_mut(child);
    child.binding_plan.received = child
        .received_upvalue_slots()
        .iter()
        .map(|&slot| {
            let name = child.locals[slot].name.as_str();
            let parent = captures
                .iter()
                .find(|(candidate, _)| candidate == name)
                .map(|(_, slot)| *slot)
                .or_else(|| {
                    slots
                        .get(name)
                        .copied()
                        .filter(|&slot| !locals[slot].compiler_temporary)
                });
            match parent {
                Some(slot) => received
                    .get(&slot)
                    .map_or(CaptureRoute::ParentLocal(LocalSlot(slot)), |&index| {
                        CaptureRoute::ParentUpvalue(index)
                    }),
                None => CaptureRoute::Named,
            }
        })
        .collect();
}
