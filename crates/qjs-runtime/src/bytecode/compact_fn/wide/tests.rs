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
    assert!(
        !program
            .ops
            .iter()
            .any(|op| matches!(op, WideOp::Exit { ip, .. }
            if !matches!(nested_function(source, "pick").code[*ip as usize], Op::Jump(_)))),
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
