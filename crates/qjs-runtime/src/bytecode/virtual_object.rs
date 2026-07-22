//! Control-flow and alias analysis foundation for virtual object lowering.
//!
//! This pass is intentionally analysis-only. It proves which literal
//! allocation sites have no observable identity before a later change teaches
//! the VM how to scalar-replace them. Keeping the proof separate from the
//! rewrite makes every unsupported bytecode effect fail closed and prevents a
//! benchmark-shaped instruction matcher from becoming a semantic shortcut.

#![allow(dead_code)]

use std::{collections::BTreeSet, rc::Rc};

use crate::value::ObjectLiteralShape;

use super::ir::{Bytecode, Op};

mod cfg;
mod flow;
#[cfg(test)]
mod tests;

use cfg::ControlFlowGraph;

type CandidateId = usize;

#[derive(Clone, Debug)]
pub(super) struct VirtualObjectAnalysis {
    pub(super) cfg: ControlFlowGraph,
    pub(super) slot_authority: SlotAuthority,
    pub(super) candidates: Vec<VirtualCandidate>,
    pub(super) complete: bool,
}

#[derive(Clone, Debug)]
pub(super) struct VirtualCandidate {
    pub(super) allocation_ip: usize,
    pub(super) kind: VirtualKind,
    pub(super) uses: BTreeSet<VirtualUse>,
    pub(super) escape_reasons: BTreeSet<EscapeReason>,
    seen: bool,
}

impl VirtualCandidate {
    pub(super) fn is_virtualizable(&self) -> bool {
        self.seen && self.escape_reasons.is_empty()
    }
}

#[derive(Clone, Debug)]
pub(super) enum VirtualKind {
    Object(Rc<ObjectLiteralShape>),
    DenseArray { length: usize },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum VirtualUse {
    Alias { ip: usize, slot: usize },
    Discard { ip: usize },
    FieldRead { ip: usize, input: usize },
    FieldWrite { ip: usize, input: usize },
    ObjectGuard { ip: usize },
    ArrayLengthRead { ip: usize },
    ElementRead { ip: usize, index: usize },
    ElementWrite { ip: usize, index: usize },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) enum EscapeReason {
    AmbiguousAlias { ip: usize },
    DynamicScope,
    HomeObjectSideEffect { ip: usize },
    IdentityUse { ip: usize },
    InvalidControlFlow,
    InvalidStack { ip: usize },
    StoredInAggregate { ip: usize },
    Suspension { ip: usize },
    UnknownProperty { ip: usize },
    UnsafeSlot { ip: usize, slot: usize },
    UnsupportedInstruction { ip: usize },
    Unreachable,
}

#[derive(Clone, Debug)]
pub(super) struct SlotAuthority {
    authoritative: Vec<bool>,
    pub(super) dynamic_scope: bool,
}

impl SlotAuthority {
    fn for_bytecode(bytecode: &Bytecode) -> Self {
        // A descendant direct eval can resolve and mutate an outer binding by
        // name even when the immediate body contains no eval opcode. Until
        // deopt-exposed slots are represented explicitly in the IR, reject
        // every local alias in that outer body as well.
        let dynamic_scope = bytecode_exposes_dynamic_scope(bytecode);
        let mut captured = vec![false; bytecode.locals.len()];
        let mut class_may_capture = false;
        for op in &bytecode.code {
            match op {
                Op::NewFunction {
                    lexical_captures, ..
                } => {
                    for (_, slot) in lexical_captures {
                        if let Some(captured) = captured.get_mut(*slot) {
                            *captured = true;
                        }
                    }
                }
                Op::FreshIterationScope(slots) => {
                    for slot in slots {
                        if let Some(captured) = captured.get_mut(*slot) {
                            *captured = true;
                        }
                    }
                }
                // Class element capture metadata is distributed across the
                // constructor and element thunks. Until it is normalized into
                // one table, reject local aliases in a body containing a
                // class rather than missing a less common capture route.
                Op::NewClass { .. } => class_may_capture = true,
                _ => {}
            }
        }
        let authoritative = (0..bytecode.locals.len())
            .map(|slot| {
                !bytecode.global_scope
                    && !dynamic_scope
                    && !class_may_capture
                    && !captured[slot]
                    && !bytecode.local_is_parameter(slot)
                    && !bytecode.locals[slot].is_received_upvalue()
                    && !bytecode.local_is_sloppy_global_fallback(slot)
                    && !bytecode.local_is_eval_deletable(slot)
            })
            .collect();
        Self {
            authoritative,
            dynamic_scope,
        }
    }

    pub(super) fn is_authoritative(&self, slot: usize) -> bool {
        self.authoritative.get(slot).copied().unwrap_or(false)
    }

    /// Assignment lowering may only treat a function-scoped mutable binding
    /// as an unconditional slot write. Lexical bindings can still be in TDZ,
    /// and immutable bindings must preserve the runtime throw/no-write path.
    /// Declaration initialization remains represented by `StoreLocal` and is
    /// analyzed separately.
    pub(super) fn is_assignment_authoritative(&self, bytecode: &Bytecode, slot: usize) -> bool {
        self.is_authoritative(slot)
            && bytecode
                .locals
                .get(slot)
                .is_some_and(|local| local.mutable && local.hoisted)
    }
}

fn bytecode_exposes_dynamic_scope(bytecode: &Bytecode) -> bool {
    bytecode.code.iter().any(|op| match op {
        Op::CallDirectEval { .. }
        | Op::CallDirectEvalSpread { .. }
        | Op::EnterWith
        | Op::ExitWith
        | Op::LoadIdentWith { .. }
        | Op::ResolveIdentWith { .. }
        | Op::LoadResolvedIdentWith { .. }
        | Op::StoreIdentWith { .. }
        | Op::StoreResolvedIdentWith { .. }
        | Op::TypeofIdentWith { .. }
        | Op::DeleteIdentWith { .. } => true,
        Op::NewFunction { bytecode, .. } => bytecode_exposes_dynamic_scope(bytecode),
        _ => false,
    })
}
