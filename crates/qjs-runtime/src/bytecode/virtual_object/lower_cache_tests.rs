use super::*;
use crate::bytecode::{frame_program::FrameProgramView, vm::Vm};
use crate::function::CallEnv;
use std::{collections::HashMap, mem::size_of};

fn body() -> Bytecode {
    body_source("function run(n,a){var p={x:a*1};var s=0;for(var i=0;i<n;i++)s+=p.x;return s;}")
}

fn body_source(source: &str) -> Bytecode {
    let script = qjs_parser::parse_script(source).unwrap();
    crate::bytecode::compile_script(&script)
        .unwrap()
        .code
        .iter()
        .find_map(|op| match op {
            Op::NewFunction { bytecode, .. } => Some(bytecode.as_ref().clone()),
            _ => None,
        })
        .unwrap()
}

fn frame(code: &Bytecode, n: f64, a: f64) -> Vm<'_> {
    Vm::new_with_globals(
        code,
        CallEnv::with_locals(
            crate::function::new_realm(HashMap::new()),
            HashMap::from([
                ("n".into(), Value::Number(n)),
                ("a".into(), Value::Number(a)),
            ]),
        ),
    )
}

fn caches_are_empty(code: &Bytecode) {
    let lowering = code.virtual_object_program.get().unwrap();
    assert!(lowering.full.typed_loops.get().is_none());
    assert!(lowering.data_only.typed_loops.get().is_none());
    assert!(code.typed_loop_programs.get().is_none());
}

#[test]
fn view_and_inline_variant_cache_hold_only_their_required_handles() {
    // Relative checks apply to both native architectures; Bytecode's concrete
    // size is recorded separately and is not a portable layout contract.
    assert_eq!(
        size_of::<FrameProgramView<'_>>(),
        size_of::<(&Bytecode, &[Op])>()
    );
    assert_eq!(
        size_of::<OnceCell<Box<VariantTypedPrograms>>>(),
        size_of::<usize>()
    );
}

#[test]
fn activation_and_forward_jumps_leave_all_typed_tables_unallocated() {
    let code = body();
    let mut vm = frame(&code, 0.0, 3.0);
    let view = FrameProgramView::new(&code, vm.authoritative_slots, true);
    caches_are_empty(&code);
    assert!(!vm.jump_with_selected_loop_plans(&view, 10, 2));
    assert_eq!(vm.ip, 10);
    assert!(!vm.jump_with_selected_loop_plans(&view, 2, 2));
    assert_eq!(vm.ip, 2);
    caches_are_empty(&code);

    let mut zero_trip = frame(&code, 0.0, 3.0);
    assert_eq!(zero_trip.run(), Ok(Value::Number(0.0)));
    caches_are_empty(&code);
}

#[test]
fn only_the_first_real_edge_allocates_the_selected_table() {
    let code = body();
    let view = FrameProgramView::new(&code, u128::MAX, true);
    caches_are_empty(&code);
    let mut first = frame(&code, 2.0, 3.0);
    assert_eq!(first.run(), Ok(Value::Number(6.0)));
    let lowering = code.virtual_object_program.get().unwrap();
    let table = lowering.full.typed_loops.get().unwrap().as_ref() as *const _;
    assert!(lowering.data_only.typed_loops.get().is_none());
    assert!(code.typed_loop_programs.get().is_none());
    let mut second = frame(&code, 3.0, 7.0);
    assert_eq!(second.run(), Ok(Value::Number(21.0)));
    assert!(std::ptr::eq(
        table,
        lowering.full.typed_loops.get().unwrap().as_ref()
    ));
    assert!(!view.loop_plans(true).typed.is_empty());
}

#[test]
fn an_authority_drop_does_not_reselect_the_live_executable_cache() {
    let code = body();
    let mut vm = frame(&code, 3.0, 7.0);
    let view = FrameProgramView::new(&code, vm.authoritative_slots, true);
    assert!(!std::ptr::eq(view.execution_code, code.code.as_slice()));
    let (header, backedge) = view
        .execution_code
        .iter()
        .enumerate()
        .find_map(|(ip, op)| match op {
            Op::IncrementLocal {
                skip,
                jump: Some(target),
                ..
            } if *target < ip => Some((*target, ip + 1 + skip)),
            _ => None,
        })
        .unwrap();
    vm.authoritative_slots = 0;
    // A newly selected view would now choose original code. The live view's
    // bank is deliberately not initialized: entry declines, but cache lookup
    // must still compile the stream currently held by the activation.
    assert!(std::ptr::eq(
        FrameProgramView::new(&code, 0, true).execution_code,
        code.code.as_slice()
    ));
    assert!(!vm.jump_with_selected_loop_plans(&view, header, backedge));
    let lowering = code.virtual_object_program.get().unwrap();
    assert!(lowering.full.typed_loops.get().is_some());
    assert!(lowering.data_only.typed_loops.get().is_none());
    assert!(code.typed_loop_programs.get().is_none());
    assert_eq!(vm.ip, header);
}

#[test]
fn unknown_streams_reject_both_families_even_with_an_original_numeric_plan() {
    let code = body_source(
        "function run(p) { var values=[1,2,3]; while(p--) values[p]=values[p]+1; return p; }",
    );
    let selected = FrameProgramView::new(&code, u128::MAX, true);
    let unknown = selected.execution_code.to_vec();
    let view = FrameProgramView {
        bytecode: &code,
        execution_code: &unknown,
    };
    let plans = view.loop_plans(true);
    assert!(plans.typed.is_empty());
    assert!(plans.shared_numeric_mutation.is_empty());
    assert!(!code.numeric_mutation_loop_plans.get().unwrap().is_empty());
    caches_are_empty(&code);
}

#[test]
fn a_known_stream_with_no_typed_programs_keeps_its_numeric_plans() {
    let code = body_source(
        "function run(p) { var values=[1,2,3]; while(p--) values[p]=values[p]+1; return p; }",
    );
    let selected = FrameProgramView::new(&code, u128::MAX, true);
    assert!(std::ptr::eq(selected.execution_code, code.code.as_slice()));
    code.typed_loop_programs.set(Vec::new()).unwrap();
    let plans = selected.loop_plans(true);
    assert!(plans.typed.is_empty());
    assert!(!plans.shared_numeric_mutation.is_empty());
}
