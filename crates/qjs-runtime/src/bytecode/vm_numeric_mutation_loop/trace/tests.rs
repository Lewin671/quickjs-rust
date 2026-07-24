use std::collections::BTreeSet;

use crate::bytecode::{
    compiler,
    ir::{Bytecode, Op},
    vm::Vm,
};
use crate::{Value, eval, value::ArrayRef};

use super::super::{NumericMutationLoopKind, NumericMutationLoopPlan, SpecialPlan, dense};
use super::{
    NumericTraceSourceRegion, reset_test_counters, test_attempts, test_declines,
    test_direct_writes, test_entries, test_entry_stack_depth, test_exit_stack_depth,
    test_index_conversions, test_lease_batches, test_middle_completions,
    test_native_inner_iterations, test_normal_exits, test_number_loads, test_outer_completions,
    test_plans, test_readonly_number_loads, test_seed_count,
};

const RADIX2_BODY: &str = r#"
function transform(real, imag, tableReal, tableImag, bound) {
  var span = 1, stepReal, stepImag, phaseReal, phaseImag, lane, index, off, tr, ti, tmp;
  while (span < bound) {
    stepReal = tableReal[span];
    stepImag = tableImag[span];
    phaseReal = 1;
    phaseImag = 0;
    for (lane = 0; lane < span; lane++) {
      index = lane;
      while (index < bound) {
        off = index + span;
        tr = phaseReal * real[off] - phaseImag * imag[off];
        ti = phaseReal * imag[off] + phaseImag * real[off];
        real[off] = real[index] - tr;
        imag[off] = imag[index] - ti;
        real[index] = real[index] + tr;
        imag[index] = imag[index] + ti;
        index += span << 1;
      }
      tmp = phaseReal;
      phaseReal = tmp * stepReal - phaseImag * stepImag;
      phaseImag = tmp * stepImag + phaseImag * stepReal;
    }
    span = span << 1;
  }
  return real.join(',') + '|' + imag.join(',');
}
"#;

const BASE_SETUP: &str = r#"
var real=[0,1,2,3,4,5,6,7];
var imag=[7,6,5,4,3,2,1,0];
var tableReal=[0,0.5,0.25,0.125,0,0,0,0];
var tableImag=[0,-0.5,-0.25,-0.125,0,0,0,0];
"#;

const EXPECTED_BOUND_8: &str = "28,-1,-4,-1,-16,-1,-4,-1|28,1.5,4,0.5,16,1.5,4,0.5";

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

fn numeric_array(values: &[f64]) -> Value {
    Value::Array(ArrayRef::new(
        values.iter().copied().map(Value::Number).collect(),
    ))
}

fn completion_fixture_arguments() -> Vec<Value> {
    vec![
        numeric_array(&[0.0, 1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0]),
        numeric_array(&[7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0, 0.0]),
        numeric_array(&[0.0, 0.5, 0.25, 0.125, 0.125, 0.0, 0.0, 0.0]),
        numeric_array(&[0.0, -0.5, -0.25, -0.125, -0.125, 0.0, 0.0, 0.0]),
        Value::Number(8.0),
    ]
}

fn run_completion_fixture(
    bytecode: &Bytecode,
    force_interpreter: bool,
) -> (Value, Vec<Option<Value>>) {
    let arguments = completion_fixture_arguments();
    let mut vm = Vm::new_test_direct_call(bytecode, &arguments)
        .expect("fixture direct-call VM should initialize");
    if force_interpreter {
        vm.numeric_mutation_loop_plans.clear();
        vm.numeric_loop_plans.clear();
        vm.control_loop_plans.clear();
    }
    let result = vm.run().expect("fixture bytecode should complete");
    (result, vm.locals.clone())
}

fn trace_count(bytecode: &Bytecode) -> usize {
    NumericMutationLoopPlan::compile_all(bytecode)
        .iter()
        .filter(|candidate| {
            matches!(
                &candidate.kind,
                NumericMutationLoopKind::Special(special)
                    if matches!(special.as_ref(), SpecialPlan::NumericTrace { .. })
            )
        })
        .count()
}

fn compiled_trace(bytecode: &Bytecode) -> super::NumericTracePlan {
    NumericMutationLoopPlan::compile_all(bytecode)
        .into_iter()
        .find_map(|candidate| match candidate.kind {
            NumericMutationLoopKind::Special(special) => match special.as_ref() {
                SpecialPlan::NumericTrace { plan, .. } => Some(plan.as_ref().clone()),
                _ => None,
            },
            NumericMutationLoopKind::Named(_) | NumericMutationLoopKind::Dense(_) => None,
        })
        .expect("expected one Numeric Trace plan")
}

fn slow_radix2_body() -> String {
    slow_body(RADIX2_BODY)
}

fn slow_body(body: &str) -> String {
    body.replacen("transform(", "slowTransform(", 1).replacen(
        "        off = index + span;",
        "        opaque();\n        off = index + span;",
        1,
    )
}

fn paired_body_source(body: &str, extra: &str, bound: &str) -> String {
    let slow = slow_body(body);
    format!(
        r#"
        {body}
        {slow}
        function opaque() {{}}
        function scenario(algorithm) {{
          {BASE_SETUP}
          {extra}
          return algorithm(real,imag,tableReal,tableImag,{bound});
        }}
        scenario(transform) === scenario(slowTransform);
        "#
    )
}

fn paired_trace_source(extra: &str, bound: &str) -> String {
    paired_body_source(RADIX2_BODY, extra, bound)
}

fn radix2_with_prologue(extra: &str) -> String {
    RADIX2_BODY.replacen(
        "  while (span < bound) {",
        &format!("  {extra}\n  while (span < bound) {{"),
        1,
    )
}

#[test]
fn compiles_the_radix2_pair_nest_after_the_fixed_dense_priority() {
    let bytecode = nested_function(RADIX2_BODY);
    assert_eq!(trace_count(&bytecode), 1, "{:#?}", bytecode.code);

    let plan = compiled_trace(&bytecode);
    let metadata = plan.test_metadata();
    assert_eq!(metadata.depth, 3);
    assert!(metadata.outer_header < metadata.middle_header);
    assert!(metadata.middle_header < metadata.inner_header);
    assert!(metadata.inner_backedge < metadata.middle_backedge);
    assert!(metadata.middle_backedge < metadata.outer_backedge);
    assert_eq!(metadata.outer_exit, metadata.outer_backedge + 1);
    assert_eq!(metadata.writable_receivers, 2);
    assert_eq!(metadata.readable_receivers, 2);
    assert!(metadata.materialized_live_out_alias_slots.is_empty());

    let kernel = &metadata.kernel;
    assert_eq!(kernel.raw_source_operations, 40);
    assert_eq!(kernel.canonical_operations, 30);
    assert_eq!(kernel.executable_operations, 22);
    assert_eq!(kernel.pure_prefix_operations, 3);
    assert_eq!(kernel.logical_loads, 8);
    assert_eq!(kernel.logical_stores, 4);
    assert_eq!(kernel.unique_index_vns, 2);
    assert_eq!(kernel.unique_cells, 4);
    assert_eq!(kernel.potential_alias_pairs, 2);
    assert_eq!(kernel.initial_load_cells, 4);
    assert_eq!(kernel.commit_events, 4);
    assert_eq!(kernel.local_write_count, 12);
    assert_eq!(kernel.cell_descriptors.len(), kernel.unique_cells);
    for cell in &kernel.cell_descriptors {
        assert!(cell.receiver < 4);
        assert!(cell.index_vn < kernel.executable_operations);
        assert!(cell.index_slot < kernel.unique_index_vns);
        assert!(cell.resolved_slot >= kernel.unique_index_vns);
        assert!(cell.initial_load_vn.is_some());
        assert!(cell.final_memory_version <= kernel.logical_stores);
        assert!(cell.final_store_ordinal.is_some());
    }
    assert_eq!(kernel.commit_descriptors.len(), kernel.commit_events);
    for commit in &kernel.commit_descriptors {
        assert!(commit.cell < kernel.unique_cells);
        assert!(commit.value_vn < kernel.executable_operations);
        assert!(commit.store_ordinal < kernel.logical_stores);
        assert!(commit.source_operation < kernel.raw_source_operations);
        assert!(commit.memory_version <= kernel.logical_stores);
    }
    assert_eq!(kernel.store_ordinals, vec![0, 1, 2, 3]);
}

#[test]
fn executes_the_owned_radix2_region_and_hands_back_after_the_outer_exit() {
    reset_test_counters();
    let source = format!("{RADIX2_BODY}{BASE_SETUP}transform(real,imag,tableReal,tableImag,8);");
    assert_eq!(
        eval(&source),
        Ok(Value::String(EXPECTED_BOUND_8.to_owned().into()))
    );
    assert_eq!(test_plans(), 1);
    assert_eq!(test_attempts(), 1);
    assert_eq!(test_entries(), 1);
    assert_eq!(test_normal_exits(), 1);
    assert_eq!(test_lease_batches(), 1);
    assert_eq!(test_seed_count(), 1);
    assert_eq!(test_native_inner_iterations(), 11);
    assert_eq!(test_middle_completions(), 7);
    assert_eq!(test_outer_completions(), 3);
    assert_eq!(test_declines(), 0);
    assert_eq!(test_index_conversions(), 22);
    assert_eq!(test_number_loads(), 44);
    assert_eq!(test_readonly_number_loads(), 4);
    assert_eq!(test_direct_writes(), 44);
    assert_eq!(test_entry_stack_depth(), 11);
    assert_eq!(test_exit_stack_depth(), test_entry_stack_depth());
}

#[test]
fn zero_remaining_kernel_entry_still_completes_middle_and_outer_schedules() {
    reset_test_counters();
    let source = format!(
        "{RADIX2_BODY}var real=[0,1], imag=[1,0], tableReal=[0,0.5], tableImag=[0,-0.5]; transform(real,imag,tableReal,tableImag,2);"
    );
    assert_eq!(
        eval(&source),
        Ok(Value::String("1,-1|1,1".to_owned().into()))
    );
    assert_eq!(test_attempts(), 1);
    assert_eq!(test_entries(), 1);
    assert_eq!(test_native_inner_iterations(), 0);
    assert_eq!(test_middle_completions(), 1);
    assert_eq!(test_outer_completions(), 1);
    assert_eq!(test_declines(), 0);
}

#[test]
fn trace_array_guard_matrix_preserves_interpreter_results() {
    for (name, extra, enters, leases) in [
        ("baseline", "", true, 1),
        (
            "sealed writers",
            "Object.seal(real); Object.seal(imag);",
            true,
            1,
        ),
        (
            "nonextensible writers",
            "Object.preventExtensions(real); Object.preventExtensions(imag);",
            true,
            1,
        ),
        (
            "nonwritable lengths",
            "Object.defineProperty(real,'length',{writable:false}); Object.defineProperty(imag,'length',{writable:false});",
            true,
            1,
        ),
        (
            "frozen readers",
            "Object.freeze(tableReal); Object.freeze(tableImag);",
            true,
            1,
        ),
        ("read/read alias", "tableImag=tableReal;", true, 1),
        ("writer/writer alias", "imag=real;", false, 0),
        ("writer/read alias", "tableReal=real;", false, 0),
        ("writer hole", "delete real[6];", false, 0),
        (
            "writer accessor",
            "Object.defineProperty(real,'6',{configurable:true,get:function(){return 6;},set:function(_) {}});",
            false,
            0,
        ),
        ("frozen writer", "Object.freeze(real);", false, 0),
        (
            "nonnumeric reader prefix",
            "tableReal[6]='unused';",
            false,
            1,
        ),
        ("short reader prefix", "tableReal.length=5;", false, 1),
        ("nonnumeric writer prefix", "real[6]='6';", false, 1),
        ("short writer prefix", "real.length=5;", false, 1),
        ("proxy writer", "real=new Proxy(real,{});", false, 0),
    ] {
        reset_test_counters();
        let result = eval(&paired_trace_source(extra, "8"));
        assert_eq!(result, Ok(Value::Boolean(true)), "{name}");
        assert_eq!(test_attempts(), 1, "{name}");
        assert_eq!(test_entries(), usize::from(enters), "{name}");
        assert_eq!(test_declines(), usize::from(!enters), "{name}");
        assert_eq!(test_lease_batches(), leases, "{name}");
        if !enters {
            assert_eq!(test_direct_writes(), 0, "{name}");
        }
    }
}

#[test]
fn trace_guard_miss_installs_nested_fallback_once_without_replaying_the_seed() {
    reset_test_counters();
    dense::reset_test_iterations();
    assert_eq!(
        eval(&paired_trace_source("tableReal=real;", "8")),
        Ok(Value::Boolean(true))
    );
    assert_eq!(test_attempts(), 1);
    assert_eq!(test_entries(), 0);
    assert_eq!(test_declines(), 1);
    assert_eq!(test_lease_batches(), 0);
    assert_eq!(test_direct_writes(), 0);
    assert_eq!(dense::test_nested_dense_entries(), 3);
}

#[test]
fn compiler_rejects_eval_with_and_try_anywhere_in_the_function() {
    for extra in [
        "eval('');",
        "with ({}) { phaseReal = phaseReal; }",
        "try { phaseReal = phaseReal; } finally {}",
        "using resource = { [Symbol.dispose]() {} };",
    ] {
        let source = radix2_with_prologue(extra);
        assert_eq!(trace_count(&nested_function(&source)), 0, "{extra}");
    }
}

#[test]
fn compiler_rejects_explicit_suspension_and_records_the_no_suspend_limitation() {
    let generator = RADIX2_BODY
        .replacen("function transform(", "function* transform(", 1)
        .replacen(
            "  while (span < bound) {",
            "  yield 0;\n  while (span < bound) {",
            1,
        );
    let asynchronous = RADIX2_BODY
        .replacen("function transform(", "async function transform(", 1)
        .replacen(
            "  while (span < bound) {",
            "  await 0;\n  while (span < bound) {",
            1,
        );
    assert_eq!(trace_count(&nested_function(&generator)), 0);
    assert_eq!(trace_count(&nested_function(&asynchronous)), 0);

    // Function-kind flags currently live on the enclosing NewFunction op, not
    // on the nested Bytecode. Without Yield/Await, this compiler entry point
    // therefore cannot distinguish either body from an ordinary function.
    for no_suspend in [
        RADIX2_BODY.replacen("function transform(", "function* transform(", 1),
        RADIX2_BODY.replacen("function transform(", "async function transform(", 1),
    ] {
        assert_eq!(trace_count(&nested_function(&no_suspend)), 1);
    }
}

#[test]
fn no_suspend_generator_and_async_bodies_decline_safely_and_match_the_opaque_oracle() {
    let generator = RADIX2_BODY.replacen("function transform(", "function* transform(", 1);
    let slow_generator = slow_body(&generator);
    reset_test_counters();
    let generator_source = format!(
        r#"
        {generator}
        {slow_generator}
        function opaque() {{}}
        {BASE_SETUP}
        var realSlow=real.slice(), imagSlow=imag.slice();
        var fast=transform(real,imag,tableReal,tableImag,8).next();
        var slow=slowTransform(realSlow,imagSlow,tableReal,tableImag,8).next();
        fast.done===slow.done && fast.value===slow.value;
        "#
    );
    assert_eq!(eval(&generator_source), Ok(Value::Boolean(true)));
    assert_eq!(test_attempts(), 1);
    assert_eq!(test_entries(), 0);
    assert_eq!(test_declines(), 1);

    let asynchronous = RADIX2_BODY
        .replacen(
            "function transform(real, imag, tableReal, tableImag, bound)",
            "async function transform(real, imag, tableReal, tableImag, bound, observed)",
            1,
        )
        .replacen(
            "  return real.join(',') + '|' + imag.join(',');",
            "  observed[0]=real.join(',')+'|'+imag.join(','); return observed[0];",
            1,
        );
    let slow_asynchronous = slow_body(&asynchronous);
    reset_test_counters();
    let async_source = format!(
        r#"
        {asynchronous}
        {slow_asynchronous}
        function opaque() {{}}
        {BASE_SETUP}
        var realSlow=real.slice(), imagSlow=imag.slice();
        var observed=[], observedSlow=[];
        transform(real,imag,tableReal,tableImag,8,observed);
        slowTransform(realSlow,imagSlow,tableReal,tableImag,8,observedSlow);
        observed[0]===observedSlow[0];
        "#
    );
    assert_eq!(eval(&async_source), Ok(Value::Boolean(true)));
    assert_eq!(test_attempts(), 1);
    assert_eq!(test_entries(), 0);
    assert_eq!(test_declines(), 1);
}

#[test]
fn materializes_a_direct_local_completion_live_out_at_its_source_store() {
    // Redeclaration initializers update the numeric schedule while their
    // VarStatements produce Empty completion, so the final inner expression
    // remains the completion carried through the middle and outer loops.
    let body = RADIX2_BODY
        .replacen(
            "        index += span << 1;",
            "        index += span << 1;\n        phaseReal;",
            1,
        )
        .replacen(
            "      tmp = phaseReal;\n      phaseReal = tmp * stepReal - phaseImag * stepImag;\n      phaseImag = tmp * stepImag + phaseImag * stepReal;",
            "      var tmp = phaseReal;\n      var phaseReal = tmp * stepReal - phaseImag * stepImag;\n      var phaseImag = tmp * stepImag + phaseImag * stepReal;",
            1,
        )
        .replacen(
            "    span = span << 1;",
            "    var span = span << 1;",
            1,
        )
        .replace("  return real.join(',') + '|' + imag.join(',');\n", "");
    let bytecode = nested_function(&body);
    assert_eq!(trace_count(&bytecode), 1);
    let plan = compiled_trace(&bytecode);
    let metadata = plan.test_metadata();
    assert!(
        !metadata.materialized_live_out_alias_slots.is_empty(),
        "the fixture must exercise a materialized alias live-out"
    );
    let completion_slots: Vec<_> = metadata
        .post_handoff_read_slots
        .iter()
        .copied()
        .filter(|slot| bytecode.local_is_compiler_temporary(*slot))
        .collect();
    let [completion_slot] = completion_slots.as_slice() else {
        panic!(
            "the fixture should have exactly one compiler-temporary post-handoff completion slot: {:?}",
            metadata.post_handoff_read_slots
        );
    };
    assert!(matches!(
        bytecode.code.get(metadata.outer_exit + 1),
        Some(Op::LoadLocal(slot)) if slot == completion_slot
    ));
    assert!(
        !bytecode.code[metadata.outer_exit + 2..]
            .iter()
            .any(|operation| operation_writes_slot(operation, *completion_slot)),
        "the post-handoff completion load must not be followed by a later overwrite"
    );

    let unique_targets = metadata
        .final_reaching_alias_dependencies
        .iter()
        .map(|dependency| dependency.target)
        .collect::<BTreeSet<_>>();
    assert_eq!(
        unique_targets.len(),
        metadata.final_reaching_alias_dependencies.len(),
        "each target must have exactly one final reaching alias definition"
    );
    assert_eq!(*completion_slot, 16);
    let reaching_definition = |target| {
        metadata
            .final_reaching_alias_dependencies
            .iter()
            .find(|dependency| dependency.target == target)
            .copied()
            .unwrap_or_else(|| panic!("missing final reaching definition for slot {target}"))
    };
    let outer_alias = reaching_definition(*completion_slot);
    assert_eq!(
        (outer_alias.region, outer_alias.target, outer_alias.source),
        (NumericTraceSourceRegion::OuterEpilogue, 16, 18)
    );
    let middle_alias = reaching_definition(outer_alias.source);
    assert_eq!(
        (
            middle_alias.region,
            middle_alias.target,
            middle_alias.source,
        ),
        (NumericTraceSourceRegion::MiddleEpilogue, 18, 20)
    );
    let inner_alias = reaching_definition(middle_alias.source);
    assert_eq!(
        (inner_alias.region, inner_alias.target, inner_alias.source),
        (NumericTraceSourceRegion::Inner, 20, 8)
    );
    let alias_target = inner_alias.target;
    assert!(
        metadata
            .materialized_live_out_alias_slots
            .contains(&alias_target)
    );
    assert!(bytecode.local_is_compiler_temporary(alias_target));
    assert!(
        metadata.kernel_local_write_slots.contains(&alias_target),
        "the materialized alias must survive kernel lowering as a LocalWrite"
    );
    let source_store_ips: Vec<_> = bytecode.code[metadata.inner_header..metadata.inner_backedge]
        .iter()
        .enumerate()
        .filter_map(|(offset, operation)| {
            matches!(
                operation,
                Op::StoreLocal(slot) | Op::AssignLocal(slot) if *slot == alias_target
            )
            .then_some(metadata.inner_header + offset)
        })
        .collect();
    assert!(
        !source_store_ips.is_empty(),
        "the materialized alias must originate at a store in the owned inner region"
    );

    reset_test_counters();
    let (traced_result, traced_locals) = run_completion_fixture(&bytecode, false);
    assert_eq!(test_attempts(), 1);
    assert_eq!(test_entries(), 1);
    assert_eq!(test_declines(), 0);
    let (slow_result, slow_locals) = run_completion_fixture(&bytecode, true);
    assert_eq!(
        test_attempts(),
        1,
        "the forced-slow VM must not run a loop plan"
    );
    assert_eq!(
        test_entries(),
        1,
        "the forced-slow VM must not enter the trace"
    );
    assert_eq!(
        test_declines(),
        0,
        "the forced-slow VM must not probe the trace"
    );
    assert_eq!(traced_result, slow_result);
    let traced_bits = number_slot_bits(&traced_locals, *completion_slot);
    let slow_bits = number_slot_bits(&slow_locals, *completion_slot);
    let traced_alias_bits = number_slot_bits(&traced_locals, alias_target);
    let slow_alias_bits = number_slot_bits(&slow_locals, alias_target);
    assert_eq!(
        traced_alias_bits, slow_alias_bits,
        "the materialized inner alias must match the interpreter bit-for-bit"
    );
    assert_eq!(
        slow_alias_bits,
        (-0.00390625_f64).to_bits(),
        "the materialized inner alias oracle must remain explicit and nonzero"
    );
    assert_eq!(
        traced_bits, slow_bits,
        "the trace must publish the interpreter's exact loop completion bits"
    );
    assert_eq!(
        slow_bits,
        (-0.00390625_f64).to_bits(),
        "the nonzero table[4] fixture must retain the final inner completion"
    );
}

fn operation_writes_slot(operation: &Op, target: usize) -> bool {
    match operation {
        Op::StoreLocal(slot)
        | Op::AssignLocal(slot)
        | Op::ClearLocal(slot)
        | Op::AppendStringLiteralLocal { slot, .. }
        | Op::IncrementLocal { slot, .. } => *slot == target,
        Op::CopyLocal { to, .. } => *to == target,
        Op::BinaryAssignLocals {
            target: slot,
            stores,
            ..
        } => *slot == target || stores.contains(&target),
        _ => false,
    }
}

fn number_slot_bits(locals: &[Option<Value>], slot: usize) -> u64 {
    let Some(Some(Value::Number(value))) = locals.get(slot) else {
        panic!(
            "completion slot {slot} was not a number: {:?}",
            locals.get(slot)
        );
    };
    value.to_bits()
}

#[test]
fn bitwise_payload_kernel_matches_the_opaque_oracle() {
    let body = RADIX2_BODY
        .replace(
            "tr = phaseReal * real[off] - phaseImag * imag[off];",
            "tr = (phaseReal | 0) ^ (real[off] | 0);",
        )
        .replace(
            "ti = phaseReal * imag[off] + phaseImag * real[off];",
            "ti = (phaseImag | 0) ^ (imag[off] | 0);",
        )
        .replace(
            "real[off] = real[index] - tr;",
            "real[off] = (real[index] - tr) | 0;",
        )
        .replace(
            "imag[off] = imag[index] - ti;",
            "imag[off] = (imag[index] - ti) | 0;",
        )
        .replace(
            "real[index] = real[index] + tr;",
            "real[index] = (real[index] + tr) | 0;",
        )
        .replace(
            "imag[index] = imag[index] + ti;",
            "imag[index] = (imag[index] + ti) | 0;",
        );
    assert_eq!(trace_count(&nested_function(&body)), 1);

    reset_test_counters();
    assert_eq!(
        eval(&paired_body_source(&body, "", "8")),
        Ok(Value::Boolean(true))
    );
    assert_eq!(test_entries(), 1);
    assert_eq!(test_declines(), 0);
}

#[test]
fn captured_trace_local_declines_before_successful_entry() {
    let body = radix2_with_prologue("function observeSpan(){ return span; }");
    assert_eq!(trace_count(&nested_function(&body)), 1);

    reset_test_counters();
    let source = format!("{body}{BASE_SETUP}transform(real,imag,tableReal,tableImag,8);");
    assert_eq!(
        eval(&source),
        Ok(Value::String(EXPECTED_BOUND_8.to_owned().into()))
    );
    assert_eq!(test_attempts(), 1);
    assert_eq!(test_entries(), 0);
    assert_eq!(test_declines(), 1);
}

#[test]
fn trace_matches_interpreter_for_nan_signed_zero_and_infinities() {
    reset_test_counters();
    let slow = slow_radix2_body();
    let source = format!(
        r#"
        {RADIX2_BODY}
        {slow}
        function opaque() {{}}
        function sameArray(left,right) {{
          if (left.length !== right.length) return false;
          for (var i=0;i<left.length;i++) if (!Object.is(left[i],right[i])) return false;
          return true;
        }}
        var real=[NaN,0,-0,Infinity,-Infinity,1,-1,2];
        var imag=[-Infinity,-0,0,NaN,Infinity,-1,1,-2];
        var realSlow=real.slice(), imagSlow=imag.slice();
        var tableReal=[0,0.5,0.25,0.125,0,0,0,0];
        var tableImag=[0,-0.5,-0.25,-0.125,0,0,0,0];
        transform(real,imag,tableReal,tableImag,8);
        slowTransform(realSlow,imagSlow,tableReal,tableImag,8);
        sameArray(real,realSlow) && sameArray(imag,imagSlow);
        "#
    );
    assert_eq!(eval(&source), Ok(Value::Boolean(true)));
    assert_eq!(test_entries(), 1);
    assert_eq!(test_declines(), 0);
}

#[test]
fn invalid_radix2_bound_declines_without_progress_and_matches_the_interpreter() {
    for (name, source) in [
        ("non-power quotient", paired_trace_source("", "6")),
        ("fractional bound", paired_trace_source("", "6.5")),
        (
            "fractional span",
            paired_body_source(
                &RADIX2_BODY.replacen("var span = 1,", "var span = 1.5,", 1),
                "",
                "8",
            ),
        ),
    ] {
        reset_test_counters();
        assert_eq!(eval(&source), Ok(Value::Boolean(true)), "{name}");
        assert_eq!(test_attempts(), 1, "{name}");
        assert_eq!(test_entries(), 0, "{name}");
        assert_eq!(test_declines(), 1, "{name}");
        assert_eq!(test_lease_batches(), 0, "{name}");
        assert_eq!(test_direct_writes(), 0, "{name}");
    }
}

#[test]
fn outer_exit_handoff_executes_a_getter_backed_sqrt_once() {
    let body = RADIX2_BODY.replacen(
        "  return real.join(',') + '|' + imag.join(',');",
        "  var spectrum=Math.sqrt(real[0]*real[0]+imag[0]*imag[0]); return spectrum + ':' + real.join(',') + '|' + imag.join(',');",
        1,
    );
    reset_test_counters();
    let source = format!(
        r#"
        {body}
        var getterHits=0, callHits=0;
        Object.defineProperty(Math,'sqrt',{{configurable:true,get:function(){{
          getterHits++;
          return function(value){{callHits++;return value+0.25;}};
        }}}});
        {BASE_SETUP}
        transform(real,imag,tableReal,tableImag,8)+'|'+getterHits+'|'+callHits;
        "#
    );
    let result = eval(&source);
    assert!(
        matches!(result, Ok(Value::String(ref value)) if value.ends_with("|1|1")),
        "{result:?}"
    );
    assert_eq!(test_entries(), 1);
    assert_eq!(test_normal_exits(), 1);
}
