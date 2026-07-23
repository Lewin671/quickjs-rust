use super::*;
use crate::bytecode::compiler;
use crate::{Value, eval};

const FAST_REGION: &str = r#"
function fast(real, imag, size, half) {
  var stepReal = 0.5, stepImag = 0.25;
  var phaseReal = 1, phaseImag = 0, off, tr, ti, tmp, i;
  for (var outer = 0; outer < half; outer++) {
    i = outer;
    while (i < size) {
      off = i + half;
      tr = phaseReal * real[off] - phaseImag * imag[off];
      ti = phaseReal * imag[off] + phaseImag * real[off];
      real[off] = real[i] - tr;
      imag[off] = imag[i] - ti;
      real[i] += tr;
      imag[i] += ti;
      i += half << 1;
    }
    tmp = phaseReal;
    phaseReal = tmp * stepReal - phaseImag * stepImag;
    phaseImag = tmp * stepImag + phaseImag * stepReal;
  }
  return real.join(':') + '|' + imag.join(':') + '|' + phaseReal + ':' + phaseImag;
}
"#;

const SLOW_REGION: &str = r#"
function noop() {}
function slow(real, imag, size, half) {
  var stepReal = 0.5, stepImag = 0.25;
  var phaseReal = 1, phaseImag = 0, off, tr, ti, tmp, i;
  for (var outer = 0; outer < half; outer++) {
    i = outer;
    while (i < size) {
      noop();
      off = i + half;
      tr = phaseReal * real[off] - phaseImag * imag[off];
      ti = phaseReal * imag[off] + phaseImag * real[off];
      real[off] = real[i] - tr;
      imag[off] = imag[i] - ti;
      real[i] += tr;
      imag[i] += ti;
      i += half << 1;
    }
    tmp = phaseReal;
    phaseReal = tmp * stepReal - phaseImag * stepImag;
    phaseImag = tmp * stepImag + phaseImag * stepReal;
  }
  return real.join(':') + '|' + imag.join(':') + '|' + phaseReal + ':' + phaseImag;
}
"#;

const FORWARDING_REGION: &str = r#"
function region(left, right, size, outerLimit, stride, start) {
  var i;
  for (var outer = start; outer < outerLimit; outer++) {
    i = outer;
    while (i < size) {
      left[i] = left[i] + 1;
      right[i] = left[i] + right[i];
      i += stride;
    }
  }
  return left.join(':') + '|' + right.join(':');
}
"#;

const SLOW_FORWARDING_REGION: &str = r#"
function slowRegion(left, right, size, outerLimit, stride, start) {
  var i;
  for (var outer = start; outer < outerLimit; outer++) {
    i = outer;
    while (i < size) {
      void String;
      left[i] = left[i] + 1;
      right[i] = left[i] + right[i];
      i += stride;
    }
  }
  return left.join(':') + '|' + right.join(':');
}
"#;

fn nested_function(source: &str, name: &str) -> Bytecode {
    let script = qjs_parser::parse_script(source).expect("source should parse");
    let bytecode = compiler::compile_script(&script).expect("source should compile");
    bytecode
        .code
        .iter()
        .find_map(|op| match op {
            Op::NewFunction {
                name: actual,
                bytecode,
                ..
            } if actual.as_deref() == Some(name) => Some(bytecode.as_ref().clone()),
            _ => None,
        })
        .expect("named function bytecode should be nested in the script")
}

fn enclosed_edge(bytecode: &Bytecode) -> (usize, usize, EnclosingOuter) {
    let backedges: Vec<_> = bytecode
        .code
        .iter()
        .enumerate()
        .filter_map(|(backedge, op)| match op {
            Op::Jump(header) if *header < backedge => Some((*header, backedge)),
            _ => None,
        })
        .collect();
    let enclosing = discover_enclosing_outers(bytecode, &backedges);
    backedges
        .into_iter()
        .zip(enclosing)
        .find_map(|((header, backedge), outer)| outer.map(|outer| (header, backedge, outer)))
        .expect("source should contain an enclosed backward edge")
}

#[test]
fn plan_kind_routes_nested_dense_and_preserves_semantics() {
    let bytecode = nested_function(FAST_REGION, "fast");
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
    let NumericMutationLoopKind::Special(plan) = &plans[0].kind else {
        panic!("expected nested dense plan: {plans:#?}");
    };
    assert!(matches!(
        plan.as_ref(),
        SpecialPlan::NestedDense { fallback, .. }
            if fallback.is_suppressing_legacy_dynamic()
    ));
    let outer_header = bytecode
        .code
        .iter()
        .enumerate()
        .filter_map(|(ip, op)| matches!(op, Op::Jump(_)).then_some(ip))
        .max()
        .expect("outer backedge");
    assert!(plans[0].contains_instruction(outer_header));

    let source = format!(
        "{FAST_REGION}{SLOW_REGION} var a=[1,2,3,4,5,6,7,8], b=[8,7,6,5,4,3,2,1], c=a.slice(), d=b.slice(); fast(a,b,8,2) === slow(c,d,8,2);"
    );
    assert_eq!(eval(&source), Ok(Value::Boolean(true)));
}

#[test]
fn plan_kind_routes_ordinary_dense_and_preserves_semantics() {
    let source = "function run(values, n) { for (var i = 0; i < n; i++) values[i] = values[i] + 1; return values.join(':'); }";
    let bytecode = nested_function(source, "run");
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
    assert!(matches!(&plans[0].kind, NumericMutationLoopKind::Dense(_)));
    assert_eq!(
        eval(&format!("{source} run([1,2,3,4], 4);")),
        Ok(Value::String("2:3:4:5".to_owned().into()))
    );
}

#[test]
fn plan_kind_routes_predicate_scan_and_preserves_semantics() {
    let source = "function run(values, n) { var hits = 0; for (var i = 0; i < n; i++) if (values[i] & 1) hits += i; return i + ':' + hits; }";
    let bytecode = nested_function(source, "run");
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
    assert!(matches!(
        &plans[0].kind,
        NumericMutationLoopKind::Special(plan)
            if matches!(plan.as_ref(), SpecialPlan::PredicateScan(_))
    ));
    assert_eq!(
        eval(&format!("{source} run([0,2,1,3], 4);")),
        Ok(Value::String("4:5".to_owned().into()))
    );
}

#[test]
fn nested_dense_region_runs_one_seed_and_the_remaining_transactional_iterations() {
    dense::reset_test_iterations();
    let source = format!(
        "{FAST_REGION}{SLOW_REGION} var a = [1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16], b = [16,15,14,13,12,11,10,9,8,7,6,5,4,3,2,1], c = a.slice(), d = b.slice(); fast(a,b,16,4) === slow(c,d,16,4);"
    );
    assert_eq!(eval(&source), Ok(Value::Boolean(true)));
    assert_eq!(dense::test_nested_dense_entries(), 1);
    assert_eq!(dense::test_nested_dense_seeded_iterations(), 1);
    assert_eq!(dense::test_nested_dense_outer_completions(), 4);
    assert_eq!(dense::test_nested_dense_inner_commits(), 7);
    assert_eq!(dense::test_nested_dense_bailouts(), 0);
}

#[test]
fn nested_dense_region_preserves_same_iteration_store_load_forwarding() {
    dense::reset_test_iterations();
    let source = format!(
        "{FORWARDING_REGION}{SLOW_FORWARDING_REGION} var a=[1,2,3,4,5,6,7,8], b=[10,20,30,40,50,60,70,80], c=a.slice(), d=b.slice(); region(a,b,8,2,2,0) === slowRegion(c,d,8,2,2,0);"
    );
    assert_eq!(eval(&source), Ok(Value::Boolean(true)));
    assert_eq!(dense::test_nested_dense_entries(), 1);
    assert_eq!(dense::test_nested_dense_outer_completions(), 2);
    assert_eq!(dense::test_nested_dense_inner_commits(), 7);
    assert_eq!(dense::test_nested_dense_bailouts(), 0);
}

#[test]
fn nested_dense_region_distinguishes_resumed_empty_from_fresh_empty() {
    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{FORWARDING_REGION} region([1], [10], 1, 1, 2, 0);"
        )),
        Ok(Value::String("2|12".to_owned().into()))
    );
    assert_eq!(dense::test_nested_dense_entries(), 1);
    assert_eq!(dense::test_nested_dense_outer_completions(), 1);
    assert_eq!(dense::test_nested_dense_inner_commits(), 0);
    assert_eq!(dense::test_nested_dense_bailouts(), 0);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{FORWARDING_REGION} region([1,2], [10,20], 2, 4, 4, 0);"
        )),
        Ok(Value::String("2:3|12:23".to_owned().into()))
    );
    assert_eq!(dense::test_nested_dense_entries(), 1);
    assert_eq!(dense::test_nested_dense_outer_completions(), 2);
    assert_eq!(dense::test_nested_dense_inner_commits(), 1);
    assert_eq!(dense::test_nested_dense_bailouts(), 1);
}

#[test]
fn nested_dense_region_discards_first_and_mid_iteration_failures_before_replay() {
    let source = format!(
        "{FORWARDING_REGION} var hits=0, marker={{ valueOf:function(){{ hits++; return 9; }} }};"
    );
    for (values, expected_commits) in [("[1,2,marker,4,5,6,7,8]", 0), ("[1,2,3,4,marker,6,7,8]", 1)]
    {
        dense::reset_test_iterations();
        let result = eval(&format!(
            "{source} region({values}, [10,20,30,40,50,60,70,80], 8, 2, 2, 0) + '|' + hits;"
        ));
        assert!(matches!(result, Ok(Value::String(value)) if value.ends_with("|1")));
        assert_eq!(dense::test_nested_dense_entries(), 1, "{values}");
        assert_eq!(
            dense::test_nested_dense_inner_commits(),
            expected_commits,
            "{values}"
        );
        assert_eq!(dense::test_nested_dense_bailouts(), 1, "{values}");
    }

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{FORWARDING_REGION} region([1,2,3,4,5,6,7,8], [10,20], 8, 2, 2, 0);"
        )),
        Ok(Value::String(
            "2:3:4:5:6:7:8:9|12:23:NaN:NaN:NaN:NaN:NaN:NaN"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(dense::test_nested_dense_entries(), 1);
    assert_eq!(dense::test_nested_dense_inner_commits(), 0);
    assert_eq!(dense::test_nested_dense_bailouts(), 1);
}

#[test]
fn nested_dense_region_fails_closed_for_aliases_holes_and_integrity() {
    for (setup, call, expect_entry) in [
        (
            "var left=[1,2,3,4], right=left;",
            "region(left,right,4,2,2,0)",
            false,
        ),
        (
            "var left=[1,2,,4], right=[10,20,30,40];",
            "region(left,right,4,2,2,0)",
            false,
        ),
        (
            "var left=[1,2,3,4], right=[10,20,30,40]; Object.freeze(right);",
            "region(left,right,4,2,2,0)",
            false,
        ),
        (
            "var left=[1,2,3,4], right=[10,20,30,40]; Object.seal(left); Object.seal(right);",
            "region(left,right,4,2,2,0)",
            true,
        ),
        (
            "var left=new Proxy([1,2,3,4],{}), right=[10,20,30,40];",
            "region(left,right,4,2,2,0)",
            false,
        ),
    ] {
        dense::reset_test_iterations();
        assert!(eval(&format!("{FORWARDING_REGION}{setup}{call};")).is_ok());
        assert_eq!(
            dense::test_nested_dense_entries() != 0,
            expect_entry,
            "{setup}"
        );
    }
}

#[test]
fn nested_dense_region_releases_to_index_accessors_without_duplicate_observation() {
    dense::reset_test_iterations();
    let source = format!(
        "{FORWARDING_REGION} var hits=0, left=[1,2,3,4], right=[10,20,30,40]; Object.defineProperty(left,'2',{{get:function(){{hits++;return 3;}},set:function(value){{}},configurable:true}}); region(left,right,4,2,2,0) + '|' + hits;"
    );
    let result = eval(&source);
    assert!(
        matches!(&result, Ok(Value::String(value)) if value.ends_with("|3")),
        "{result:?}"
    );
    assert_eq!(dense::test_nested_dense_entries(), 0);
    assert_eq!(dense::test_nested_dense_bailouts(), 0);
}

#[test]
fn nested_dense_region_rejects_noncanonical_outer_and_inner_counters() {
    for arguments in [
        "8,2,2,-1",
        "8,2,2,0.5",
        "8,2,1.5,0",
        "8,2,NaN,0",
        "8,NaN,2,0",
    ] {
        dense::reset_test_iterations();
        assert!(eval(&format!(
            "{FORWARDING_REGION} region([1,2,3,4,5,6,7,8], [10,20,30,40,50,60,70,80], {arguments});"
        ))
        .is_ok());
        assert_eq!(dense::test_nested_dense_entries(), 0, "{arguments}");
        assert_eq!(dense::test_nested_dense_bailouts(), 0, "{arguments}");
    }
}

#[test]
fn nested_dense_compiler_rejects_calls_getters_eval_and_abrupt_control() {
    for source in [
        "function run(a,b,n,k){ function noop(){} var i; for(var o=0;o<k;o++){ noop(); i=o; while(i<n){a[i]=a[i]+1;b[i]=b[i]+1;i+=k;} } }",
        "function run(a,b,n,k,box){ var i; for(var o=0;o<k;o++){ i=box.value; while(i<n){a[i]=a[i]+1;b[i]=b[i]+1;i+=k;} } }",
        "function run(a,b,n,k){ var i; for(var o=0;o<k;o++){ eval(''); i=o; while(i<n){a[i]=a[i]+1;b[i]=b[i]+1;i+=k;} } }",
        "function run(a,b,n,k){ var i; for(var o=0;o<k;o++){ i=o; while(i<n){ if(i===2) break; a[i]=a[i]+1;b[i]=b[i]+1;i+=k;} } }",
        "function run(a,b,n,k){ function noop(){} var i; for(var o=0;o<k;o++){ i=o; while(i<n){a[i]=a[i]+1;b[i]=b[i]+1;i+=k;} noop(); } }",
    ] {
        let bytecode = nested_function(source, "run");
        let plans = NumericMutationLoopPlan::compile_all(&bytecode);
        assert!(
            !plans.iter().any(|plan| matches!(
                &plan.kind,
                NumericMutationLoopKind::Special(special)
                    if matches!(special.as_ref(), SpecialPlan::NestedDense { .. })
            )),
            "{source}\n{:#?}",
            bytecode.code
        );
    }
}

#[test]
fn nested_dense_region_declines_captured_scalar_state() {
    dense::reset_test_iterations();
    let source = "function run(left,right){ var phase=1,i; function capture(){return phase;} for(var outer=0;outer<2;outer++){ i=outer; while(i<8){ left[i]=left[i]+phase; right[i]=right[i]+left[i]; i+=2; } phase+=1; } return capture()+':' + left.join(':') + '|' + right.join(':'); } run([1,2,3,4,5,6,7,8],[10,20,30,40,50,60,70,80]);";
    assert!(eval(source).is_ok());
    assert_eq!(dense::test_nested_dense_entries(), 0);
}

#[test]
fn nested_dense_typed_arrays_switch_to_the_dense_fallback_and_keep_inner_exit() {
    let bytecode = nested_function(FORWARDING_REGION, "region");
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
    let NumericMutationLoopKind::Special(plan) = &plans[0].kind else {
        panic!("expected nested dense composite: {plans:#?}");
    };
    let SpecialPlan::NestedDense { plan, fallback } = plan.as_ref() else {
        panic!("expected nested dense composite: {plans:#?}");
    };
    assert!(fallback.is_suppressing_legacy_dynamic());
    assert_ne!(plan.exit(), fallback.exit());

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{FORWARDING_REGION} region(new Uint16Array([1,2,3,4,5,6,7,8]), new Uint16Array([10,20,30,40,50,60,70,80]), 8, 2, 2, 0);"
        )),
        Ok(Value::String(
            "2:3:4:5:6:7:8:9|12:23:34:45:56:67:78:89".to_owned().into()
        ))
    );
    assert_eq!(dense::test_nested_dense_entries(), 0);
    assert_eq!(dense::test_typed_array_dense_path_hits(), 2);
    assert_eq!(dense::test_typed_array_dense_suppressions(), 0);
}

#[test]
fn nested_dense_guard_miss_switches_to_the_ordinary_dense_fallback() {
    dense::reset_test_iterations();
    let source = format!(
        "{FORWARDING_REGION} var checks=0, limit={{valueOf:function(){{checks++;return 2;}}}}; region([1,2,3,4,5,6,7,8], [10,20,30,40,50,60,70,80], 8, limit, 2, 0) + '|' + checks;"
    );
    assert_eq!(
        eval(&source),
        Ok(Value::String(
            "2:3:4:5:6:7:8:9|12:23:34:45:56:67:78:89|3"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(dense::test_nested_dense_entries(), 0);
    assert_eq!(dense::test_writable_path_hits(), 2);
}

#[test]
fn nested_dense_fallback_maps_decline_and_suppress_without_losing_semantics() {
    dense::reset_test_iterations();
    let declined = format!(
        "{FORWARDING_REGION} var checks=0, size={{valueOf:function(){{checks++;return 4;}}}}; region([1,2,3,4], [10,20,30,40], size, 2, 2, 0) + '|' + checks;"
    );
    assert_eq!(
        eval(&declined),
        Ok(Value::String("2:3:4:5|12:23:34:45|6".to_owned().into()))
    );
    assert_eq!(dense::test_nested_dense_entries(), 0);
    assert_eq!(dense::test_compact_dynamic_attempts(), 4);
    assert_eq!(dense::test_compact_dynamic_declines(), 4);
    assert_eq!(dense::test_compact_dynamic_suppressions(), 0);

    dense::reset_test_iterations();
    let suppressed = format!(
        "{FORWARDING_REGION} var buffer=new ArrayBuffer(16), left=new Uint16Array(buffer,0,4), right=new Uint16Array(buffer,8,4); left.set([1,2,3,4]); right.set([10,20,30,40]); region(left,right,4,2,2,0);"
    );
    assert_eq!(
        eval(&suppressed),
        Ok(Value::String("2:3:4:5|12:23:34:45".to_owned().into()))
    );
    assert_eq!(dense::test_nested_dense_entries(), 0);
    assert_eq!(dense::test_typed_array_dense_path_hits(), 0);
    assert_eq!(dense::test_typed_array_dense_suppressions(), 1);
    assert_eq!(dense::test_compact_dynamic_suppressions(), 1);
}

#[test]
fn nested_dense_rejects_immutable_outer_updates_after_committed_inner_stores() {
    let source = r#"
function run(left, right) {
  var caught = false;
  try {
    const outer = 0, limit = 2;
    for (; outer < limit; outer++) {
      var i = outer;
      while (i < 4) {
        left[i] = left[i] + 1;
        right[i] = right[i] + 1;
        i += 2;
      }
    }
  } catch (error) {
    caught = error instanceof TypeError;
  }
  return caught + '|' + left.join(':') + '|' + right.join(':');
}
"#;
    let bytecode = nested_function(source, "run");
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    assert!(!plans.iter().any(|plan| matches!(
        &plan.kind,
        NumericMutationLoopKind::Special(special)
            if matches!(special.as_ref(), SpecialPlan::NestedDense { .. })
    )));

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!("{source} run([1,2,3,4], [10,20,30,40]);")),
        Ok(Value::String("true|2:2:4:4|11:20:31:40".to_owned().into()))
    );
    assert_eq!(dense::test_nested_dense_entries(), 0);
}

#[test]
fn nested_dense_discovery_work_is_bounded_for_sequential_and_deep_loops() {
    fn assert_bounded(source: &str, minimum_backedges: usize) {
        let bytecode = nested_function(source, "run");
        let backedges = bytecode
            .code
            .iter()
            .enumerate()
            .filter(|(ip, op)| matches!(op, Op::Jump(header) if *header < *ip))
            .count();
        assert!(
            backedges >= minimum_backedges,
            "{backedges}\n{:#?}",
            bytecode.code
        );
        dense::reset_test_iterations();
        let _ = NumericMutationLoopPlan::compile_all(&bytecode);
        let work = dense::test_nested_dense_discovery_work();
        let logarithmic_factor = usize::BITS as usize - backedges.leading_zeros() as usize;
        let bound = bytecode.code.len() + backedges * (16 + 4 * logarithmic_factor);
        assert!(
            work <= bound,
            "discovery work {work} exceeded {bound} for {} ops and {backedges} backedges",
            bytecode.code.len()
        );
    }

    let mut sequential = String::from("function run(a,b,n){var i; var sum=0;");
    for _ in 0..96 {
        sequential.push_str("for(i=0;i<n;i++){a[i]=a[i]+1;b[i]=b[i]+a[i];}sum+=i;");
    }
    sequential.push_str("return sum;}");
    assert_bounded(&sequential, 96);

    let mut deep = String::from("function run(a,b,n){var i;");
    for level in 0..24 {
        deep.push_str(&format!(
            "for(var level{level}=0;level{level}<n;level{level}++){{"
        ));
    }
    deep.push_str("for(i=0;i<n;i++){a[i]=a[i]+1;b[i]=b[i]+a[i];}");
    for _ in 0..24 {
        deep.push('}');
    }
    deep.push('}');
    assert_bounded(&deep, 25);
}

#[test]
fn nested_dense_discovery_handles_same_headers_crossings_and_nearest_outers() {
    let same_header = dense::test_discover_enclosing_intervals(
        &[(0, 40), (0, 12), (20, 26)],
        &[true, false, false],
    );
    assert_eq!(same_header, vec![None, None, Some((0, 40))]);

    let nearest = dense::test_discover_enclosing_intervals(
        &[(0, 80), (10, 60), (10, 20), (30, 40)],
        &[true, true, false, false],
    );
    assert_eq!(
        nearest,
        vec![None, Some((0, 80)), Some((0, 80)), Some((10, 60))]
    );

    let crossing = dense::test_discover_enclosing_intervals(
        &[(0, 70), (10, 90), (20, 40)],
        &[true, false, false],
    );
    assert_eq!(crossing, vec![None, None, Some((0, 70))]);
}

#[test]
fn nested_dense_discovery_keeps_real_counted_outer_across_same_header_continue() {
    let mut bytecode = nested_function(FAST_REGION, "fast");
    let (outer_backedge, outer_header) = bytecode
        .code
        .iter()
        .enumerate()
        .filter_map(|(backedge, op)| match op {
            Op::Jump(header) if *header < backedge => Some((backedge, *header)),
            _ => None,
        })
        .max()
        .expect("outer backedge");
    let continue_backedge = outer_backedge - 7;
    assert!(continue_backedge > outer_header);
    bytecode.code[continue_backedge] = Op::Jump(outer_header);

    let discovered = dense::test_discover_enclosing_bytecode(&bytecode);
    let mut by_header = std::collections::BTreeMap::<usize, Vec<usize>>::new();
    for ((header, backedge), _) in &discovered {
        by_header.entry(*header).or_default().push(*backedge);
    }
    let (&outer_header, outer_edges) = by_header
        .iter()
        .find(|(_, backedges)| backedges.len() >= 2)
        .expect("continue and canonical backedge should share the outer header");
    assert!(
        discovered.iter().any(|((inner_header, _), outer)| {
            *inner_header > outer_header
                && outer.is_some_and(|(header, backedge)| {
                    header == outer_header && outer_edges.contains(&backedge)
                })
        }),
        "{discovered:#?}\n{:#?}",
        bytecode.code
    );
}

#[test]
fn nested_dense_probe_never_retranslates_an_enclosed_dynamic_edge() {
    let single_store = nested_function(
        "function run(input,output,size,outerLimit,stride){var i;for(var outer=0;outer<outerLimit;outer++){i=outer;while(i<size){output[i]=input[i]+1;i+=stride;}}}",
        "run",
    );
    let (header, backedge, outer) = enclosed_edge(&single_store);
    dense::reset_test_iterations();
    let plan = NumericMutationLoopPlan::compile(&single_store, header, backedge, Some(outer))
        .expect("single-store inner loop should keep its dense plan");
    assert!(matches!(plan.kind, NumericMutationLoopKind::Dense(_)));
    assert_eq!(dense::test_dynamic_dense_compilations(), 1);

    let dynamic_miss = nested_function(
        "function run(left,right,size,outerLimit,stride){function noop(){}var i;for(var outer=0;outer<outerLimit;outer++){i=outer;while(i<size){noop();left[i]=left[i]+1;right[i]=right[i]+1;i+=stride;}}}",
        "run",
    );
    let (header, backedge, outer) = enclosed_edge(&dynamic_miss);
    dense::reset_test_iterations();
    assert!(
        NumericMutationLoopPlan::compile(&dynamic_miss, header, backedge, Some(outer)).is_none()
    );
    assert_eq!(dense::test_dynamic_dense_compilations(), 1);

    let successful = nested_function(FAST_REGION, "fast");
    let (header, backedge, outer) = enclosed_edge(&successful);
    dense::reset_test_iterations();
    let plan = NumericMutationLoopPlan::compile(&successful, header, backedge, Some(outer))
        .expect("multi-store inner loop should compile as nested dense");
    assert!(matches!(
        plan.kind,
        NumericMutationLoopKind::Special(special)
            if matches!(special.as_ref(), SpecialPlan::NestedDense { .. })
    ));
    assert_eq!(dense::test_dynamic_dense_compilations(), 1);
}

#[test]
fn fixed_dense_precedence_skips_the_disjoint_dynamic_opcode_family() {
    let bytecode = nested_function(
        "function run(n){var a=[0,0,0,0],sum=0;for(var i=0;i<n;i++){a[0]=a[3]+1;a[1]=a[0]+1;a[2]=a[1]-1;a[3]=a[2];sum+=a[3];}return sum;}",
        "run",
    );
    dense::reset_test_iterations();
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
    assert!(matches!(plans[0].kind, NumericMutationLoopKind::Dense(_)));
    // Fixed plans use GetPropIndex/SetPropIndex bytecode, which the dynamic
    // translator intentionally does not accept, so no overlap is constructible.
    assert_eq!(dense::test_dynamic_dense_compilations(), 0);
}

#[test]
fn enclosing_fixed_dense_short_circuits_before_the_dynamic_probe() {
    let bytecode = nested_function(
        "function run(n,outerLimit){var a=[0,0,0,0],sum=0;for(var outer=0;outer<outerLimit;outer++){for(var i=0;i<n;i++){a[0]=a[3]+1;a[1]=a[0]+1;a[2]=a[1]-1;a[3]=a[2];sum+=a[3];}}return sum;}",
        "run",
    );
    let (header, backedge, outer) = enclosed_edge(&bytecode);
    dense::reset_test_iterations();
    let plan = NumericMutationLoopPlan::compile(&bytecode, header, backedge, Some(outer))
        .expect("enclosed fixed inner loop should compile as dense");
    assert!(matches!(plan.kind, NumericMutationLoopKind::Dense(_)));
    assert_eq!(dense::test_dynamic_dense_compilations(), 0);
}

#[test]
fn nested_dense_region_counts_each_generic_two_level_stage() {
    dense::reset_test_iterations();
    let source = r#"
function transform(real, imag, size) {
  var half = 1, stepReal, stepImag, phaseReal, phaseImag, off, tr, ti, tmp, i;
  while (half < size) {
    stepReal = 0.5;
    stepImag = 0.25;
    phaseReal = 1;
    phaseImag = 0;
    for (var outer = 0; outer < half; outer++) {
      i = outer;
      while (i < size) {
        off = i + half;
        tr = phaseReal * real[off] - phaseImag * imag[off];
        ti = phaseReal * imag[off] + phaseImag * real[off];
        real[off] = real[i] - tr;
        imag[off] = imag[i] - ti;
        real[i] += tr;
        imag[i] += ti;
        i += half << 1;
      }
      tmp = phaseReal;
      phaseReal = tmp * stepReal - phaseImag * stepImag;
      phaseImag = tmp * stepImag + phaseImag * stepReal;
    }
    half = half << 1;
  }
  return real[0] + imag[0];
}
transform([1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16],
          [16,15,14,13,12,11,10,9,8,7,6,5,4,3,2,1], 16);
"#;
    assert!(eval(source).is_ok());
    assert_eq!(dense::test_nested_dense_entries(), 4);
    assert_eq!(dense::test_nested_dense_seeded_iterations(), 4);
    assert_eq!(dense::test_nested_dense_outer_completions(), 15);
    assert_eq!(dense::test_nested_dense_inner_commits(), 28);
    assert_eq!(dense::test_nested_dense_bailouts(), 0);
}
