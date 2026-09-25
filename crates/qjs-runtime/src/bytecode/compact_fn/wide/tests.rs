use qjs_ast::BinaryOp;

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
        .filter(|op| matches!(op, WideOp::SetPropNamed { .. } | WideOp::SetPropThis { .. }))
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

/// Whether every backward edge of the program exits to the interpreter first:
/// the only backward jumps are the probed ones, each right after its exit.
fn backward_edges_exit(program: &super::WideProgram) -> bool {
    let has_exit = program
        .ops
        .iter()
        .any(|op| matches!(op, WideOp::Exit { .. }));
    let unprobed_backward_jump = program.ops.iter().enumerate().any(|(index, op)| {
        let backward = matches!(op, WideOp::Jump { target } | WideOp::JumpIfFalsy { target, .. }
            | WideOp::JumpIfTruthy { target, .. } if (*target as usize) <= index);
        backward
            && (index == 0
                || !matches!(program.ops[index - 1], WideOp::Exit { .. })
                || !matches!(op, WideOp::Jump { .. }))
    });
    has_exit && !unprobed_backward_jump
}

#[test]
fn a_loop_the_typed_tier_claims_exits_to_the_interpreter_at_its_backedge() {
    // The typed loop tier compiles a program for this property-walking loop,
    // so the wide tier runs the body only up to the loop and hands the loop
    // to the interpreter, where that program runs at the backedge.
    let source = "function walk(node) { var sum = 0; while (node) { sum = sum + node.item; node = node.next; } return sum; }";
    let program = compile::compile(&nested_function(source, "walk"))
        .expect("a body whose loop an accelerator claims should be admitted with an exit");
    assert!(backward_edges_exit(&program), "{:#?}", program.ops);
    assert_eq!(
        value_of(&format!(
            "{source}
             var list = null;
             for (var i = 1; i <= 10; i++) list = {{ item: i, next: list }};
             walk(list) + walk(null);"
        )),
        Value::Number(55.0)
    );
}

#[test]
fn a_body_with_a_counted_numeric_loop_hands_the_loop_to_its_accelerators() {
    let source =
        "function sum(n) { var s = 0; for (var i = 0; i < n; i++) { s = s + i; } return s; }";
    let program = compile::compile(&nested_function(source, "sum"))
        .expect("the body should be admitted with its loop left to the interpreter");
    assert!(
        backward_edges_exit(&program),
        "a loop a frame-based accelerator claims must not be taken away from it: {:#?}",
        program.ops
    );
    assert_eq!(
        value_of(&format!("{source} sum(100) + sum(0);")),
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

#[test]
fn lexical_locals_are_admitted_behind_their_dead_zone_marker() {
    let program = compile::compile(&nested_function(
        "function lex(a) { const x = a + 1; let y = x * 2; { let z = y; y = z + 1; } return y; }",
        "lex",
    ))
    .expect("a body with let/const locals should be admitted");
    assert!(
        program
            .ops
            .iter()
            .any(|op| matches!(op, WideOp::ClearLocal { .. })),
        "{:#?}",
        program.ops
    );
    assert!(!program.lexical_slots.is_empty());
    assert_eq!(
        value_of(
            "function lex(a) { const x = a + 1; let y = x * 2; { let z = y; y = z + 1; } return y; }
             lex(1) * 100 + lex(2);"
        ),
        Value::Number(507.0)
    );
}

#[test]
fn a_dead_zone_read_and_a_const_assignment_keep_their_errors() {
    assert_eq!(
        value_of(
            "function tdz(a) { var r; try { r = q; } catch (e) { r = e.constructor.name; } let q = a; return r + ':' + q; }
             tdz(5) === 'ReferenceError:5';"
        ),
        Value::Boolean(true)
    );
    assert_eq!(
        value_of(
            "function ce(a) { const c = a; try { c = 2; } catch (e) { return e.constructor.name; } return c; }
             ce(1) === 'TypeError';"
        ),
        Value::Boolean(true)
    );
    assert_eq!(
        value_of(
            "function shadow() { let a = 1; { let a = 2; } return a; }
             function loopc(n) { let s = 0; for (let i = 0; i < n; i++) { const d = i * 2; s = s + d; } return s; }
             shadow() * 100 + loopc(5);"
        ),
        Value::Number(120.0)
    );
}

#[test]
fn global_reads_resolve_like_the_interpreter() {
    assert_eq!(
        value_of("var G = 41; function g() { return G + 1; } g();"),
        Value::Number(42.0)
    );
    assert_eq!(
        value_of(
            "Object.defineProperty(globalThis, 'acc', { get: function () { return 9; } });
             function ga() { return acc + 1; } ga();"
        ),
        Value::Number(10.0)
    );
    assert_eq!(
        value_of(
            "function ug() { try { return undefinedName; } catch (e) { return e.constructor.name; } }
             ug() === 'ReferenceError';"
        ),
        Value::Boolean(true)
    );
    assert_eq!(
        value_of(
            "let counter = 0;
             function bump() { return counter + 1; }
             var first = bump();
             counter = 5;
             first * 10 + bump();"
        ),
        Value::Number(16.0)
    );
}

#[test]
fn construction_array_literals_and_indexed_reads_inside_a_body() {
    assert_eq!(
        value_of(
            "function P(v) { this.v = v; }
             function mk(n) { var p = new P(n); return p.v * 2; }
             function arr(a, b) { var t = [a, b, a + b]; return t[2] + t.length; }
             function idx(v) { return v[0] * v[1]; }
             mk(4) * 10000 + arr(1, 2) * 100 + idx([3, 4]);"
        ),
        Value::Number(80_612.0)
    );
    assert!(error_of("function mk() { return new 3(); } mk();").contains("TypeError"));
    assert!(error_of("function idx(v) { return v[0]; } idx(null);").contains("TypeError"));
}

#[test]
fn a_body_whose_lowering_keeps_a_literal_in_slots_keeps_the_interpreter() {
    assert!(
        compile::compile(&nested_function(
            "function run(n) { var sum = 0; for (var i = 0; i < n; i++) { var values = [1, 2, 3]; sum += values[2]; } return sum; }",
            "run",
        ))
        .is_none()
    );
}

#[test]
fn a_body_with_a_throw_statement_is_admitted_and_throws_the_same_value() {
    let source = "function hash(key) {
        switch (typeof key) {
        case 'number': return key | 0;
        case 'boolean': return key ? 1 : 0;
        default: throw new Error('bad key');
        }
    }";
    let program = compile::compile(&nested_function(source, "hash"))
        .expect("a body whose only unusual operation is a throw should be admitted");
    assert!(
        program
            .ops
            .iter()
            .any(|op| matches!(op, WideOp::Throw { .. })),
        "{:#?}",
        program.ops
    );
    // The thrown value keeps its identity through the tier and reaches the
    // interpreter's handler; an uncaught throw reports like the interpreter.
    assert_eq!(
        value_of(
            "var token = { tag: 1 };
             function raise(x) { if (x) throw token; return 7; }
             function outer(x) { return raise(x) + 1; }
             var caught;
             try { outer(true); } catch (e) { caught = e; }
             caught === token && outer(false) === 8;"
        ),
        Value::Boolean(true)
    );
    assert!(error_of("function f(x) { if (x) throw 'boom'; return 1; } f(1);").contains("boom"));
}

#[test]
fn equality_without_a_frame_matches_the_general_path() {
    // Loose and strict equality inside an admitted body, including the
    // operand pairs that still need the general path (an object operand).
    assert_eq!(
        value_of(
            "function eq(a, b) { return [a == b, a != b, a === b, a !== b].join(); }
             var hint = { valueOf: function () { return 'x'; } };
             [eq('x', 'x'), eq('x', 'y'), eq(null, undefined), eq(undefined, undefined),
              eq(true, true), eq(true, false), eq(hint, 'x'), eq(1, '1'),
              eq('\\uD83D' + '\\uDE00', '\\uD83D\\uDE00')].join('|');"
        ),
        Value::String(
            "true,false,true,false|false,true,false,true|true,false,false,true|\
             true,false,true,false|true,false,true,false|false,true,false,true|\
             true,false,false,true|true,false,false,true|true,false,true,false"
                .into()
        )
    );
}

#[test]
fn an_exit_mid_expression_hands_over_locals_and_operands() {
    // `SetProp` is left to the interpreter; the exit happens with the
    // receiver, key and value on the operand stack and a lexical still in its
    // dead zone, and the interpreter continues with every local intact.
    let source = "function fill(target, key, a, b) {
        var before = a * 2;
        let later;
        target[key] = before + b;
        later = target[key] + 1;
        return [before, later, target[key], typeof a].join();
    }";
    let program = compile::compile(&nested_function(source, "fill"))
        .expect("a computed store should exit, not decline the body");
    assert!(
        program
            .ops
            .iter()
            .any(|op| matches!(op, WideOp::Exit { depth, .. } if *depth >= 3)),
        "{:#?}",
        program.ops
    );
    assert_eq!(
        value_of(&format!(
            "{source} fill({{}}, 'x', 3, 4) + '|' + fill([], 0, 1, 1);"
        )),
        Value::String("6,11,10,number|2,4,3,number".into())
    );
    // A dead-zone read after the exit still throws the interpreter's error.
    assert!(
        error_of("function early(o) { o['k'] = 1; let late = late; return late; } early({});")
            .contains("ReferenceError")
    );
}

#[test]
fn an_exit_in_a_nested_activation_returns_to_its_caller() {
    assert_eq!(
        value_of(
            "function store(o, k, v) { o[k] = v; return o[k] * 2; }
             function outer(o) { var first = store(o, 'a', 5); return first + store(o, 'b', 7); }
             var o = {}; outer(o) + o.a + o.b;"
        ),
        Value::Number(36.0)
    );
}

#[test]
fn code_past_an_exit_sees_the_receiver_parameters_and_handlers() {
    assert_eq!(
        value_of(
            "function Box(v) { this.v = v; }
             Box.prototype.put = function (o, k, unusedUntilLater) {
                 o[k] = 1;
                 return this.v + unusedUntilLater;
             };
             function risky(o, k) { o[k] = 1; throw new TypeError('after exit'); }
             var caught = '';
             try { risky({}, 'z'); } catch (e) { caught = e.message; }
             new Box(40).put({}, 'x', 2) + '|' + caught;"
        ),
        Value::String("42|after exit".into())
    );
}

#[test]
fn an_update_of_a_local_reads_and_writes_the_local_in_place() {
    // `s += ','` compiles to its own fused append, so the separator is a
    // parameter here.
    let source = "function join(parts, sep) {
        var s = '';
        for (var i = 0; i < parts.length; i++) { if (i) s += sep; s += parts[i]; s = s + '.'; }
        return s;
    }";
    let program =
        compile::compile(&nested_function(source, "join")).expect("the body should be admitted");
    let in_place = program
        .ops
        .iter()
        .filter(|op| matches!(op, WideOp::Binary { dst, left, .. } if dst == left && *dst < program.local_registers))
        .count();
    assert_eq!(in_place, 3, "{:#?}", program.ops);
    assert_eq!(
        value_of(&format!(
            "{source} join(['a', 'b', 'c'], ',') + '|' + join([], ',');"
        )),
        Value::String("a.,b.,c.|".into())
    );
}

#[test]
fn an_in_place_update_keeps_the_operand_order_of_the_general_path() {
    assert_eq!(
        value_of(
            "function twice(s) { s = s + s; return s; }
             function reads(s, o) { s = s + o.v; return s; }
             function writes(s) { s = s + (s = 'x'); return s; }
             function calls(n) { var t = 1; n = n * g(); return n + t; }
             function g() { return 3; }
             function throws(s) { try { s = s + boom(); } catch (e) { return s; } }
             function boom() { throw 1; }
             function nested(acc, t) { acc = (acc + g() + t) | 0; return acc; }
             [twice('ab'), reads('a', { v: 'b' }), writes('y'), calls(2), throws('kept'),
              nested(10, 5)].join(',');"
        ),
        Value::String("abab,ab,yx,7,kept,18".into())
    );
}

#[test]
fn a_guarded_math_call_runs_as_a_method_call_without_an_exit() {
    let source = "function pick(n) { var s = 0; for (var i = 0; i < n; i++) s += Math.floor(i / 2); return s; }";
    let program = compile::compile(&nested_function(source, "pick"))
        .expect("a body with a guarded Math call should be admitted");
    assert!(
        program
            .ops
            .iter()
            .any(|op| matches!(op, WideOp::CallResolved { argc: 1, .. })),
        "{:#?}",
        program.ops
    );
    // Only the loop's probes exit: its backedge, and its header entry.
    assert!(
        !program
            .ops
            .iter()
            .enumerate()
            .any(|(pc, op)| matches!(op, WideOp::Exit { ip, .. }
            if !matches!(nested_function(source, "pick").code[*ip as usize], Op::Jump(_))
                && program.probed_header(pc + 1).is_none())),
        "{:#?}",
        program.ops
    );
    assert_eq!(
        value_of(&format!("{source} pick(10);")),
        Value::Number(20.0)
    );
    // A replaced `Math` or a non-number argument takes the ordinary call.
    assert_eq!(
        value_of(
            "function f(x) { return Math.floor(x); }
             var a = f(2.5) + f('7.9');
             var saved = Math; Math = { floor(v) { return this === Math ? v * 10 : -1; } };
             var b = f(3); Math = saved; a + ':' + b;"
        ),
        Value::String("9:30".into())
    );
}

#[test]
fn a_discarded_update_or_compound_assignment_of_a_local_is_one_operation() {
    // A body with no exit: its statement completion values are dead, so the
    // statement forms fold as well as the discarded ones.
    let source = "function step(x, o) { if (o) { x++; ++x; x--; x += o.v; if (x > 3) { x -= 1; } } return x; }";
    let program =
        compile::compile(&nested_function(source, "step")).expect("the body should be admitted");
    let in_place = program
        .ops
        .iter()
        .filter(|op| {
            matches!(op, WideOp::Update { dst, .. } | WideOp::Binary { dst, .. }
                if *dst < program.local_registers)
        })
        .count();
    assert_eq!(in_place, 5, "{:#?}", program.ops);
    assert!(
        !program
            .ops
            .iter()
            .any(|op| matches!(op, WideOp::ToNumeric { .. } | WideOp::Dup { .. })),
        "{:#?}",
        program.ops
    );
    assert_eq!(
        value_of(&format!(
            "{source} step(1, {{ v: 5 }}) + ':' + step(0, {{ v: 0 }});"
        )),
        Value::String("6:1".into())
    );
}

#[test]
fn a_folded_update_converts_its_local_once_like_the_general_path() {
    assert_eq!(
        value_of(
            "function bump(x) { x++; return x; }
             function drop(x) { --x; return x; }
             function add(x) { x += 1; return x; }
             var log = [];
             var o = { valueOf() { log.push('v'); return 4; } };
             [bump('5'), drop('5'), add('5'), bump(o), log.length, bump(10n), typeof bump(null)].join(',');"
        ),
        Value::String("6,4,51,5,1,11,number".into())
    );
}

#[test]
fn a_per_iteration_let_loop_without_closures_is_admitted() {
    let source = "function sum(xs) { var t = 0; for (let i = 0; i < xs.length; i++) { let x = xs[i]; t += x * i; } return t; }";
    compile::compile(&nested_function(source, "sum"))
        .expect("a let loop whose bindings no closure captures should be admitted");
    assert_eq!(
        value_of(&format!("{source} sum([1, 2, 3]);")),
        Value::Number(8.0)
    );
    // Captured per-iteration bindings keep one cell per iteration.
    let captured = "function make(n) { var fs = []; for (let i = 0; i < n; i++) fs.push(() => i); return fs; }";
    assert!(compile::compile(&nested_function(captured, "make")).is_none());
    assert_eq!(
        value_of(&format!("{captured} make(3).map(f => f()).join(',');")),
        Value::String("0,1,2".into())
    );
}

#[test]
fn a_declaration_without_an_initializer_leaves_the_stack_balanced() {
    // `var x;` in a loop body used to leave an `undefined` behind on every
    // iteration, so the loop header's stack depth disagreed with its
    // backedge and the body was declined.
    let source = "function f(n) { var t = 0; for (var i = 0; i < n; i++) { var x; let y; x = i; y = x; t += y; } return t; }";
    compile::compile(&nested_function(source, "f"))
        .expect("a body whose loop declares uninitialized bindings should be admitted");
    assert_eq!(value_of(&format!("{source} f(5);")), Value::Number(10.0));
}

#[test]
fn primitive_string_reads_answer_like_the_interpreter() {
    assert_eq!(
        value_of(
            "function probe(s, i) { return [s.length, s[i], s.charCodeAt(i), s.missing, s[9], s[-1], s['0']].join('|'); }
             var before = probe('abc', 1);
             String.prototype.missing = 'patched';
             before + '/' + probe('xy\\u00e9', 2);"
        ),
        Value::String("3|b|98||||a/3|é|233|patched|||x".into())
    );
}

#[test]
fn a_remembered_property_creation_still_meets_later_setters_and_locks() {
    assert_eq!(
        value_of(
            "function Point(x) { this.x = x; }
             var made = [];
             for (var i = 0; i < 5; i++) made.push(new Point(i).x);
             var log = [];
             Object.defineProperty(Point.prototype, 'x', {
                 set(v) { log.push('proto:' + v); }, get() { return 'accessor'; }, configurable: true });
             var a = new Point(7);
             delete Point.prototype.x;
             var b = new Point(8);
             Object.defineProperty(Object.prototype, 'x', {
                 set(v) { log.push('object:' + v); }, configurable: true });
             var c = new Point(9);
             delete Object.prototype.x;
             Object.defineProperty(Point.prototype, 'x', { value: 1, writable: false, configurable: true });
             var d = new Point(10);
             delete Point.prototype.x;
             Object.setPrototypeOf(Point.prototype, { set x(v) { log.push('swapped:' + v); } });
             var e = new Point(11);
             [made.join(''), a.x, b.x, c.hasOwnProperty('x'), d.x, d.hasOwnProperty('x'),
              e.hasOwnProperty('x'), log.join(',')].join('|');"
        ),
        Value::String("01234||8|false||false|false|proto:7,object:9,swapped:11".into())
    );
}

#[test]
fn a_constructor_entered_on_the_wide_driver_builds_like_the_general_path() {
    assert_eq!(
        value_of(
            "function Node(l, r) { this.l = l; this.r = r; }
             function Boxed(v) { this.v = v; return { wrapped: v }; }
             function Prim(v) { this.v = v; return 7; }
             function Odd() { this.p = Object.getPrototypeOf(this) === Odd.prototype; }
             function Thrower(v) { this.v = v; if (v) throw new Error('bad'); }
             Odd.prototype = 5;
             function build(d) { return d ? new Node(build(d - 1), build(d - 1)) : new Node(null, null); }
             function count(n) { return n.l ? 1 + count(n.l) + count(n.r) : 1; }
             var caught = '';
             try { new Thrower(1); } catch (e) { caught = e.message; }
             [count(build(4)), new Boxed(3).wrapped, new Prim(4).v, new Odd().p,
              new Node(1).r, new Node(1, 2, 3).r, caught, new Thrower(0).v].join(',');"
        ),
        Value::String("31,3,4,false,,2,bad,0".into())
    );
}

#[test]
fn a_plain_computed_store_continues_on_the_tier_and_others_keep_their_semantics() {
    assert_eq!(
        value_of(
            "function put(t, k, v) { t[k] = v; return t[k]; }
             var holes = new Array(3);
             var log = [];
             var withSetter = Object.create({ set s(v) { log.push(v); } });
             var frozen = Object.freeze({ f: 1 });
             function strictPut(t, k, v) { 'use strict'; t[k] = v; }
             var caught = '';
             try { strictPut(frozen, 'f', 2); } catch (e) { caught = e.constructor.name; }
             [put([1, 2], 1, 9), put(holes, 2, 'h') + holes.length, put({}, 'k', 3),
              put({ k: 1 }, 'k', 4), put(withSetter, 's', 5), put(frozen, 'f', 6),
              put(globalThis, 'gw', 7) + gw, log.join(''), caught].join(',');"
        ),
        Value::String("9,h3,3,4,,1,14,5,TypeError".into())
    );
}

#[test]
fn an_object_literal_built_at_its_exit_matches_the_interpreter() {
    assert_eq!(
        value_of(
            "Object.prototype.inherited = 'proto';
             function make(a, b) { var o = { a: a, b: b }; var p = { x: a, y: b, z: a + b }; return [o, p]; }
             function withMethod(v) { return { v: v, m() { return this.v + super.inherited; } }; }
             var total = 0;
             for (var i = 0; i < 100; i++) { var pair = make(i, 1); total += pair[0].a + pair[1].z; }
             var o = make(2, 3)[0];
             [total, Object.keys(o).join(''), o.inherited, withMethod(4).m(),
              Object.getPrototypeOf(o) === Object.prototype].join(',');"
        ),
        Value::String("10000,ab,proto,4proto,true".into())
    );
}

#[test]
fn a_constant_index_store_at_its_exit_matches_the_interpreter() {
    assert_eq!(
        value_of(
            "function first(a, v) { a[0] = v; return a[0]; }
             var typed = new Int8Array(2);
             var frozen = Object.freeze([1]);
             var obj = {};
             var hole = [];
             [first([5], 6), first(typed, 300), first(frozen, 9), first(obj, 'o'), obj[0],
              first(hole, 'h') + hole.length].join(',');"
        ),
        Value::String("6,44,1,o,o,h1".into())
    );
}

#[test]
fn a_named_read_of_this_borrows_the_receiver() {
    let source = "function Pt(x) { this.x = x; }
         Pt.prototype.get = function (o, c) { return this.x + (c ? this : o).x; };";
    let program = compile::compile(&nested_function(
        "function get(o, c) { return this.x + (c ? this : o).x; }",
        "get",
    ))
    .expect("a method reading this should be admitted");
    assert_eq!(
        program
            .ops
            .iter()
            .filter(|op| matches!(op, WideOp::GetPropThis { .. }))
            .count(),
        1,
        "{:#?}",
        program.ops
    );
    assert_eq!(
        value_of(&format!(
            "{source}
             var p = new Pt(1), q = new Pt(10);
             function s() {{ 'use strict'; return this === undefined ? 'u' : this.length; }}
             function loose() {{ return this.length; }}
             var withGetter = Object.create({{ get x() {{ return 5; }} }}, {{ get: {{ value: Pt.prototype.get }} }});
             [p.get(q, false), p.get(q, true), withGetter.get(q, true), s.call('abc'), s(),
              loose.call('abcd')].join(',');"
        )),
        Value::String("11,2,10,3,u,4".into())
    );
}

#[test]
fn a_named_write_to_this_borrows_the_receiver() {
    let source = "function Pt(x, y) { this.x = x; this.y = this.x + y; this.z = (this.x = 5); }";
    let program =
        compile::compile(&nested_function(source, "Pt")).expect("a constructor should be admitted");
    assert_eq!(
        program
            .ops
            .iter()
            .filter(|op| matches!(op, WideOp::SetPropThis { .. }))
            .count(),
        4,
        "{:#?}",
        program.ops
    );
    assert_eq!(
        value_of(&format!(
            "{source}
             var p = new Pt(1, 2);
             var log = [];
             function Watched() {{ this.w = 1; }}
             Object.defineProperty(Watched.prototype, 'w', {{ set(v) {{ log.push(v); }} }});
             new Watched();
             [p.x, p.y, p.z, log.join('')].join(',');"
        )),
        Value::String("5,3,5,1".into())
    );
}

#[test]
fn a_method_lookup_reads_the_receiver_in_place() {
    let source =
        "function call(o, s, n) { return o.m(1) + s.charAt(1) + n.toFixed(1) + o.m.call(o, 2); }";
    let program =
        compile::compile(&nested_function(source, "call")).expect("the body should be admitted");
    assert!(
        !program
            .ops
            .iter()
            .any(|op| matches!(op, WideOp::Dup { .. })),
        "{:#?}",
        program.ops
    );
    assert_eq!(
        value_of(&format!(
            "{source} call({{ k: 'x', m(v) {{ return this.k + v; }} }}, 'abc', 2);"
        )),
        Value::String("x1b2.0x2".into())
    );
}

#[test]
fn a_branch_on_a_local_tests_it_in_place() {
    let source = "function pick(a, b) { if (a) { b = 1; } else { b = 2; } var c = a && b; while (b) { b = b - 1; } return c + ':' + b; }";
    compile::compile(&nested_function(source, "pick")).expect("the body should be admitted");
    assert_eq!(
        value_of(&format!(
            "{source} [pick({{}}, 0), pick(0, 0), pick('', 5), pick(NaN, 1)].join(',');"
        )),
        Value::String("1:0,0:0,:0,NaN:0".into())
    );
}

#[test]
fn a_comparison_branch_is_one_operation_on_its_operands() {
    let source = "function order(a, b) {
        var out = 0;
        if (a < b) out = out + 1;
        if (a === b) out = out + 10;
        if (a >= 2) out = out + 100;
        if (typeof a == 'object') out = out + 1000;
        return out;
    }";
    let program =
        compile::compile(&nested_function(source, "order")).expect("the body should be admitted");
    let fused = program
        .ops
        .iter()
        .filter(|op| matches!(op, WideOp::CompareJump { .. }))
        .count();
    assert_eq!(fused, 4, "{:#?}", program.ops);
    assert!(
        !program.ops.iter().any(|op| matches!(
            op,
            WideOp::JumpIfFalsy { .. }
                | WideOp::Binary {
                    op: BinaryOp::Lt | BinaryOp::StrictEq | BinaryOp::Ge | BinaryOp::Eq,
                    ..
                }
        )),
        "{:#?}",
        program.ops
    );
    // Numbers, NaN, strings, and objects whose conversion is observable,
    // converted left before right exactly once per comparison.
    assert_eq!(
        value_of(&format!(
            "{source}
             var log = [];
             function v(name, n) {{ return {{ valueOf() {{ log.push(name); return n; }} }}; }}
             [order(1, 2), order(2, 2), order(NaN, NaN), order('b', 'a'), order('a', 'a'),
              order(v('x', 1), v('y', 3)), log.join('')].join(',');"
        )),
        Value::String("1,110,0,0,10,1001,xyx".into())
    );
}

#[test]
fn a_stored_result_is_written_to_the_local_directly() {
    let source =
        "function sum(a, b) { var t = a + b; var u = t; var w = typeof u; return t + u + w; }";
    let program =
        compile::compile(&nested_function(source, "sum")).expect("the body should be admitted");
    let moves = program
        .ops
        .iter()
        .filter(|op| {
            matches!(op, WideOp::Move { dst, src }
                if *dst < program.local_registers && *src >= program.local_registers)
        })
        .count();
    assert_eq!(moves, 0, "{:#?}", program.ops);
    assert_eq!(
        value_of(&format!("{source} sum(1, 2) + '|' + sum('a', 'b');")),
        Value::String("6number|ababstring".into())
    );
}

#[test]
fn a_loop_condition_reentered_from_its_backedge_reads_the_current_values() {
    // The loop header is the comparison's first operand load, so the
    // backedge lands on the fused comparison.
    let source = "function count(n) {
        var i = 0, hits = 0, guard = 0;
        while (i < n) { if (++guard > 1000) throw 'hang'; if (i !== 3) hits++; i++; }
        return hits;
    }";
    assert_eq!(
        value_of(&format!("{source} count(10);")),
        Value::Number(9.0)
    );
}

#[test]
fn a_function_assigning_an_existing_global_variable_runs_here() {
    let source = "var last = 42, A = 3877, C = 29573, M = 139968;
        function rand(max) { last = (last * A + C) % M; return max * last / M; }";
    let program =
        compile::compile(&nested_function(source, "rand")).expect("the body should be admitted");
    assert!(
        program
            .ops
            .iter()
            .any(|op| matches!(op, WideOp::Exit { .. })),
        "{:#?}",
        program.ops
    );
    assert_eq!(
        value_of(&format!(
            "{source}
             var sum = 0; for (var i = 0; i < 100; i++) sum += rand(100);
             [Math.round(sum), last, globalThis.last].join(',');"
        )),
        value_of(
            "var last = 42, A = 3877, C = 29573, M = 139968;
             var sum = 0; for (var i = 0; i < 100; i++) {
                 last = (last * A + C) % M; sum += 100 * last / M;
             }
             [Math.round(sum), last, globalThis.last].join(',');"
        )
    );
}

#[test]
fn global_stores_the_fast_path_cannot_prove_keep_their_semantics() {
    let source = "function set(v) { g = v; return v; }
        // Created by the first store, then overwritten.
        set(1); set(2);
        var created = g;
        // Read-only: a sloppy store is silently ignored.
        Object.defineProperty(globalThis, 'g', { value: 7, writable: false, configurable: true });
        set(3);
        var readOnly = g;
        // Writable again, through a redefinition the store must observe.
        Object.defineProperty(globalThis, 'g', { value: 8, writable: true, configurable: true });
        set(4); set(5);
        [created, readOnly, g, globalThis.g].join(',');";
    assert_eq!(value_of(source), Value::String("2,7,5,5".into()));
    assert_eq!(
        value_of(
            "let lex = 1; function setLex(v) { lex = v; } setLex(2); setLex(3);
             [lex, 'lex' in globalThis].join(',');"
        ),
        Value::String("3,false".into())
    );
    assert!(
        error_of("const fixed = 1; function setFixed() { fixed = 2; } setFixed(); setFixed();")
            .contains("TypeError")
    );
}

#[test]
fn a_global_store_in_an_accelerated_loop_or_keeping_its_appended_value_stays_interpreted() {
    for (source, name) in [
        (
            "var n = 0; function count(k) { for (var i = 0; i < k; i++) { n = n + 1; } return n; }",
            "count",
        ),
        (
            "var text = ''; function add(s) { return text = text + s; }",
            "add",
        ),
    ] {
        assert!(
            compile::compile(&nested_function(source, name)).is_none(),
            "{source}"
        );
    }
    // A loop no accelerator compiles -- it constructs an object -- keeps
    // its global stores on this tier.
    let source = "var last = 0; function Step(v) { this.v = v + 1; }
        function walk(k) { for (i = 0; i < k; i++) { last = new Step(i).v; } return last + i; }";
    compile::compile(&nested_function(source, "walk")).expect("the body should be admitted");
    assert_eq!(
        value_of(&format!("{source} walk(10) + last + i;")),
        Value::Number(40.0)
    );
    // A loop the typed tier compiles only by calling a user method, which
    // deoptimizes it whenever the method is not a closed-form leaf -- 3d-raytrace's
    // `for (i = 0; ...) triangle.intersect(...)` -- is no reason to stay.
    let source = "function Tri(k) { this.k = k; }
        Tri.prototype.hit = function (x) { var o = { x: x }; return o.x < this.k; };
        var tris = [new Tri(3), new Tri(5), new Tri(7)];
        function blocked(x) { for (i = 0; i < tris.length; i++) { if (tris[i].hit(x)) return i; } return -1; }";
    compile::compile(&nested_function(source, "blocked")).expect("the body should be admitted");
    assert_eq!(
        value_of(&format!(
            "{source} [blocked(4), blocked(9), blocked(1), i].join();"
        )),
        Value::String("1,-1,0,0".into())
    );
}

#[test]
fn a_caller_that_assigned_the_global_itself_reads_the_callee_s_store() {
    // Test262 S12.2_A3: the enclosing function routes its own sloppy
    // assignment through the realm cell, which the store here writes.
    assert_eq!(
        value_of(
            "var shared = 'OUT';
             (function () {
                 shared = 'IN';
                 (function () { shared = 'INNER'; })();
                 (function () { var shared = 'SHADOW'; })();
                 if (shared !== 'INNER') throw new Error('stale ' + shared);
             })();
             shared;"
        ),
        Value::String("INNER".into())
    );
}

#[test]
fn the_rest_of_a_body_runs_here_after_an_accelerator_finishes_its_loop() {
    // The AES round shape: a numeric-mutation loop over table lookups, then
    // more work on locals and the receiver, called far more often than the
    // exit-heavy judgement's sample.
    let source = "function Box() { this.k = [1, 2, 3, 4, 5, 6, 7, 8]; this.out = 0; }
        Box.prototype.round = function (a, b) {
            var k = this.k, x = a, y = b, j, n = k.length;
            for (j = 0; j < n; j++) { x = (x ^ k[j]) + y; y = (y * 3) & 1023; }
            var tail = [x & 255, y & 255];
            this.out = tail[0] + tail[1];
            return this.out + j;
        };";
    assert_eq!(
        value_of(&format!(
            "{source}
             var box = new Box(), total = 0;
             for (var i = 0; i < 200; i++) total += box.round(i, i + 1);
             total;"
        )),
        value_of(&format!(
            "{source}
             function reference(a, b) {{
                 var k = [1, 2, 3, 4, 5, 6, 7, 8], x = a, y = b, j;
                 for (j = 0; j < 8; j++) {{ x = (x ^ k[j]) + y; y = (y * 3) & 1023; }}
                 return (x & 255) + (y & 255) + j;
             }}
             var total = 0;
             for (var i = 0; i < 200; i++) total += reference(i, i + 1);
             total;"
        ))
    );
}

#[test]
fn array_and_string_method_reads_follow_their_prototypes() {
    // Each read site sees the realm prototype's method, an own shadowing
    // property, a replaced prototype method, and a subclass prototype.
    let source = "function use(list, text) {
        list.push(text.charAt(0));
        return list.length + ':' + list.join('') + ':' + text.toUpperCase();
    }";
    assert_eq!(
        value_of(&format!(
            "{source}
             var out = [];
             for (var i = 0; i < 3; i++) out.push(use([i], 'ab'));
             var shadow = [9]; shadow.join = function () {{ return 'own'; }};
             out.push(use(shadow, 'cd'));
             var saved = Array.prototype.push;
             Array.prototype.push = function (v) {{ return saved.call(this, v, v); }};
             var patched = use([7], 'ef');
             Array.prototype.push = saved;
             out.push(patched);
             String.prototype.charAt = function () {{ return '#'; }};
             out.push(use([], 'gh'));
             class Stack extends Array {{ join() {{ return 'stack'; }} }}
             out.push(use(new Stack(), 'ij'));
             out.join(' ');"
        )),
        Value::String("2:0a:AB 2:1a:AB 2:2a:AB 2:own:CD 3:7ee:EF 1:#:GH 1:stack:IJ".into())
    );
}

#[test]
fn number_method_reads_follow_number_prototype() {
    let source = "function show(n) { return n.toString() + '/' + n.toFixed(1); }";
    assert_eq!(
        value_of(&format!(
            "{source}
             var out = [show(1), show(2.5)];
             Number.prototype.toFixed = function () {{ return 'fixed'; }};
             out.push(show(3));
             out.join(' ');"
        )),
        Value::String("1/1.0 2.5/2.5 3/fixed".into())
    );
}

#[test]
fn builtin_statics_arrays_and_instanceof_keep_their_hooks() {
    let source = "function probe(n, C) {
        var a = new Array(n), b = new Array(1, 2), c = new Array('x');
        return [a.length, 0 in a, b.join(), c[0], String.fromCharCode(65 + n),
                Array.isArray(a), a instanceof Array, b instanceof C].join(',');
    }";
    assert_eq!(
        value_of(&format!(
            "{source}
             function Plain() {{}}
             class Base {{}} class Derived extends Base {{}}
             class Hooked {{ static [Symbol.hasInstance](v) {{ return v === 7; }} }}
             var out = [probe(2, Array), probe(0, Plain), probe(1, Derived)];
             String.fromCharCode = function () {{ return 'patched'; }};
             out.push(probe(3, Base));
             out.push([7] instanceof Hooked, 7 instanceof Hooked);
             try {{ new Array(-1); }} catch (e) {{ out.push(e instanceof RangeError); }}
             try {{ probe(1.5, Array); }} catch (e) {{ out.push(e instanceof RangeError); }}
             out.join(' | ');"
        )),
        Value::String(
            "2,false,1,2,x,C,true,true,true | 0,false,1,2,x,A,true,true,false | \
             1,false,1,2,x,B,true,true,false | 3,false,1,2,x,patched,true,true,false | \
             false | true | true | true"
                .into()
        )
    );
}

#[test]
fn a_repeated_method_read_sees_an_own_property_added_later() {
    let source = "function Counter() { this.n = 0; }
        Counter.prototype.step = function () { return 1; };
        function run(c, k) { var t = 0; for (var i = 0; i < k; i++) { var o = new Counter(); t += c.step(); } return t; }";
    assert_eq!(
        value_of(&format!(
            "{source}
             var c = new Counter(), out = [run(c, 5)];
             c.step = function () {{ return 10; }};
             out.push(run(c, 5));
             delete c.step;
             out.push(run(c, 5));
             Object.defineProperty(c, 'step', {{ get() {{ return function () {{ return 100; }}; }}, configurable: true }});
             out.push(run(c, 5));
             out.join(',');"
        )),
        Value::String("5,50,5,500".into())
    );
}

#[test]
fn compound_member_assignment_converts_its_key_once_and_checks_its_object() {
    let source = "function bump(o, k) { o[k] += 1; o[k] *= 2; return o[k]; }";
    let program =
        compile::compile(&nested_function(source, "bump")).expect("the body should be admitted");
    assert!(
        program
            .ops
            .iter()
            .any(|op| matches!(op, WideOp::ToPropertyKey { .. })),
        "{:#?}",
        program.ops
    );
    assert_eq!(
        value_of(&format!(
            "{source}
             var log = [];
             var key = {{ toString() {{ log.push('key'); return 'x'; }} }};
             var sym = Symbol('s');
             var o = {{ x: 1, 1.5: 2, [sym]: 3 }}, a = [5, 6];
             var out = [bump(o, key), bump(o, 1.5), bump(o, sym), bump(a, 1), bump(a, '0'), log.join('')];
             try {{ bump(null, 'x'); }} catch (e) {{ out.push(e instanceof TypeError); }}
             try {{ bump(undefined, key); }} catch (e) {{ out.push(e instanceof TypeError, log.length); }}
             out.join(',');"
        )),
        Value::String("4,6,8,14,12,keykeykey,true,true,3".into())
    );
}

#[test]
fn an_assignment_to_an_undeclared_name_creates_the_global() {
    let source = "function make(v) { fresh = v; also = fresh + 1; return also; }";
    assert_eq!(
        value_of(&format!(
            "{source}
             var out = [make(1), make(5), fresh, also, 'fresh' in globalThis,
                        Object.getOwnPropertyDescriptor(globalThis, 'fresh').enumerable];
             out.join(',');"
        )),
        Value::String("2,6,5,6,true,true".into())
    );
}

#[test]
fn an_array_hole_reads_undefined_unless_a_prototype_has_the_index() {
    assert_eq!(
        value_of(
            "function get(a, i) { return a[i]; } \
         var a = new Array(4); a[1] = 7; \
         var r = [get(a, 0), get(a, 1), get(a, 9)]; \
         Array.prototype[2] = 'p'; r.push(get(a, 2)); \
         Object.prototype[3] = 'q'; r.push(get(a, 3)); \
         Object.defineProperty(Array.prototype, 0, { get: function () { return 'g'; }, configurable: true }); \
         r.push(get(a, 0)); \
         delete Array.prototype[2]; delete Array.prototype[0]; delete Object.prototype[3]; \
         r.push(get(a, 2), get(a, 3)); \
         r.map(String).join();"
        ),
        Value::String(
            "undefined,7,undefined,p,q,g,undefined,undefined"
                .to_owned()
                .into()
        )
    );
}

#[test]
fn a_local_stored_and_reloaded_is_not_copied_back() {
    let source = "function pick(o) { var x = o.a; if (x) { return x; } return 0; }";
    let program =
        compile::compile(&nested_function(source, "pick")).expect("the body should be admitted");
    let copied_back = program.ops.windows(2).any(|pair| {
        matches!(pair, [WideOp::Move { dst: a, src: t }, WideOp::Move { dst: t2, src: a2 }]
            if a == a2 && t == t2)
    });
    assert!(!copied_back, "{:#?}", program.ops);
    assert_eq!(
        value_of(&format!(
            "{source} pick({{ a: 5 }}) + ':' + pick({{ a: 0 }}) + ':' + pick({{}});"
        )),
        Value::String("5:0:0".to_owned().into())
    );
}

#[test]
fn a_global_string_append_is_admitted_and_extends_in_place_semantics() {
    let program = compile::compile(&nested_function(
        "var acc = ''; function add(r) { acc += ',' + r; }",
        "add",
    ))
    .expect("a statement appending to a global string should be admitted");
    assert!(
        program
            .ops
            .iter()
            .any(|op| matches!(op, WideOp::Exit { .. })),
        "{:#?}",
        program.ops
    );
    assert_eq!(
        value_of(
            "var acc = \"\", other = 0, log = []; \
         function add(r) { acc += \",\" + r; } \
         function addNum(r) { other += r; } \
         for (var i = 0; i < 5; i++) add(i); \
         var alias = acc; add(\"x\"); \
         log.push(acc, alias); \
         addNum(2); addNum(3); log.push(other); \
         acc2 = \"\"; \
         function add2(r) { acc2 = acc2 + r; } \
         add2(\"a\"); add2(1); add2(null); \
         log.push(acc2); \
         Object.defineProperty(globalThis, \"acc2\", { get: function () { return \"G\"; }, set: function (v) { log.push(\"set:\" + v); }, configurable: true }); \
         add2(\"y\"); \
         log.push(acc2); \
         var o = { toString: function () { return \"T\"; } }; \
         delete globalThis.acc2; acc2 = \"s\"; add2(o); log.push(acc2); \
         log.join(\"|\");"
        ),
        Value::String(
            ",0,1,2,3,4,x|,0,1,2,3,4|5|a1null|set:Gy|G|sT"
                .to_owned()
                .into()
        )
    );
}

#[test]
fn an_inlined_method_receives_object_receivers_unchanged_and_coerces_the_rest() {
    assert_eq!(
        value_of(
            "function kind() { return typeof this + \":\" + (this instanceof Object); } \
         function strictKind() { \"use strict\"; return typeof this; } \
         function wrap(f, r) { return f.call(r); } \
         var out = []; \
         function viaMethod(r) { r.k = kind; return r.k(); } \
         out.push(viaMethod([1]), viaMethod(function () {}), viaMethod({})); \
         var sym = Symbol(\"s\"); \
         Symbol.prototype.k = kind; Symbol.prototype.sk = strictKind; \
         out.push(sym.k(), sym.sk()); \
         Number.prototype.k = kind; out.push((5).k()); \
         out.join(\",\");"
        ),
        Value::String(
            "object:true,function:true,object:true,object:true,symbol,object:true"
                .to_owned()
                .into()
        )
    );
}

#[test]
fn a_for_in_loop_stays_on_the_tier_with_the_interpreter_s_semantics() {
    let source =
        "function keys(o) { var r = []; for (var k in o) { r.push(k); } return r.join(','); }";
    let program =
        compile::compile(&nested_function(source, "keys")).expect("a for-in body is admitted");
    assert!(
        program
            .ops
            .iter()
            .filter(|op| matches!(op, WideOp::Exit { .. }))
            .count()
            >= 2,
        "{:#?}",
        program.ops
    );
    assert_eq!(
        value_of(
            "function keys(o) { var r = []; for (var k in o) { r.push(k); } return r.join(\",\"); } \
         function firstBig(table, limit) { for (var c in table) { if (table[c] > limit) return c; } return \"none\"; } \
         function pairs(a, b) { var n = 0; for (var x in a) { for (var y in b) { n++; } } return n; } \
         function deleting(o) { var seen = []; for (var k in o) { seen.push(k); delete o.b; } return seen.join(\"\"); } \
         var proto = { inherited: 1 }; var child = Object.create(proto); child.own = 2; child[1] = 3; \
         var log = []; \
         var proxy = new Proxy({ p: 1, q: 2 }, { ownKeys: function (t) { log.push(\"ownKeys\"); return Reflect.ownKeys(t); } }); \
         var out = [keys(child), firstBig({ a: 1, b: 5, c: 9 }, 4), pairs({ x: 1, y: 2 }, [1, 2, 3]), deleting({ a: 1, b: 2, c: 3 }), keys(proxy), log.length > 0, keys(null), keys(\"ab\")]; \
         for (var i = 0; i < 3; i++) out.push(firstBig({ z: i, w: 10 }, i)); \
         out.join(\"|\");"
        ),
        Value::String(
            "1,own,inherited|b|6|ac|p,q|true||0,1|w|w|w"
                .to_owned()
                .into()
        )
    );
}

#[test]
fn methods_with_a_home_object_run_inline_and_super_or_private_bodies_keep_their_path() {
    // Class and object-literal methods carry a home object, which only
    // `super` observes; a body using `super` or a private name is not
    // compiled for this tier and keeps the general call.
    assert_eq!(
        value_of(
            "class Base { value() { return 1; } twice() { return this.value() * 2; } } \
         class Derived extends Base { value() { return super.value() + 10; } plus(n) { return this.twice() + n; } } \
         class Secret { #k = 5; get() { return this.#k; } add(n) { return this.get() + n; } } \
         var lit = { base: 3, m() { return this.base + 1; }, n() { return this.m() * 2; } }; \
         var out = []; \
         var d = new Derived(), s = new Secret(), b = new Base(); \
         var total = 0; \
         for (var i = 0; i < 1000; i++) { total += d.plus(i) + s.add(i) + lit.n() + b.twice(); } \
         out.push(total, d.twice(), s.add(1), lit.n()); \
         out.join(\",\");"
        ),
        Value::String("1036000,22,6,8".to_owned().into())
    );
}

#[test]
fn a_base_class_constructor_runs_inline_after_its_fields() {
    // Fields install in order before the body (an initializer sees earlier
    // fields and statics), an object result replaces the receiver, a direct
    // call still throws, a throwing initializer propagates, and a body that
    // reads new.target keeps the general path.
    assert_eq!(
        value_of(
            "var log = []; \
         class V { x = 0; y = this.x + 1; z = V.base; constructor(x, y) { log.push(\"ctor:\" + this.y); this.x = x; this.y = y; } sum() { return this.x + this.y + this.z; } } \
         V.base = 100; \
         class R { a = 1; constructor(flag) { if (flag) return { replaced: true }; return 5; } } \
         class T { t = (function () { throw new Error(\"field\"); })(); } \
         class N { constructor() { this.nt = new.target === N; } } \
         function make(n) { var s = 0; for (var i = 0; i < n; i++) { var v = new V(i, 2); s += v.sum(); } return s; } \
         var out = [make(10), log.length, log[0]]; \
         out.push(JSON.stringify(new R(true)), new R(false).a, new V(1, 1) instanceof V, Object.keys(new V(3, 4)).join(\"\")); \
         try { V(1, 2); out.push(\"no\"); } catch (e) { out.push(e instanceof TypeError); } \
         try { new T(); out.push(\"no\"); } catch (e) { out.push(e.message); } \
         function makeN() { return new N().nt; } out.push(makeN()); \
         out.join(\"|\");"
        ),
        Value::String(
            "1065|10|ctor:1|{\"replaced\":true}|1|true|xyz|true|field|true"
                .to_owned()
                .into()
        )
    );
}

#[test]
fn a_loop_judged_short_keeps_running_correctly_when_it_grows() {
    // Entered a hundred times for one or two iterations, the loop is judged
    // short and runs on this tier; later long runs must still be exact.
    assert_eq!(
        value_of(
            "function B() { this.x = 0; this.y = 2; }
             function f(o, n) { for (var i = 0; i < n; i++) { o.x += o.y * i; } return o.x; }
             var o = new B(), short = 0;
             for (var k = 0; k < 200; k++) short = f(o, 1 + (k & 1));
             var shortTotal = o.x;
             o.x = 0;
             var long = f(o, 1000);
             [shortTotal, long].join();"
        ),
        Value::String("200,999000".into())
    );
}

#[test]
fn loose_equality_between_strings_compares_their_text() {
    assert_eq!(
        value_of(
            "function same(a, b) { return [a == b, a != b, typeof a != typeof b].join(); }
             var out = [];
             for (var i = 0; i < 50; i++) out = [same('ab', 'a' + 'b'), same('x', 'y'), same('1', 1)];
             out.join('|');"
        ),
        Value::String("true,false,false|false,true,false|true,false,true".into())
    );
}

#[test]
fn element_reads_answer_dense_arrays_in_place_and_everything_else_generally() {
    assert_eq!(
        value_of(
            "function at(a, i) { return a[i]; }
             function first(a) { return a[0] + ':' + a[2]; }
             var dense = [1, 2, 3], holes = [1, , 3], text = 'abc', obj = { 0: 'o', 2: 'p' };
             var out;
             for (var k = 0; k < 50; k++) {
                 out = [at(dense, 1), at(dense, 1.5), at(dense, -1), at(dense, 7), at(holes, 1),
                        at(text, 2), at(obj, 0), first(dense), first(text), first(obj)];
             }
             Array.prototype[1] = 'inherited';
             out.push(at(holes, 1), at(dense, 1));
             delete Array.prototype[1];
             out.join();"
        ),
        Value::String("2,,,,,c,o,1:3,a:c,o:p,inherited,2".into())
    );
}

#[test]
fn wide_calls_run_numeric_call_chains_and_see_rebound_callees() {
    assert_eq!(
        value_of(
            "function add(x, y) { var lsw = (x & 0xFFFF) + (y & 0xFFFF); var msw = (x >> 16) + (y >> 16) + (lsw >> 16); return (msw << 16) | (lsw & 0xFFFF); }
             function rol(num, cnt) { return (num << cnt) | (num >>> (32 - cnt)); }
             function cmn(q, a, b, x, s, t) { return add(rol(add(add(a, q), add(x, t)), s), b); }
             function ff(a, b, c, d, x, s, t) { return cmn((b & c) | ((~b) & d), a, b, x, s, t); }
             function run(n) { var h = 1732584193, out = []; for (var i = 0; i < n; i++) { h = ff(h, i, i * 3, i ^ 5, i * 7, 7, -680876936); } out.push(h);
                 out.push(ff(1, 2, 3, 4, '5', 7, 11));
                 rol = function (num, cnt) { return num + cnt; };
                 out.push(ff(1, 2, 3, 4, 5, 7, 11));
                 return out.join(); }
             run(200);"
        ),
        Value::String("1178343728,2946,32".into())
    );
}

#[test]
fn numeric_plans_see_missing_arguments_as_undefined() {
    assert_eq!(
        value_of(
            "function same(a, b) { if (a === b) return 1; return 2; }
             function run(n) { var t = 0; for (var i = 0; i < n; i++) { t += same(); t += same(i); } return t; }
             run(50);"
        ),
        Value::Number(150.0)
    );
}

/// Once a loop's typed program has run from its backedge, a later entry runs
/// it from the header, before the first iteration: with the loop's locals
/// still `undefined`, around nested loops, `continue`, a header reached by a
/// jump from above, and a program that deoptimizes on its data.
#[test]
fn loops_entered_from_above_run_their_typed_program_from_the_header() {
    assert_eq!(
        value_of(
            "function Body(x, v, m) { this.x = x; this.v = v; this.m = m; }
             function advance(bodies, dt) {
                 var dx, mag;
                 var size = bodies.length;
                 for (var i = 0; i < size; i++) {
                     var bi = bodies[i];
                     for (var j = i + 1; j < size; j++) {
                         var bj = bodies[j];
                         dx = bi.x - bj.x;
                         mag = dt / (dx * dx + 1);
                         bi.v -= dx * bj.m * mag;
                         bj.v += dx * bi.m * mag;
                     }
                 }
                 for (var k = 0; k < size; k++) bodies[k].x += dt * bodies[k].v;
             }
             function skip(n) { var s = 0, k = 0; while (k < n) { k++; if (k & 1) continue; s += k; } return s; }
             function forward(n, flag) { var s = 0, k = 0; if (flag) { s = 100; } else { s = 1; } while (k < n) { s += k; k++; } return s; }
             function mixed(a) { var t = 0; for (var q = 0; q < a.length; q++) t += a[q]; return t; }
             function run() {
                 var bodies = [new Body(0, 0, 1), new Body(1, 0, 2), new Body(3, 1, 3), new Body(7, 2, 1), new Body(12, 0, 5)];
                 for (var r = 0; r < 300; r++) advance(bodies, 0.01);
                 var out = [];
                 for (var b = 0; b < bodies.length; b++) out.push(bodies[b].x.toFixed(6), bodies[b].v.toFixed(6));
                 var s = 0, f = 0, m = 0;
                 for (var r = 0; r < 200; r++) { s += skip(r & 15); f += forward(r & 7, r & 1); }
                 for (var r = 0; r < 200; r++) m += mixed(r < 150 ? [1, 2, 3] : [1, 'x', 3]) === 6 ? 1 : 0;
                 out.push(s, f, m);
                 return out.join();
             }
             run();"
        ),
        Value::String(
            "8.228696,3.479297,7.939108,4.596806,5.797929,2.684094,12.254605,-0.456076,7.848939,-3.053823,4072,11500,150"
                .into()
        )
    );
}

/// A named read answers first from the entry that last hit; instances of one
/// constructor share it, while an object with its properties in another
/// order, an accessor under the same name, or a literal falls through to the
/// full cache walk.
#[test]
fn named_reads_answer_constructor_instances_from_the_hot_entry_only() {
    assert_eq!(
        value_of(
            "function P(a, b) { this.a = a; this.b = b; }
             function Q(a, b) { this.b = b; this.a = a; }
             function readA(o) { return o.a; }
             function run() {
                 var out = 0, objs = [new P(1, 2), new P(3, 4), new Q(5, 6), new P(7, 8)];
                 var acc = new P(0, 0);
                 Object.defineProperty(acc, 'a', { get: function () { return 100; } });
                 objs.push(acc, { b: 1, a: 9 });
                 for (var i = 0; i < 600; i++) out += readA(objs[i % objs.length]);
                 return out;
             }
             run();"
        ),
        Value::Number(12500.0)
    );
}

/// Operands loaded from locals are read where they are until something
/// else needs the stack register; a store to the local in between, a
/// reassignment inside the expression, or an in-place string append must
/// all still see the value the bytecode loaded.
#[test]
fn forwarded_local_operands_see_the_value_loaded() {
    assert_eq!(
        value_of(
            "function f1(a) { return a + (a = 5); }
             function f2(a, b) { var t = a; a = b; return t + a; }
             function f3(o, i) { return o[i] + (i = 0, o[i]); }
             function f4(o) { var x = o; return x.p + (x = { p: 100 }).p + x.p; }
             function f5(s, t) { s = s + t; s = s + s; return s; }
             function f6(a, b) { var c = a * b - a; return c < a ? c : a; }
             function f7(arr, i) { var v = arr[i]; arr[i] = 9; return v + arr[i]; }
             function f8(a) { var b = a; a++; return b * 10 + a; }
             function f9(o) { var k = 'x'; return o[k] + (k = 'y', o[k]); }
             function f10(a, b) { return (a = b) + a + b; }
             function run() {
                 var out = [];
                 for (var i = 0; i < 50; i++) {
                     out = [f1(2), f2(3, 4), f3([7, 8], 1), f4({ p: 1 }), f5('ab', 'c'), f6(3, 4),
                            f7([1, 2], 0), f8(4), f9({ x: 1, y: 2 }), f10(1, 2)];
                 }
                 return out.join();
             }
             run();"
        ),
        Value::String("7,7,15,201,abcabc,3,10,45,3,6".into())
    );
}

/// A method one prototype further up (`Sub.prototype.__proto__ =
/// Base.prototype`) is cached by both prototypes; shadowing it on the
/// parent, deleting that again, replacing the parent's prototype, an own
/// property on the receiver, and a new value on the holder are all seen.
#[test]
fn inherited_methods_two_prototypes_up_see_every_change() {
    assert_eq!(
        value_of(
            "function Base() {}
             Base.prototype = { who: function () { return 'base'; }, n: 1 };
             function Sub() { this.x = 0; }
             Sub.prototype = { own: function () { return 'sub'; } };
             Sub.prototype.__proto__ = Base.prototype;
             function call(o) { return o.who() + o.n; }
             function run() {
                 var objs = [new Sub(), new Sub(), new Sub()];
                 var out = [];
                 for (var round = 0; round < 7; round++) {
                     var r = '';
                     for (var i = 0; i < 30; i++) r = call(objs[i % 3]);
                     out.push(r);
                     if (round == 0) Base.prototype.n = 2;
                     if (round == 1) Sub.prototype.who = function () { return 'shadow'; };
                     if (round == 2) delete Sub.prototype.who;
                     if (round == 3) Object.setPrototypeOf(Sub.prototype, { who: function () { return 'other'; }, n: 9 });
                     if (round == 4) objs[1].who = function () { return 'own'; };
                     if (round == 5) Base.prototype.who = function () { return 'late'; };
                 }
                 out.push(call(objs[1]));
                 return out.join();
             }
             run();"
        ),
        Value::String("base1,base2,shadow2,base2,other9,other9,other9,own9".into())
    );
}
