use super::super::{
    frame_program::FrameProgramView,
    ir::{Bytecode, Op},
    vm::Vm,
};
use crate::{Value, function::CallEnv};
use std::collections::HashMap;

fn function(source: &str) -> Bytecode {
    let script = qjs_parser::parse_script(source).unwrap();
    let bytecode = crate::bytecode::compile_script(&script).unwrap();
    bytecode
        .code
        .iter()
        .find_map(|op| match op {
            Op::NewFunction { bytecode, .. } => Some(bytecode.as_ref().clone()),
            _ => None,
        })
        .unwrap()
}

fn field(value: Value) -> Option<Value> {
    let Value::Object(object) = value else {
        panic!("expected object result");
    };
    object.get("s")
}

fn env(n: f64, a: f64, b: f64) -> CallEnv {
    CallEnv::with_locals(
        crate::function::new_realm(HashMap::new()),
        HashMap::from([
            ("n".into(), Value::Number(n)),
            ("a".into(), Value::Number(a)),
            ("b".into(), Value::Number(b)),
        ]),
    )
}

#[test]
fn selected_variant_caches_are_independent_and_original_is_untouched() {
    let code = function(
        "function run(n,a,b){var p={x:a*1,y:b*1};var s=0;for(var i=0;i<n;i++)s+=p.x+p.y;return {s:s};}",
    );
    let original = FrameProgramView::new(&code, 0, false);
    let full = FrameProgramView::new(&code, u128::MAX, true);
    let data = FrameProgramView::new(&code, u128::MAX, false);
    assert!(std::ptr::eq(original.execution_code, code.code.as_slice()));
    assert!(!std::ptr::eq(full.execution_code, code.code.as_slice()));
    let fp = full.loop_plans().typed;
    let dp = data.loop_plans().typed;
    assert!(!fp.is_empty());
    assert_eq!(fp.len(), dp.len());
    assert!(!std::ptr::eq(fp, dp));
    assert!(code.typed_loop_programs.get().is_none());
    let op = original.loop_plans().typed;
    assert!(!op.is_empty());
    assert!(!std::ptr::eq(op, fp));
    assert!(fp.iter().any(|plan| !plan.virtual_slots.is_empty()));
    assert!(op.iter().all(|plan| plan.virtual_slots.is_empty()));
    assert!(
        std::ptr::eq(full.execution_code, data.execution_code),
        "data-only clone may share immutable code"
    );
    let scratch = fp[0].take_scratch();
    fp[0].recycle_scratch(scratch);
    assert!(fp[0].scratch_pool.get().is_some());
    let cloned = code.clone();
    let clone_view = FrameProgramView::new(&cloned, u128::MAX, true);
    assert!(std::ptr::eq(clone_view.execution_code, full.execution_code));
    assert!(
        clone_view
            .loop_plans()
            .typed
            .iter()
            .all(|plan| plan.scratch_pool.get().is_none())
    );
}

#[test]
fn refresh_clears_bank_and_stream_local_declined_indexes() {
    let code = function(
        "function run(n,a){var p={x:a*1};var s=0;for(var i=0;i<n;i++)s+=p.x;return {s:s};}",
    );
    let mut vm = Vm::new_with_globals(&code, env(4.0, 3.0, 0.0));
    vm.declined_typed_loop_programs = u128::MAX;
    vm.virtual_values.push(Value::Number(99.0));
    vm.refresh_virtual_object_execution();
    assert_eq!(vm.declined_typed_loop_programs, 0);
    assert!(vm.virtual_values.is_empty());
    assert_eq!(field(vm.run().unwrap()), Some(Value::Number(12.0)));
}

fn forced(source: &str, choose: impl Fn(&Op) -> bool, expected_prefix: Vec<Value>) {
    let code = function(source);
    let mut vm = Vm::new_with_globals(&code, env(5.0, 2.0, 3.0));
    let view = FrameProgramView::new(
        &code,
        vm.authoritative_slots,
        vm.current.virtual_function_context_safe,
    );
    let ip = view
        .execution_code
        .iter()
        .position(choose)
        .expect("selected physical op");
    assert!(
        view.loop_plans()
            .typed
            .iter()
            .any(|p| p.sites.iter().any(|s| s.ip as usize == ip))
    );
    let (result, snapshot) = super::test_handback::at(ip, || vm.run());
    let (resume, stack) = snapshot.expect("native entry must reach forced site");
    assert_eq!(resume, ip);
    assert_eq!(
        stack, expected_prefix,
        "restore physical operands, not expanded virtual values"
    );
    assert_eq!(field(result.unwrap()), Some(Value::Number(25.0)));
}

#[test]
fn virtual_binary_handback_restores_only_the_physical_prefix() {
    forced(
        "function run(n,a,b){var p={x:a*1,y:b*1};var s=0;for(var i=0;i<n;i++)s+=p.x+p.y;return {s:s};}",
        |op| matches!(op, Op::LoadVirtualBinary { .. }),
        vec![Value::Number(5.0)],
    );
}

#[test]
fn virtual_load_discard_handback_retains_receiver_operand_until_vm_consumes_it() {
    forced(
        "function run(n,a,b){var p={x:a+b};var s=0,last=0;for(var i=0;i<n;i++)s+=(last=(true?p:p).x);return {s:s,last:last};}",
        |op| matches!(op, Op::LoadVirtualValue { discard: 1, .. }),
        vec![Value::Number(5.0), Value::Undefined],
    );
}

#[test]
fn virtual_computed_load_discard_two_handback_keeps_receiver_and_key() {
    super::test_handback::with_unforwarded(|| {
        forced(
            "function run(n,a,b){var p={x:a+b};var s=0,last=0;for(var i=0;i<n;i++)s+=(last=(true?p:p)[(true?'x':'x')]);return {s:s,last:last};}",
            |op| matches!(op, Op::LoadVirtualValue { discard: 2, .. }),
            vec![
                Value::Number(5.0),
                Value::Undefined,
                Value::String("x".into()),
            ],
        )
    });
}

#[test]
fn virtual_length_discard_handback_retains_the_physical_receiver() {
    // Five elements make the existing forced helper's result 25. Length is
    // literal-layout metadata, not a bank value or a folded loop result.
    forced(
        "function run(n,a,b){var p=[a,b,3,4,5];var s=0,last=0;for(var i=0;i<n;i++)s+=(last=(true?p:p).length);return {s:s,last:last};}",
        |op| matches!(op, Op::LoadVirtualLength { discard: 1, .. }),
        vec![Value::Number(5.0), Value::Undefined],
    );
}

#[test]
fn full_and_data_only_execute_different_streams_with_fresh_bank_inputs() {
    let code = function(
        "function run(n,a,b){var p={x:a*1};var add=function(x){return x+1;};var s=0;for(var i=0;i<n;i++)s+=p.x;s+=add(a);return {s:s};}",
    );
    let full = FrameProgramView::new(&code, u128::MAX, true);
    let data = FrameProgramView::new(&code, u128::MAX, false);
    assert!(!std::ptr::eq(full.execution_code, data.execution_code));
    assert!(
        full.execution_code
            .iter()
            .any(|op| matches!(op, Op::CallVirtualFunction { .. }))
    );
    assert!(
        !data
            .execution_code
            .iter()
            .any(|op| matches!(op, Op::CallVirtualFunction { .. }))
    );
    for (allow, a, expected) in [(true, 2.0, 11.0), (false, 3.0, 16.0), (true, 7.0, 36.0)] {
        let mut vm = Vm::new_with_globals(&code, env(4.0, a, 0.0));
        vm.current.virtual_function_context_safe = allow;
        assert_eq!(field(vm.run().unwrap()), Some(Value::Number(expected)));
    }
}

#[test]
fn residency_handback_preserves_current_bank_and_local_values() {
    let result = super::test_residency::with_limit(2, || {
        crate::eval(
        "function run(n,a){var p={x:a*1,y:3};var s=0;for(var i=0;i<n;i++)s+=p.x+p.y;return {s:s};}
         [run(0,1).s,run(1,2).s,run(7,4).s,run(8,5).s].join('|');")
    });
    assert_eq!(result, Ok(Value::String("0|5|49|64".into())));
}

#[test]
fn fused_backedge_coordinates_deduplicate_retained_jump_and_fail_closed() {
    let code = function(
        "function run(n,a,b){var p={x:a*1};var s=0;for(var i=0;i<n;i++)s+=p.x;return {s:s};}",
    );
    let view = FrameProgramView::new(&code, u128::MAX, true);
    let mut stream = view.execution_code.to_vec();
    let (ip, slot, skip, header) = stream
        .iter()
        .enumerate()
        .find_map(|(ip, op)| match *op {
            Op::IncrementLocal {
                slot,
                skip,
                jump: Some(header),
            } => Some((ip, slot, skip, header)),
            _ => None,
        })
        .unwrap();
    let tail = ip + 1 + skip;
    assert!(matches!(stream[tail],Op::Jump(target) if target==header));
    let plans = super::compile_stream(&code, &stream);
    assert_eq!(
        plans
            .iter()
            .filter(|p| p.header == header && p.backedge == tail)
            .count(),
        1
    );
    stream[ip] = Op::IncrementLocal {
        slot,
        skip,
        jump: None,
    };
    assert_eq!(
        super::compile_stream(&code, &stream).len(),
        1,
        "jumpNone falls through to retained ordinary Jump"
    );
    stream[ip] = Op::IncrementLocal {
        slot,
        skip,
        jump: Some(header),
    };
    // A direct target into the consumed suffix has no program index. Do not
    // relocate it to a convenient retained operation or invent its stack.
    stream[header] = Op::Jump(tail);
    assert!(super::compile_stream(&code, &stream).is_empty());
    stream[header] = view.execution_code[header].clone();
    stream[ip] = Op::IncrementLocal {
        slot,
        skip,
        jump: Some(header.saturating_sub(1)),
    };
    assert!(super::compile_stream(&code, &stream).is_empty());
}
