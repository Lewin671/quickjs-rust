use super::{WideOp, compile};
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

fn value_of(source: &str) -> Value {
    eval(source).expect("source must evaluate")
}

fn error_of(source: &str) -> String {
    eval(source).expect_err("source must fail").message
}

const ITEM_CHECK: &str = "function TreeNode(left, right, item) {
    this.left = left; this.right = right; this.item = item;
}
function itemCheck(node) {
    if (node.left == null) return node.item;
    return node.item + itemCheck(node.left) - itemCheck(node.right);
}";

#[test]
fn a_method_body_reading_and_writing_this_is_admitted() {
    let program = compile::compile(&nested_function(ITEM_CHECK, "TreeNode"))
        .expect("a constructor body writing this should be admitted");
    let writes = program
        .ops
        .iter()
        .filter(|op| matches!(op, WideOp::SetPropNamed { .. }))
        .count();
    assert_eq!(writes, 3, "{:#?}", program.ops);
    assert!(program.requires_this);
    let program = compile::compile(&nested_function(ITEM_CHECK, "itemCheck"))
        .expect("a body reading named properties of a parameter should be admitted");
    assert!(
        program
            .ops
            .iter()
            .any(|op| matches!(op, WideOp::GetPropNamed { .. })),
        "{:#?}",
        program.ops
    );
    assert!(!program.requires_this);
}

#[test]
fn a_body_the_numeric_tier_admits_also_compiles_here() {
    let program = compile::compile(&nested_function(
        "function fib(n) { return n < 2 ? n : fib(n - 1) + fib(n - 2); }",
        "fib",
    ))
    .expect("a numeric body is a wide body too");
    assert!(
        program
            .ops
            .iter()
            .any(|op| matches!(op, WideOp::Call { .. }))
    );
}

#[test]
fn constructed_trees_check_the_same_as_the_interpreter() {
    assert_eq!(
        value_of(
            "function TreeNode(left, right, item) { this.left = left; this.right = right; this.item = item; }
             function bottomUpTree(item, depth) {
               if (depth > 0) return new TreeNode(bottomUpTree(2 * item - 1, depth - 1), bottomUpTree(2 * item, depth - 1), item);
               return new TreeNode(null, null, item);
             }
             function itemCheck(node) {
               if (node.left == null) return node.item;
               return node.item + itemCheck(node.left) - itemCheck(node.right);
             }
             itemCheck(bottomUpTree(0, 6));"
        ),
        Value::Number(-1.0)
    );
}

#[test]
fn prototype_methods_receive_their_receiver_and_see_writes() {
    assert_eq!(
        value_of(
            "function Counter(start) { this.n = start; }
             Counter.prototype.step = function (by) { var before = this.n; this.n = before + by; return before; };
             Counter.prototype.twice = function (by) { return this.step(by) + this.step(by); };
             var c = new Counter(10);
             var seen = c.twice(3);
             seen * 1000 + c.n;"
        ),
        Value::Number(23_016.0)
    );
}

#[test]
fn a_sloppy_method_called_plainly_sees_the_global_receiver() {
    assert_eq!(
        value_of(
            "globalThis.marker = 7;
             function readMarker() { return this.marker; }
             function viaPlainCall() { return readMarker(); }
             viaPlainCall();"
        ),
        Value::Number(7.0)
    );
    assert_eq!(
        value_of(
            "function strictThis() { 'use strict'; return this === undefined; }
             function viaPlainCall() { return strictThis(); }
             viaPlainCall();"
        ),
        Value::Boolean(true)
    );
}

#[test]
fn a_primitive_receiver_is_boxed_for_a_sloppy_method_and_kept_for_a_strict_one() {
    assert_eq!(
        value_of(
            "Number.prototype.kind = function () { return typeof this; };
             Number.prototype.strictKind = function () { 'use strict'; return typeof this; };
             function probe(n) { return n.kind() + ',' + n.strictKind(); }
             probe(3) === 'object,number';"
        ),
        Value::Boolean(true)
    );
}

#[test]
fn a_loop_the_typed_tier_claims_keeps_the_interpreter() {
    // The typed loop tier compiles a program for this property-walking loop,
    // so the wide tier must leave the body to the interpreter, where that
    // program runs at the backedge.
    assert!(
        compile::compile(&nested_function(
            "function walk(node) { var sum = 0; while (node) { sum = sum + node.item; node = node.next; } return sum; }",
            "walk",
        ))
        .is_none()
    );
    assert_eq!(
        value_of(
            "function walk(node) { var sum = 0; while (node) { sum = sum + node.item; node = node.next; } return sum; }
             var list = null;
             for (var i = 1; i <= 10; i++) list = { item: i, next: list };
             walk(list);"
        ),
        Value::Number(55.0)
    );
}

#[test]
fn a_body_with_a_counted_numeric_loop_keeps_the_interpreter_and_its_accelerators() {
    assert!(
        compile::compile(&nested_function(
            "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s = s + i; } return s; }",
            "sum",
        ))
        .is_none(),
        "a loop a frame-based accelerator claims must not be taken away from it"
    );
    assert_eq!(
        value_of(
            "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s = s + i; } return s; } sum(100);"
        ),
        Value::Number(4950.0)
    );
}

#[test]
fn a_getter_setter_and_proxy_keep_their_hooks() {
    assert_eq!(
        value_of(
            "var log = [];
             var o = { get x() { log.push('get'); return 5; }, set x(v) { log.push('set' + v); } };
             function touch(obj) { var v = obj.x; obj.x = v + 1; return v; }
             var r = touch(o);
             r === 5 && log.join() === 'get,set6';"
        ),
        Value::Boolean(true)
    );
    assert_eq!(
        value_of(
            "var p = new Proxy({}, { get: function (t, k) { return k + '!'; } });
             function read(obj) { return obj.name; }
             read(p) === 'name!';"
        ),
        Value::Boolean(true)
    );
}

#[test]
fn a_strict_write_to_a_read_only_property_throws_and_unwinds() {
    assert!(
        error_of(
            "'use strict';
             var frozen = Object.freeze({ v: 1 });
             function write(obj) { obj.v = 2; return obj.v; }
             function outer(obj) { return write(obj) + 1; }
             outer(frozen);"
        )
        .contains("TypeError")
    );
    // The same call afterwards must run cleanly: the register stack unwound.
    assert_eq!(
        value_of(
            "var frozen = Object.freeze({ v: 1 });
             function write(obj) { obj.v = 2; return obj.v; }
             function outer(obj) { return write(obj) + 1; }
             outer(frozen);"
        ),
        Value::Number(2.0)
    );
}

#[test]
fn a_thrown_error_in_a_nested_method_reaches_the_interpreter_catch() {
    assert_eq!(
        value_of(
            "function Box(v) { this.v = v; }
             Box.prototype.check = function () { if (this.v < 0) throw new RangeError('neg'); return this.v; };
             function total(a, b) { return a.check() + b.check(); }
             var msg = '';
             try { total(new Box(1), new Box(-1)); } catch (e) { msg = e.message; }
             msg === 'neg' && total(new Box(2), new Box(3)) === 5;"
        ),
        Value::Boolean(true)
    );
}

#[test]
fn deep_method_recursion_reports_a_range_error() {
    // Method recursion through resolved calls stays on this tier's frame
    // stack, so its depth is bounded by `MAX_FRAMES` and reported as a
    // catchable RangeError rather than by the native stack.
    assert!(
        error_of(
            "function Node(d) { this.d = d; }
             Node.prototype.dive = function (n) { return this.dive(n + 1) + 1; };
             new Node(0).dive(0);"
        )
        .contains("RangeError")
    );
}

#[test]
fn undefined_and_null_receivers_throw_the_ordinary_type_error() {
    assert!(error_of("function read(o) { return o.x; } read(undefined);").contains("TypeError"));
    assert_eq!(
        value_of(
            "function write(o) { o.x = 1; }
             var caught = null;
             try { write(null); } catch (e) { caught = e; }
             caught instanceof TypeError;"
        ),
        Value::Boolean(true)
    );
}
