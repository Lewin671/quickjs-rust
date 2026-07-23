use super::*;
use crate::bytecode::compiler;
use crate::value::ArrayRef;
use crate::{Value, eval};
use std::collections::HashMap;

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

fn assert_reduction_selected(source: &str) {
    let bytecode = nested_function(source);
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
    let NumericMutationLoopKind::Dense(plan) = &plans[0].kind else {
        panic!("expected dense plan: {:#?}", bytecode.code);
    };
    assert!(plan.is_legacy_reduction(), "{:#?}", bytecode.code);
}

fn assert_reduction_rejected(source: &str) {
    let bytecode = nested_function(source);
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
    let NumericMutationLoopKind::Dense(plan) = &plans[0].kind else {
        panic!("expected dense plan: {:#?}", bytecode.code);
    };
    assert!(!plan.is_legacy_reduction(), "{:#?}", bytecode.code);
}

fn assert_strided_reduction_selected(source: &str) {
    let bytecode = nested_function(source);
    let plans = NumericMutationLoopPlan::compile_all(&bytecode);
    assert_eq!(plans.len(), 1, "{:#?}", bytecode.code);
    let NumericMutationLoopKind::Dense(plan) = &plans[0].kind else {
        panic!("expected dense plan: {:#?}", bytecode.code);
    };
    assert!(plan.is_two_lane_strided_reduction(), "{:#?}", bytecode.code);
}

#[test]
fn exact_index_reduction_checked_arithmetic_stays_in_the_array_index_range() {
    let max = (u32::MAX - 1) as usize;
    assert_eq!(dense::test_checked_array_index_product(1, max), Some(max));
    assert_eq!(
        dense::test_checked_array_index_product(2, max / 2),
        Some(max)
    );
    assert_eq!(dense::test_checked_array_index_product(2, max), None);
    assert_eq!(dense::test_checked_array_index_product(max, max), None);
    assert_eq!(dense::test_checked_next_array_index(max - 1, 1), Some(max));
    assert_eq!(dense::test_checked_next_array_index(max, 1), None);
    assert_eq!(dense::test_checked_next_array_index(max, 0), Some(max));
}

#[test]
fn reduction_selects_dft_like_direct_this_and_local_two_lane_loop() {
    let source = "function transform(buffer, stride) { var real = 0, imag = 0; for (var index = 0; index < buffer.length; index++) { real += this.positive[stride * index] * buffer[index]; imag += this.negative[stride * index] * buffer[index]; } return real + ':' + imag; }";
    assert_strided_reduction_selected(source);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} var owner = {{ positive: [1,2,3,4,5,6,7,8,9,10], negative: [10,9,8,7,6,5,4,3,2,1] }}; transform.call(owner, [1,2,3,4,5,6,7,8,9,10], 1);"
        )),
        Ok(Value::String("385:220".to_owned().into()))
    );
    assert_eq!(dense::test_reduction_path_hits(), 1);
    assert_eq!(dense::test_reduction_iterations(), 9);
    assert_eq!(dense::test_exact_index_reduction_path_hits(), 1);
    assert_eq!(dense::test_shared_sample_stride_reduction_path_hits(), 1);
}

#[test]
fn strided_reduction_accepts_both_multiplier_orders_and_distinct_strides() {
    let source = "function transform(sample, firstStride, secondStride, bound) { var real = 0, imag = 0; for (var index = 0; index < bound; index++) { real += this.first[firstStride * index] * sample[index]; imag += this.second[index * secondStride] * sample[index]; } return index + ':' + real + ':' + imag; }";
    assert_strided_reduction_selected(source);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} transform.call({{ first: [1,0,2,0,3,0,4], second: [10,0,20,0,30,0,40] }}, [1,2,3,4], 2, 2, 4);"
        )),
        Ok(Value::String("4:30:300".to_owned().into()))
    );
    assert_eq!(dense::test_exact_index_reduction_path_hits(), 1);
    assert_eq!(dense::test_shared_sample_stride_reduction_path_hits(), 1);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} transform.call({{ first: [1,0,2,0,3,0,4], second: [10,0,0,20,0,0,30,0,0,40] }}, [1,2,3,4], 2, 3, 4);"
        )),
        Ok(Value::String("4:30:300".to_owned().into()))
    );
    assert_eq!(dense::test_reduction_path_hits(), 1);
    assert_eq!(dense::test_reduction_iterations(), 3);
    assert_eq!(dense::test_exact_index_reduction_path_hits(), 1);
    assert_eq!(dense::test_shared_sample_stride_reduction_path_hits(), 0);
}

#[test]
fn shared_sample_stride_requires_the_same_compiled_sample_receiver() {
    let source = "function transform(firstSample, secondSample, first, second, stride, bound) { var real = 0, imag = 0; for (var index = 0; index < bound; index++) { real += first[stride * index] * firstSample[index]; imag += second[index * stride] * secondSample[index]; } return index + ':' + real + ':' + imag; }";
    assert_strided_reduction_selected(source);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} var shared = [1,2,3,4]; transform(shared, shared, [1,2,3,4], [10,20,30,40], 1, 4);"
        )),
        Ok(Value::String("4:30:300".to_owned().into()))
    );
    assert_eq!(dense::test_exact_index_reduction_path_hits(), 1);
    assert_eq!(dense::test_shared_sample_stride_reduction_path_hits(), 0);
}

#[test]
fn strided_reduction_fractional_counter_deoptimizes_before_sample_load() {
    let source = "function transform(sample, first, second, firstStride, secondStride, start, bound) { var real = 0, imag = 0; for (var index = start; index < bound; index++) { real += first[firstStride * index] * sample[index]; imag += second[index * secondStride] * sample[index]; } return index + ':' + real + ':' + imag; }";
    assert_strided_reduction_selected(source);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} var sample = []; sample[0.5] = 10; sample[1.5] = 20; transform(sample, [0,2,0,3], [0,4,0,5], 2, 2, 0.5, 2);"
        )),
        Ok(Value::String("2.5:80:140".to_owned().into()))
    );
    assert_eq!(dense::test_reduction_path_hits(), 0);
    assert_eq!(dense::test_reduction_iterations(), 0);
    assert_eq!(dense::test_exact_index_reduction_path_hits(), 0);
    assert!(dense::test_read_only_bailouts() > 0);
}

#[test]
fn strided_reduction_preserves_negative_zero_stride_edges_and_mid_loop_oob_replay() {
    let source = "function transform(sample, first, second, firstStride, secondStride, start, bound) { var negativeZero = 1 / start === -Infinity, real = 0, imag = 0; for (var index = start; index < bound; index++) { real += first[firstStride * index] * sample[index]; imag += second[index * secondStride] * sample[index]; } return negativeZero + ':' + index + ':' + real + ':' + imag; }";
    assert_strided_reduction_selected(source);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} transform([1,1,1], [1,2,3], [4,5,6], 1, 1, -0, 3);"
        )),
        Ok(Value::String("true:3:6:15".to_owned().into()))
    );
    assert_eq!(dense::test_reduction_path_hits(), 1);
    assert_eq!(dense::test_exact_index_reduction_path_hits(), 1);
    assert_eq!(dense::test_shared_sample_stride_reduction_path_hits(), 1);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} transform([1,2,3], [2], [3], 0, -0, -0, 3);"
        )),
        Ok(Value::String("true:3:12:18".to_owned().into()))
    );
    assert_eq!(dense::test_exact_index_reduction_path_hits(), 1);
    assert_eq!(dense::test_shared_sample_stride_reduction_path_hits(), 1);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} var nan = []; nan.NaN = 2; var infinite = []; infinite.NaN = 3; infinite.Infinity = 3; var negative = [1]; negative[-1] = 2; negative[-2] = 3; var fractional = [1,3]; fractional[0.5] = 2; transform([1,1,1], nan, infinite, NaN, Infinity, 0, 3) + '|' + transform([1,1,1], negative, fractional, -1, 0.5, 0, 3);"
        )),
        Ok(Value::String("false:3:6:9|false:3:6:6".to_owned().into()))
    );
    assert_eq!(dense::test_reduction_iterations(), 0);
    assert_eq!(dense::test_exact_index_reduction_path_hits(), 0);
    assert!(dense::test_read_only_bailouts() > 0);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} transform([1], [2], [3], -Infinity, 1, 0, 0);"
        )),
        Ok(Value::String("false:0:0:0".to_owned().into()))
    );
    assert_eq!(dense::test_exact_index_reduction_path_hits(), 0);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} transform([1,1,1,1], [1,2,3,4], [10,20], 1, 1, 0, 4);"
        )),
        Ok(Value::String("false:4:10:NaN".to_owned().into()))
    );
    assert_eq!(dense::test_reduction_iterations(), 1);
    assert_eq!(dense::test_exact_index_reduction_path_hits(), 1);
    assert_eq!(dense::test_shared_sample_stride_reduction_path_hits(), 1);
    assert!(dense::test_read_only_bailouts() > 0);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} var hits = 0, marker = {{ valueOf: function () {{ hits++; return 30; }} }}; transform([1,1,1,1], [1,2,3,4], [10,20,marker,40], 1, 1, 0, 4) + ':' + hits;"
        )),
        Ok(Value::String("false:4:10:100:1".to_owned().into()))
    );
    assert_eq!(dense::test_reduction_iterations(), 2);
    assert_eq!(dense::test_exact_index_reduction_path_hits(), 2);
    assert_eq!(dense::test_shared_sample_stride_reduction_path_hits(), 2);
    assert!(dense::test_read_only_bailouts() > 0);
}

#[test]
fn shared_sample_stride_replays_a_replacing_sample_object_between_lane_reads() {
    let source = "function transform(sample, first, second, stride) { var real = 0, imag = 0; for (var index = 0; index < sample.length; index++) { real += first[stride * index] * sample[index]; imag += second[index * stride] * sample[index]; } return index + ':' + real + ':' + imag; }";
    assert_strided_reduction_selected(source);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} var hits = 0, sample = [1,1,0], marker = {{ valueOf: function () {{ hits++; sample[2] = 3; return 2; }} }}; sample[2] = marker; transform(sample, [1,10,100], [2,20,200], 1) + ':' + hits;"
        )),
        Ok(Value::String("3:211:622:1".to_owned().into()))
    );
    assert_eq!(dense::test_reduction_iterations(), 1);
    assert_eq!(dense::test_shared_sample_stride_reduction_path_hits(), 1);
    assert_eq!(dense::test_read_only_bailouts(), 1);
}

#[test]
fn reduction_selects_one_and_three_lane_index_forms() {
    let one_lane = "function dot(left, right, bound) { var sum = 0; for (var index = 1; index < bound; index++) sum += left[index - 1] * right[index - 1]; return index + ':' + sum; }";
    assert_reduction_selected(one_lane);
    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!("{one_lane} dot([2,3,4,5], [10,20,30,40], 5);")),
        Ok(Value::String("5:400".to_owned().into()))
    );
    assert_eq!(dense::test_reduction_path_hits(), 1);
    assert_eq!(dense::test_reduction_iterations(), 3);

    let three_lane = "function project(a, b, c, d, e, f, bound, offset) { var forward = 0, shifted = 0, reverse = 0; for (var index = 0; index < bound; index++) { forward += a[index] * b[index]; shifted += c[index + offset] * d[index]; reverse += e[offset - index] * f[index]; } return index + ':' + forward + ':' + shifted + ':' + reverse; }";
    assert_reduction_selected(three_lane);
    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{three_lane} project([1,2,3,4], [1,1,1,1], [0,0,0,0,1,2,3,4], [2,2,2,2], [0,1,2,3,4], [1,1,1,1], 4, 4);"
        )),
        Ok(Value::String("4:10:20:10".to_owned().into()))
    );
    assert_eq!(dense::test_reduction_path_hits(), 1);
    assert_eq!(dense::test_reduction_iterations(), 3);
}

#[test]
fn reduction_keeps_aliased_reads_independent() {
    let source = "function squares(left, right) { var sum = 0; for (var index = 0; index < left.length; index++) sum += left[index] * right[index]; return sum; }";
    assert_reduction_selected(source);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} var shared = [1,2,3,4]; squares(shared, shared);"
        )),
        Ok(Value::Number(30.0))
    );
    assert_eq!(dense::test_reduction_path_hits(), 1);
    assert_eq!(dense::test_reduction_iterations(), 3);
}

#[test]
fn reduction_discards_first_lane_work_when_second_lane_deoptimizes() {
    let source = "function reduce(a, b, c, d, bound) { var first = 0, second = 0; for (var index = 0; index < bound; index++) { first += a[index] * b[index]; second += c[index] * d[index]; } return index + ':' + first + ':' + second; }";
    assert_reduction_selected(source);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "var coercions = 0; var marker = {{ valueOf: function () {{ coercions++; return 3; }} }}; {source} reduce([1,10,100,1000], [1,1,1,1], [1,2,marker,4], [1,1,1,1], 4) + ':' + coercions;"
        )),
        Ok(Value::String("4:1111:10:1".to_owned().into()))
    );
    assert_eq!(dense::test_reduction_iterations(), 2);
    assert_eq!(dense::test_read_only_bailouts(), 1);
}

#[test]
fn strided_exact_index_reduction_uses_separate_multiply_then_add_rounding() {
    let source = "function transform(initial, sample, first, second, stride) { var real = initial, imag = 0; for (var index = 0; index < sample.length; index++) { real += first[stride * index] * sample[index]; imag += second[index * stride] * sample[index]; } return real === 0 && 1 / real === Infinity && imag === 1.0000000000000002; }";
    assert_strided_reduction_selected(source);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} transform(-1.0000000000000004, [0,1.0000000000000002], [0,1.0000000000000002], [0,1], 1);"
        )),
        Ok(Value::Boolean(true))
    );
    assert_eq!(dense::test_reduction_path_hits(), 1);
    assert_eq!(dense::test_reduction_iterations(), 1);
    assert_eq!(dense::test_exact_index_reduction_path_hits(), 1);
    assert_eq!(dense::test_shared_sample_stride_reduction_path_hits(), 1);
}

#[test]
fn reduction_preserves_iteration_order_and_special_numbers() {
    let source = "function dot(initial, left, right) { var sum = initial; for (var index = 0; index < left.length; index++) sum += left[index] * right[index]; return sum; }";
    assert_reduction_selected(source);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!("{source} dot(0, [0,1e16,-1e16,1], [1,1,1,1]);")),
        Ok(Value::Number(1.0))
    );
    assert_eq!(dense::test_reduction_iterations(), 3);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} var negativeZero = dot(-0, [-0,-0,-0], [1,1,1]); var nan = dot(0, [0,NaN,1], [1,1,1]); var positive = dot(0, [0,Infinity,1], [1,1,1]); var negative = dot(0, [0,-Infinity,1], [1,1,1]); 1 / negativeZero === -Infinity && nan !== nan && positive === Infinity && negative === -Infinity;"
        )),
        Ok(Value::Boolean(true))
    );
    assert_eq!(dense::test_reduction_path_hits(), 4);
    assert_eq!(dense::test_reduction_iterations(), 8);
}

#[test]
fn reduction_zero_progress_deopt_does_not_publish_partial_lane_work() {
    let source = "function reduce(a, b, c, d, bound) { var first = 0, second = 0; for (var index = 0; index < bound; index++) { first += a[index] * b[index]; second += c[index] * d[index]; } return index + ':' + first + ':' + second; }";
    assert_reduction_selected(source);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "var coercions = 0; var marker = {{ valueOf: function () {{ coercions++; return 2; }} }}; {source} reduce([1,10], [1,1], [1,marker], [1,1], 2) + ':' + coercions;"
        )),
        Ok(Value::String("2:11:3:1".to_owned().into()))
    );
    assert_eq!(dense::test_reduction_path_hits(), 0);
    assert_eq!(dense::test_reduction_iterations(), 0);
    assert_eq!(dense::test_read_only_bailouts(), 1);
}

#[test]
fn reduction_publishes_counter_accumulator_and_duplicate_result_shadows() {
    let source = "function dot(left, right, bound) { var sum = 0, last = -1; for (var index = 0; index < bound; index++) last = (sum += left[index] * right[index]); return index + ':' + sum + ':' + last; }";
    assert_reduction_selected(source);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!("{source} dot([1,2,3,4], [1,2,3,4], 4);")),
        Ok(Value::String("4:30:30".to_owned().into()))
    );
    assert_eq!(dense::test_reduction_path_hits(), 1);
    assert_eq!(dense::test_reduction_iterations(), 3);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!("{source} dot([1], [1], 0);")),
        Ok(Value::String("0:0:-1".to_owned().into()))
    );
    assert_eq!(dense::test_reduction_iterations(), 0);
}

#[test]
fn reduction_rejects_extra_arithmetic_shared_accumulators_cross_lane_writes_and_countdown() {
    assert_reduction_rejected(
        "function extra(a, b, bound) { var sum = 0; for (var index = 0; index < bound; index++) sum += a[index] * b[index] + 1; return sum; }",
    );
    assert_reduction_rejected(
        "function shared(a, b, c, d, bound) { var sum = 0; for (var index = 0; index < bound; index++) { sum += a[index] * b[index]; sum += c[index] * d[index]; } return sum; }",
    );
    assert_reduction_rejected(
        "function crossed(a, b, c, d, bound) { var first = 0, second = 0; for (var index = 0; index < bound; index++) { first += a[index] * b[index]; second += c[index] * d[index]; first = second; } return first + second; }",
    );
    assert_reduction_rejected(
        "function countdown(a, b, bound) { var sum = 0; while (bound--) sum += a[bound] * b[bound]; return sum; }",
    );
    assert_reduction_rejected(
        "function descending(a, b, index) { var sum = 0; for (; index >= 0; index--) sum += a[index] * b[index]; return sum; }",
    );
}

#[test]
fn reduction_runtime_guards_reject_direct_eval_and_captured_slots() {
    let direct = "function direct(a, b, bound) { var sum = 0; eval(''); for (var index = 0; index < bound; index++) sum += a[index] * b[index]; return index + ':' + sum; }";
    assert_reduction_selected(direct);
    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!("{direct} direct([1,2,3], [1,2,3], 3);")),
        Ok(Value::String("3:14".to_owned().into()))
    );
    assert_eq!(dense::test_reduction_path_hits(), 0);

    let captured = "function captured(a, b, bound) { var sum = 0; function read() { return index + sum + bound; } for (var index = 0; index < bound; index++) sum += a[index] * b[index]; return read() + ':' + sum; }";
    assert_reduction_selected(captured);
    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!("{captured} captured([1,2,3], [1,2,3], 3);")),
        Ok(Value::String("20:14".to_owned().into()))
    );
    assert_eq!(dense::test_reduction_path_hits(), 0);
}

#[test]
fn reduction_sparse_input_replays_prototype_getter_once() {
    let source = "function dot(left, right, bound) { var sum = 0; for (var index = 0; index < bound; index++) sum += left[index] * right[index]; return index + ':' + sum; }";
    assert_reduction_selected(source);

    dense::reset_test_iterations();
    assert_eq!(
        eval(&format!(
            "{source} var hits = 0, left = [1,,3], right = [1,1,1]; Object.defineProperty(Array.prototype, '1', {{ configurable: true, get: function () {{ hits++; return 5; }} }}); var result = dot(left, right, 3); delete Array.prototype[1]; result + ':' + hits;"
        )),
        Ok(Value::String("3:9:1".to_owned().into()))
    );
    assert!(dense::test_read_only_bailouts() > 0);
    assert_eq!(dense::test_reduction_iterations(), 0);
}

#[test]
fn reduction_direct_this_source_rejects_accessor_proxy_and_typed_array_owners() {
    let source = "function dot(right, bound) { var sum = 0; for (var index = 0; index < bound; index++) sum += this.left[index] * right[index]; return sum; }";
    assert_reduction_selected(source);

    for (setup, owner, expected_hits) in [
        (
            "var hits = 0, owner = {}; Object.defineProperty(owner, 'left', { get: function () { hits++; return [1,2,3]; } });",
            "owner",
            3.0,
        ),
        (
            "var hits = 0, owner = new Proxy({ left: [1,2,3] }, { get: function (target, key) { if (key === 'left') hits++; return target[key]; } });",
            "owner",
            3.0,
        ),
        (
            "var hits = 0, owner = new Uint8Array(1); owner.left = [1,2,3];",
            "owner",
            0.0,
        ),
    ] {
        dense::reset_test_iterations();
        assert_eq!(
            eval(&format!(
                "{source} {setup} dot.call({owner}, [1,1,1], 3) + ':' + hits;"
            )),
            Ok(Value::String(format!("6:{expected_hits}").into())),
            "setup: {setup}"
        );
        assert_eq!(dense::test_reduction_path_hits(), 0, "setup: {setup}");
    }
}

#[test]
fn reduction_direct_this_source_rejects_module_namespace_exotics() {
    let array = ArrayRef::new(vec![Value::Number(1.0)]);
    let mut properties = HashMap::new();
    properties.insert("left".to_owned(), Value::Array(array));
    let ordinary = crate::ObjectRef::new(properties.clone());
    assert!(dense::test_legacy_direct_this_array_source_resolves(
        &Value::Object(ordinary),
        "left"
    ));

    let namespace = crate::ObjectRef::new(properties);
    namespace.mark_module_namespace_exotic();
    assert!(!dense::test_legacy_direct_this_array_source_resolves(
        &Value::Object(namespace),
        "left"
    ));
}

#[test]
fn reduction_readable_lease_fails_closed_on_mutable_borrow() {
    let array = ArrayRef::new(vec![Value::Number(1.0), Value::Number(2.0)]);
    let arrays = [array.clone()];
    let mut reduction_ran = false;
    array
        .with_dense_writable_elements(|_| {
            assert!(
                ArrayRef::with_dense_readable_element_sets(&arrays, |_| {
                    reduction_ran = true;
                })
                .is_none()
            );
        })
        .expect("dense array should grant the outer writable lease");
    assert!(!reduction_ran);
    assert!(ArrayRef::with_dense_readable_element_sets(&arrays, |_| ()).is_some());
}
