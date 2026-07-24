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

fn typed_dense_input_layouts(source: &str) -> Vec<(usize, usize, usize)> {
    let bytecode = nested_function(source);
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    plans
        .iter()
        .filter_map(|plan| match &plan.kind {
            NumericMutationLoopKind::Dense(plan) => plan.legacy_input_layout(),
            _ => None,
        })
        .collect()
}

fn typed_dense_input_layout(source: &str) -> (usize, usize, usize) {
    let layouts = typed_dense_input_layouts(source);
    assert_eq!(layouts.len(), 1, "expected one legacy dense plan");
    layouts[0]
}

fn typed_dense_binary_bundle_layouts(source: &str) -> Vec<Vec<(usize, usize, usize)>> {
    let bytecode = nested_function(source);
    NumericMutationLoopPlan::compile_all(&bytecode)
        .iter()
        .filter_map(|plan| match &plan.kind {
            NumericMutationLoopKind::Dense(plan) => plan.legacy_binary_bundle_layouts(),
            _ => None,
        })
        .collect()
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
    assert_eq!(typed_dense_input_layout(function), (8, 19, 46));
    assert!(
        typed_dense_binary_bundle_layouts(function)[0]
            .iter()
            .any(|&(_, lanes, chain_length)| lanes == 4 && chain_length == 7)
    );

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
    assert_eq!(dense::test_typed_binary_bundle_iterations(), 1);
    assert!(dense::test_typed_binary_bundle_steps() > 0);
    assert!(dense::test_typed_binary_bundle_lane_ops() > dense::test_typed_binary_bundle_steps());
    assert_eq!(
        dense::test_typed_logical_operations() - dense::test_typed_executor_steps(),
        dense::test_typed_binary_bundle_lane_ops() - dense::test_typed_binary_bundle_steps()
    );
}

#[test]
fn gaussian_bidirectional_shapes_keep_the_expected_compact_input_layouts() {
    let function = r#"
        function convolveRGBA(src, out, line, coeff, width, height) {
            var rgba;
            var prev_src_r, prev_src_g, prev_src_b, prev_src_a;
            var curr_src_r, curr_src_g, curr_src_b, curr_src_a;
            var curr_out_r, curr_out_g, curr_out_b, curr_out_a;
            var prev_out_r, prev_out_g, prev_out_b, prev_out_a;
            var prev_prev_out_r, prev_prev_out_g, prev_prev_out_b, prev_prev_out_a;
            var src_index, out_index, line_index;
            var i, j;
            var coeff_a0, coeff_a1, coeff_b1, coeff_b2;

            for (i = 0; i < height; i++) {
                src_index = i * width;
                out_index = i;
                line_index = 0;
                rgba = src[src_index];
                prev_src_r = rgba & 0xff;
                prev_src_g = (rgba >> 8) & 0xff;
                prev_src_b = (rgba >> 16) & 0xff;
                prev_src_a = (rgba >> 24) & 0xff;
                prev_prev_out_r = prev_src_r * coeff[6];
                prev_prev_out_g = prev_src_g * coeff[6];
                prev_prev_out_b = prev_src_b * coeff[6];
                prev_prev_out_a = prev_src_a * coeff[6];
                prev_out_r = prev_prev_out_r;
                prev_out_g = prev_prev_out_g;
                prev_out_b = prev_prev_out_b;
                prev_out_a = prev_prev_out_a;
                coeff_a0 = coeff[0];
                coeff_a1 = coeff[1];
                coeff_b1 = coeff[4];
                coeff_b2 = coeff[5];

                for (j = 0; j < width; j++) {
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

                src_index--;
                line_index -= 4;
                out_index += height * (width - 1);
                rgba = src[src_index];
                prev_src_r = rgba & 0xff;
                prev_src_g = (rgba >> 8) & 0xff;
                prev_src_b = (rgba >> 16) & 0xff;
                prev_src_a = (rgba >> 24) & 0xff;
                prev_prev_out_r = prev_src_r * coeff[7];
                prev_prev_out_g = prev_src_g * coeff[7];
                prev_prev_out_b = prev_src_b * coeff[7];
                prev_prev_out_a = prev_src_a * coeff[7];
                prev_out_r = prev_prev_out_r;
                prev_out_g = prev_prev_out_g;
                prev_out_b = prev_prev_out_b;
                prev_out_a = prev_prev_out_a;
                curr_src_r = prev_src_r;
                curr_src_g = prev_src_g;
                curr_src_b = prev_src_b;
                curr_src_a = prev_src_a;
                coeff_a0 = coeff[2];
                coeff_a1 = coeff[3];

                for (j = width - 1; j >= 0; j--) {
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
                    rgba = src[src_index];
                    curr_src_r = rgba & 0xff;
                    curr_src_g = (rgba >> 8) & 0xff;
                    curr_src_b = (rgba >> 16) & 0xff;
                    curr_src_a = (rgba >> 24) & 0xff;
                    rgba = ((line[line_index] + prev_out_r) << 0) +
                        ((line[line_index + 1] + prev_out_g) << 8) +
                        ((line[line_index + 2] + prev_out_b) << 16) +
                        ((line[line_index + 3] + prev_out_a) << 24);
                    out[out_index] = rgba;
                    src_index--;
                    line_index -= 4;
                    out_index -= height;
                }
            }
        }
    "#;

    assert_eq!(
        typed_dense_input_layouts(function),
        vec![(8, 19, 46), (9, 25, 58)]
    );
    let bundle_layouts = typed_dense_binary_bundle_layouts(function);
    assert_eq!(bundle_layouts.len(), 2);
    assert!(bundle_layouts.iter().all(|layouts| {
        layouts
            .iter()
            .any(|&(_, lanes, chain_length)| lanes == 4 && chain_length == 7)
    }));

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} \
             var src = new Uint32Array([0x04030201, 0x08070605, 0x0c0b0a09]); \
             var out = new Uint32Array(3); \
             var line = new Float32Array(12); \
             var coeff = new Float64Array([1, 0, 1, 0, 0, 0, 1, 1]); \
             convolveRGBA(src, out, line, coeff, 3, 1); \
             out.join(':');"
        )),
        Ok(Value::String(
            "201984006:336728078:404100114".to_owned().into()
        ))
    );
    assert_eq!(dense::test_typed_array_dense_path_hits(), 2);
    assert_eq!(dense::test_typed_array_dense_suppressions(), 0);
    assert!(dense::test_typed_binary_bundle_iterations() >= 2);
    assert!(dense::test_typed_executor_steps() < dense::test_typed_logical_operations());
}

#[test]
fn typed_binary_bundle_preserves_float64_nan_infinity_and_negative_zero() {
    let function = r#"
        function recur(input, output, bound, factor, carry) {
            var a = -0, b = Infinity, c = -Infinity, d = NaN, sample = 0;
            for (var index = 0; index < bound; index++) {
                sample = input[index];
                a = sample * factor + a * carry;
                b = sample * factor + b * carry;
                c = sample * factor + c * carry;
                d = sample * factor + d * carry;
                output[index * 4] = a;
                output[index * 4 + 1] = b;
                output[index * 4 + 2] = c;
                output[index * 4 + 3] = d;
            }
            return [Object.is(output[8], -0), output[9] === Infinity,
                output[10] === -Infinity, output[11] !== output[11]].join(':');
        }
    "#;
    assert!(
        typed_dense_binary_bundle_layouts(function)[0]
            .iter()
            .any(|&(_, lanes, chain_length)| lanes == 4 && chain_length == 3)
    );

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} recur(new Float64Array([-0, -0, -0]), new Float64Array(12), 3, 1, 1);"
        )),
        Ok(Value::String("true:true:true:true".to_owned().into()))
    );
    assert_eq!(dense::test_typed_array_dense_path_hits(), 1);
    assert!(dense::test_typed_binary_bundle_iterations() > 0);
    assert!(dense::test_typed_executor_steps() < dense::test_typed_logical_operations());
}

#[test]
fn typed_binary_bundle_uses_exact_to_int32_and_to_uint32_shift_semantics() {
    let function = r#"
        function bitwise(input, output, bound, shift, mask, zero) {
            var a = 0, b = 0, c = 0, d = 0;
            var x0 = 0, x1 = 0, x2 = 0, x3 = 0;
            for (var index = 0; index < bound; index++) {
                var base = index * 4;
                x0 = input[base];
                x1 = input[base + 1];
                x2 = input[base + 2];
                x3 = input[base + 3];
                a = ((x0 << shift) ^ mask) >>> zero;
                b = ((x1 << shift) ^ mask) >>> zero;
                c = ((x2 << shift) ^ mask) >>> zero;
                d = ((x3 << shift) ^ mask) >>> zero;
                output[base] = a;
                output[base + 1] = b;
                output[base + 2] = c;
                output[base + 3] = d;
            }
            return output.join(':');
        }
    "#;
    assert!(
        typed_dense_binary_bundle_layouts(function)[0]
            .iter()
            .any(|&(_, lanes, chain_length)| lanes == 4 && chain_length == 3)
    );

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} \
             bitwise(new Float64Array([4294967295, 2147483648, NaN, -1, \
                 4294967295, 2147483648, NaN, -1]), new Uint32Array(8), 2, 1, \
                 2147483648, 0);"
        )),
        Ok(Value::String(
            "2147483646:2147483648:2147483648:2147483646:2147483646:2147483648:2147483648:2147483646"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(dense::test_typed_array_dense_path_hits(), 1);
    assert!(dense::test_typed_binary_bundle_lane_ops() >= 12);
}

#[test]
fn typed_binary_bundle_executes_three_two_step_lanes() {
    let function = r#"
        function affine3(input, output, bound, factor, bias) {
            var x0 = 0, x1 = 0, x2 = 0;
            var a = 0, b = 0, c = 0;
            for (var index = 0; index < bound; index++) {
                var base = index * 3;
                x0 = input[base];
                x1 = input[base + 1];
                x2 = input[base + 2];
                a = x0 * factor + bias;
                b = x1 * factor + bias;
                c = x2 * factor + bias;
                output[base] = a;
                output[base + 1] = b;
                output[base + 2] = c;
            }
            return output.join(':');
        }
    "#;
    assert!(
        typed_dense_binary_bundle_layouts(function)[0]
            .iter()
            .any(|&(_, lanes, chain_length)| lanes == 3 && chain_length == 2)
    );

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} affine3(new Float64Array([1,-2,3,4,-4,6,7,8,-1]), new Float64Array(9), 3, 2, 1);"
        )),
        Ok(Value::String("3:-3:7:9:-7:13:15:17:-1".to_owned().into()))
    );
    // The first iteration reaches the backedge normally; the bundle executor
    // consumes the two remaining iterations.
    assert_eq!(dense::test_typed_array_dense_path_hits(), 1);
    assert_eq!(dense::test_typed_binary_bundle_iterations(), 2);
    assert_eq!(dense::test_typed_binary_bundle_steps(), 4);
    assert_eq!(dense::test_typed_binary_bundle_lane_ops(), 12);
    assert_eq!(
        dense::test_typed_logical_operations() - dense::test_typed_executor_steps(),
        dense::test_typed_binary_bundle_lane_ops() - dense::test_typed_binary_bundle_steps()
    );
}

#[test]
fn typed_binary_bundle_executes_two_four_step_lanes_at_minimum_savings() {
    let function = r#"
        function affine2(input, output, bound, factor, bias, divisor, subtractor) {
            var x0 = 0, x1 = 0;
            var a = 0, b = 0;
            for (var index = 0; index < bound; index++) {
                var base = index * 2;
                x0 = input[base];
                x1 = input[base + 1];
                a = ((x0 * factor + bias) / divisor) - subtractor;
                b = ((x1 * factor + bias) / divisor) - subtractor;
                output[base] = a;
                output[base + 1] = b;
            }
            return output.join(':');
        }
    "#;
    let layouts = typed_dense_binary_bundle_layouts(function);
    assert_eq!(layouts.len(), 1);
    assert_eq!(layouts[0].len(), 1, "layouts={:?}", layouts[0]);
    assert_eq!((layouts[0][0].1, layouts[0][0].2), (2, 4));

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} affine2(new Float64Array([1,-2,5,-0,9,-6]), new Float64Array(6), 3, 2, 4, 2, 3);"
        )),
        Ok(Value::String("0:-3:4:-1:8:-7".to_owned().into()))
    );
    // The first iteration reaches the backedge normally; the bundle executor
    // consumes the two remaining iterations.
    assert_eq!(dense::test_typed_array_dense_path_hits(), 1);
    assert_eq!(dense::test_typed_binary_bundle_iterations(), 2);
    assert_eq!(dense::test_typed_binary_bundle_steps(), 8);
    assert_eq!(dense::test_typed_binary_bundle_lane_ops(), 16);
    assert_eq!(
        dense::test_typed_logical_operations() - dense::test_typed_executor_steps(),
        dense::test_typed_binary_bundle_lane_ops() - dense::test_typed_binary_bundle_steps()
    );
}

#[test]
fn typed_binary_bundle_executes_all_remaining_admitted_binary_operations() {
    let function = r#"
        function remainingOps(input, output, bound, subtractor, divisor,
                remainderDivisor, shift, andMask, orMask) {
            var x0 = 0, x1 = 0, x2 = 0, x3 = 0;
            var x4 = 0, x5 = 0, x6 = 0, x7 = 0;
            var x8 = 0, x9 = 0, x10 = 0, x11 = 0;
            var a = 0, b = 0, c = 0, d = 0;
            for (var index = 0; index < bound; index++) {
                var base = index * 12;
                x0 = input[base];
                x1 = input[base + 1];
                x2 = input[base + 2];
                x3 = input[base + 3];
                x4 = input[base + 4];
                x5 = input[base + 5];
                x6 = input[base + 6];
                x7 = input[base + 7];
                x8 = input[base + 8];
                x9 = input[base + 9];
                x10 = input[base + 10];
                x11 = input[base + 11];

                a = (x0 - subtractor) / divisor;
                b = (x1 - subtractor) / divisor;
                c = (x2 - subtractor) / divisor;
                d = (x3 - subtractor) / divisor;
                output[base] = a;
                output[base + 1] = b;
                output[base + 2] = c;
                output[base + 3] = d;

                a = (x4 % remainderDivisor) >> shift;
                b = (x5 % remainderDivisor) >> shift;
                c = (x6 % remainderDivisor) >> shift;
                d = (x7 % remainderDivisor) >> shift;
                output[base + 4] = a;
                output[base + 5] = b;
                output[base + 6] = c;
                output[base + 7] = d;

                a = (x8 & andMask) | orMask;
                b = (x9 & andMask) | orMask;
                c = (x10 & andMask) | orMask;
                d = (x11 & andMask) | orMask;
                output[base + 8] = a;
                output[base + 9] = b;
                output[base + 10] = c;
                output[base + 11] = d;
            }
            return [output[12] === -Infinity, output[13] === Infinity,
                output[14] === 2.5, Object.is(output[15], -0),
                output[16] === -1, output[17] === 0, output[18] === 0,
                output[19] === 0, output[20] === 511, output[21] === 256,
                output[22] === 256, output[23] === 511].join(':');
        }
    "#;
    let layouts = typed_dense_binary_bundle_layouts(function);
    assert_eq!(layouts.len(), 1);
    assert!(
        layouts[0]
            .iter()
            .filter(|&&(_, lanes, chain_length)| lanes == 4 && chain_length == 2)
            .count()
            >= 3,
        "layouts={:?}",
        layouts[0]
    );

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} remainingOps(new Float64Array([Infinity,-Infinity,-0,5,-9,9,Infinity,-0,4294967295,2147483648,NaN,511,Infinity,-Infinity,-0,5,-9,9,Infinity,-0,4294967295,2147483648,NaN,511]), new Float64Array(24), 2, 5, -2, 4, 1, 255, 256);"
        )),
        Ok(Value::String(
            "true:true:true:true:true:true:true:true:true:true:true:true"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(dense::test_typed_array_dense_path_hits(), 1);
    assert!(dense::test_typed_binary_bundle_iterations() > 0);
    assert!(dense::test_typed_binary_bundle_steps() >= 6);
    assert!(dense::test_typed_binary_bundle_lane_ops() >= 24);
    assert!(dense::test_typed_executor_steps() < dense::test_typed_logical_operations());
}

#[test]
fn typed_binary_bundle_discards_staged_stores_before_oob_replay() {
    let function = r#"
        function mutate(output, probe, bound, factor, zero) {
            var a = 0, b = 0, c = 0, d = 0, sample = 0;
            for (var index = 0; index < bound; index++) {
                sample = output[index];
                a = sample * factor + a * zero;
                b = sample * factor + b * zero;
                c = sample * factor + c * zero;
                d = sample * factor + d * zero;
                output[index] = output[index] + a + b + c + d;
                probe[index * 2] = probe[index * 2] + 1;
            }
            return output.join(':') + '|' + probe.join(':');
        }
    "#;
    assert!(
        typed_dense_binary_bundle_layouts(function)[0]
            .iter()
            .any(|&(_, lanes, chain_length)| lanes == 4 && chain_length == 3)
    );

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} mutate(new Float64Array([1,2,3]), new Float64Array(4), 3, 1, 0);"
        )),
        Ok(Value::String("5:10:15|1:0:1:0".to_owned().into()))
    );
    assert_eq!(dense::test_typed_array_dense_path_hits(), 1);
    assert_eq!(dense::test_typed_array_dense_suppressions(), 0);
    assert!(dense::test_typed_binary_bundle_iterations() >= 2);
}

#[test]
fn typed_dense_input_prefix_refreshes_loop_carried_locals_across_entries() {
    let function = "function recur(input, output, bound, factor) { var carried = factor; for (var index = 0; index < bound; index++) { output[index] = input[index] * factor + carried + 1; carried = output[index]; factor = factor + 1; } return factor + '|' + carried + '|' + output.join(':'); }";
    let (constant_count, local_count, dynamic_count) = typed_dense_input_layout(function);
    assert!(constant_count > 0);
    assert!(local_count > 0);
    assert!(dynamic_count > 0);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} \
             recur(new Float64Array([1,2,3,4]), new Float64Array(4), 4, 2) + ';' + \
             recur(new Float64Array([-1,0,Infinity,NaN]), new Float64Array(4), 4, -0);"
        )),
        Ok(Value::String(
            "6|46|5:12:25:46;4|NaN|1:2:Infinity:NaN".to_owned().into()
        ))
    );
    let iterations = dense::test_iterations();
    assert!(iterations > 2);
    assert_eq!(dense::test_typed_array_dense_attempts(), 2);
    assert_eq!(dense::test_typed_array_dense_path_hits(), 2);
    assert_eq!(
        dense::test_typed_constant_prefix_loads(),
        constant_count * 2
    );
    assert_eq!(
        dense::test_typed_local_prefix_loads(),
        local_count * iterations
    );
    assert_eq!(
        dense::test_typed_logical_operations(),
        dynamic_count * iterations
    );
    assert_eq!(dense::test_typed_binary_bundle_iterations(), 0);
    assert_eq!(
        dense::test_typed_executor_steps(),
        dense::test_typed_logical_operations()
    );
}

#[test]
fn typed_dense_input_prefix_does_not_run_when_the_first_guard_fails() {
    let function = "function once(input, output, bound, factor) { for (var index = 0; index < bound; index++) { output[index] = input[index] * factor + 1; factor = factor + 1; } return factor + '|' + output.join(':'); }";
    assert_typed_dense_plan(function);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} once(new Float64Array([2]), new Float64Array(1), 1, 3);"
        )),
        Ok(Value::String("4|7".to_owned().into()))
    );
    assert_eq!(dense::test_iterations(), 0);
    assert_eq!(dense::test_typed_array_dense_attempts(), 1);
    assert_eq!(dense::test_typed_array_dense_path_hits(), 0);
    assert_eq!(dense::test_typed_constant_prefix_loads(), 0);
    assert_eq!(dense::test_typed_local_prefix_loads(), 0);
    assert_eq!(dense::test_typed_logical_operations(), 0);
}

#[test]
fn typed_dense_input_prefix_preserves_staged_store_rollback_before_oob_replay() {
    let function = "function mutate(input, output, probe, bound, factor) { for (var index = 0; index < bound; index++) { output[index] = output[index] + input[index] * factor; probe[index * 2] = probe[index * 2] + 1; } return output.join(':') + '|' + probe.join(':'); }";
    let (constant_count, local_count, dynamic_count) = typed_dense_input_layout(function);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{function} mutate(new Float64Array([1,2,3]), new Float64Array([10,10,10]), new Float64Array(4), 3, 2);"
        )),
        Ok(Value::String("12:14:16|1:0:1:0".to_owned().into()))
    );
    let committed_iterations = dense::test_iterations();
    assert!(committed_iterations > 0);
    // The successful region deoptimizes on the last probe, then the replayed
    // backedge performs one final zero-iteration attempt.
    assert_eq!(dense::test_typed_array_dense_attempts(), 2);
    assert_eq!(dense::test_typed_array_dense_path_hits(), 1);
    assert_eq!(dense::test_typed_array_dense_suppressions(), 0);
    assert_eq!(dense::test_typed_constant_prefix_loads(), constant_count);
    assert_eq!(
        dense::test_typed_local_prefix_loads(),
        local_count * (committed_iterations + 1)
    );
    assert!(dense::test_typed_logical_operations() > dynamic_count * committed_iterations);
}

#[test]
fn compact_array_executor_preserves_the_reordered_input_prefix() {
    let function = "function recur(input, output, bound, factor) { var carried = factor; for (var index = 0; index < bound; index++) { output[index] = input[index] * factor + carried + 1; carried = output[index]; factor = factor + 1; } return factor + '|' + carried + '|' + output.join(':'); }";
    assert_typed_dense_plan(function);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!("{function} recur([1,2,3,4], [0,0,0,0], 4, 2);")),
        Ok(Value::String("6|46|5:12:25:46".to_owned().into()))
    );
    assert!(dense::test_compact_dynamic_hits() > 0);
    assert_eq!(dense::test_typed_array_dense_path_hits(), 0);
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
