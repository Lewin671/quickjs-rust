use super::*;
use crate::bytecode::compiler;
use crate::value::ArrayRef;
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
fn writable_dense_region_hoists_own_data_arrays_numbers_and_math_round() {
    let generate = "function generate() { var offset = 0; for (var index = 0; index < this.size; index++) { offset = Math.round((this.base + index) * this.step); this.signal[index] = this.table[offset % this.tableLength] * this.amplitude; } return this.signal.join(':'); }";
    let add = "function add(other) { for (var index = 0; index < this.size; index++) this.signal[index] += other.signal[index]; return this.signal.join(':'); }";
    let project = "function project(source) { for (var index = 0; index < source.size; index++) this.output[index] = source.values[index] * source.scale; return this.output.join(':'); }";
    for source in [generate, add, project] {
        let bytecode = nested_function(source);
        let plans = NumericMutationLoopPlan::compile_all(&bytecode);
        assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
        assert!(matches!(plans[0].kind, NumericMutationLoopKind::Dense(_)));
    }

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{generate} var owner = {{ size: 10, base: 0, step: 0.5, tableLength: 4, amplitude: 2, signal: [0,0,0,0,0,0,0,0,0,0], table: [1,2,3,4] }}; generate.call(owner);"
        )),
        Ok(Value::String("2:4:4:6:6:8:8:2:2:4".to_owned().into()))
    );
    assert_eq!(dense::test_iterations(), 9);
    assert_eq!(dense::test_writable_path_hits(), 1);
    assert_eq!(dense::test_math_round_operations(), 9);
    assert_eq!(dense::test_writable_lease_suppressions(), 0);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{add} var owner = {{ size: 10, signal: [1,2,3,4,5,6,7,8,9,10] }}; add.call(owner, {{ signal: [10,20,30,40,50,60,70,80,90,100] }});"
        )),
        Ok(Value::String(
            "11:22:33:44:55:66:77:88:99:110".to_owned().into()
        ))
    );
    assert_eq!(dense::test_iterations(), 9);
    assert_eq!(dense::test_writable_path_hits(), 1);
    assert_eq!(dense::test_math_round_operations(), 0);
    assert_eq!(dense::test_writable_lease_suppressions(), 0);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{project} project.call({{ output: [0,0,0,0,0,0,0,0,0,0] }}, {{ size: 10, scale: 3, values: [1,2,3,4,5,6,7,8,9,10] }});"
        )),
        Ok(Value::String(
            "3:6:9:12:15:18:21:24:27:30".to_owned().into()
        ))
    );
    assert_eq!(dense::test_iterations(), 9);
    assert_eq!(dense::test_writable_path_hits(), 1);
    assert_eq!(dense::test_writable_lease_suppressions(), 0);
}

#[test]
fn writable_dense_region_suppresses_fresh_hole_retries_per_invocation() {
    let source = "function fill() { for (var index = 0; index < this.size; index++) this.output[index] = this.input[index] + this.bias; return this.output.join(':'); }";
    let bytecode = nested_function(source);
    assert_eq!(
        NumericMutationLoopPlan::compile_all(&bytecode).len(),
        1,
        "{:#?}",
        bytecode.code
    );

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} var owner = {{ size: 10, bias: 1, input: [0,1,2,3,4,5,6,7,8,9], output: new Array(10) }}; fill.call(owner); var first = owner.output.join(':'); fill.call(owner); first + '|' + owner.output.join(':');"
        )),
        Ok(Value::String(
            "1:2:3:4:5:6:7:8:9:10|1:2:3:4:5:6:7:8:9:10"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(dense::test_writable_lease_suppressions(), 1);
    assert_eq!(dense::test_iterations(), 9);
    assert_eq!(dense::test_writable_path_hits(), 1);
}

#[test]
fn writable_dense_math_round_preserves_negative_ties_and_zero() {
    let source = "function roundAll(input) { for (var index = 0; index < this.size; index++) this.output[index] = Math.round(input[index]); return this.output.join(':') + '|' + (1 / this.output[2]) + ':' + (1 / this.output[3]) + ':' + (1 / this.output[4]); }";
    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} roundAll.call({{ size: 10, output: [9,9,9,9,9,9,9,9,9,9] }}, [-2.5,-1.5,-0.5,-0.4,-0,0,0.4,0.5,1.5,2.5]);"
        )),
        Ok(Value::String(
            "-2:-1:0:0:0:0:0:1:2:3|-Infinity:-Infinity:-Infinity"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(dense::test_iterations(), 9);
    assert_eq!(dense::test_math_round_operations(), 9);
}

#[test]
fn writable_dense_math_round_handles_non_finite_large_and_adjacent_ties() {
    let source = "function roundEdges(input) { for (var index = 0; index < this.size; index++) this.output[index] = Math.round(input[index]); var output = this.output; return output[1] !== output[1] && output[2] === Infinity && output[3] === -Infinity && output[4] === 9007199254740991 && output[5] === -9007199254740991 && 1/output[6] === Infinity && output[7] === 1 && output[8] === 1 && output[9] === -1 && 1/output[10] === -Infinity && 1/output[11] === -Infinity && output[12] === 1 && output[13] === 2 && output[14] === 2 && output[15] === -2 && output[16] === -1 && output[17] === -1; }";
    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} var e52 = 1/4503599627370496, e53 = 1/9007199254740992, e54 = 1/18014398509481984; roundEdges.call({{ size: 18, output: [0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0,0] }}, [7.2, NaN, Infinity, -Infinity, 9007199254740991, -9007199254740991, 0.5-e54, 0.5, 0.5+e53, -0.5-e53, -0.5, -0.5+e54, 1.5-e52, 1.5, 1.5+e52, -1.5-e52, -1.5, -1.5+e52]);"
        )),
        Ok(Value::Boolean(true))
    );
    assert_eq!(dense::test_iterations(), 17);
    assert_eq!(dense::test_math_round_operations(), 17);
    assert_eq!(dense::test_writable_path_hits(), 1);
    assert_eq!(dense::test_writable_lease_suppressions(), 0);
}

#[test]
fn writable_dense_math_round_revalidates_replacement_getter_proxy_and_inheritance() {
    let source = "function roundAll(input) { for (var index = 0; index < this.size; index++) this.output[index] = Math.round(input[index]); return this.output.join(':'); }";
    let input = "[0.1,0.6,1.1,1.6,2.1,2.6,3.1,3.6,4.1,4.6]";
    let owner = "{ size: 10, output: [0,0,0,0,0,0,0,0,0,0] }";

    for (setup, expected) in [
        (
            "Math.round = function () { return 7; }; var hits = 0;",
            "7:7:7:7:7:7:7:7:7:7|0",
        ),
        (
            "var original = Math.round, hits = 0; Object.defineProperty(Math, 'round', { configurable: true, get: function () { hits++; return original; } });",
            "0:1:1:2:2:3:3:4:4:5|10",
        ),
        (
            "var original = Math, hits = 0; Math = new Proxy(original, { get: function (target, key) { if (key === 'round') hits++; return target[key]; } });",
            "0:1:1:2:2:3:3:4:4:5|10",
        ),
        (
            "var original = Math.round, hits = 0; delete Math.round; Object.setPrototypeOf(Math, { round: original });",
            "0:1:1:2:2:3:3:4:4:5|0",
        ),
        (
            "var original = Math, hits = 0; Object.defineProperty(globalThis, 'Math', { configurable: true, get: function () { hits++; return original; } });",
            "0:1:1:2:2:3:3:4:4:5|10",
        ),
    ] {
        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "{source} {setup} roundAll.call({owner}, {input}) + '|' + hits;"
            )),
            Ok(Value::String(expected.to_owned().into())),
            "setup: {setup}"
        );
        assert_eq!(dense::test_iterations(), 0, "setup: {setup}");
        assert_eq!(dense::test_math_round_operations(), 0, "setup: {setup}");
    }
}

#[test]
fn writable_dense_own_data_sources_reject_accessors_inheritance_and_proxies() {
    let source = "function copy(source) { for (var index = 0; index < this.size; index++) this.output[index] = source.values[index] + this.bias; return this.output.join(':'); }";
    let values = "[0,1,2,3,4,5,6,7,8,9]";

    for (setup, source_expression, expected_hits) in [
        (
            format!(
                "var hits = 0, values = {values}, source = {{}}; Object.defineProperty(source, 'values', {{ get: function () {{ hits++; return values; }} }});"
            ),
            "source",
            10.0,
        ),
        (
            format!("var hits = 0, source = Object.create({{ values: {values} }});"),
            "source",
            0.0,
        ),
        (
            format!(
                "var hits = 0, source = new Proxy({{ values: {values} }}, {{ get: function (target, key) {{ if (key === 'values') hits++; return target[key]; }} }});"
            ),
            "source",
            10.0,
        ),
        (
            format!("var hits = 0, source = new Uint8Array(1); source.values = {values};"),
            "source",
            0.0,
        ),
        (
            format!("var hits = 0, source = Symbol('source'); Symbol.prototype.values = {values};"),
            "source",
            0.0,
        ),
    ] {
        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "{source} {setup} copy.call({{ size: 10, bias: 1, output: [0,0,0,0,0,0,0,0,0,0] }}, {source_expression}) + '|' + hits;"
            )),
            Ok(Value::String(
                format!("1:2:3:4:5:6:7:8:9:10|{expected_hits}").into()
            )),
            "setup: {setup}"
        );
        assert_eq!(dense::test_iterations(), 0, "setup: {setup}");
    }

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} var hits = 0, output = [0,0,0,0,0,0,0,0,0,0], owner = {{ size: 10, bias: 1 }}; Object.defineProperty(owner, 'output', {{ get: function () {{ hits++; return output; }} }}); copy.call(owner, {{ values: {values} }}) + '|' + hits;"
        )),
        Ok(Value::String("1:2:3:4:5:6:7:8:9:10|11".to_owned().into()))
    );
    assert_eq!(dense::test_iterations(), 0);
}

#[test]
fn writable_dense_named_receivers_fail_closed_for_aliases_and_retry_next_call() {
    let source = "function add(source) { for (var index = 0; index < this.size; index++) this.output[index] = this.output[index] + source.values[index]; return this.output.join(':'); }";
    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} var shared = [1,2,3,4,5,6,7,8,9,10], owner = {{ size: 10, output: shared }}; var first = add.call(owner, {{ values: shared }}); var second = add.call(owner, {{ values: [1,1,1,1,1,1,1,1,1,1] }}); first + '|' + second;"
        )),
        Ok(Value::String(
            "2:4:6:8:10:12:14:16:18:20|3:5:7:9:11:13:15:17:19:21"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(dense::test_writable_lease_suppressions(), 1);
    assert_eq!(dense::test_writable_path_hits(), 1);
    assert_eq!(dense::test_iterations(), 9);
}

#[test]
fn writable_dense_own_data_input_output_alias_suppresses_without_progress() {
    let source = "function copy() { for (var index = 0; index < this.size; index++) this.output[index] = this.input[index] + 1; return this.output.join(':'); }";
    let bytecode = nested_function(source);
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    let [
        NumericMutationLoopPlan {
            kind: NumericMutationLoopKind::Dense(plan),
            ..
        },
    ] = plans.as_slice()
    else {
        panic!("expected one dense plan: {plans:#?}");
    };
    assert!(!plan.is_legacy_dynamic(), "{plan:#?}");
    assert!(!plan.is_suppressing_legacy_dynamic(), "{plan:#?}");

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} var shared = [0,1,2,3,4,5,6,7,8,9]; copy.call({{ size: 10, input: shared, output: shared }});"
        )),
        Ok(Value::String("1:2:3:4:5:6:7:8:9:10".to_owned().into()))
    );
    assert_eq!(dense::test_writable_lease_suppressions(), 1);
    assert_eq!(dense::test_iterations(), 0);
    assert_eq!(dense::test_writable_path_hits(), 0);
}

#[test]
fn writable_dense_suppression_removes_same_header_for_current_invocation() {
    let source = "function weave(size) { for (var index = 0; index < size; index++) this.output[index % 2] = this.input[index]; return this.output.join(':'); }";
    let bytecode = nested_function(source);
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    let [
        NumericMutationLoopPlan {
            kind: NumericMutationLoopKind::Dense(plan),
            ..
        },
    ] = plans.as_slice()
    else {
        panic!("expected one dense plan: {plans:#?}");
    };
    assert!(!plan.is_legacy_dynamic(), "{plan:#?}");
    assert!(!plan.is_suppressing_legacy_dynamic(), "{plan:#?}");

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} weave.call({{ input: [10,11,12,13], output: new Array(2) }}, 4);"
        )),
        Ok(Value::String("12:13".to_owned().into()))
    );
    assert_eq!(dense::test_writable_lease_suppressions(), 1);
    assert_eq!(dense::test_iterations(), 0);
    assert_eq!(dense::test_writable_path_hits(), 0);
}

#[test]
fn writable_dense_integrity_guards_allow_existing_writes_but_reject_frozen_arrays() {
    let source = "function bump() { for (var index = 0; index < this.size; index++) this.output[index] = this.output[index] + 1; return this.output.join(':'); }";
    for (setup, expected, iterations, hits, suppressions) in [
        ("Object.seal(output);", "1:2:3:4:5:6:7:8:9:10", 9, 1, 0),
        (
            "Object.defineProperty(output, 'length', { writable: false });",
            "1:2:3:4:5:6:7:8:9:10",
            9,
            1,
            0,
        ),
        ("Object.freeze(output);", "0:1:2:3:4:5:6:7:8:9", 0, 0, 1),
    ] {
        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "{source} var output = [0,1,2,3,4,5,6,7,8,9]; {setup} bump.call({{ size: 10, output: output }});"
            )),
            Ok(Value::String(expected.to_owned().into())),
            "setup: {setup}"
        );
        assert_eq!(dense::test_iterations(), iterations, "setup: {setup}");
        assert_eq!(dense::test_writable_path_hits(), hits, "setup: {setup}");
        assert_eq!(
            dense::test_writable_lease_suppressions(),
            suppressions,
            "setup: {setup}"
        );
    }
}

#[test]
fn writable_dense_sparse_source_replays_prototype_getter_once() {
    let source = "function copy() { for (var index = 0; index < this.size; index++) this.output[index] = this.input[index] + 1; return this.output.join(':'); }";
    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} var gets = 0, input = [1,,3], output = [0,0,0]; Object.defineProperty(Array.prototype, '1', {{ configurable: true, get: function () {{ gets++; return 40; }} }}); var result = copy.call({{ size: 3, input: input, output: output }}); delete Array.prototype[1]; result + '|' + gets;"
        )),
        Ok(Value::String("2:41:4|1".to_owned().into()))
    );
    assert_eq!(dense::test_writable_lease_suppressions(), 1);
    assert_eq!(dense::test_iterations(), 0);
    assert_eq!(dense::test_writable_path_hits(), 0);
}

#[test]
fn distinct_dense_writable_lease_releases_prefix_on_real_borrow_conflict() {
    let first = ArrayRef::new(vec![Value::Number(1.0)]);
    let second = ArrayRef::new(vec![Value::Number(2.0)]);
    let arrays = [first.clone(), second.clone()];
    let mut mutation_ran = false;

    second
        .with_dense_readable_elements(|_| {
            assert!(
                ArrayRef::with_distinct_dense_writable_elements(&arrays, |_| {
                    mutation_ran = true;
                })
                .is_none()
            );
            assert!(first.with_dense_writable_elements(|_| ()).is_some());
        })
        .expect("fully dense array should permit the outer read lease");
    assert!(!mutation_ran);
    assert!(
        ArrayRef::with_distinct_dense_writable_elements(&arrays, |_| ()).is_some(),
        "dropping the conflicting read must make the multi-write lease available"
    );
}

#[test]
fn writable_dense_sources_re_resolve_between_calls_and_after_mid_loop_deopt() {
    let source = "function copy(source) { for (var index = 0; index < this.size; index++) this.output[index] = source.values[index] + this.bias; return this.output.join(':'); }";
    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} var owner = {{ size: 10, bias: 1, output: [0,0,0,0,0,0,0,0,0,0] }}, sourceOwner = {{ values: [0,1,2,3,4,5,6,7,8,9] }}; var first = copy.call(owner, sourceOwner); owner.output = [0,0,0,0,0,0,0,0,0,0]; owner.bias = 10; sourceOwner.values = [10,11,12,13,14,15,16,17,18,19]; var second = copy.call(owner, sourceOwner); first + '|' + second;"
        )),
        Ok(Value::String(
            "1:2:3:4:5:6:7:8:9:10|20:21:22:23:24:25:26:27:28:29"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(dense::test_writable_path_hits(), 2);
    assert_eq!(dense::test_iterations(), 18);

    dense::reset_test_iterations();
    assert_eq!(
        eval(
            "var owner, coercions = 0, replacement = [10,20,30,40,50]; var marker = { valueOf: function () { coercions++; owner.input = replacement; return 7; } }; function replay() { for (var index = 0; index < this.size; index++) this.output[index] = this.input[index] + this.bias; return this.output.join(':') + '|' + coercions; } owner = { size: 5, bias: 1, input: [1,2,3,marker,5], output: [0,0,0,0,0] }; replay.call(owner);"
        ),
        Ok(Value::String("2:3:4:8:51|1".to_owned().into()))
    );
    assert_eq!(dense::test_writable_path_hits(), 2);
    assert_eq!(dense::test_iterations(), 3);
}

#[test]
fn writable_dense_sources_reject_owner_writes_captures_eval_and_discarded_reads() {
    let written_owner = nested_function(
        "function copy(source) { for (var index = 0; index < this.size; index++) { source = source; this.output[index] = source.values[index]; } }",
    );
    assert!(
        NumericMutationLoopPlan::compile_all(&written_owner).is_empty(),
        "{:#?}",
        written_owner.code
    );

    let discarded = nested_function(
        "function copy(source) { for (var index = 0; index < this.size; index++) { source.tick; this.output[index] = source.values[index]; } }",
    );
    assert!(
        NumericMutationLoopPlan::compile_all(&discarded).is_empty(),
        "{:#?}",
        discarded.code
    );

    dense::reset_test_iterations();
    assert_eq!(
        eval(
            "function make(source) { return function copy() { for (var index = 0; index < this.size; index++) this.output[index] = source.values[index]; return this.output.join(':'); }; } var copy = make({ values: [0,1,2,3,4,5,6,7,8,9] }); copy.call({ size: 10, output: [0,0,0,0,0,0,0,0,0,0] });"
        ),
        Ok(Value::String("0:1:2:3:4:5:6:7:8:9".to_owned().into()))
    );
    assert_eq!(dense::test_iterations(), 0);

    dense::reset_test_iterations();
    assert_eq!(
        eval(
            "function copy(source) { eval(''); for (var index = 0; index < this.size; index++) this.output[index] = source.values[index]; return this.output.join(':'); } copy.call({ size: 10, output: [0,0,0,0,0,0,0,0,0,0] }, { values: [0,1,2,3,4,5,6,7,8,9] });"
        ),
        Ok(Value::String("0:1:2:3:4:5:6:7:8:9".to_owned().into()))
    );
    assert_eq!(dense::test_iterations(), 0);

    dense::reset_test_iterations();
    assert_eq!(
        eval(
            "var hits = 0; function copy(source) { for (var index = 0; index < this.size; index++) { source.tick; this.output[index] = source.values[index]; } return this.output.join(':'); } var source = { values: [0,1,2,3,4,5,6,7,8,9] }; Object.defineProperty(source, 'tick', { get: function () { hits++; return 1; } }); copy.call({ size: 10, output: [0,0,0,0,0,0,0,0,0,0] }, source) + '|' + hits;"
        ),
        Ok(Value::String("0:1:2:3:4:5:6:7:8:9|10".to_owned().into()))
    );
    assert_eq!(dense::test_iterations(), 0);
}

#[test]
fn fannkuch_copy_loop_uses_legacy_dynamic_executor() {
    let source = "function copy(perm, perm1, n) { for (var i = 0; i < n; i++) perm[i] = perm1[i]; return perm.join(':'); }";
    let bytecode = nested_function(source);
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    let [
        NumericMutationLoopPlan {
            kind: NumericMutationLoopKind::Dense(plan),
            ..
        },
    ] = plans.as_slice()
    else {
        panic!("expected one dense plan: {plans:#?}");
    };
    assert!(plan.is_legacy_dynamic(), "{plan:#?}");

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} copy([0,0,0,0,0,0,0,0], [7,6,5,4,3,2,1,0], 8);"
        )),
        Ok(Value::String("7:6:5:4:3:2:1:0".to_owned().into()))
    );
    assert_eq!(dense::test_iterations(), 7);
}

#[test]
fn fannkuch_hole_dense_reentry_and_swap_lifecycle_uses_both_compact_contracts() {
    let source = "function fannkuch(n) { var check = 0, perm = Array(n), perm1 = Array(n), count = Array(n), maxPerm = Array(n), maxFlipsCount = 0, m = n - 1; for (var i = 0; i < n; i++) perm1[i] = i; var r = n; while (true) { if (check < 30) { var s = ''; for (var i = 0; i < n; i++) s += (perm1[i] + 1).toString(); check++; } while (r != 1) { count[r - 1] = r; r--; } if (!(perm1[0] == 0 || perm1[m] == m)) { for (var i = 0; i < n; i++) perm[i] = perm1[i]; var flipsCount = 0, k; while (!((k = perm[0]) == 0)) { var k2 = (k + 1) >> 1; for (var i = 0; i < k2; i++) { var temp = perm[i]; perm[i] = perm[k - i]; perm[k - i] = temp; } flipsCount++; } if (flipsCount > maxFlipsCount) { maxFlipsCount = flipsCount; for (var i = 0; i < n; i++) maxPerm[i] = perm1[i]; } } while (true) { if (r == n) return maxFlipsCount; var perm0 = perm1[0], i = 0; while (i < r) { var j = i + 1; perm1[i] = perm1[j]; i = j; } perm1[r] = perm0; count[r] = count[r] - 1; if (count[r] > 0) break; r++; } } }";
    let bytecode = nested_function(source);
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    let mut bool_compact = 0;
    let mut suppressing_compact = 0;
    for plan in &plans {
        let NumericMutationLoopKind::Dense(plan) = &plan.kind else {
            continue;
        };
        bool_compact += usize::from(plan.is_legacy_dynamic());
        suppressing_compact += usize::from(plan.is_suppressing_legacy_dynamic());
    }
    assert!(bool_compact > 0, "{plans:#?}");
    assert!(suppressing_compact > 0, "{plans:#?}");

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!("{source} fannkuch(6);")),
        Ok(Value::Number(10.0))
    );
    assert!(dense::test_compact_dynamic_attempts() > 0);
    assert!(dense::test_compact_dynamic_declines() > 0);
    assert!(dense::test_compact_dynamic_hits() > 0);
    assert!(dense::test_iterations() > 0);
    assert!(dense::test_writable_path_hits() > 0);
}

#[test]
fn dft_inner_loop_uses_legacy_dynamic_executor() {
    let source = "function forward(buffer) { var real = 0, imag = 0, k = 2; for (var n = 0; n < buffer.length; n++) { real += this.cosTable[k*n] * buffer[n]; imag += this.sinTable[k*n] * buffer[n]; } return real + ':' + imag; }";
    let bytecode = nested_function(source);
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    let [
        NumericMutationLoopPlan {
            kind: NumericMutationLoopKind::Dense(plan),
            ..
        },
    ] = plans.as_slice()
    else {
        panic!("expected one dense plan: {plans:#?}");
    };
    assert!(plan.is_legacy_dynamic(), "{plan:#?}");

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} forward.call({{ cosTable: [1,99,2,99,3,99,4], sinTable: [10,99,20,99,30,99,40] }}, [1,2,3,4]);"
        )),
        Ok(Value::String("30:300".to_owned().into()))
    );
    assert_eq!(dense::test_read_only_path_hits(), 1);
    assert_eq!(dense::test_iterations(), 3);
}

#[test]
fn multi_output_fresh_arrays_use_compact_suppressing_executor() {
    let source = "function split(buffer) { var len = buffer.length / 2, left = new Array(len), right = new Array(len), mix = new Array(len); for (var i = 0; i < len; i++) { left[i] = buffer[2*i]; right[i] = buffer[2*i+1]; mix[i] = (left[i] + right[i]) / 2; } return left.join(':') + '|' + right.join(':') + '|' + mix.join(':'); }";
    let bytecode = nested_function(source);
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    let [
        NumericMutationLoopPlan {
            kind: NumericMutationLoopKind::Dense(plan),
            ..
        },
    ] = plans.as_slice()
    else {
        panic!("expected one dense plan: {plans:#?}");
    };
    assert!(!plan.is_legacy_dynamic(), "{plan:#?}");
    assert!(plan.is_suppressing_legacy_dynamic(), "{plan:#?}");

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!("{source} split([1,2,3,4,5,6,7,8]);")),
        Ok(Value::String(
            "1:3:5:7|2:4:6:8|1.5:3.5:5.5:7.5".to_owned().into()
        ))
    );
    assert_eq!(dense::test_writable_lease_suppressions(), 1);
    assert_eq!(dense::test_iterations(), 0);
    assert_eq!(dense::test_compact_dynamic_attempts(), 1);
    assert_eq!(dense::test_compact_dynamic_declines(), 0);
    assert_eq!(dense::test_compact_dynamic_hits(), 0);
    assert_eq!(dense::test_compact_dynamic_suppressions(), 1);
}

#[test]
fn multi_output_non_array_receivers_suppress_stable_retries() {
    let source = "function split(buffer, left, right, mix) { var len = buffer.length / 2; for (var i = 0; i < len; i++) { left[i] = buffer[2*i]; right[i] = buffer[2*i+1]; mix[i] = (left[i] + right[i]) / 2; } return left[3] + ':' + right[3] + ':' + mix[3]; }";
    let bytecode = nested_function(source);
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    let [
        NumericMutationLoopPlan {
            kind: NumericMutationLoopKind::Dense(plan),
            ..
        },
    ] = plans.as_slice()
    else {
        panic!("expected one dense plan: {plans:#?}");
    };
    assert!(plan.is_suppressing_legacy_dynamic(), "{plan:#?}");

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} split([1,2,3,4,5,6,7,8], new Float64Array(4), new Float64Array(4), new Float64Array(4));"
        )),
        Ok(Value::String("7:8:7.5".to_owned().into()))
    );
    assert_eq!(dense::test_iterations(), 0);
    assert_eq!(dense::test_compact_dynamic_attempts(), 1);
    assert_eq!(dense::test_compact_dynamic_declines(), 0);
    assert_eq!(dense::test_compact_dynamic_hits(), 0);
    assert_eq!(dense::test_compact_dynamic_suppressions(), 1);
}
