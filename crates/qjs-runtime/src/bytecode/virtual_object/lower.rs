//! One-for-one bytecode lowering for proven non-escaping literals.
//!
//! Lowered instructions keep the original instruction offsets and execute in
//! the ordinary VM dispatch loop. A frame owns a flat bank of scalar `Value`
//! slots; aliases retain an unobservable `undefined` placeholder while every
//! proven field/element use is redirected to that bank. Unsupported uses leave
//! the complete candidate on the ordinary allocation path.

use std::{
    collections::{BTreeMap, BTreeSet},
    rc::Rc,
};

use super::{VirtualCandidate, VirtualKind, VirtualUse, analyze};
use crate::bytecode::ir::{Bytecode, Op, decode_index_receiver};

#[derive(Clone, Debug)]
pub(in crate::bytecode) struct VirtualObjectProgram {
    full: VirtualObjectVariant,
    data_only: VirtualObjectVariant,
}

#[derive(Clone, Debug)]
struct VirtualObjectVariant {
    lowered_code: Option<Rc<[Op]>>,
    required_authoritative_slots: u128,
}

impl VirtualObjectProgram {
    pub(in crate::bytecode) fn code<'a>(&'a self, original: &'a [Op]) -> &'a [Op] {
        self.full.code(original)
    }

    pub(in crate::bytecode) fn code_for_frame<'a>(
        &'a self,
        original: &'a [Op],
        authoritative_slots: u128,
        allow_virtual_functions: bool,
    ) -> &'a [Op] {
        if allow_virtual_functions && self.full.is_frame_compatible(authoritative_slots) {
            return self.full.code(original);
        }
        if self.data_only.is_frame_compatible(authoritative_slots) {
            return self.data_only.code(original);
        }
        original
    }
}

impl VirtualObjectVariant {
    fn code<'a>(&'a self, original: &'a [Op]) -> &'a [Op] {
        self.lowered_code.as_deref().unwrap_or(original)
    }

    fn is_frame_compatible(&self, authoritative_slots: u128) -> bool {
        authoritative_slots & self.required_authoritative_slots == self.required_authoritative_slots
    }
}

pub(in crate::bytecode) fn lower(bytecode: &Bytecode) -> VirtualObjectProgram {
    let analysis = analyze(bytecode);
    if !analysis.complete {
        return original_program();
    }

    // Loop plans are compiled from the immutable source stream and prepare
    // stable receivers/callees from their real local values. Replacing one of
    // those aliases with an undefined placeholder would make preparation fail
    // and silently discard an already-proven faster execution route. Keep any
    // candidate touched by a specialized loop materialized until the plans can
    // consume virtual slots directly.
    let numeric_loop_plans = bytecode
        .numeric_loop_plans
        .get_or_init(|| super::super::vm_numeric_loop::NumericLoopPlan::compile_all(bytecode));
    let control_loop_plans = bytecode
        .control_loop_plans
        .get_or_init(|| super::super::vm_control_loop::ControlLoopPlan::compile_all(bytecode));
    let numeric_mutation_loop_plans = bytecode.numeric_mutation_loop_plans.get_or_init(|| {
        super::super::vm_numeric_mutation_loop::NumericMutationLoopPlan::compile_all(bytecode)
    });

    let full = lower_variant(
        bytecode,
        &analysis,
        numeric_loop_plans,
        control_loop_plans,
        numeric_mutation_loop_plans,
        true,
    );
    let has_virtual_function = analysis.candidates.iter().any(|candidate| {
        candidate.is_virtualizable()
            && matches!(candidate.kind, VirtualKind::Function(_))
            && !candidate_intersects_specialized_loop(
                candidate,
                numeric_loop_plans,
                control_loop_plans,
                numeric_mutation_loop_plans,
            )
    });
    let data_only = if has_virtual_function {
        lower_variant(
            bytecode,
            &analysis,
            numeric_loop_plans,
            control_loop_plans,
            numeric_mutation_loop_plans,
            false,
        )
    } else {
        full.clone()
    };
    VirtualObjectProgram { full, data_only }
}

fn lower_variant(
    bytecode: &Bytecode,
    analysis: &super::VirtualObjectAnalysis,
    numeric_loop_plans: &[super::super::vm_numeric_loop::NumericLoopPlan],
    control_loop_plans: &[super::super::vm_control_loop::ControlLoopPlan],
    numeric_mutation_loop_plans: &[
        super::super::vm_numeric_mutation_loop::NumericMutationLoopPlan
    ],
    include_functions: bool,
) -> VirtualObjectVariant {
    let mut replacements = BTreeMap::new();
    let mut required_authoritative_slots = BTreeSet::new();
    let mut slot_count = 0usize;
    for candidate in analysis.candidates.iter().filter(|candidate| {
        candidate.is_virtualizable()
            && (include_functions || !matches!(candidate.kind, VirtualKind::Function(_)))
            && !candidate_intersects_specialized_loop(
                candidate,
                numeric_loop_plans,
                control_loop_plans,
                numeric_mutation_loop_plans,
            )
    }) {
        let Some(candidate_replacements) = candidate_replacements(bytecode, candidate, slot_count)
        else {
            continue;
        };
        let input_count = candidate_input_count(candidate);
        let Some(next_slot_count) = slot_count.checked_add(input_count) else {
            return original_variant();
        };
        if candidate_replacements
            .iter()
            .any(|(ip, _)| replacements.contains_key(ip))
        {
            return original_variant();
        }
        replacements.extend(candidate_replacements);
        required_authoritative_slots.extend(candidate.uses.iter().filter_map(|use_kind| {
            if let VirtualUse::Alias { slot, .. } = use_kind {
                Some(*slot)
            } else {
                None
            }
        }));
        slot_count = next_slot_count;
    }
    if replacements.is_empty() {
        return original_variant();
    }

    let mut code = bytecode.code.clone();
    for (ip, replacement) in replacements {
        let Some(op) = code.get_mut(ip) else {
            return original_variant();
        };
        *op = replacement;
    }
    fuse_superinstructions(bytecode, analysis, &mut code);
    collect_superinstruction_authority(&code, &mut required_authoritative_slots);
    let Some(required_authoritative_slots) =
        required_authoritative_slots
            .iter()
            .try_fold(0_u128, |mask, slot| {
                u32::try_from(*slot)
                    .ok()
                    .and_then(|slot| 1_u128.checked_shl(slot))
                    .map(|bit| mask | bit)
            })
    else {
        return original_variant();
    };
    VirtualObjectVariant {
        lowered_code: Some(Rc::from(code.into_boxed_slice())),
        required_authoritative_slots,
    }
}

fn candidate_intersects_specialized_loop(
    candidate: &VirtualCandidate,
    numeric: &[super::super::vm_numeric_loop::NumericLoopPlan],
    control: &[super::super::vm_control_loop::ControlLoopPlan],
    mutation: &[super::super::vm_numeric_mutation_loop::NumericMutationLoopPlan],
) -> bool {
    let in_plan = |ip| {
        numeric.iter().any(|plan| plan.contains_instruction(ip))
            || control.iter().any(|plan| plan.contains_instruction(ip))
            || mutation.iter().any(|plan| plan.contains_instruction(ip))
    };
    in_plan(candidate.allocation_ip) || candidate.uses.iter().map(virtual_use_ip).any(in_plan)
}

fn virtual_use_ip(use_kind: &VirtualUse) -> usize {
    match use_kind {
        VirtualUse::Alias { ip, .. }
        | VirtualUse::Discard { ip }
        | VirtualUse::FieldRead { ip, .. }
        | VirtualUse::FieldWrite { ip, .. }
        | VirtualUse::ObjectGuard { ip }
        | VirtualUse::ArrayLengthRead { ip }
        | VirtualUse::ElementRead { ip, .. }
        | VirtualUse::ElementWrite { ip, .. }
        | VirtualUse::DirectCall { ip, .. } => *ip,
    }
}

fn original_program() -> VirtualObjectProgram {
    VirtualObjectProgram {
        full: original_variant(),
        data_only: original_variant(),
    }
}

fn original_variant() -> VirtualObjectVariant {
    VirtualObjectVariant {
        lowered_code: None,
        required_authoritative_slots: 0,
    }
}

fn collect_superinstruction_authority(code: &[Op], required: &mut BTreeSet<usize>) {
    for op in code {
        match op {
            Op::InitVirtualObject {
                local: Some(slot), ..
            }
            | Op::InitVirtualConstants {
                local: Some(slot), ..
            }
            | Op::InitVirtualFunction {
                local: Some(slot), ..
            }
            | Op::IncrementLocal { slot, .. } => {
                required.insert(*slot);
            }
            Op::BinaryAssignLocals { target, stores, .. } => {
                required.extend([*target, stores[0], stores[1]]);
            }
            Op::CopyLocal { from, to, .. } => {
                required.extend([*from, *to]);
            }
            _ => {}
        }
    }
}

fn candidate_input_count(candidate: &VirtualCandidate) -> usize {
    match &candidate.kind {
        VirtualKind::Object(shape) => shape.input_len(),
        VirtualKind::DenseArray { length } => *length,
        VirtualKind::Function(_) => 0,
    }
}

fn candidate_replacements(
    bytecode: &Bytecode,
    candidate: &VirtualCandidate,
    slot_base: usize,
) -> Option<Vec<(usize, Op)>> {
    let input_count = candidate_input_count(candidate);
    match (bytecode.code.get(candidate.allocation_ip)?, &candidate.kind) {
        (Op::NewObjectDataLiteral { shape }, VirtualKind::Object(expected))
            if Rc::ptr_eq(shape, expected) => {}
        (Op::NewArray { elements }, VirtualKind::DenseArray { length })
            if elements.len() == *length => {}
        (Op::NewFunction { bytecode, .. }, VirtualKind::Function(expected))
            if Rc::ptr_eq(bytecode, expected) => {}
        _ => return None,
    }

    let allocation_replacement = match candidate.kind {
        VirtualKind::Function(_) => Op::InitVirtualFunction {
            local: None,
            skip: 0,
        },
        _ => Op::InitVirtualObject {
            slot: slot_base,
            count: input_count,
            local: None,
            skip: 0,
        },
    };
    let mut replacements = vec![(candidate.allocation_ip, allocation_replacement)];
    for use_kind in &candidate.uses {
        let replacement = match use_kind {
            VirtualUse::Alias { .. } | VirtualUse::Discard { .. } => continue,
            VirtualUse::FieldRead { ip, input } => (
                *ip,
                Op::LoadVirtualValue {
                    slot: slot_base.checked_add(*input)?,
                    discard: get_stack_input_count(bytecode.code.get(*ip)?)?,
                },
            ),
            VirtualUse::ElementRead { ip, index } => (
                *ip,
                Op::LoadVirtualValue {
                    slot: slot_base.checked_add(*index)?,
                    discard: get_stack_input_count(bytecode.code.get(*ip)?)?,
                },
            ),
            VirtualUse::FieldWrite { ip, input } => (
                *ip,
                Op::StoreVirtualValue {
                    slot: slot_base.checked_add(*input)?,
                    discard: set_receiver_input_count(bytecode.code.get(*ip)?)?,
                },
            ),
            VirtualUse::ElementWrite { ip, index } => (
                *ip,
                Op::StoreVirtualValue {
                    slot: slot_base.checked_add(*index)?,
                    discard: set_receiver_input_count(bytecode.code.get(*ip)?)?,
                },
            ),
            VirtualUse::ArrayLengthRead { ip } => {
                let VirtualKind::DenseArray { length } = candidate.kind else {
                    return None;
                };
                (
                    *ip,
                    Op::LoadVirtualLength {
                        length,
                        discard: get_stack_input_count(bytecode.code.get(*ip)?)?,
                    },
                )
            }
            VirtualUse::ObjectGuard { ip } => {
                if !matches!(bytecode.code.get(*ip), Some(Op::RequireObjectCoercible)) {
                    return None;
                }
                (*ip, Op::GuardVirtualObject)
            }
            VirtualUse::DirectCall { ip, argc } => {
                if !matches!(bytecode.code.get(*ip), Some(Op::Call(found)) if found == argc)
                    || !matches!(candidate.kind, VirtualKind::Function(_))
                {
                    return None;
                }
                (
                    *ip,
                    Op::CallVirtualFunction {
                        allocation_ip: candidate.allocation_ip,
                        argc: *argc,
                    },
                )
            }
        };
        replacements.push(replacement);
    }
    Some(replacements)
}

fn fuse_superinstructions(
    bytecode: &Bytecode,
    analysis: &super::VirtualObjectAnalysis,
    code: &mut [Op],
) {
    fuse_virtual_initializers(bytecode, analysis, code);
    fuse_virtual_binaries(analysis, code);
    fuse_binary_assignments(bytecode, analysis, code);
    fuse_local_increments(bytecode, analysis, code);
    fuse_local_copies(analysis, code);
    fold_redundant_completion_copies(code);
    fuse_local_comparisons(analysis, code);
}

fn fuse_virtual_initializers(
    bytecode: &Bytecode,
    analysis: &super::VirtualObjectAnalysis,
    code: &mut [Op],
) {
    for ip in 0..code.len().saturating_sub(1) {
        let Op::InitVirtualObject {
            slot,
            count,
            local: None,
            skip: 0,
        } = code[ip]
        else {
            continue;
        };
        let local = match code.get(ip + 1) {
            Some(Op::StoreLocal(local)) if analysis.slot_authority.is_authoritative(*local) => {
                *local
            }
            Some(Op::AssignLocal(local))
                if analysis
                    .slot_authority
                    .is_assignment_authoritative(bytecode, *local) =>
            {
                *local
            }
            _ => continue,
        };
        if range_is_linear(analysis, ip, ip + 1) {
            code[ip] = Op::InitVirtualObject {
                slot,
                count,
                local: Some(local),
                skip: 1,
            };
        }
    }

    for ip in 0..code.len().saturating_sub(1) {
        let Op::InitVirtualFunction {
            local: None,
            skip: 0,
        } = code[ip]
        else {
            continue;
        };
        let local = match code.get(ip + 1) {
            Some(Op::StoreLocal(local)) if analysis.slot_authority.is_authoritative(*local) => {
                *local
            }
            Some(Op::AssignLocal(local))
                if analysis
                    .slot_authority
                    .is_assignment_authoritative(bytecode, *local) =>
            {
                *local
            }
            _ => continue,
        };
        if range_is_linear(analysis, ip, ip + 1) {
            code[ip] = Op::InitVirtualFunction {
                local: Some(local),
                skip: 1,
            };
        }
    }

    for ip in 0..code.len() {
        let Op::InitVirtualObject {
            slot,
            count,
            local,
            skip,
        } = code[ip]
        else {
            continue;
        };
        if count == 0 || ip < count {
            continue;
        }
        let start = ip - count;
        let constants = code[start..ip]
            .iter()
            .map(|op| match op {
                Op::LoadConst(index) => Some(*index),
                _ => None,
            })
            .collect::<Option<Vec<_>>>();
        let Some(constants) = constants else {
            continue;
        };
        if range_is_linear(analysis, start, ip + skip) {
            let mut fused_skip = count + skip;
            loop {
                let next = start + fused_skip + 1;
                if !matches!(code.get(next), Some(Op::LoadConst(_)))
                    || !matches!(code.get(next + 1), Some(Op::Pop))
                    || !range_is_linear(analysis, start, next + 1)
                {
                    break;
                }
                fused_skip += 2;
            }
            code[start] = Op::InitVirtualConstants {
                slot,
                constants,
                local,
                skip: fused_skip,
            };
        }
    }
}

fn fuse_virtual_binaries(analysis: &super::VirtualObjectAnalysis, code: &mut [Op]) {
    for ip in 0..code.len().saturating_sub(2) {
        let (
            Op::LoadVirtualValue {
                slot: left,
                discard: 0,
            },
            Op::LoadVirtualValue {
                slot: right,
                discard: 0,
            },
            Op::Binary(op),
        ) = (&code[ip], &code[ip + 1], &code[ip + 2])
        else {
            continue;
        };
        if range_is_linear(analysis, ip, ip + 2) {
            code[ip] = Op::LoadVirtualBinary {
                left: *left,
                right: *right,
                op: *op,
                skip: 2,
            };
        }
    }
}

fn fuse_binary_assignments(
    bytecode: &Bytecode,
    analysis: &super::VirtualObjectAnalysis,
    code: &mut [Op],
) {
    for ip in 0..code.len().saturating_sub(5) {
        let (
            Op::Binary(op),
            Op::Dup,
            Op::AssignLocal(target),
            Op::Dup,
            Op::StoreLocal(first),
            Op::StoreLocal(second),
        ) = (
            &code[ip],
            &code[ip + 1],
            &code[ip + 2],
            &code[ip + 3],
            &code[ip + 4],
            &code[ip + 5],
        )
        else {
            continue;
        };
        if analysis
            .slot_authority
            .is_assignment_authoritative(bytecode, *target)
            && analysis.slot_authority.is_authoritative(*first)
            && analysis.slot_authority.is_authoritative(*second)
            && range_is_linear(analysis, ip, ip + 5)
        {
            code[ip] = Op::BinaryAssignLocals {
                op: *op,
                target: *target,
                stores: [*first, *second],
                skip: 5,
            };
        }
    }
}

fn fuse_local_increments(
    bytecode: &Bytecode,
    analysis: &super::VirtualObjectAnalysis,
    code: &mut [Op],
) {
    for ip in 0..code.len().saturating_sub(5) {
        let (
            Op::LoadLocal(slot),
            Op::ToNumeric,
            Op::Dup,
            Op::Update(qjs_ast::UpdateOp::Increment),
            Op::AssignLocal(assigned),
            Op::Pop,
        ) = (
            &code[ip],
            &code[ip + 1],
            &code[ip + 2],
            &code[ip + 3],
            &code[ip + 4],
            &code[ip + 5],
        )
        else {
            continue;
        };
        if slot == assigned
            && analysis
                .slot_authority
                .is_assignment_authoritative(bytecode, *slot)
            && range_is_linear(analysis, ip, ip + 5)
        {
            let jump = match code.get(ip + 6) {
                Some(Op::Jump(target)) => Some(*target),
                _ => None,
            };
            code[ip] = Op::IncrementLocal {
                slot: *slot,
                skip: 5,
                jump,
            };
        }
    }
}

fn fuse_local_copies(analysis: &super::VirtualObjectAnalysis, code: &mut [Op]) {
    for ip in 0..code.len().saturating_sub(1) {
        let (Op::LoadLocal(from), Op::StoreLocal(to)) = (&code[ip], &code[ip + 1]) else {
            continue;
        };
        if analysis.slot_authority.is_authoritative(*from)
            && analysis.slot_authority.is_authoritative(*to)
            && range_is_linear(analysis, ip, ip + 1)
        {
            code[ip] = Op::CopyLocal {
                from: *from,
                to: *to,
                skip: 1,
            };
        }
    }
}

fn fold_redundant_completion_copies(code: &mut [Op]) {
    for ip in 0..code.len() {
        let Op::BinaryAssignLocals { stores, skip, .. } = code[ip] else {
            continue;
        };
        let next = ip + skip + 1;
        let Some(Op::CopyLocal {
            from,
            to,
            skip: copy_skip,
        }) = code.get(next)
        else {
            continue;
        };
        if *from == stores[0] && *to == stores[1] {
            let Some(fused_skip) = skip.checked_add(copy_skip + 1) else {
                continue;
            };
            if let Op::BinaryAssignLocals { skip, .. } = &mut code[ip] {
                *skip = fused_skip;
            }
        }
    }
}

fn fuse_local_comparisons(analysis: &super::VirtualObjectAnalysis, code: &mut [Op]) {
    for ip in 0..code.len().saturating_sub(3) {
        let (Op::LoadLocal(left), Op::LoadLocal(right), Op::Binary(op), Op::JumpIfFalse(target)) =
            (&code[ip], &code[ip + 1], &code[ip + 2], &code[ip + 3])
        else {
            continue;
        };
        if range_is_linear(analysis, ip, ip + 3) {
            let discard = matches!(code.get(ip + 4), Some(Op::Pop))
                && matches!(code.get(*target), Some(Op::Pop));
            code[ip] = Op::CompareLocalsJumpFalse {
                left: *left,
                right: *right,
                op: *op,
                target: *target,
                skip: 3,
                discard,
            };
        }
    }
}

fn range_is_linear(analysis: &super::VirtualObjectAnalysis, start: usize, end: usize) -> bool {
    analysis
        .cfg
        .blocks
        .iter()
        .any(|block| block.start <= start && end < block.end)
}

fn get_stack_input_count(op: &Op) -> Option<usize> {
    match op {
        Op::GetPropNamed { cache, .. } => Some(usize::from(cache.local_slot().is_none())),
        Op::GetPropIndex(encoded) => {
            let (_, local_slot) = decode_index_receiver(*encoded);
            Some(usize::from(local_slot.is_none()))
        }
        Op::GetProp => Some(2),
        _ => None,
    }
}

fn set_receiver_input_count(op: &Op) -> Option<usize> {
    match op {
        Op::SetPropNamed { .. } | Op::SetPropIndex { .. } => Some(1),
        Op::SetProp { .. } => Some(2),
        _ => None,
    }
}

#[cfg(test)]
thread_local! {
    static TEST_VIRTUAL_INIT_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static TEST_VIRTUAL_FUNCTION_INIT_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
pub(in crate::bytecode) fn record_virtual_init_for_test() {
    TEST_VIRTUAL_INIT_HITS.set(TEST_VIRTUAL_INIT_HITS.get() + 1);
}

#[cfg(test)]
pub(in crate::bytecode) fn record_virtual_function_init_for_test() {
    TEST_VIRTUAL_FUNCTION_INIT_HITS.set(TEST_VIRTUAL_FUNCTION_INIT_HITS.get() + 1);
}

#[cfg(test)]
fn reset_test_hits() {
    TEST_VIRTUAL_INIT_HITS.set(0);
}

#[cfg(test)]
fn test_hits() -> usize {
    TEST_VIRTUAL_INIT_HITS.get()
}

#[cfg(test)]
fn reset_virtual_function_test_hits() {
    TEST_VIRTUAL_FUNCTION_INIT_HITS.set(0);
}

#[cfg(test)]
fn virtual_function_test_hits() -> usize {
    TEST_VIRTUAL_FUNCTION_INIT_HITS.get()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::compiler;
    use crate::{Value, eval};

    fn nested_function(source: &str) -> Bytecode {
        let script = qjs_parser::parse_script(source).expect("source should parse");
        let bytecode = compiler::compile_script(&script).expect("source should compile");
        bytecode
            .code
            .iter()
            .find_map(|op| match op {
                Op::NewFunction { bytecode, .. } => Some(bytecode.as_ref().clone()),
                _ => None,
            })
            .expect("function bytecode should be nested in the script")
    }

    #[test]
    fn preserves_materialized_aliases_used_by_specialized_numeric_loops() {
        let object = nested_function(
            "function run(n) { var value = { a: 1, b: 2, c: 3 }; var total = 0; for (var i = 0; i < n; i++) { total += value.a; total += value.b; total += value.c; } return total; }",
        );
        assert!(
            !super::super::super::vm_numeric_loop::NumericLoopPlan::compile_all(&object).is_empty()
        );
        let object_program = lower(&object);
        let object_code = object_program.code(&object.code);
        assert!(
            object_code
                .iter()
                .any(|op| matches!(op, Op::NewObjectDataLiteral { .. }))
        );
        assert!(
            !object_code
                .iter()
                .any(|op| matches!(op, Op::InitVirtualObject { .. }))
        );

        let array = nested_function(
            "function run(n) { var value = [1, 2, 3, 4]; var total = 0; for (var i = 0; i < n; i++) { total += value[0]; total += value[1]; total += value[2]; total += value[3]; } return total; }",
        );
        assert!(
            !super::super::super::vm_numeric_loop::NumericLoopPlan::compile_all(&array).is_empty()
        );
        let array_program = lower(&array);
        let array_code = array_program.code(&array.code);
        assert!(
            array_code
                .iter()
                .any(|op| matches!(op, Op::NewArray { .. }))
        );
        assert!(
            !array_code
                .iter()
                .any(|op| matches!(op, Op::InitVirtualObject { .. }))
        );

        let function = nested_function(
            "function run(n) { var add = function (value) { return value + 1; }; var total = 0; for (var i = 0; i < n; i++) { total += add(i); } return total; }",
        );
        assert!(
            !super::super::super::vm_numeric_loop::NumericLoopPlan::compile_all(&function)
                .is_empty()
        );
        let function_program = lower(&function);
        let function_code = function_program.code(&function.code);
        assert!(
            function_code
                .iter()
                .any(|op| matches!(op, Op::NewFunction { .. }))
        );
        assert!(
            !function_code
                .iter()
                .any(|op| matches!(op, Op::InitVirtualFunction { .. }))
        );
    }

    #[test]
    fn lowers_general_non_escaping_function_literal_calls_in_the_shared_stream() {
        let bytecode = nested_function(
            "function run(n) { var total = 0; for (var i = 0; i < n; i++) { var add = function (value) { return value + 1; }; total += add(0); } return total; }",
        );
        let program = lower(&bytecode);
        let code = program.code(&bytecode.code);
        assert_eq!(code.len(), bytecode.code.len());
        assert!(
            code.iter()
                .any(|op| matches!(op, Op::InitVirtualFunction { .. }))
        );
        assert!(
            code.iter()
                .any(|op| matches!(op, Op::CallVirtualFunction { argc: 1, .. }))
        );

        reset_virtual_function_test_hits();
        assert_eq!(
            eval(
                "function run(n) { var total = 0; for (var i = 0; i < n; i++) { var add = function (value) { return value + 1; }; total += add(0); } return total; } run(5);"
            ),
            Ok(Value::Number(5.0))
        );
        assert_eq!(virtual_function_test_hits(), 5);
    }

    #[test]
    fn virtual_function_calls_cover_vm_fallback_aliases_and_argument_order() {
        reset_virtual_function_test_hits();
        assert_eq!(
            eval(
                "function run() { var decorate = function (value) { return value + '!'; }; var alias = decorate; return decorate('a') + alias('b'); } run();"
            ),
            Ok(Value::String("a!b!".to_owned().into()))
        );
        assert_eq!(virtual_function_test_hits(), 1);

        reset_virtual_function_test_hits();
        assert_eq!(
            eval(
                "function run() { var zero = function () { return 7; }; var one = function (a) { return a; }; var two = function (a, b) { return a * 10 + b; }; var three = function (a, b, c) { return a * 100 + b * 10 + c; }; var five = function (a, b, c, d, e) { return a * 10000 + b * 1000 + c * 100 + d * 10 + e; }; return [zero(), one(1), two(2, 3), three(4, 5, 6), five(6, 7, 8, 9, 0)].join(':'); } run();"
            ),
            Ok(Value::String("7:1:23:456:67890".to_owned().into()))
        );
        assert_eq!(virtual_function_test_hits(), 5);

        reset_virtual_function_test_hits();
        assert_eq!(
            eval(
                "var order = ''; function mark(value) { order += value; return value; } (function (a, b, c, d, e) { return a * 10000 + b * 1000 + c * 100 + d * 10 + e; })(mark(1), mark(2), mark(3), mark(4), mark(5)) + ':' + order;"
            ),
            Ok(Value::String("12345:12345".to_owned().into()))
        );
        assert_eq!(virtual_function_test_hits(), 1);
    }

    #[test]
    fn independent_holdout_scalar_replaces_branching_five_argument_literal() {
        reset_virtual_function_test_hits();
        let source = r#"
            function checksum(limit) {
                var score = 0;
                for (var index = 0; index < limit; index++) {
                    score += (function (tag, value, second, third, fourth) {
                        var text = tag + ":" + (value + second + third + fourth);
                        if (value % 2) return text.length;
                        return text.indexOf(":") + second + third + fourth;
                    })("x", index, 2, 3, 4);
                }
                return score;
            }
            checksum(4);
        "#;
        assert_eq!(eval(source), Ok(Value::Number(28.0)));
        assert_eq!(virtual_function_test_hits(), 4);
    }

    #[test]
    fn virtual_function_call_propagates_thrown_values_through_the_existing_vm() {
        reset_virtual_function_test_hits();
        let error = eval("(function () { throw 7; })();").expect_err("call should throw");
        assert!(error.message.contains('7'), "{}", error.message);
        assert_eq!(virtual_function_test_hits(), 1);
    }

    #[test]
    fn runtime_creation_contexts_keep_function_literals_materialized() {
        reset_virtual_function_test_hits();
        assert_eq!(
            eval(
                "var outer = function named() { return (function () { return typeof named; })(); }; outer();"
            ),
            Ok(Value::String("function".to_owned().into()))
        );
        assert_eq!(virtual_function_test_hits(), 0);

        reset_virtual_function_test_hits();
        assert_eq!(
            eval(
                "function outer() { let x = 7; return eval('(function () { return x; })()'); } outer();"
            ),
            Ok(Value::Number(7.0))
        );
        assert_eq!(virtual_function_test_hits(), 0);

        reset_virtual_function_test_hits();
        assert_eq!(
            eval(
                "function outer() { with ({ x: 9 }) { return eval('(function () { return x; })()'); } } outer();"
            ),
            Ok(Value::Number(9.0))
        );
        assert_eq!(virtual_function_test_hits(), 0);

        // The data-only stream keeps unrelated scalar replacement enabled even
        // when the same frame must materialize a function literal.
        reset_test_hits();
        reset_virtual_function_test_hits();
        assert_eq!(
            eval(
                "var outer = function named() { return ({ x: 4 }).x + ((function () { return typeof named; })() === 'function' ? 1 : 0); }; outer();"
            ),
            Ok(Value::Number(5.0))
        );
        assert_eq!(test_hits(), 1);
        assert_eq!(virtual_function_test_hits(), 0);

        reset_test_hits();
        reset_virtual_function_test_hits();
        assert_eq!(
            eval(
                "function outer() { let x = 7; return eval('({ y: 2 }).y + (function () { return x; })()'); } outer();"
            ),
            Ok(Value::Number(9.0))
        );
        assert_eq!(test_hits(), 1);
        assert_eq!(virtual_function_test_hits(), 0);
    }

    #[test]
    fn observable_function_context_and_identity_keep_real_allocations() {
        for (source, expected) in [
            (
                "function run() { var x = 3; var f = function () { return x; }; return f(); } run();",
                Value::Number(3.0),
            ),
            (
                "function run() { var f = function () { return this === globalThis; }; return f(); } run();",
                Value::Boolean(true),
            ),
            (
                "function run() { var f = function () { return arguments[0]; }; return f(4); } run();",
                Value::Number(4.0),
            ),
            (
                "function run() { var f = function () { return 3; }; return typeof new f() === 'object'; } run();",
                Value::Boolean(true),
            ),
            (
                "function run() { var f = function () {}; return f.name; } run();",
                Value::String("f".to_owned().into()),
            ),
            (
                "function run() { var f = function () {}; return typeof f.prototype; } run();",
                Value::String("object".to_owned().into()),
            ),
            (
                "function run() { var f = function inner(n) { return n ? n * inner(n - 1) : 1; }; return f(5); } run();",
                Value::Number(120.0),
            ),
            (
                "function run() { var f = function* () { yield 4; }; return f().next().value; } run();",
                Value::Number(4.0),
            ),
            (
                "function run() { var f = function () { return new.target; }; return f() === undefined; } run();",
                Value::Boolean(true),
            ),
            (
                "function run() { var f = function () { return 1; }; eval(\"f = function () { return 2; }\"); return f(); } run();",
                Value::Number(2.0),
            ),
            (
                "function run() { var f = function () {}; return typeof f; } run();",
                Value::String("function".to_owned().into()),
            ),
            (
                "function run() { var f = function () {}; return f === f; } run();",
                Value::Boolean(true),
            ),
            (
                "function sink(value) { return value.name; } function run() { var f = function () {}; return sink(f); } run();",
                Value::String("f".to_owned().into()),
            ),
            (
                "function run() { var f = function () {}; return ({ value: f }).value.name; } run();",
                Value::String("f".to_owned().into()),
            ),
            (
                "function run() { var f = function () {}; return [f][0].name; } run();",
                Value::String("f".to_owned().into()),
            ),
            (
                "function run() { var f = function () {}; var target = { value: null }; target.value = f; return target.value.name; } run();",
                Value::String("f".to_owned().into()),
            ),
            (
                "function run() { var f = function (a, b) { return a * 10 + b; }; return f(...[2, 3]); } run();",
                Value::Number(23.0),
            ),
            (
                "function run() { var f = function (value) { return value; }; var holder = { method: f }; return holder.method(3); } run();",
                Value::Number(3.0),
            ),
            (
                "function run() { var f = function (value) { return value; }; return f.call(undefined, 3); } run();",
                Value::Number(3.0),
            ),
            (
                "function run() { with ({}) { var f = function () { return 1; }; return f(); } } run();",
                Value::Number(1.0),
            ),
            (
                "class A { value() { return 3; } } class B extends A { run() { var f = () => super.value(); return f(); } } new B().run();",
                Value::Number(3.0),
            ),
            (
                "class C { #value = 2; run() { var f = function (receiver) { return receiver.#value; }; return f(this); } } new C().run();",
                Value::Number(2.0),
            ),
        ] {
            reset_virtual_function_test_hits();
            assert_eq!(eval(source), Ok(expected), "{source}");
            assert_eq!(virtual_function_test_hits(), 0, "{source}");
        }
    }

    #[test]
    fn outer_generator_and_async_suspension_keep_function_identity_materialized() {
        reset_virtual_function_test_hits();
        assert_eq!(
            eval(
                "function* run() { var f = function (value) { return value + 1; }; yield 0; return f(1); } var iterator = run(); iterator.next(); iterator.next().value;"
            ),
            Ok(Value::Number(2.0))
        );
        assert_eq!(virtual_function_test_hits(), 0);

        reset_virtual_function_test_hits();
        let value = eval(
            "var log = []; async function run() { var f = function (value) { return value + 1; }; await 0; log.push(f(1)); } run(); log;",
        )
        .expect("async call should drain its jobs");
        let Value::Array(log) = value else {
            panic!("expected async log array, got {value:?}");
        };
        assert_eq!(log.to_vec(), vec![Value::Number(2.0)]);
        assert_eq!(virtual_function_test_hits(), 0);
    }

    #[test]
    fn emits_shared_dispatch_superinstructions_without_offset_changes() {
        let bytecode = nested_function(
            "function run(n) { var sum = 0; for (var i = 0; i < n; i++) { var point = { x: 1, y: 2 }; sum += point.x + point.y; } return sum; }",
        );
        let program = lower(&bytecode);
        let code = program.code(&bytecode.code);
        assert_eq!(code.len(), bytecode.code.len());
        assert!(
            code.iter()
                .any(|op| matches!(op, Op::InitVirtualConstants { .. }))
        );
        assert!(
            code.iter()
                .any(|op| matches!(op, Op::LoadVirtualBinary { .. }))
        );
        assert!(
            code.iter()
                .any(|op| matches!(op, Op::BinaryAssignLocals { .. }))
        );
        assert!(
            code.iter()
                .any(|op| matches!(op, Op::IncrementLocal { .. }))
        );
        assert!(
            code.iter()
                .any(|op| matches!(op, Op::CompareLocalsJumpFalse { discard: true, .. }))
        );
    }

    #[test]
    fn lowers_object_and_array_literals_without_a_second_executor() {
        reset_test_hits();
        let source = r#"
            function run(n) {
                var sum = 0;
                for (var i = 0; i < n; i++) {
                    var point = { x: i * 2, y: i - 1 };
                    var values = [point.x, point.y, 3];
                    sum += values[0] + values[1] + values.length;
                }
                return sum;
            }
            run(5);
        "#;
        assert_eq!(eval(source), Ok(Value::Number(40.0)));
        assert_eq!(test_hits(), 10);
    }

    #[test]
    fn preserves_virtual_writes_inside_loops() {
        reset_test_hits();
        let source = r#"
            function run(n) {
                var sum = 0, point, values;
                for (var i = 0; i < n; i++) {
                    point = { x: i, y: 1 };
                    point.x += 2;
                    values = [point.x, point.y];
                    values[1] = i * 3;
                    sum += point.x + values[1];
                }
                return sum;
            }
            run(4);
        "#;
        assert_eq!(eval(source), Ok(Value::Number(32.0)));
        assert_eq!(test_hits(), 8);
    }

    #[test]
    fn preserves_straight_line_fields_after_virtual_writes() {
        reset_test_hits();
        let source = r#"
            function run() {
                var point = { x: 1, y: 2 };
                point.x = 4;
                var values = [point.x, point.y];
                values[0] += 3;
                return point.x + values[0] + values[1];
            }
            run();
        "#;
        assert_eq!(eval(source), Ok(Value::Number(13.0)));
        assert_eq!(test_hits(), 1);
    }

    #[test]
    fn leaves_escaping_and_effectful_literals_on_the_allocation_path() {
        reset_test_hits();
        let source = r#"
            function run() {
                var calls = 0;
                function next() { calls++; return calls; }
                var escaped = { x: next() };
                var method = { value() { return this; } };
                return [escaped, method, calls];
            }
            var result = run();
            result[0].x + result[2] + (result[1].value() === result[1]);
        "#;
        assert_eq!(eval(source), Ok(Value::Number(3.0)));
        assert_eq!(test_hits(), 0);
    }

    #[test]
    fn effectful_field_values_fail_closed_and_preserve_thrown_identity() {
        reset_test_hits();
        let source = r#"
            var log = "";
            var marker = {};
            var first = { valueOf() { log += "a"; return 1; } };
            var second = { valueOf() { log += "b"; throw marker; } };
            function run() {
                var point = { x: first, y: second };
                return point.x + point.y;
            }
            var same = false;
            try { run(); } catch (error) { same = error === marker; log += "c"; }
            same + ":" + log;
        "#;
        assert_eq!(
            eval(source),
            Ok(Value::String("true:abc".to_owned().into()))
        );
        assert_eq!(test_hits(), 0);
    }

    #[test]
    fn fused_binary_fallback_preserves_string_addition() {
        reset_test_hits();
        let source = r#"
            function run() {
                var point = { x: "left", y: "right" };
                return point.x + ":" + point.y;
            }
            run();
        "#;
        assert_eq!(
            eval(source),
            Ok(Value::String("left:right".to_owned().into()))
        );
        assert_eq!(test_hits(), 1);
    }

    #[test]
    fn fused_increment_fallback_preserves_to_numeric_and_backedge() {
        reset_test_hits();
        let source = r#"
            function run(n) {
                var coercions = 0;
                var i = { valueOf() { coercions++; return 0; } };
                var sum = 0;
                for (; i < n; i++) {
                    var point = { x: 1, y: 2 };
                    sum += point.x + point.y;
                }
                return coercions + ":" + i + ":" + sum;
            }
            run(1);
        "#;
        assert_eq!(eval(source), Ok(Value::String("2:1:3".to_owned().into())));
        assert_eq!(test_hits(), 1);
    }

    #[test]
    fn fused_comparison_observes_mapped_argument_updates() {
        reset_test_hits();
        let source = r#"
            function run(n) {
                var sum = 0;
                for (var i = 0; i < n; i++) {
                    var point = { x: 1, y: 2 };
                    sum += point.x + point.y;
                    arguments[0] = 1;
                }
                return sum;
            }
            run(3);
        "#;
        assert_eq!(eval(source), Ok(Value::Number(3.0)));
        assert_eq!(test_hits(), 1);
    }

    #[test]
    fn virtual_number_fast_path_preserves_edge_values_and_bigint_fallback() {
        reset_test_hits();
        let source = r#"
            function numeric() {
                var point = { x: -0, y: 0 };
                return (Object.is(point.x + point.y, 0) ? 1 : 0) +
                    (Object.is(point.x * point.y, -0) ? 2 : 0) +
                    ((1 / point.x) === -Infinity ? 4 : 0) +
                    (Number.isNaN(point.x / point.y) ? 8 : 0);
            }
            function bigint() {
                var point = { x: 1n, y: 2n };
                return String(point.x + point.y);
            }
            numeric() + ":" + bigint();
        "#;
        assert_eq!(eval(source), Ok(Value::String("15:3".to_owned().into())));
        assert_eq!(test_hits(), 2);
    }

    #[test]
    fn fused_loop_handles_zero_iterations_without_stack_residue() {
        reset_test_hits();
        let source = r#"
            function run(n) {
                var sum = 0;
                for (var i = 0; i < n; i++) {
                    var values = [1, 2, 3];
                    sum += values[2];
                }
                return sum;
            }
            run(0) + run(2);
        "#;
        assert_eq!(eval(source), Ok(Value::Number(6.0)));
        assert_eq!(test_hits(), 2);
    }

    #[test]
    fn frame_authority_guard_disables_lowering_beyond_inline_slot_mask() {
        reset_test_hits();
        let declarations = (0..130)
            .map(|index| format!("v{index} = {index}"))
            .collect::<Vec<_>>()
            .join(", ");
        let source = format!(
            "function run() {{ var {declarations}; var point = {{ x: 1, y: 2 }}; return point.x + point.y + v0; }} run();"
        );
        assert_eq!(eval(&source), Ok(Value::Number(3.0)));
        assert_eq!(test_hits(), 0);
    }

    #[test]
    fn named_generator_frame_refresh_disables_incompatible_lowering() {
        reset_test_hits();
        let source = r#"
            var make = function* named(n) {
                var sum = 0;
                for (var i = 0; i < n; i++) {
                    var point = { x: 1, y: 2 };
                    sum += point.x + point.y;
                }
                yield sum;
            };
            make(2).next().value;
        "#;
        assert_eq!(eval(source), Ok(Value::Number(6.0)));
        assert_eq!(test_hits(), 0);
    }

    #[test]
    fn generator_resume_keeps_offsets_for_candidates_after_yield() {
        reset_test_hits();
        let source = r#"
            function* run(n) {
                yield 1;
                var sum = 0;
                for (var i = 0; i < n; i++) {
                    var values = [1, 2, 3];
                    sum += values[2];
                }
                return sum;
            }
            var iterator = run(2);
            iterator.next().value + iterator.next().value;
        "#;
        assert_eq!(eval(source), Ok(Value::Number(7.0)));
        assert_eq!(test_hits(), 2);
    }

    #[test]
    fn fused_increment_reuses_the_original_loop_backedge() {
        let bytecode = nested_function(
            "function run(n) { if (n < 0) { var point = { x: 1, y: 2 }; if (point.x + point.y === 99) return -1; } for (var i = 0; i < n; i++) {} return i; }",
        );
        let program = lower(&bytecode);
        let code = program.code(&bytecode.code);
        let increment_ip = code
            .iter()
            .position(|op| matches!(op, Op::IncrementLocal { .. }))
            .expect("the local increment should be fused");
        let Op::IncrementLocal { skip, jump, .. } = code[increment_ip] else {
            unreachable!();
        };
        let Some(Op::Jump(original_target)) = code.get(increment_ip + skip + 1) else {
            panic!("the original loop backedge should remain available to plan compilation");
        };
        assert_eq!(jump, Some(*original_target));
        assert_eq!(
            eval(
                "function run(n) { if (n < 0) { var point = { x: 1, y: 2 }; if (point.x + point.y === 99) return -1; } for (var i = 0; i < n; i++) {} return i; } run(1000);"
            ),
            Ok(Value::Number(1000.0))
        );
    }
}
