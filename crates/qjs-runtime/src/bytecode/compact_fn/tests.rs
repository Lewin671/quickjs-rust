use super::{CompactOp, compile};
use crate::bytecode::{compiler, ir::Bytecode, ir::Op};
use crate::{Value, eval};

/// Extracts one named nested function body from a script.
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

const CALL_TREE: &str = "function callTree(depth, value) {
    if (depth <= 0) { return value + 1; }
    var left = callTree(depth - 1, value);
    var right = callTree(depth - 1, value);
    return left + right - (value + 1);
}";

#[test]
fn the_recursive_sentinel_body_is_admitted_with_real_calls() {
    let program = compile::compile(&nested_function(CALL_TREE, "callTree"))
        .expect("the recursive sentinel body should be admitted");
    // Asserting the program is non-empty would pass for any body that compiles
    // at all. What matters is that the two recursive calls are represented as
    // register calls, because that is the operation the tier exists to run.
    let calls = program
        .ops
        .iter()
        .filter(|op| matches!(op, CompactOp::Call { argc: 2, .. }))
        .count();
    assert_eq!(calls, 2, "{:#?}", program.ops);
    assert!(
        program
            .ops
            .iter()
            .any(|op| matches!(op, CompactOp::LoadUpvalueLocal { .. })),
        "the self-reference must be read live through its upvalue cell: {:#?}",
        program.ops
    );
}

#[test]
fn the_recursive_sentinel_computes_the_same_results_as_the_interpreter() {
    // `callTree` returns `value + 1` for every depth, by induction over the
    // tree, so a wrong register assignment shows up as a wrong number rather
    // than a crash.
    for (depth, value, expected) in [
        (0.0, 7.0, 8.0),
        (1.0, 7.0, 8.0),
        (5.0, 3.0, 4.0),
        (10.0, 0.0, 1.0),
    ] {
        assert_eq!(
            eval(&format!("{CALL_TREE} callTree({depth}, {value});")),
            Ok(Value::Number(expected)),
            "depth {depth} value {value}"
        );
    }
}

#[test]
fn an_admitted_body_propagates_a_callee_error() {
    // The tier has no handler of its own, so an error raised beneath a
    // register call must surface unchanged at the outer boundary.
    let source = "function boom() { return null.x; }
        function outer(n) { var v = boom(); return v + n; }
        try { outer(1); } catch (e) { e instanceof TypeError; }";
    assert_eq!(eval(source), Ok(Value::Boolean(true)));
}

#[test]
fn a_non_numeric_binary_keeps_interpreter_semantics() {
    // String concatenation, `valueOf` dispatch, and their ordering all live in
    // the bridge rather than in the executor.
    let source = "function join(a, b) { return a + b; }
        var hook = { valueOf: function () { return 40; } };
        join('x', 'y') + '|' + join(hook, 2) + '|' + join(1, '2');";
    assert_eq!(eval(source), Ok(Value::String("xy|42|12".into())));
}

#[test]
fn a_body_with_a_loop_is_rejected() {
    // Backward edges are out of scope for this unit; admitting one would rely
    // on untested behaviour rather than on a bounded proof.
    let source =
        "function loops(n) { var t = 0; for (var i = 0; i < n; i++) { t = t + i; } return t; }";
    assert!(compile::compile(&nested_function(source, "loops")).is_none());
}

#[test]
fn a_body_with_a_handler_is_rejected() {
    let source = "function guarded(n) { try { return n + 1; } catch (e) { return 0; } }";
    assert!(compile::compile(&nested_function(source, "guarded")).is_none());
}

#[test]
fn a_body_that_creates_a_closure_is_rejected() {
    let source = "function maker(n) { var f = function () { return n; }; return f; }";
    assert!(compile::compile(&nested_function(source, "maker")).is_none());
}

#[test]
fn a_generator_body_is_rejected() {
    let source = "function* gen(n) { yield n; return n + 1; }";
    assert!(compile::compile(&nested_function(source, "gen")).is_none());
}

#[test]
fn a_body_reading_a_property_is_rejected() {
    // Property operations are not in the admitted set; this guards against the
    // opcode whitelist silently widening.
    let source = "function read(o) { return o.x; }";
    assert!(compile::compile(&nested_function(source, "read")).is_none());
}

#[test]
fn a_rejected_body_still_evaluates_correctly() {
    // The fallback is the whole safety story for every rejection above.
    let source =
        "function loops(n) { var t = 0; for (var i = 0; i < n; i++) { t = t + i; } return t; }
        loops(5);";
    assert_eq!(eval(source), Ok(Value::Number(10.0)));
}

#[test]
fn deep_admitted_recursion_matches_the_interpreter() {
    // Each activation still builds its own nested `Vm`, so recursion depth is
    // still bounded by the Rust stack. The bound is nonetheless much deeper
    // than the ordinary path's: measured on this test's thread, the generic
    // interpreter overflows past depth 40 while the compact tier reaches 250,
    // because its activation frame is a fraction of
    // `run_current_activation`'s ~4.3 KB. 200 is chosen to exercise real depth
    // while staying inside the smaller stack a test thread gets.
    let source = "function down(n) { if (n <= 0) { return 0; } return down(n - 1) + 1; }
        down(200);";
    assert_eq!(eval(source), Ok(Value::Number(200.0)));
}

#[test]
fn a_discarded_register_does_not_outlive_its_pop() {
    // `Op::Pop` must lower to `Drop`, not merely to a compile-time depth
    // change: a register holding the last reference has to be released where
    // the source says so.
    let program = compile::compile(&nested_function(CALL_TREE, "callTree"))
        .expect("the recursive sentinel body should be admitted");
    assert!(
        program
            .ops
            .iter()
            .any(|op| matches!(op, CompactOp::Drop { .. })),
        "{:#?}",
        program.ops
    );
}

#[test]
fn a_body_reading_a_lexically_declared_local_is_rejected() {
    // Parameters and hoisted `var`s hold a value on entry; a `let` does not,
    // so admitting one would require reproducing temporal-dead-zone
    // diagnostics inside the tier. This is deliberately conservative -- the
    // body below could never observe the dead zone -- and the point of the
    // test is that widening admission must be a decision, not an accident.
    let source = "function lexical(n) { let x = n + 1; return x; }";
    assert!(compile::compile(&nested_function(source, "lexical")).is_none());
}

#[test]
fn a_rejected_lexical_body_still_evaluates_correctly() {
    let source = "function lexical(n) { let x = n + 1; return x; } lexical(41);";
    assert_eq!(eval(source), Ok(Value::Number(42.0)));
}

#[test]
fn an_admitted_body_calling_a_native_keeps_its_semantics() {
    // A native callee is not direct-leaf, so it leaves the register dispatch
    // and goes through `call_function`. That is a different entry from the one
    // the ordinary interpreter would have used, so it needs its own coverage.
    let source = "function useNative(a, b) { return Math.max(a, b); }
        useNative(3, 9) + Math.min(1, 2);";
    assert_eq!(eval(source), Ok(Value::Number(10.0)));
}

#[test]
fn an_admitted_body_calling_a_bound_function_keeps_its_receiver() {
    let source = "function target(x) { return this.base + x; }
        var bound = target.bind({ base: 100 });
        function useBound(n) { return bound(n); }
        useBound(5);";
    assert_eq!(eval(source), Ok(Value::Number(105.0)));
}

#[test]
fn an_admitted_body_calling_a_proxy_keeps_its_trap() {
    let source = "var proxied = new Proxy(function (x) { return x + 1; }, {
            apply: function (target, thisArg, args) { return args[0] * 10; }
        });
        function useProxy(n) { return proxied(n); }
        useProxy(7);";
    assert_eq!(eval(source), Ok(Value::Number(70.0)));
}

#[test]
fn an_admitted_body_calling_a_class_constructor_still_throws() {
    let source = "class C {}
        function useClass(n) { return C(n); }
        try { useClass(1); 'no throw'; } catch (e) { e instanceof TypeError; }";
    assert_eq!(eval(source), Ok(Value::Boolean(true)));
}

#[test]
fn an_admitted_body_sees_a_replaced_self_binding() {
    // The self-reference is read live from its cell on every call, so
    // reassigning the outer binding must be observed rather than baked in.
    let source = "function counter(n) { if (n <= 0) { return 0; } return counter(n - 1) + 1; }
        var first = counter(3);
        counter = function () { return 100; };
        first + counter(3);";
    assert_eq!(eval(source), Ok(Value::Number(103.0)));
}

#[test]
fn a_callee_with_a_home_object_does_not_share_the_caller_environment() {
    // A method carries a home object, so `direct_leaf_function_env` would
    // install a private environment its caller's environment does not have.
    // Sharing would silently drop that, so the callee must fall back.
    let source = "class Base { #secret = 7; reveal() { return this.#secret; } }
        var instance = new Base();
        function useMethod(n) { return instance.reveal() + n; }
        useMethod(35);";
    assert_eq!(eval(source), Ok(Value::Number(42.0)));
}

#[test]
fn a_shared_environment_callee_still_resolves_globals_through_the_realm() {
    // The shared environment must remain a real realm view: a callee reached
    // through the fast path resolves free names exactly as it would have.
    let source = "var scale = 3;
        function inner(n) { return n * scale; }
        function outer(n) { return inner(n) + inner(n); }
        outer(7);";
    assert_eq!(eval(source), Ok(Value::Number(42.0)));
}

#[test]
fn a_shared_environment_callee_sees_a_global_written_between_calls() {
    // Sharing an environment must not freeze what it resolves to.
    let source = "var offset = 1;
        function reader(n) { return n + offset; }
        function driver(n) { var first = reader(n); offset = 100; return first + reader(n); }
        driver(0);";
    assert_eq!(eval(source), Ok(Value::Number(101.0)));
}

#[test]
fn errors_from_a_shared_environment_callee_keep_their_thrown_value() {
    let source = "var sentinel = { tag: 'boom' };
        function thrower(n) { return n.missing.deeper; }
        function driver(n) { return thrower(n); }
        try { driver(null); 'no throw'; } catch (e) { e instanceof TypeError; }";
    assert_eq!(eval(source), Ok(Value::Boolean(true)));
}

#[test]
fn a_captured_binding_read_before_initialization_still_throws() {
    // Whether a received cell is initialized is a fact about the *caller's*
    // execution, so no admission narrowing can rule it out: this closure is
    // called before the `let` it captures is reached. Regression test for the
    // 12 Test262 `closure-get-before-initialization` cases the VM-free
    // rewrite broke by dropping the marker check.
    let source = "(function () {
            function f() { return x + 1; }
            var threw = false;
            try { f(); } catch (e) { threw = e instanceof ReferenceError; }
            let x;
            return threw;
        }());";
    assert_eq!(eval(source), Ok(Value::Boolean(true)));
}

#[test]
fn a_captured_binding_read_after_initialization_still_reads() {
    let source = "(function () {
            function f() { return x + 1; }
            let x = 41;
            return f();
        }());";
    assert_eq!(eval(source), Ok(Value::Number(42.0)));
}

#[test]
fn a_captured_const_read_before_initialization_still_throws() {
    let source = "(function () {
            function f() { return c; }
            var threw = false;
            try { f(); } catch (e) { threw = e instanceof ReferenceError; }
            const c = 1;
            return threw;
        }());";
    assert_eq!(eval(source), Ok(Value::Boolean(true)));
}
