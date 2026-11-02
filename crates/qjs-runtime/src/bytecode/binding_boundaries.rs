//! Lower semantic checkpoints before code is shared or a tier map is built.
//! One transient relocation owns every PC rewrite; ordinary operations do not
//! poll scope/reference side tables. Jump and resume entries include cleanup.
use super::ir::{Bytecode, Op};
use std::collections::{BTreeMap, BTreeSet};

pub(super) fn lower(body: &mut Bytecode) {
    let len = body.code.len();
    if len == 0 {
        return;
    }
    // All-transparent trees need only a bulk scope fill, not a relocation.
    if body.binding_plan.references.events.is_empty()
        && body.binding_plan.references.lifetimes.is_empty()
        && body
            .binding_plan
            .scopes
            .iter()
            .skip(1)
            .all(|scope| transparent(scope, &body.binding_plan.scopes))
    {
        body.binding_plan.pc_scopes.fill(crate::binding::ScopeId(0));
        return;
    }
    // Empty records own no cells, with object or separate variable holder.
    let mut canonical = Vec::with_capacity(body.binding_plan.scopes.len());
    for scope in body.binding_plan.scopes.iter() {
        let parent = scope
            .parent
            .filter(|_| transparent(scope, &body.binding_plan.scopes));
        canonical.push(parent.map_or(scope.id, |parent| canonical[parent.0]));
    }
    for scope in &mut body.binding_plan.pc_scopes {
        *scope = canonical[scope.0];
    }
    let scopes = &body.binding_plan.pc_scopes;
    let references = &body.binding_plan.references;
    let mut points = BTreeSet::new();
    for pc in 1..len {
        if scopes[pc] != scopes[pc - 1] {
            points.insert(pc);
        }
    }
    points.extend(references.events.iter().map(|event| event.pc));
    let mut cleanup_points: BTreeSet<_> =
        references.lifetimes.iter().map(|life| life.end).collect();
    points.extend(cleanup_points.iter().copied());
    if points.is_empty() {
        return;
    }
    for (pc, op) in body.code.iter().enumerate() {
        match op {
            Op::Jump(target)
            | Op::JumpIfFalse(target)
            | Op::JumpIfTrue(target)
            | Op::JumpIfNotNullish(target)
            | Op::AbruptJump(target) => {
                let crosses_scope = *target < len && scopes[pc] != scopes[*target];
                let leaves_reference = references.lifetimes.iter().any(|life| {
                    pc >= life.start
                        && pc < life.end
                        && !(*target >= life.start && *target < life.end)
                });
                if crosses_scope || leaves_reference {
                    points.insert(*target);
                }
                if leaves_reference {
                    cleanup_points.insert(*target);
                }
            }
            Op::EnterTry { catch, finally, .. } => {
                points.extend(catch.iter().chain(finally.iter()).copied());
                cleanup_points.extend(catch.iter().chain(finally.iter()).copied());
            }
            Op::Yield | Op::Await | Op::YieldDelegate { .. } => {
                points.insert(pc);
                points.insert(pc + 1);
            }
            _ => {}
        }
    }
    debug_assert!(points.iter().all(|&pc| pc <= len));
    debug_assert!(body.compact_function_program.get().is_none());
    debug_assert!(body.compact_wide_program.get().is_none());
    debug_assert!(body.virtual_object_program.get().is_none());
    debug_assert!(body.typed_loop_programs.get().is_none());
    debug_assert!(body.numeric_mutation_loop_plans.get().is_none());
    debug_assert!(body.template_objects.borrow().is_empty());
    let events: BTreeMap<_, _> = references
        .events
        .iter()
        .enumerate()
        .map(|(index, event)| (event.pc, index))
        .collect();
    let mut boundary = Vec::with_capacity(len + 1);
    let mut next = 0;
    let mut point_cursor = points.iter().copied().peekable();
    for pc in 0..=len {
        boundary.push(next);
        if point_cursor.peek() == Some(&pc) {
            point_cursor.next();
            next += 1;
        }
        if pc < len {
            next += 1;
        }
    }
    let mut code = Vec::with_capacity(next);
    let mut pc_scopes = Vec::with_capacity(next);
    let mut point_cursor = points.iter().copied().peekable();
    for (pc, mut op) in std::mem::take(&mut body.code).into_iter().enumerate() {
        if point_cursor.peek() == Some(&pc) {
            point_cursor.next();
            code.push(Op::BindingBoundary {
                event: events.get(&pc).copied(),
                cleanup: cleanup_points.contains(&pc),
            });
            pc_scopes.push(scopes[pc]);
        }
        relocate_op(&mut op, pc, &boundary);
        code.push(op);
        pc_scopes.push(scopes[pc]);
    }
    if points.contains(&len) {
        code.push(Op::BindingBoundary {
            event: events.get(&len).copied(),
            cleanup: cleanup_points.contains(&len),
        });
        pc_scopes.push(scopes[len - 1]);
    }
    let plan = &mut body.binding_plan;
    for site in &mut plan.references.sites {
        site.start = boundary[site.start];
    }
    for life in &mut plan.references.lifetimes {
        life.start = boundary[life.start];
        life.end = boundary[life.end];
    }
    for event in &mut plan.references.events {
        event.pc = operation_pc(&boundary, event.pc);
    }
    for site in &mut plan.exposure.sites {
        site.pc = operation_pc(&boundary, site.pc);
    }
    plan.prologue_end = plan.prologue_end.map(|pc| operation_pc(&boundary, pc));
    plan.pc_scopes = pc_scopes.into_boxed_slice();
    body.code = code;
    plan.validate(body.locals.len(), body.code.len());
}

fn relocate_op(op: &mut Op, old_pc: usize, boundary: &[usize]) {
    match op {
        Op::Jump(target)
        | Op::JumpIfFalse(target)
        | Op::JumpIfTrue(target)
        | Op::JumpIfNotNullish(target)
        | Op::AbruptJump(target) => *target = boundary[*target],
        Op::EnterTry { catch, finally, .. } => {
            *catch = catch.map(|pc| boundary[pc]);
            *finally = finally.map(|pc| boundary[pc]);
        }
        // These forms are normally produced later by virtual lowering. Keeping
        // their rewrite explicit prevents a future pre-lowering pass hiding PCs.
        Op::IncrementLocal { skip, jump, .. } => {
            *jump = jump.map(|pc| boundary[pc]);
            *skip = boundary[old_pc + 1 + *skip] - boundary[old_pc + 1];
        }
        Op::CompareLocalsJumpFalse { skip, target, .. } => {
            *target = boundary[*target];
            *skip = boundary[old_pc + 1 + *skip] - boundary[old_pc + 1];
        }
        Op::InitVirtualObject { skip, .. }
        | Op::InitVirtualConstants { skip, .. }
        | Op::LoadVirtualBinary { skip, .. }
        | Op::BinaryAssignLocals { skip, .. }
        | Op::CopyLocal { skip, .. }
        | Op::InitVirtualFunction { skip, .. }
        | Op::LoadVirtualNumber { skip, .. } => {
            *skip = boundary[old_pc + 1 + *skip] - boundary[old_pc + 1];
        }
        Op::CallVirtualFunction { allocation_ip, .. } => {
            *allocation_ip = operation_pc(boundary, *allocation_ip)
        }
        _ => {}
    }
}

// The next boundary begins immediately after this original operation, even
// when that next position has an inserted checkpoint. No second map is needed.
fn operation_pc(boundary: &[usize], pc: usize) -> usize {
    boundary[pc + 1] - 1
}

fn transparent(
    scope: &crate::binding::ScopeDescriptor,
    scopes: &crate::binding::ScopeDescriptors,
) -> bool {
    scope.declarations.is_empty()
        && matches!(
            scope.kind,
            crate::binding::ScopeKind::Block
                | crate::binding::ScopeKind::FunctionBody
                | crate::binding::ScopeKind::Loop
                | crate::binding::ScopeKind::Catch
        )
        && scope
            .parent
            .is_some_and(|parent| scope.variable_target == scopes[parent.0].variable_target)
}
