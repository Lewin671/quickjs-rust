use super::*;
use crate::bytecode::compiler;
use crate::{Value, array_buffer, eval, typed_array};

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

fn assert_typed_dense_plan(source: &str) {
    let bytecode = nested_function(source);
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
    let NumericMutationLoopKind::Dense(plan) = &plans[0].kind else {
        panic!("expected dense plan: {:#?}", bytecode.code);
    };
    assert!(plan.is_legacy_dynamic() || plan.is_suppressing_legacy_dynamic());
}

fn assert_suppressing_typed_dense_plan(source: &str) {
    let bytecode = nested_function(source);
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
    let NumericMutationLoopKind::Dense(plan) = &plans[0].kind else {
        panic!("expected dense plan: {:#?}", bytecode.code);
    };
    assert!(plan.is_suppressing_legacy_dynamic(), "{:#?}", bytecode.code);
}

fn assert_suppressed(source: &str, expected: Value) {
    dense::reset_test_iterations();
    assert_eq!(eval(source), Ok(expected));
    assert_eq!(dense::test_typed_array_dense_path_hits(), 0);
    assert!(
        dense::test_typed_array_dense_suppressions() > 0,
        "expected TypedArray suppression for: {source}"
    );
}

#[test]
fn fixed_number_typed_arrays_apply_all_nine_storage_conversions() {
    let function = "function copy(input, output, bound) { for (var index = 0; index < bound; index++) { output[index] = input[index]; } return output.join(',') + ':' + (1 / output[4]); }";
    assert_typed_dense_plan(function);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} var input = new Float64Array([-1.5, 255.5, NaN, Infinity, -0]); [copy(input, new Uint8Array(5), 5), copy(input, new Int8Array(5), 5), copy(input, new Uint8ClampedArray(5), 5), copy(input, new Uint16Array(5), 5), copy(input, new Int16Array(5), 5), copy(input, new Uint32Array(5), 5), copy(input, new Int32Array(5), 5), copy(input, new Float32Array(5), 5), copy(input, new Float64Array(5), 5)].join('|');"
        )),
        Ok(Value::String(
            "255,255,0,0,0:Infinity|-1,-1,0,0,0:Infinity|0,255,0,255,0:Infinity|65535,255,0,0,0:Infinity|-1,255,0,0,0:Infinity|4294967295,255,0,0,0:Infinity|-1,255,0,0,0:Infinity|-1.5,255.5,NaN,Infinity,0:-Infinity|-1.5,255.5,NaN,Infinity,0:-Infinity"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(dense::test_typed_array_dense_path_hits(), 9);
    assert_eq!(dense::test_typed_array_dense_suppressions(), 0);
}

#[test]
fn fixed_number_typed_arrays_apply_all_nine_load_codecs() {
    let function = "function copy(input, output, bound) { for (var index = 0; index < bound; index++) { output[index] = input[index]; } return output[4]; }";
    assert_typed_dense_plan(function);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} [copy(new Uint8Array([0,0,0,0,255]), new Float64Array(5), 5), copy(new Int8Array([0,0,0,0,-1]), new Float64Array(5), 5), copy(new Uint8ClampedArray([0,0,0,0,254.5]), new Float64Array(5), 5), copy(new Uint16Array([0,0,0,0,65535]), new Float64Array(5), 5), copy(new Int16Array([0,0,0,0,-32768]), new Float64Array(5), 5), copy(new Uint32Array([0,0,0,0,4294967295]), new Float64Array(5), 5), copy(new Int32Array([0,0,0,0,-2147483648]), new Float64Array(5), 5), copy(new Float32Array([0,0,0,0,1.5]), new Float64Array(5), 5), 1 / copy(new Float64Array([0,0,0,0,-0]), new Float64Array(5), 5)].join('|');"
        )),
        Ok(Value::String(
            "255|-1|254|65535|-32768|4294967295|-2147483648|1.5|-Infinity"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(dense::test_typed_array_dense_path_hits(), 9);
    assert_eq!(dense::test_typed_array_dense_suppressions(), 0);
}

#[test]
fn typed_dense_descending_three_receiver_region_commits_zero_and_preserves_offsets() {
    let function = "function reverse(source, line, output, index) { for (; index >= 0; index--) { output[index] = source[index] + line[index]; } return index; }";
    assert_typed_dense_plan(function);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} var buffer = new ArrayBuffer(28), raw = new Uint8Array(buffer); raw.fill(170); var output = new Uint32Array(buffer, 4, 5); var finalIndex = reverse(new Uint32Array([1,2,3,4,5]), new Float32Array([1.5,2.5,3.5,4.5,5.5]), output, 4); finalIndex + '|' + output.join(':') + '|' + [raw[0],raw[3],raw[24],raw[27]].join(':');"
        )),
        Ok(Value::String(
            "-1|2:4:6:8:10|170:170:170:170".to_owned().into()
        ))
    );
    assert_eq!(dense::test_iterations(), 4);
    assert_eq!(dense::test_typed_array_dense_path_hits(), 1);
    assert_eq!(dense::test_typed_array_dense_suppressions(), 0);
}

#[test]
fn typed_dense_matches_clamped_ties_and_float32_rounding_edges() {
    let function = "function copyTwice(input, output, bound) { for (var index = 0; index < bound; index++) { output[index] = input[index]; output[index] = output[index]; } }";
    assert_typed_dense_plan(function);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} \
             var clamped = new Uint8ClampedArray(5); \
             copyTwice(new Float64Array([0.5, 1.5, 2.5, 3.5, 254.5]), clamped, 5); \
             var rounded = new Float32Array(6); \
             copyTwice(new Float64Array([1 + Math.pow(2, -24), 1 + 3 * Math.pow(2, -24), 3.5e38, Math.pow(2, -149), Math.pow(2, -150), -0]), rounded, 6); \
             clamped.join(':') + '|' + [rounded[0] === 1, rounded[1] === 1 + Math.pow(2, -22), rounded[2] === Infinity, rounded[3] === Math.pow(2, -149), rounded[4] === 0, Object.is(rounded[5], -0)].join(':');"
        )),
        Ok(Value::String(
            "0:2:2:4:254|true:true:true:true:true:true"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(dense::test_typed_array_dense_path_hits(), 2);
    assert_eq!(dense::test_typed_array_dense_suppressions(), 0);
}

#[test]
fn typed_dense_multi_store_observes_converted_pending_values_but_returns_rhs() {
    let function = "function mutate(first, second, bound) { var assigned = 0; for (var index = 0; index < bound; index++) { assigned = (first[index] = first[index] + 300); second[index] = first[index] + 1; } return assigned + '|' + first.join(':') + '|' + second.join(':'); }";
    assert_typed_dense_plan(function);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} mutate(new Uint8Array([1,2,3,4]), new Uint16Array(4), 4);"
        )),
        Ok(Value::String(
            "304|45:46:47:48|46:47:48:49".to_owned().into()
        ))
    );
    assert_eq!(
        dense::test_typed_array_dense_path_hits(),
        1,
        "suppressed={}, iterations={}",
        dense::test_typed_array_dense_suppressions(),
        dense::test_iterations()
    );
    assert_eq!(dense::test_typed_array_dense_suppressions(), 0);
    assert!(dense::test_iterations() > 0);
}

#[test]
fn repeated_local_writes_beyond_plan_limit_coalesce_to_the_last_value() {
    let updates = "value = value + 1;".repeat(70);
    let function = format!(
        "function mutate(input, output, bound) {{ var value = 0; for (var index = 0; index < bound; index++) {{ value = input[index]; {updates} output[index] = value; }} return value + '|' + output.join(':'); }}"
    );
    assert_typed_dense_plan(&function);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} mutate(new Uint16Array([1,2]), new Uint16Array(2), 2);"
        )),
        Ok(Value::String("72|71:72".to_owned().into()))
    );
    assert_eq!(dense::test_typed_array_dense_attempts(), 1);
    assert_eq!(dense::test_typed_array_dense_path_hits(), 1);
    assert_eq!(dense::test_typed_array_dense_suppressions(), 0);
}

#[test]
fn gaussian_forward_shape_compiles_and_runs_the_typed_multi_store_path() {
    let function = r#"
        function forward(src, line, width) {
            var rgba = 0;
            var prev_src_r = 0, prev_src_g = 0, prev_src_b = 0, prev_src_a = 0;
            var curr_src_r = 0, curr_src_g = 0, curr_src_b = 0, curr_src_a = 0;
            var curr_out_r = 0, curr_out_g = 0, curr_out_b = 0, curr_out_a = 0;
            var prev_out_r = 0, prev_out_g = 0, prev_out_b = 0, prev_out_a = 0;
            var prev_prev_out_r = 0, prev_prev_out_g = 0;
            var prev_prev_out_b = 0, prev_prev_out_a = 0;
            var src_index = 0, line_index = 0;
            var coeff_a0 = 1, coeff_a1 = 0, coeff_b1 = 0, coeff_b2 = 0;
            for (var j = 0; j < width; j++) {
                rgba = src[src_index];
                curr_src_r = rgba & 0xff;
                curr_src_g = (rgba >> 8) & 0xff;
                curr_src_b = (rgba >> 16) & 0xff;
                curr_src_a = (rgba >> 24) & 0xff;
                curr_out_r = curr_src_r * coeff_a0 + prev_src_r * coeff_a1 + prev_out_r * coeff_b1 + prev_prev_out_r * coeff_b2;
                curr_out_g = curr_src_g * coeff_a0 + prev_src_g * coeff_a1 + prev_out_g * coeff_b1 + prev_prev_out_g * coeff_b2;
                curr_out_b = curr_src_b * coeff_a0 + prev_src_b * coeff_a1 + prev_out_b * coeff_b1 + prev_prev_out_b * coeff_b2;
                curr_out_a = curr_src_a * coeff_a0 + prev_src_a * coeff_a1 + prev_out_a * coeff_b1 + prev_prev_out_a * coeff_b2;
                prev_prev_out_r = prev_out_r;
                prev_prev_out_g = prev_out_g;
                prev_prev_out_b = prev_out_b;
                prev_prev_out_a = prev_out_a;
                prev_out_r = curr_out_r;
                prev_out_g = curr_out_g;
                prev_out_b = curr_out_b;
                prev_out_a = curr_out_a;
                prev_src_r = curr_src_r;
                prev_src_g = curr_src_g;
                prev_src_b = curr_src_b;
                prev_src_a = curr_src_a;
                line[line_index] = prev_out_r;
                line[line_index + 1] = prev_out_g;
                line[line_index + 2] = prev_out_b;
                line[line_index + 3] = prev_out_a;
                line_index += 4;
                src_index++;
            }
            return j + '|' + src_index + '|' + line_index + '|' + line.join(':');
        }
    "#;
    assert_suppressing_typed_dense_plan(function);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} forward(new Uint32Array([0x04030201,0x08070605]), new Float32Array(8), 2);"
        )),
        Ok(Value::String("2|2|8|1:2:3:4:5:6:7:8".to_owned().into()))
    );
    // The first iteration reaches the backedge normally; the compiled region
    // consumes the one remaining iteration.
    assert_eq!(dense::test_iterations(), 1);
    assert_eq!(dense::test_typed_array_dense_attempts(), 1);
    assert_eq!(dense::test_typed_array_dense_path_hits(), 1);
    assert_eq!(dense::test_typed_array_dense_suppressions(), 0);
}

#[test]
fn typed_dense_respects_view_offsets_lengths_and_surrounding_bytes() {
    let function = "function copy(input, output, bound) { for (var index = 0; index < bound; index++) { output[index] = input[index]; output[index] = output[index]; } }";
    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} var buffer = new ArrayBuffer(12), raw = new Uint8Array(buffer); raw.fill(170); var output = new Uint16Array(buffer, 2, 4); copy(new Uint16Array([1,258,65535,7]), output, 4); [raw[0], raw[1], output.join(':'), raw[10], raw[11]].join('|');"
        )),
        Ok(Value::String(
            "170|170|1:258:65535:7|170|170".to_owned().into()
        ))
    );
    assert_eq!(dense::test_typed_array_dense_path_hits(), 1);
    assert_eq!(dense::test_typed_array_dense_suppressions(), 0);
}

#[test]
fn typed_dense_replays_only_the_failed_oob_or_fractional_iteration() {
    let oob = "function sparse(output, probe, bound) { for (var index = 0; index < bound; index++) { output[index] = output[index] + 1; probe[index * 2] = probe[index * 2] + 1; } return output.join(':') + '|' + probe.join(':'); }";
    assert_typed_dense_plan(oob);
    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{oob} sparse(new Uint8Array([1,1,1,1]), new Uint8Array([10,10,10,10]), 4);"
        )),
        Ok(Value::String("2:2:2:2|11:10:11:10".to_owned().into()))
    );
    assert_eq!(
        dense::test_typed_array_dense_path_hits(),
        1,
        "suppressed={}, iterations={}",
        dense::test_typed_array_dense_suppressions(),
        dense::test_iterations()
    );

    let fractional = "function fractional(input, output, bound) { for (var index = 0; index < bound; index = index + 0.5) { output[index - index] = output[index - index] + 1; output[index] = input[index]; } return index + '|' + output.join(':'); }";
    assert_typed_dense_plan(fractional);
    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{fractional} fractional(new Uint8Array([10,20]), new Uint8Array([0,0]), 2);"
        )),
        Ok(Value::String("2|13:20".to_owned().into()))
    );
    assert!(dense::test_typed_array_dense_attempts() > 0);
    assert_eq!(dense::test_typed_array_dense_suppressions(), 0);
}

#[test]
fn typed_dense_rejects_aliases_mixed_receivers_and_proxy_receivers() {
    let function = "function copyTwice(input, output, bound) { for (var index = 0; index < bound; index++) { output[index] = input[index]; output[index] = input[index]; } return output.join(':'); }";

    assert_suppressed(
        &format!("{function} var view = new Uint8Array([1,2,3,4]); copyTwice(view, view, 4);"),
        Value::String("1:2:3:4".to_owned().into()),
    );
    assert_suppressed(
        &format!(
            "{function} var buffer = new ArrayBuffer(8), input = new Uint8Array(buffer, 0, 4), output = new Uint8Array(buffer, 4, 4); input.set([1,2,3,4]); copyTwice(input, output, 4);"
        ),
        Value::String("1:2:3:4".to_owned().into()),
    );
    assert_suppressed(
        &format!(
            "{function} var buffer = new ArrayBuffer(6), input = new Uint8Array(buffer, 0, 4), output = new Uint8Array(buffer, 2, 4); input.set([1,2,3,4]); copyTwice(input, output, 4);"
        ),
        Value::String("1:2:1:2".to_owned().into()),
    );
    assert_suppressed(
        &format!("{function} copyTwice([1,2,3,4], new Uint8Array(4), 4);"),
        Value::String("1:2:3:4".to_owned().into()),
    );
    assert_suppressed(
        &format!(
            "{function} var input = new Proxy(new Uint8Array([1,2,3,4]), {{}}); copyTwice(input, new Uint8Array(4), 4);"
        ),
        Value::String("1:2:3:4".to_owned().into()),
    );
}

#[test]
fn typed_dense_rejects_observable_length_controls() {
    let function = "function copy(input, output) { for (var index = 0; index < input.length; index++) { output[index] = input[index]; output[index] = input[index]; } return output.join(':'); }";
    assert_suppressed(
        &format!(
            "{function} var input = new Uint8Array([1,2,3,4]); Object.defineProperty(input, 'length', {{ value: 2 }}); copy(input, new Uint8Array(4));"
        ),
        Value::String("1:2:0:0".to_owned().into()),
    );
}

#[test]
fn typed_dense_rejects_non_fixed_or_non_number_backings() {
    let function = "function copyTwice(input, output, bound) { for (var index = 0; index < bound; index++) { output[index] = input[index]; output[index] = input[index]; } return output.length; }";

    assert_suppressed(
        &format!(
            "{function} copyTwice(new BigInt64Array([1n,2n,3n,4n]), new BigUint64Array(4), 4);"
        ),
        Value::Number(4.0),
    );
    assert_suppressed(
        &format!(
            "{function} var output = new Uint8Array(4); __quickjsRustDetachArrayBuffer(output.buffer); copyTwice(new Uint8Array([1,2,3,4]), output, 4);"
        ),
        Value::Number(0.0),
    );
    assert_suppressed(
        &format!(
            "{function} var buffer = new ArrayBuffer(4, {{ maxByteLength: 8 }}), output = new Uint8Array(buffer, 0, 4); copyTwice(new Uint8Array([1,2,3,4]), output, 4);"
        ),
        Value::Number(4.0),
    );
    assert_suppressed(
        &format!(
            "{function} var buffer = new ArrayBuffer(4, {{ maxByteLength: 8 }}), output = new Uint8Array(buffer); copyTwice(new Uint8Array([1,2,3,4]), output, 4);"
        ),
        Value::Number(4.0),
    );
    assert_suppressed(
        &format!(
            "{function} copyTwice(new Uint8Array([1,2,3,4]), new Uint8Array(new SharedArrayBuffer(4)), 4);"
        ),
        Value::Number(4.0),
    );
    assert_suppressed(
        &format!(
            "{function} var output = new Uint8Array(new ArrayBuffer(4).transferToImmutable()); copyTwice(new Uint8Array([1,2,3,4]), output, 4);"
        ),
        Value::Number(4.0),
    );
}

#[test]
fn typed_dense_backing_borrow_conflicts_are_fallible() {
    let Value::Object(view) = eval("new Uint8Array(4);").expect("typed array should construct")
    else {
        panic!("expected typed array object");
    };
    let buffer = typed_array::typed_array_buffer(&view).expect("view should have a buffer");
    let lease = array_buffer::try_borrow_fixed_array_buffer_bytes_mut(&buffer)
        .expect("first backing lease should succeed");
    assert!(array_buffer::try_borrow_fixed_array_buffer_bytes_mut(&buffer).is_none());
    drop(lease);
    assert!(array_buffer::try_borrow_fixed_array_buffer_bytes_mut(&buffer).is_some());
}

#[test]
fn typed_dense_compiler_rejects_object_literals_and_observable_calls() {
    for source in [
        "function objects(output, bound) { for (var index = 0; index < bound; index++) { output[index] = { value: index }; output[index] = { value: index }; } }",
        "function calls(input, output, bound) { function convert(value) { return value + 1; } for (var index = 0; index < bound; index++) { output[index] = convert(input[index]); output[index] = convert(input[index]); } }",
    ] {
        let bytecode = nested_function(source);
        assert!(
            NumericMutationLoopPlan::compile_all(&bytecode).is_empty(),
            "{:#?}",
            bytecode.code
        );
    }
}
