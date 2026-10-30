//! Effect guards must be renewed on entry and must never suppress observers.
use super::tests::nested_function;
use crate::{Value, eval};

#[test]
fn primitive_native_calls_have_a_guarded_and_a_normal_region() {
    let bytecode = nested_function(
        "function run(text, n) { var sum = 0; for (var i = 0; i < n; i++) { sum += text.slice(i % 3, i % 3 + 2).length; } return sum; }",
    );
    let programs = super::compile_all(&bytecode);
    let program = programs
        .iter()
        .find(|p| !p.native_call_guards.is_empty())
        .expect("pure-call variant");
    assert!(!program.hoisted_reads.is_empty());
    let normal = program
        .unhoisted
        .as_ref()
        .expect("normal admission remains available");
    assert!(normal.native_call_guards.is_empty());
    assert!(normal.hoisted_reads.is_empty());
    assert_eq!(
        eval(
            "function run(text, n) { var sum = 0; for (var i = 0; i < n; i++) { sum += text.slice(i % 3, i % 3 + 2).length; } return sum; } run('abcdef', 100);"
        ),
        Ok(Value::Number(200.0))
    );
}
#[test]
fn replaced_native_and_getter_are_observed_on_every_required_read() {
    assert_eq!(
        eval(
            "var original = String.prototype.slice; var hits = 0; \
        function run(text, n) { var sum = 0; for (var i = 0; i < n; i++) { sum += text.slice(0, 2).length; } return sum; } \
        var out = [run('abcdef', 4)]; \
        String.prototype.slice = function(a, b) { String.prototype.slice = function(a, b) { return 'qqq'; }; return 'x'; }; \
        out.push(run('abcdef', 4)); \
        Object.defineProperty(String.prototype, 'slice', { get: function() { hits++; return original; }, configurable: true }); \
        out.push(run('abcdef', 4), hits); \
        Object.defineProperty(String.prototype, 'slice', { value: original, writable: true, configurable: true }); \
        out.push(run('abcdef', 4)); out.join(',');"
        ),
        Ok(Value::String("8,10,8,4,8".into()))
    );
}
#[test]
fn native_receiver_coercion_keeps_each_callback() {
    assert_eq!(
        eval(
            "var calls = 0; var object = { slice: String.prototype.slice, toString: function() { calls++; return 'abcdef'; } }; \
        function run(text, n) { var sum = 0; for (var i = 0; i < n; i++) { sum += text.slice(0, 2).length; } return sum; } \
        run(object, 5) * 10 + calls;"
        ),
        Ok(Value::Number(105.0))
    );
}
#[test]
fn temporary_callee_provenance_does_not_survive_a_join() {
    use super::TypedOp;
    let ops = [
        TypedOp::MoveBoxed { dst: 1, src: 0 },
        TypedOp::JumpIfFalsy { cond: 0, target: 2 },
        TypedOp::CallClosedFormLeaf {
            dst: 2,
            receiver: 0,
            callee: 1,
            args: [0; super::helper_graph::MAX_HELPER_ARITY],
            arity: 0,
        },
    ];
    assert!(super::pure_calls::guards(&ops, &[0], &[]).is_none());
}
