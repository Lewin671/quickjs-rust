use crate::{Value, eval_bytecode_source};

fn assert_bytecode_evaluates(source: &str) {
    assert!(eval_bytecode_source(source).is_ok(), "{source}");
}

#[test]
fn evaluates_core_expressions_with_bytecode() {
    assert_bytecode_evaluates("1 + 2 * 3;");
    assert_bytecode_evaluates("'x' + 1;");
    assert_bytecode_evaluates("true ? 1 : missing;");
    assert_bytecode_evaluates("false || 5;");
    assert_bytecode_evaluates("0 && missing;");
    assert_bytecode_evaluates("null ?? 42;");
    assert_bytecode_evaluates("typeof neverDeclared;");
}

#[test]
fn block_scoped_lexicals_are_captured_independently() {
    assert_eq!(
        eval_bytecode_source(
            "let x = 'outside'; \
             var before = function() { return x; }; \
             var inside; \
             { let x = 'inside'; inside = function() { return x; }; } \
             before() + ':' + inside() + ':' + x;"
        ),
        Ok(Value::String("outside:inside:outside".to_owned()))
    );
}

#[test]
fn evaluates_number_binary_fast_paths_with_bytecode() {
    assert_eq!(
        eval_bytecode_source("(8 << 2) + (32 >> 1) + (-1 >>> 31);"),
        Ok(Value::Number(49.0))
    );
    assert_eq!(
        eval_bytecode_source("(7 & 3) + (4 | 1) + (6 ^ 3);"),
        Ok(Value::Number(13.0))
    );
    assert_eq!(
        eval_bytecode_source("(1 == 1) && (1 != 2) && !(NaN == NaN);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval_bytecode_source("new Boolean(true) == true;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval_bytecode_source("({ valueOf: function() { return 1; } }) == true;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval_bytecode_source("'2' < '10';"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval_bytecode_source("'\\u{10000}' <= '\\uFFFF';"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval_bytecode_source(
            "let object = { valueOf: function() { return -2; }, toString: function() { return '-2'; } }; '-1' < object;"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval_bytecode_source(
            "let accessed = false; let left = { valueOf: function() { accessed = true; return 4; } }; let right = { valueOf: function() { return 2; } }; (left <= right) + ':' + accessed;"
        ),
        Ok(Value::String("false:true".to_owned()))
    );
    assert!(matches!(
        eval_bytecode_source("(function() { return 1; }) * {};"),
        Ok(Value::Number(value)) if value.is_nan()
    ));
    assert!(matches!(
        eval_bytecode_source("({}) * (function() { return 1; });"),
        Ok(Value::Number(value)) if value.is_nan()
    ));
}

#[test]
fn evaluates_number_unary_fast_paths_with_bytecode() {
    assert_eq!(
        eval_bytecode_source("(+3) + (-4) + (~1);"),
        Ok(Value::Number(-3.0))
    );
    assert_eq!(
        eval_bytecode_source("let x = 2; ~x;"),
        Ok(Value::Number(-3.0))
    );
    assert_eq!(
        eval_bytecode_source(
            "let accessed = false; let object = { valueOf: function() { accessed = true; return 1; } }; (+object) + ':' + accessed;"
        ),
        Ok(Value::String("1:true".to_owned()))
    );
    assert_eq!(
        eval_bytecode_source("Object.is(-(0), -0);"),
        Ok(Value::Boolean(true))
    );
    assert!(matches!(
        eval_bytecode_source("+function() { return 1; };"),
        Ok(Value::Number(value)) if value.is_nan()
    ));
    assert!(matches!(
        eval_bytecode_source("-function() { return 1; };"),
        Ok(Value::Number(value)) if value.is_nan()
    ));
    assert_eq!(
        eval_bytecode_source("~function() { return 1; };"),
        Ok(Value::Number(-1.0))
    );
    assert_eq!(
        eval_bytecode_source("+'Infinity';"),
        Ok(Value::Number(f64::INFINITY))
    );
    assert!(matches!(
        eval_bytecode_source("+'INFINITY';"),
        Ok(Value::Number(value)) if value.is_nan()
    ));
    assert!(matches!(
        eval_bytecode_source("+'infinity';"),
        Ok(Value::Number(value)) if value.is_nan()
    ));
    assert_eq!(
        eval_bytecode_source(
            "let object = { hits: 0, valueOf: function() { this.hits = 1; return 4; } }; (+object) + object.hits;"
        ),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval_bytecode_source("let date = new Date(0); date + 0 === date.toString() + '0';"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_slot_locals_with_bytecode() {
    assert_bytecode_evaluates("let x = 2; const y = 3; x * y;");
    assert_bytecode_evaluates("var x = 1, y = 2, missing; x + y;");
    assert_bytecode_evaluates("x; var x = 1; x;");
    assert_bytecode_evaluates("let x = 1; x += 2; x;");
    assert_bytecode_evaluates("let x = 1; x++; x;");
    assert_bytecode_evaluates("let x = 1; ++x;");
    assert_eq!(
        eval_bytecode_source(
            "const x = 1; let ok = false; try { x = 2; } catch (error) { ok = error instanceof TypeError; } ok;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval_bytecode_source("function f(x) { { let x = 'inner'; } return x; } f('outer');"),
        Ok(Value::String("outer".to_owned()))
    );
    assert_eq!(
        eval_bytecode_source("function f(x) { { const x = 'inner'; } return x; } f('outer');"),
        Ok(Value::String("outer".to_owned()))
    );
    assert_eq!(
        eval_bytecode_source("let x = 'outer'; { let x = 'inner'; } x;"),
        Ok(Value::String("outer".to_owned()))
    );
    assert_eq!(
        eval_bytecode_source("let x = 'outer'; try { let x = 'inner'; } catch (e) {} x;"),
        Ok(Value::String("outer".to_owned()))
    );
    assert_eq!(
        eval_bytecode_source("let x = 'outer'; try { 1; } finally { let x = 'inner'; } x;"),
        Ok(Value::String("outer".to_owned()))
    );
    assert_eq!(
        eval_bytecode_source("let x = 'outer'; switch (1) { case 1: let x = 'inner'; } x;"),
        Ok(Value::String("outer".to_owned()))
    );
}

#[test]
fn evaluates_strict_identifier_assignment_with_bytecode() {
    assert_eq!(
        eval_bytecode_source(
            "\"use strict\"; let hit = 0; try { missing = (hit = 1); } catch (error) { hit = hit + (error instanceof ReferenceError); } hit;"
        ),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval_bytecode_source(
            "let value = 1; function set() { \"use strict\"; value = 3; } set(); value;"
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval_bytecode_source(
            "function set() { \"use strict\"; missing = 1; } let caught = false; try { set(); } catch (error) { caught = error instanceof ReferenceError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval_bytecode_source(
            "this.strictGlobal = 1; function set() { \"use strict\"; strictGlobal = 4; } set(); this.strictGlobal + strictGlobal;"
        ),
        Ok(Value::Number(8.0))
    );
}

#[test]
fn reinitializes_lexical_declarations_in_loop_blocks_with_bytecode() {
    assert_eq!(
        eval_bytecode_source(
            "let i = 0; let total = 0; while (i < 3) { const value = i; total = total + value; i = i + 1; } total;"
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval_bytecode_source(
            "let i = 0; for (; i < 3; i = i + 1) { let value = i; value = value + 1; } i;"
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval_bytecode_source(
            "let caught = false; try { const value = 1; value = 2; } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn rejects_unresolved_identifier_compound_assignment_with_bytecode() {
    assert_eq!(
        eval_bytecode_source(
            "let hit = 0; try { missing += (hit = 1); } catch (error) { hit = hit + (error instanceof ReferenceError); } hit;"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval_bytecode_source(
            "let hit = 0; try { missing++; } catch (error) { hit = error instanceof ReferenceError; } hit;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval_bytecode_source("this.count = 1; count += 2; count++; ++count; this.count + count;"),
        Ok(Value::Number(10.0))
    );
    assert_eq!(
        eval_bytecode_source(
            "this.flag = false; flag ||= 7; let first = flag; flag &&= 3; let second = flag; flag ??= 9; first + second + flag;"
        ),
        Ok(Value::Number(13.0))
    );
}

#[test]
fn evaluates_delete_with_bytecode() {
    assert_bytecode_evaluates("let object = { value: 1 }; delete object.value;");
    assert_bytecode_evaluates("let object = { value: 1 }; delete object.value; object.value;");
    assert_bytecode_evaluates(
        "let key = 'value'; let object = { value: 1 }; delete object[key]; object.value;",
    );
    assert_bytecode_evaluates(
        "let key = Symbol(); let object = { [key]: 1 }; delete object[key]; object[key];",
    );
    assert_bytecode_evaluates("let array = [1, 2]; delete array[0]; array[0];");
    assert_bytecode_evaluates("let x = 1; delete x;");
    assert_bytecode_evaluates("delete missing;");
    assert_bytecode_evaluates("delete 1;");
}

#[test]
fn evaluates_branch_and_loop_bytecode_subset() {
    assert_bytecode_evaluates("let x = 1; if (x > 0) { x = 7; } else { x = 3; } x;");
    assert_bytecode_evaluates("if (false) { 1; }");
    assert_bytecode_evaluates("let x = 0; while (x < 3) { x = x + 1; } x;");
    assert_bytecode_evaluates("let x = 0; while (x < 3) { x = x + 1; x; }");
    assert_bytecode_evaluates("let x = 0; while (true) { x++; if (x === 3) break; } x;");
    assert_bytecode_evaluates(
        "let x = 0; let sum = 0; while (x < 5) { x++; if (x === 3) continue; sum = sum + x; } sum;",
    );
    assert_bytecode_evaluates("let x = 0; do { x++; } while (x < 3); x;");
    assert_bytecode_evaluates("let x = 0; do { x++; x; } while (x < 3);");
    assert_bytecode_evaluates("let x = 0; do { x++; if (x < 3) continue; x; } while (x < 5);");
    assert_bytecode_evaluates(
        "let sum = 0; for (var i = 0; i < 4; i = i + 1) { sum = sum + i; } sum;",
    );
    assert_bytecode_evaluates("for (var i = 0; i < 3; i = i + 1) { i; }");
    assert_bytecode_evaluates(
        "let sum = 0; for (var i = 0; i < 6; i = i + 1) { if (i === 2) continue; if (i === 5) break; sum = sum + i; } sum;",
    );
    assert_bytecode_evaluates(
        "let x = 2; switch (x) { case 1: 'one'; break; case 2: 'two'; break; default: 'other'; }",
    );
    assert_bytecode_evaluates("let x = 3; switch (x) { case 1: 'one'; break; default: 'other'; }");
    assert_bytecode_evaluates("let x = 1; switch (x) { case 1: 'one'; case 2: 'two'; }");
    assert_bytecode_evaluates(
        "let sum = 0; for (var i = 0; i < 4; i++) { switch (i) { case 1: continue; case 3: break; default: sum = sum + i; } sum = sum + 10; } sum;",
    );
    assert_bytecode_evaluates(
        "function f(x) { switch (x) { case 1: return 'one'; default: return 'other'; } } f(1);",
    );
    assert_bytecode_evaluates(
        "let out = ''; for (var key in { b: 2, a: 1 }) { out = out + key; } out;",
    );
    assert_eq!(
        eval_bytecode_source("let object = {}; object.Infinity = 1; Infinity in object;"),
        Ok(Value::Boolean(true))
    );
    assert_bytecode_evaluates(
        "let out = ''; let key; for (key in ['x', 'y']) { out = out + key; } out;",
    );
    assert_bytecode_evaluates("let out = ''; for (var key in null) { out = out + key; } out;");
    assert_bytecode_evaluates(
        "let out = ''; for (var key in { a: 1, b: 2, c: 3 }) { if (key === 'b') continue; out = out + key; if (key === 'c') break; } out;",
    );
    assert_bytecode_evaluates("let holder = {}; for (holder.key in { z: 1 }) {} holder.key;");
    assert_bytecode_evaluates(
        "function keys(object) { let out = ''; for (var key in object) { out = out + key; } return out; } keys({ b: 2, a: 1 });",
    );
    assert_bytecode_evaluates("try { 1; } catch (error) { 2; }");
    assert_bytecode_evaluates("try { throw 'hit'; } catch (error) { error; }");
    assert_eq!(
        eval_bytecode_source("try { throw 'x'; } catch (e) {} typeof e;"),
        Ok(Value::String("undefined".to_owned()))
    );
    assert_eq!(
        eval_bytecode_source("let e = 'outer'; try { throw 'x'; } catch (e) {} e;"),
        Ok(Value::String("outer".to_owned()))
    );
    assert_eq!(
        eval_bytecode_source(
            "function f() { try { throw 'x'; } catch (e) { return 'catch'; } finally { return typeof e; } } f();"
        ),
        Ok(Value::String("undefined".to_owned()))
    );
    assert_bytecode_evaluates("let x = 1; try { x = 2; } finally { x = x + 3; } x;");
    assert_bytecode_evaluates(
        "let x = ''; try { throw 'a'; } catch (error) { x = x + error; } finally { x = x + 'f'; } x;",
    );
    let error = eval_bytecode_source(
        "let x = ''; try { throw 'a'; } catch (error) { throw 'b'; } finally { x = x + 'f'; }",
    )
    .expect_err("rethrow through finally should fail");
    assert_eq!(error.message, "throw statement executed: b");
    assert_bytecode_evaluates(
        "function f() { let x = 1; try { return x; } finally { x = 3; } } f();",
    );
    assert_bytecode_evaluates(
        "function f() { try { throw 'a'; } catch (error) { return error; } finally { 2; } } f();",
    );
    assert_bytecode_evaluates("function f() { try { return 1; } finally { return 2; } } f();");
    assert_bytecode_evaluates(
        "function f() { try { throw 'a'; } finally { return 'final'; } } f();",
    );
    assert_eq!(
        eval_bytecode_source(
            "let caught = false; try { Array.prototype.map.call(undefined, function() {}); } catch (error) { caught = error.constructor === TypeError && error.message === 'Array.prototype.map called on null or undefined'; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval_bytecode_source(
            "function f() { throw 'native bridge must not rewrite JS throws'; } let caught; try { f(); } catch (error) { caught = error; } caught;"
        ),
        Ok(Value::String(
            "native bridge must not rewrite JS throws".to_owned()
        ))
    );
    assert_eq!(
        eval_bytecode_source(
            "function f() { throw { name: 'kept' }; } let caught; try { f(); } catch (error) { caught = error.name; } caught;"
        ),
        Ok(Value::String("kept".to_owned()))
    );
    assert_eq!(
        eval_bytecode_source(
            "let caught = false; try { new String.prototype.charAt(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval_bytecode_source(
            "let object = { valueOf: function() { throw 'error'; }, toString: function() { return 1; } }; let caught; try { object >>> 0; } catch (error) { caught = error; } caught;"
        ),
        Ok(Value::String("error".to_owned()))
    );
    assert_eq!(
        eval_bytecode_source(
            "let object = { valueOf: function() { throw 'unary'; }, toString: function() { return 1; } }; let caught; try { +object; } catch (error) { caught = error; } caught;"
        ),
        Ok(Value::String("unary".to_owned()))
    );
    assert_eq!(
        eval_bytecode_source(
            "let object = { valueOf: function() { return {}; }, toString: function() { return {}; } }; let caught = false; try { object >>> 0; } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval_bytecode_source(
            "let object = { valueOf: function() { return {}; }, toString: function() { return {}; } }; let caught = false; try { -object; } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn try_catch_finally_completion_value_update_empty() {
    // Break from catch with no prior value: UpdateEmpty produces undefined
    assert_eq!(
        eval_bytecode_source(
            "eval(\"for (var i = 0; i < 2; ++i) { if (i) { try { throw null; } catch (e) { break; } } 'bad completion'; }\")"
        ),
        Ok(Value::Undefined)
    );
    // Break from finally with preceding value 42: completion is 42
    assert_eq!(
        eval_bytecode_source(
            "eval('99; do { -99; try { 39 } catch (e) { -1 } finally { 42; break; -2 }; } while (false);')"
        ),
        Ok(Value::Number(42.0))
    );
    // Break from finally with no preceding value: completion is undefined
    assert_eq!(
        eval_bytecode_source(
            "eval('99; do { -99; try { 39 } catch (e) { -1 } finally { break; -2 }; } while (false);')"
        ),
        Ok(Value::Undefined)
    );
    // Break from finally discards pending throw (throw inside try, break inside finally)
    assert_eq!(
        eval_bytecode_source(
            "eval('do { try { throw 1; } finally { break; } } while (false); try { 42; } finally { 99; }')"
        ),
        Ok(Value::Number(42.0))
    );
    // Catch body resets completion: try body value does not leak through catch break
    assert_eq!(
        eval_bytecode_source(
            "eval(\"for (var i = 0; i < 2; ++i) { if (i) { try { 'try_val'; throw null; } catch (e) { break; } } 'iter_val'; }\")"
        ),
        Ok(Value::Undefined)
    );
}

#[test]
fn evaluates_objects_arrays_members_and_calls_with_bytecode() {
    assert_bytecode_evaluates("let o = { count: 1 }; o.count;");
    assert_bytecode_evaluates("let o = { count: 1 }; o.count = 3; o.count;");
    assert_bytecode_evaluates("let key = 'count'; let o = { [key]: 2 }; o[key];");
    assert_bytecode_evaluates("let values = [1, 2, 3]; values[1];");
    assert_bytecode_evaluates("let values = [1, 2, 3]; values.length;");
    assert_bytecode_evaluates("'abc'.length;");
    assert_bytecode_evaluates("Math.max(1, 5, 3);");
    assert_bytecode_evaluates("let values = [1]; values.push(2); values.length;");
    assert_bytecode_evaluates("function f() { return 1; } f();");
    assert_bytecode_evaluates("function add(a, b) { return a + b; } add(2, 3);");
    assert_bytecode_evaluates("let base = 10; function add(x) { return base + x; } add(5);");
    assert_bytecode_evaluates("let f = function() { return 2; }; f();");
    assert_bytecode_evaluates("let f = function(x) { return x * 2; }; f(4);");
    assert_bytecode_evaluates(
        "function C() { this.value = 4; } let instance = new C(); instance.value;",
    );
    assert_eq!(
        eval_bytecode_source(
            "delete String.prototype.charAt.length; String.prototype.charAt.hasOwnProperty('length');"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval_bytecode_source(
            "let out = ''; String.prototype.charAt.extra = true; for (var key in String.prototype.charAt) { out = out + key; } out;"
        ),
        Ok(Value::String("extra".to_owned()))
    );
}

#[test]
fn seeds_bytecode_function_env_from_referenced_caller_bindings() {
    assert_eq!(
        eval_bytecode_source("let value = 1; function read() { return value; } value = 2; read();"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval_bytecode_source(
            "let seen = ''; [1, 2].forEach(function(value) { seen = seen + value; }); seen;",
        ),
        Ok(Value::String("12".to_owned()))
    );
    assert_eq!(
        eval_bytecode_source(
            "(function() { function a() { return 1; } function b() { return 2; } function c() { return a() + b(); } return [a(), b(), c()].join('|'); })();",
        ),
        Ok(Value::String("1|2|3".to_owned()))
    );
    assert_eq!(
        eval_bytecode_source(
            "(function() { function a() { return 1; } function b() { return 2; } function make() { return function() { return a() + b(); }; } return make()(); })();",
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval_bytecode_source(
            "function makeCounter() { let index = 0; return function() { index = index + 1; return index; }; } let next = makeCounter(); next() + ':' + next();",
        ),
        Ok(Value::String("1:2".to_owned()))
    );
    assert_eq!(
        eval_bytecode_source(
            "function makePair() { let index = 0; return [function() { index = index + 1; return index; }, function() { index = index + 1; return index; }]; } let pair = makePair(); pair[0]() + ':' + pair[1]() + ':' + pair[0]();",
        ),
        Ok(Value::String("1:2:3".to_owned()))
    );
    assert_eq!(
        eval_bytecode_source(
            "function combo(callback) { function test(input) { callback(input); } test(1); } \
             function wrap(callback) { return combo(function(input) { callback(input); }); } \
             var seen = 0; wrap(function(value) { seen = value; }); seen;",
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval_bytecode_source(
            "function throws(expected, fn) { try { fn(); } catch (error) { if (error.constructor !== expected) { throw error; } return; } throw new Error('missing'); } \
             { const rab = new ArrayBuffer(64, { maxByteLength: 1024 }); let called = false; throws(TypeError, () => rab.resize({ valueOf() { __quickjsRustDetachArrayBuffer(rab); called = true; } })); if (!called) { throw new Error('first'); } } \
             { const rab = new ArrayBuffer(64, { maxByteLength: 1024 }); __quickjsRustDetachArrayBuffer(rab); let called = false; throws(TypeError, () => rab.resize({ valueOf() { called = true; } })); called; }",
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn reports_unsupported_bytecode_surface() {
    let error = eval_bytecode_source("break;").expect_err("top-level break must not compile");
    assert_eq!(error.message, "break outside loop");

    let error = eval_bytecode_source("continue;").expect_err("top-level continue must not compile");
    assert_eq!(error.message, "`continue` has no target");
}

#[test]
fn propagates_bytecode_throw_errors() {
    assert_eq!(
        eval_bytecode_source("if (false) { throw; } 1;"),
        Ok(Value::Number(1.0))
    );
    let error = eval_bytecode_source("throw 'expected';").expect_err("throw should fail");
    assert_eq!(error.message, "throw statement executed: expected");
}
