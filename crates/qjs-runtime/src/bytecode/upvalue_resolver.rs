//! Upvalue classification for the environment-model rewrite (T016 / S1).
//!
//! Given a compiled frame, decide — once, at compile time — which of its
//! bindings must live in shared [`Upvalue`](crate::function) cells and how each
//! nested function sources the bindings it closes over, indexed rather than
//! name-keyed. This is the pure analysis that the cell-slot migration
//! (`docs/design/env-model-rewrite.md`, S2+) consumes; it is introduced ahead of
//! its first consumer, so the items are `dead_code`-allowed until then.
//!
//! ## What the existing data already tells us
//!
//! Each nested `Op::NewFunction` already carries `lexical_captures:
//! Vec<(storage_name, parent_slot)>` — the enclosing-frame slots it references.
//! Combined with the enclosing frame's `Local::from_env` flags, that is enough
//! to classify everything *within one parent frame*:
//!
//! - A captured parent slot with `from_env == true` is a binding the parent
//!   *itself* received as a cell from *its* parent — already an upvalue, so the
//!   child sources it as [`UpvalueSource::ParentUpvalue`] by the parent's
//!   upvalue index.
//! - A captured parent slot with `from_env == false` is a genuine local of the
//!   parent that must be boxed into a fresh cell at frame entry — the child
//!   sources it as [`UpvalueSource::ParentLocal`].
//!
//! ## Resolution scope
//!
//! This pass covers `Op::NewFunction` captures. Class method captures attach
//! parent slots lazily when `Op::NewClass` executes, through the same
//! `captured_upvalues_for_function` path; they do not contribute to this
//! frame-entry plan. `var`-channel captures remain realm-backed until that
//! channel moves onto cells, as documented in the design doc.

use super::ir::{Bytecode, Local, Op};

/// Where a nested function reads one captured binding from, resolved against the
/// enclosing frame at closure-creation time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code)] // Consumed starting at T016 S2; see module docs.
pub(super) enum UpvalueSource {
    /// A non-captured local slot of the enclosing frame, boxed into a cell.
    ParentLocal(usize),
    /// One of the enclosing frame's own received upvalue cells, by upvalue index.
    ParentUpvalue(u16),
}

/// The per-frame upvalue plan.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[allow(dead_code)] // Consumed starting at T016 S2; see module docs.
pub(super) struct UpvaluePlan {
    /// This frame's own `from_env` slots, in slot order. The index into this
    /// `Vec` is the frame's upvalue index — the cells it receives from its
    /// parent and re-exposes to its own children.
    pub(super) upvalue_slots: Vec<usize>,
    /// This frame's non-`from_env` local slots captured by at least one nested
    /// function — the slots boxed into fresh cells at frame entry. Sorted and
    /// deduplicated.
    pub(super) cell_slots: Vec<usize>,
    /// For each nested `Op::NewFunction` in code order, that function's capture
    /// sources, parallel to its `lexical_captures`.
    pub(super) child_sources: Vec<Vec<UpvalueSource>>,
}

/// Classifies the upvalue plan for a compiled frame. Pure; depends only on the
/// frame's `locals` flags and its nested functions' `lexical_captures`.
#[allow(dead_code)] // Consumed starting at T016 S2; see module docs.
pub(super) fn resolve_upvalues(bytecode: &Bytecode) -> UpvaluePlan {
    let children: Vec<&[(String, usize)]> = bytecode
        .code
        .iter()
        .filter_map(|op| match op {
            Op::NewFunction {
                lexical_captures, ..
            } => Some(lexical_captures.as_slice()),
            _ => None,
        })
        .collect();
    resolve_from_parts(&bytecode.locals, &children)
}

/// Core classification, factored out of [`resolve_upvalues`] so it can be tested
/// without constructing whole `Op::NewFunction` payloads. `children` is each
/// nested function's `lexical_captures`, in code order.
#[allow(dead_code)] // Consumed starting at T016 S2; see module docs.
fn resolve_from_parts(locals: &[Local], children: &[&[(String, usize)]]) -> UpvaluePlan {
    let upvalue_slots: Vec<usize> = locals
        .iter()
        .enumerate()
        .filter(|(_, local)| local.from_env)
        .map(|(slot, _)| slot)
        .collect();
    let upvalue_index = |slot: usize| -> u16 {
        // Total by construction: `upvalue_slots` is exactly the `from_env`
        // slots, and this is only reached for a `from_env` slot.
        upvalue_slots
            .iter()
            .position(|&candidate| candidate == slot)
            .map_or(0, |index| index as u16)
    };

    let mut cell_slots: Vec<usize> = Vec::new();
    let mut child_sources: Vec<Vec<UpvalueSource>> = Vec::with_capacity(children.len());
    for captures in children {
        let mut sources = Vec::with_capacity(captures.len());
        for (_storage_name, slot) in *captures {
            let slot = *slot;
            if locals.get(slot).is_some_and(|local| local.from_env) {
                sources.push(UpvalueSource::ParentUpvalue(upvalue_index(slot)));
            } else {
                if !cell_slots.contains(&slot) {
                    cell_slots.push(slot);
                }
                sources.push(UpvalueSource::ParentLocal(slot));
            }
        }
        child_sources.push(sources);
    }
    cell_slots.sort_unstable();

    UpvaluePlan {
        upvalue_slots,
        cell_slots,
        child_sources,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn local(name: &str, from_env: bool) -> Local {
        Local {
            name: name.to_owned(),
            hoisted: false,
            hoisted_function: false,
            parameter: false,
            catch_binding: false,
            mutable: true,
            from_env,
            sloppy_global_fallback: false,
        }
    }

    fn cap(name: &str, slot: usize) -> (String, usize) {
        (name.to_owned(), slot)
    }

    #[test]
    fn captured_plain_local_becomes_a_cell_sourced_as_parent_local() {
        // `let x; (function(){ return x; })` — slot 0 is a plain local of the
        // enclosing frame captured by one child.
        let locals = [local("x", false)];
        let child = [cap("x", 0)];
        let plan = resolve_from_parts(&locals, &[&child]);
        assert_eq!(plan.upvalue_slots, Vec::<usize>::new());
        assert_eq!(plan.cell_slots, vec![0]);
        assert_eq!(
            plan.child_sources,
            vec![vec![UpvalueSource::ParentLocal(0)]]
        );
    }

    #[test]
    fn captured_from_env_slot_is_sourced_as_parent_upvalue_not_reboxed() {
        // The parent itself captured `x` from *its* parent (slot 1, from_env),
        // and a grandchild captures it again: it is already a cell, so the child
        // sources it as ParentUpvalue and it is NOT added to cell_slots.
        let locals = [local("a", false), local("x", true)];
        let child = [cap("x", 1)];
        let plan = resolve_from_parts(&locals, &[&child]);
        assert_eq!(plan.upvalue_slots, vec![1]);
        assert_eq!(plan.cell_slots, Vec::<usize>::new());
        assert_eq!(
            plan.child_sources,
            vec![vec![UpvalueSource::ParentUpvalue(0)]]
        );
    }

    #[test]
    fn upvalue_index_follows_slot_order_across_multiple_from_env_slots() {
        // Two received cells at slots 0 and 2; a child capturing slot 2 must get
        // upvalue index 1 (its position among from_env slots), not the raw slot.
        let locals = [local("u0", true), local("v", false), local("u2", true)];
        let child = [cap("u2", 2)];
        let plan = resolve_from_parts(&locals, &[&child]);
        assert_eq!(plan.upvalue_slots, vec![0, 2]);
        assert_eq!(
            plan.child_sources,
            vec![vec![UpvalueSource::ParentUpvalue(1)]]
        );
    }

    #[test]
    fn a_slot_captured_by_two_children_is_one_cell() {
        let locals = [local("x", false)];
        let first = [cap("x", 0)];
        let second = [cap("x", 0)];
        let plan = resolve_from_parts(&locals, &[&first, &second]);
        assert_eq!(plan.cell_slots, vec![0]);
        assert_eq!(
            plan.child_sources,
            vec![
                vec![UpvalueSource::ParentLocal(0)],
                vec![UpvalueSource::ParentLocal(0)],
            ]
        );
    }

    #[test]
    fn mixed_local_and_upvalue_captures_in_one_child() {
        // Child captures a plain parent local (slot 0) and a received cell
        // (slot 1, from_env) — one of each source kind, order preserved.
        let locals = [local("plain", false), local("cell", true)];
        let child = [cap("plain", 0), cap("cell", 1)];
        let plan = resolve_from_parts(&locals, &[&child]);
        assert_eq!(plan.cell_slots, vec![0]);
        assert_eq!(plan.upvalue_slots, vec![1]);
        assert_eq!(
            plan.child_sources,
            vec![vec![
                UpvalueSource::ParentLocal(0),
                UpvalueSource::ParentUpvalue(0),
            ]]
        );
    }

    #[test]
    fn no_nested_functions_yields_an_empty_plan() {
        let locals = [local("x", false)];
        let plan = resolve_from_parts(&locals, &[]);
        assert_eq!(plan, UpvaluePlan::default());
    }
}
