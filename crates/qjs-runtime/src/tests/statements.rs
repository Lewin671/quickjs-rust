use crate::{Value, eval};

#[test]
fn with_statement_resolves_object_bindings() {
    // The with-object shadows outer and global names for free references.
    assert_eq!(eval("with ({ a: 1 }) { a; }"), Ok(Value::Number(1.0)));
    assert_eq!(eval("var x = 5; with ({}) { x; }"), Ok(Value::Number(5.0)));
    // Assignments inside the body write back to the with-object property.
    assert_eq!(
        eval("var o = { a: 1 }; with (o) { a = 2; } o.a;"),
        Ok(Value::Number(2.0))
    );
    // Getters on the with-object run during identifier resolution.
    assert_eq!(
        eval("with ({ get a() { return 42; } }) { a; }"),
        Ok(Value::Number(42.0))
    );
    // typeof inside the body consults the with-object.
    assert_eq!(
        eval("with ({ a: 1 }) { typeof a; }"),
        Ok(Value::String("number".to_owned().into()))
    );
    // The body resolves `this` from the surrounding scope, not the object.
    assert_eq!(
        eval("function f() { with ({ x: 10 }) { return x; } } f();"),
        Ok(Value::Number(10.0))
    );
}

#[test]
fn with_statement_method_call_uses_with_object_as_this() {
    // Calling a method resolved through a with-object binds `this` to that
    // object (GetThisValue of an object-environment reference), while a name
    // that falls through to an outer scope keeps the ordinary undefined/global
    // receiver.
    assert_eq!(
        eval(
            "var seen; \
             var o = { m() { seen = this; } }; \
             with (o) { m(); } \
             seen === o;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "var o = { x: 1 }; \
             function f() { return this === globalThis || this === undefined; } \
             var ok; with (o) { ok = f(); } ok;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn with_statement_var_initializer_targets_with_object() {
    // A `var` initializer inside `with` is an ordinary PutValue through the
    // scope chain: when the with-object owns the name, the property is updated
    // and no global binding is created.
    assert_eq!(
        eval("var o = { foo: 1 }; with (o) { var foo = 2; } o.foo;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("var o = { foo: 1 }; with (o) { var foo = 2; } typeof globalThis.foo;"),
        Ok(Value::String("undefined".to_owned().into()))
    );
    // When the with-object lacks the name, the hoisted var binding is written.
    assert_eq!(
        eval("var o = { foo: 1 }; with (o) { var bar = 7; } bar;"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("var o = { foo: 1 }; with (o) { var bar = 7; } 'bar' in o;"),
        Ok(Value::Boolean(false))
    );
    // At function scope the hoisted local is the fallback target.
    assert_eq!(
        eval(
            "function f() { var o = { foo: 1 }; with (o) { var foo = 2; var bar = 3; } \
             return o.foo + ':' + bar; } f();"
        ),
        Ok(Value::String("2:3".to_owned().into()))
    );
    // The PutValue target is resolved before the initializer runs, so a RHS
    // side effect cannot redirect the store after deleting the property.
    assert_eq!(
        eval(
            "var o = { foo: 1 }; with (o) { var foo = delete o.foo; } \
             String(o.foo) + ':' + String(foo);"
        ),
        Ok(Value::String("true:undefined".to_owned().into()))
    );
}

#[test]
fn for_in_assignment_targets_respect_lexical_tdz() {
    assert_eq!(
        eval(
            "var result = '';
             try { for (x in { a: 1 }) {} result = 'no throw'; }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             let x;
             result;"
        ),
        Ok(Value::String("reference".to_owned().into()))
    );
}

#[test]
fn for_in_lexical_heads_create_tdz_for_rhs() {
    assert_eq!(
        eval(
            "let result = '';
             try { let x = 1; for (let x in { x }) {} result = 'no throw'; }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             result;"
        ),
        Ok(Value::String("reference".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let result = '';
             try { let x = 1; for (const x in { x }) {} result = 'no throw'; }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             result;"
        ),
        Ok(Value::String("reference".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let x = 'outside';
             var probeExpr, probeDecl, probeBody;
             for (let [x, _ = probeDecl = function() { return x; }]
                  in { i: probeExpr = function() { typeof x; } })
               probeBody = function() { return x; };
             let exprResult;
             try { probeExpr(); exprResult = 'no throw'; }
             catch (error) { exprResult = error instanceof ReferenceError ? 'reference' : error.name; }
             exprResult + ':' + probeDecl() + ':' + probeBody();"
        ),
        Ok(Value::String("reference:i:i".to_owned().into()))
    );
}

#[test]
fn var_declarations_without_initializers_do_not_reset_bindings() {
    assert_eq!(eval("var x = 1; var x; x;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval(
            "var iterCount = 0;
             for (var x in { attr: null }) {
               var x;
               if (x === 'attr') iterCount += 1;
             }
             iterCount;"
        ),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn with_statement_honors_symbol_unscopables() {
    // A truthy `Symbol.unscopables` entry hides the property, so the free name
    // resolves to the outer binding instead.
    assert_eq!(
        eval(
            "var a = 1;
             var o = { a: 2 };
             o[Symbol.unscopables] = { a: true };
             with (o) { a; }"
        ),
        Ok(Value::Number(1.0))
    );
    // A falsy entry leaves the property visible.
    assert_eq!(
        eval(
            "var a = 1;
             var o = { a: 2 };
             o[Symbol.unscopables] = { a: false };
             with (o) { a; }"
        ),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn with_captured_by_function_does_not_capture_function_var_initializers() {
    assert_eq!(
        eval(
            "var scope = { value: 'outer', p: 1 };
             with (scope) {
               var f = function() {
                 p = 2;
                 var value = 'local';
                 return value;
               };
             }
             f() + ':' + scope.value + ':' + scope.p;"
        ),
        Ok(Value::String("local:outer:2".to_owned().into()))
    );
}

#[test]
fn function_created_in_with_resolves_object_before_captured_outer_slot() {
    assert_eq!(
        eval(
            "function objectWins() {
               var a = { a: 10 };
               with (a) { return () => a; }
             }
             function outerFallback() {
               var a = 7;
               with ({}) { return () => a; }
             }
             objectWins()() + ':' + outerFallback()();"
        ),
        Ok(Value::String("10:7".to_owned().into()))
    );
}

#[test]
fn with_update_expression_uses_resolved_unscopables_binding_once() {
    assert_eq!(
        eval(
            "var calls = 0, flag = true, outer, inner;
             with (outer = { x: 7 }) {
               with (inner = { x: 4, get [Symbol.unscopables]() {
                 calls++;
                 return { x: flag = !flag };
               } }) {
                 x++;
               }
             }
             calls + ':' + outer.x + ':' + inner.x;"
        ),
        Ok(Value::String("1:7:5".to_owned().into()))
    );
}

#[test]
fn with_empty_abrupt_completion_updates_to_undefined() {
    assert_eq!(
        eval("eval('1; do { 2; with({}) { 3; break; } 4; } while (false);');"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("eval('5; do { 6; with({}) { break; } 7; } while (false);');"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("eval('8; do { 9; with({}) { 10; continue; } 11; } while (false)');"),
        Ok(Value::Number(10.0))
    );
    assert_eq!(
        eval("eval('12; do { 13; with({}) { continue; } 14; } while (false)');"),
        Ok(Value::Undefined)
    );
}

#[test]
fn with_statement_unwinds_on_abrupt_completion() {
    // break out of a with body keeps the with-object stack balanced.
    assert_eq!(
        eval(
            "var r = 0;
             for (var i = 0; i < 3; i++) {
                 with ({}) { if (i === 1) break; r += i; }
             }
             r;"
        ),
        Ok(Value::Number(0.0))
    );
    // A throw crossing a with body restores the outer scope.
    assert_eq!(
        eval(
            "var x = 7;
             try { with ({ x: 1 }) { throw 0; } } catch (e) {}
             x;"
        ),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn with_statement_rejected_in_strict_mode() {
    assert!(eval("'use strict'; with ({}) {}").is_err());
}

#[test]
fn block_scoped_function_hoisting_respects_strict_mode() {
    // Sloppy mode: Annex B B.3.3 hoists the block function into the enclosing
    // var scope, so the name is visible after the block.
    assert_eq!(
        eval("{ function f() { return 1; } } typeof f;"),
        Ok(Value::String("function".to_owned().into()))
    );
    assert_eq!(
        eval("eval('{ function g() {} }'); typeof g;"),
        Ok(Value::String("function".to_owned().into()))
    );
    // Strict mode: the block function stays lexically scoped to the block.
    assert!(
        eval("'use strict'; { function f() {} } f;").is_err(),
        "strict block function must not hoist to the enclosing scope"
    );
    // Direct eval in strict code is strict even without its own prologue.
    assert!(
        eval("'use strict'; eval('{ function f() {} }'); typeof f;")
            .is_ok_and(|value| value == Value::String("undefined".to_owned().into())),
        "strict direct eval block function must not leak to global scope"
    );
    // A strict prologue inside the eval body has the same effect.
    assert!(
        eval("eval('\"use strict\"; { function f() {} }'); typeof f;")
            .is_ok_and(|value| value == Value::String("undefined".to_owned().into()))
    );
}

#[test]
fn evaluates_variable_declarations() {
    assert_eq!(
        eval("let x = 2; const y = 3; x * y;"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(eval("var missing; missing;"), Ok(Value::Undefined));
    assert_eq!(eval("x; var x;"), Ok(Value::Undefined));
    assert_eq!(eval("x; var x = 1; x;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("if (false) { var x = 1; } x;"), Ok(Value::Undefined));
    assert_eq!(
        eval("function f() { return x; var x = 2; } f();"),
        Ok(Value::Undefined)
    );
    assert!(eval("x; let x;").is_err());
    assert_eq!(
        eval("var x = 1, y = 2, missing; x + y;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("{ const value = 1; Object.is(value, 1); } { const value = 2; Object.is(value, 2); }"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn script_completion_ignores_empty_declarations() {
    assert_eq!(eval("eval('class C {}');"), Ok(Value::Undefined));
    assert_eq!(eval("eval('1; class C {}');"), Ok(Value::Number(1.0)));
    assert_eq!(eval("eval('1; function f() {}');"), Ok(Value::Number(1.0)));
    assert_eq!(eval("eval('1; var x;');"), Ok(Value::Number(1.0)));
    assert_eq!(eval("eval('1; ;');"), Ok(Value::Number(1.0)));
}

#[test]
fn evaluates_variable_declaration_destructuring() {
    assert_eq!(
        eval("let [first = 1, , third] = [undefined, 2, 3]; first + third;"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("const {value, key: renamed = 2} = {value: 5}; value + renamed;"),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn var_destructuring_resolves_with_binding_before_value_read() {
    assert_eq!(
        eval(
            "var log = [];
             var sourceKey = { toString: function() { log.push('sourceKey'); return 'p'; } };
             var source = {
               get p() {
                 log.push('get source');
                 return undefined;
               }
             };
             var env = new Proxy({}, {
               has: function(_target, key) {
                 log.push('binding::' + key);
                 return false;
               }
             });
             var defaultValue = 0;
             var varTarget;
             with (env) {
               var { [sourceKey]: varTarget = defaultValue } = source;
             }
             log.join(',');"
        ),
        Ok(Value::String(
            "binding::source,binding::sourceKey,sourceKey,binding::varTarget,get source,binding::defaultValue"
                .to_owned()
                .into()
        ))
    );
}

#[test]
fn evaluates_variable_declaration_rest_destructuring() {
    assert_eq!(
        eval("let [first, ...others] = [1, 2, 3]; first + ':' + others.join('|');"),
        Ok(Value::String("1:2|3".to_owned().into()))
    );
    assert_eq!(
        eval("const {p, ...rest} = {p: 1, q: 2, r: 3}; p + ':' + Object.keys(rest).join('|');"),
        Ok(Value::String("1:q|r".to_owned().into()))
    );
    // A hand-rolled iterable stands in for a generator until generator
    // evaluation lands in T010 S2.
    assert_eq!(
        eval(
            "function range() {
               var n = 0;
               return { [Symbol.iterator]() { return this; },
                        next() { n = n + 1; return { value: n, done: n > 2 }; } };
             }
             let [x, y] = range(); x + y;"
        ),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn array_destructuring_steps_iterator_lazily_and_closes_when_unfinished() {
    assert_eq!(
        eval(
            "var steps = []; var iterable = {};
             iterable[Symbol.iterator] = function() {
               var n = 0;
               return {
                 next: function() { n += 1; steps.push('next' + n); return { value: n, done: n > 3 }; },
                 return: function() { steps.push('return'); return {}; }
               };
             };
             var [first, , third] = iterable;
             steps.join(',') + '|' + first + '|' + third;"
        ),
        Ok(Value::String("next1,next2,next3,return|1|3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var steps = []; var iterable = {};
             iterable[Symbol.iterator] = function() {
               var n = 0;
               return {
                 next: function() { n += 1; steps.push('next' + n); return { value: n, done: n > 2 }; },
                 return: function() { steps.push('return'); return {}; }
               };
             };
             var [head, ...tail] = iterable;
             steps.join(',') + '|' + head + '|' + tail.join('+');"
        ),
        Ok(Value::String("next1,next2,next3|1|2".to_owned().into()))
    );
}

#[test]
fn array_destructuring_step_errors_skip_iterator_close() {
    assert_eq!(
        eval(
            "var returned = false; var iterable = {};
             iterable[Symbol.iterator] = function() {
               var n = 0;
               return {
                 next: function() { n += 1; if (n === 2) { throw new Error('boom'); } return { value: n, done: false }; },
                 return: function() { returned = true; return {}; }
               };
             };
             var caught = '';
             try { var [a, b] = iterable; } catch (error) { caught = error.message; }
             caught + ':' + returned;"
        ),
        Ok(Value::String("boom:false".to_owned().into()))
    );
}

#[test]
fn array_destructuring_closes_iterator_on_abrupt_binding_completion() {
    assert_eq!(
        eval(
            "var returned = false; var iterable = {};
             iterable[Symbol.iterator] = function() {
               return {
                 next: function() { return { value: undefined, done: false }; },
                 return: function() { returned = true; return {}; }
               };
             };
             var caught = '';
             try { var [a = (function() { throw new Error('dflt'); })()] = iterable; }
             catch (error) { caught = error.message; }
             caught + ':' + returned;"
        ),
        Ok(Value::String("dflt:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var returned = false; var iterable = {};
             iterable[Symbol.iterator] = function() {
               return {
                 next: function() { return { value: null, done: false }; },
                 return: function() { returned = true; return {}; }
               };
             };
             var caught = false;
             try { var [{nested}] = iterable; } catch (error) { caught = true; }
             caught + ':' + returned;"
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn destructuring_defaults_do_not_treat_html_dda_as_undefined() {
    assert_eq!(
        eval(
            "let initCount = 0; const counter = function() { initCount += 1; }; const [x = counter()] = [__quickjsRustIsHTMLDDA]; (x === __quickjsRustIsHTMLDDA) + ':' + initCount;"
        ),
        Ok(Value::String("true:0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let initCount = 0; const counter = function() { initCount += 1; }; const {x = counter()} = {x: __quickjsRustIsHTMLDDA}; (x === __quickjsRustIsHTMLDDA) + ':' + initCount;"
        ),
        Ok(Value::String("true:0".to_owned().into()))
    );
}

#[test]
fn evaluates_if_else_statements() {
    assert_eq!(
        eval("let x = 1; if (x > 0) { x = 7; } else { x = 3; } x;"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("let x = 1; if (x < 0) x = 7; else x = 3; x;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn if_empty_abrupt_completion_updates_to_undefined() {
    assert_eq!(
        eval("eval('1; do { if (false) { } else { break; } } while (false)')"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("eval('2; do { if (false) { } else { continue; } } while (false)')"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("eval('3; do { if (true) { break; } else { } } while (false)')"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("eval('4; do { if (true) { continue; } else { } } while (false)')"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("eval('5; do { 6; if (true) { 7; break; } 8; } while (false)')"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("eval('9; do { 10; if (true) { break; } 11; } while (false)')"),
        Ok(Value::Undefined)
    );
}

#[test]
fn evaluates_while_statements() {
    assert_eq!(
        eval("let x = 0; while (x < 3) { x = x + 1; } x;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_do_while_statements() {
    assert_eq!(
        eval("let x = 0; do { x = x + 1; } while (false); x;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("let x = 0; do { x++; } while (x < 3); x;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("let x = 0; do { x++; if (x === 2) continue; } while (x < 3); x;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_for_statements() {
    assert_eq!(
        eval("let sum = 0; for (var i = 0; i < 4; i = i + 1) { sum = sum + i; } sum;"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("let i = 0; for (; i < 3; ) i = i + 1; i;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_for_in_statements() {
    assert_eq!(
        eval("let count = 0; for (var key in { a: 1, b: 2 }) { count++; } count;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let total = 0; let item; let values = [1, 2, 3]; for (item in values) { total += values[item]; } total;"
        ),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("let count = 0; for (var key in null) { count++; } count;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let proto = {}; Object.defineProperty(proto, 'inherited', { value: 1, enumerable: true }); let object = Object.create(proto); object.own = 2; let seen = ''; for (var key in object) { seen = seen + key + ':'; } seen;"
        ),
        Ok(Value::String("own:inherited:".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = Object.create(null); object.aa = 1; object.ba = 2; object.ca = 3; \
             let seen = ''; \
             for (var key in object) { delete object.ba; seen = seen + key + object[key]; } \
             seen;"
        ),
        Ok(Value::String("aa1ca3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "Object.defineProperty(Boolean.prototype, 'visible', { value: 1, enumerable: true, configurable: true }); let seen = false; for (var key in new Boolean()) { if (key === 'visible') seen = true; } delete Boolean.prototype.visible; seen;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "Object.defineProperty(Function.prototype, 'visible', { value: 1, enumerable: true, configurable: true }); \
             let bound = (function() {}).bind({}); let seen = false; \
             for (var key in bound) { if (key === 'visible') seen = true; } \
             delete Function.prototype.visible; seen;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("var stored; for (var key = 0 in stored = key, { a: 1 }) {} stored + ':' + key;"),
        Ok(Value::String("0:a".to_owned().into()))
    );
    // for-in over a Proxy enumerates through its ownKeys /
    // getOwnPropertyDescriptor traps and filters non-enumerable keys.
    assert_eq!(
        eval(
            "let o = { vis: 1 }; Object.defineProperty(o, 'hid', { value: 2, enumerable: false }); \
             let seen = ''; for (var k in new Proxy(o, {})) { seen += k + ':'; } seen;"
        ),
        Ok(Value::String("vis:".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let p = new Proxy({}, { \
                 ownKeys() { return ['x', 'y']; }, \
                 getOwnPropertyDescriptor() { return { value: 1, enumerable: true, configurable: true }; } \
             }); \
             let seen = ''; for (var k in p) { seen += k; } seen;"
        ),
        Ok(Value::String("xy".to_owned().into()))
    );
    // A Proxy whose target is itself a Proxy forwards enumeration when traps
    // are absent, and walks the prototype chain via getPrototypeOf.
    assert_eq!(
        eval(
            "let base = { inh: 1 }; let o = Object.create(base); o.own = 2; \
             let seen = []; for (var k in new Proxy(new Proxy(o, {}), {})) { seen.push(k); } \
             seen.sort().join(',');"
        ),
        Ok(Value::String("inh,own".to_owned().into()))
    );
}

#[test]
fn evaluates_for_of_statements() {
    assert_eq!(
        eval("let total = 0; for (var value of [1, 2, 3]) { total += value; } total;"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval(
            "let seen = ''; for (const value of new Set(['a', 'b'])) { seen = seen + value; } seen;"
        ),
        Ok(Value::String("ab".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let seen = ''; for (let entry of new Map([['a', 1], ['b', 2]])) { seen = seen + entry[0] + entry[1]; } seen;"
        ),
        Ok(Value::String("a1b2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let total = 0; for (var value of [1, 2, 3, 4]) { if (value === 2) continue; if (value === 4) break; total += value; } total;"
        ),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("let target = {}; for (target.value of [5, 6]) {} target.value;"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("let total = 0; for (var let of [1, 2]) { total = total + let; } total;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "let closed = 0; let iter = { next: function() { return { done: false, value: 1 }; }, return: function() { closed += 1; return {}; } }; let source = {}; source[Symbol.iterator] = function() { return iter; }; for (var value of source) { break; } closed;"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let iter = { next: function() { return { done: false, value: 1 }; }, return: function() { return null; } }; let source = {}; source[Symbol.iterator] = function() { return iter; }; let caught = false; try { for (var value of source) { break; } } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let iter = { next: function() { return { done: false, value: 1 }; }, return: __quickjsRustIsHTMLDDA }; iter[Symbol.iterator] = function() { return iter; }; let caught = false; try { for (var value of iter) { break; } } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // An iterator `next()` result that is not an Object throws a TypeError. A
    // Symbol primitive is not an Object even though symbols are modeled as
    // object wrappers internally.
    assert_eq!(
        eval(
            "let source = {}; source[Symbol.iterator] = function () { \
                 let done = { value: null, done: true }; let n = Symbol('s'); \
                 return { next: function () { let r = n; n = done; return r; } }; \
             }; \
             let caught = false; try { for (var value of source) {} } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_break_and_continue() {
    assert_eq!(
        eval("let i = 0; while (true) { i = i + 1; if (i === 3) break; } i;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "let sum = 0; for (var i = 0; i < 5; i = i + 1) { if (i === 2) continue; sum = sum + i; } sum;"
        ),
        Ok(Value::Number(8.0))
    );
    assert_eq!(
        eval("eval('2; while (true) { 3; break; }');"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("eval('2; for (var value of [0]) { 3; break; }');"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("eval('2; for (var value of [0, 1]) { 3; continue; }');"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("eval('4; outer: do { while (true) { continue outer; } } while (false)');"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("eval('5; outer: do { while (true) { 6; continue outer; } } while (false)');"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval(
            "eval('7; outer: do { for (var value of [0]) { 8; continue outer; } } while (false)');"
        ),
        Ok(Value::Number(8.0))
    );
    assert_eq!(
        eval("eval('done: { 9; break done; 10; }');"),
        Ok(Value::Number(9.0))
    );
}

#[test]
fn evaluates_switch_statements() {
    assert_eq!(
        eval(
            "let x = 2; let out = 0; switch (x) { case 1: out = 1; break; case 2: out = 2; break; default: out = 3; } out;"
        ),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let x = 4; let out = 0; switch (x) { case 1: out = 1; break; default: out = 3; } out;"
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "let x = 1; let out = 0; switch (x) { case 1: out += 1; case 2: out += 2; default: out += 4; } out;"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let x = '1'; let out = 0; switch (x) { case 1: out = 1; break; default: out = 2; } out;"
        ),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let x = 'outside'; var probeExpr, probeSelector, probeStmt; \
             switch (probeExpr = function() { return x; }, null) { \
               case probeSelector = function() { return x; }, null: \
                 probeStmt = function() { return x; }; \
                 let x = 'inside'; \
             } \
             probeExpr() + ':' + probeSelector() + ':' + probeStmt();"
        ),
        Ok(Value::String("outside:inside:inside".to_owned().into()))
    );
    assert!(eval("switch (0) { default: function* x() {} } x;").is_err());
    assert!(eval("switch (0) { default: async function x() {} } x;").is_err());
    assert!(eval("switch (0) { default: async function* x() {} } x;").is_err());
}

#[test]
fn evaluates_throw_statement_only_when_reached() {
    assert_eq!(eval("if (false) { throw; } 1;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("if (false) { throw 'no'; } 1;"),
        Ok(Value::Number(1.0))
    );
    let error = eval("throw;").expect_err("throw should fail evaluation");
    assert_eq!(error.message, "throw statement executed: undefined");
    let error = eval("throw 'expected';").expect_err("throw should fail evaluation");
    assert_eq!(error.message, "throw statement executed: expected");
    let error = eval("throw 42;").expect_err("throw should fail evaluation");
    assert_eq!(error.message, "throw statement executed: 42");
}

#[test]
fn evaluates_try_catch_finally_statements() {
    assert_eq!(
        eval("try { throw 'caught'; } catch (error) { error; }"),
        Ok(Value::String("caught".to_owned().into()))
    );
    assert_eq!(
        eval("let x = 1; try { throw 2; } catch (error) { x = error; } x;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("let x = 1; try { x += 1; } finally { x += 2; } x;"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("let x = 1; try { throw 1; } catch (error) { x += error; } finally { x += 2; } x;"),
        Ok(Value::Number(4.0))
    );
    let error =
        eval("try { throw 'try'; } finally { throw 'finally'; }").expect_err("throw should fail");
    assert_eq!(error.message, "throw statement executed: finally");
    assert_eq!(
        eval("let error = 'outer'; try { throw 'inner'; } catch (error) { error; } error;"),
        Ok(Value::String("outer".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let x = 1; let ranCatch = false; \
             try { x = 2; throw new Error(); } \
             catch { let x = 3; let y = true; ranCatch = true; } \
             let yHidden = false; try { y; } catch (error) { yHidden = error instanceof ReferenceError; } \
             ranCatch + ':' + x + ':' + yHidden;"
        ),
        Ok(Value::String("true:2:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var seen; \
             try { { let hidden = 18; throw 25; } } \
             catch (error) { \
               seen = error; \
               (function() { \
                 try { eval('hidden'); seen = 'visible'; } \
                 catch (inner) { seen = inner instanceof ReferenceError ? seen : 'wrong'; } \
               })(); \
             } \
             seen;"
        ),
        Ok(Value::Number(25.0))
    );
    assert_eq!(
        eval(
            "var probeParam, probeBlock; let x = 'outside'; \
             try { throw []; } \
             catch ([_ = probeParam = function() { return x; }]) { \
               probeBlock = function() { return x; }; let x = 'inside'; \
             } \
             probeParam() + ':' + probeBlock();"
        ),
        Ok(Value::String("outside:inside".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var x = 1; var probeBefore = function() { return x; }; \
             var probeTry, probeParam, probeBlock; \
             try { var x = 2; probeTry = function() { return x; }; throw []; } \
             catch ([_ = (eval('var x = 3;'), probeParam = function() { return x; })]) { \
               var x = 4; probeBlock = function() { return x; }; \
             } \
             probeBefore() + ':' + probeTry() + ':' + probeParam() + ':' + probeBlock() + ':' + x;"
        ),
        Ok(Value::String("4:4:4:4:4".to_owned().into()))
    );
    assert_eq!(
        eval("try { throw { marker: 7 }; } catch ({ marker }) { marker; }"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let marker = 'outer'; try { throw { marker: 'inner' }; } catch ({ marker }) { marker; } marker;"
        ),
        Ok(Value::String("outer".to_owned().into()))
    );
    assert_eq!(
        eval("try { throw 'thrown'; } catch (foo) { var foo = 'initializer in catch'; foo; }"),
        Ok(Value::String("initializer in catch".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function capturedFoo() { return foo; } foo = 'prior to throw'; try { throw new Error(); } catch (foo) { var foo = 'initializer in catch'; } capturedFoo();"
        ),
        Ok(Value::String("prior to throw".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let before, during, after; try { throw 'exception'; } catch (err) { before = err; for (var err = 'loop initializer'; err !== 'increment'; err = 'increment') { during = err; } after = err; } before + ',' + during + ',' + after;"
        ),
        Ok(Value::String(
            "exception,loop initializer,increment".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let before, during, after; try { throw 'exception'; } catch (err) { before = err; for (var err in { propertyName: null }) { during = err; } after = err; } before + ',' + during + ',' + after;"
        ),
        Ok(Value::String(
            "exception,propertyName,propertyName".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let before, during, after; try { throw 'exception'; } catch (err) { before = err; for (var err of [2]) { during = err; } after = err; } before + ',' + during + ',' + after;"
        ),
        Ok(Value::String("exception,2,2".to_owned().into()))
    );
}

#[test]
fn evaluates_debugger_statement_as_noop() {
    assert_eq!(eval("debugger; 1;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("let x = 0; if (true) debugger; x = 2; x;"),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn evaluates_destructuring_loop_heads() {
    assert_eq!(
        eval(
            "var out = []; for (const [a, b = 10] of [[1, 2], [3]]) { out.push(a + ':' + b); }
             out.join(',');"
        ),
        Ok(Value::String("1:2,3:10".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var out = []; for (var {x, y = 9} of [{x: 1}, {x: 2, y: 3}]) { out.push(x + ':' + y); }
             out.join(',');"
        ),
        Ok(Value::String("1:9,2:3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var a, b, out = []; for ([a, b] of [[1, 2], [3, 4]]) { out.push(a + b); } out.join(',');"
        ),
        Ok(Value::String("3,7".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var out = []; for (const [first, second] in {ab: 1}) { out.push(first + second); } out.join(',');"
        ),
        Ok(Value::String("ab".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var o = {}, out = []; for ({length: o.len} of ['x', 'xy']) { out.push(o.len); } out.join(',');"
        ),
        Ok(Value::String("1,2".to_owned().into()))
    );
}

#[test]
fn for_of_lexical_heads_capture_fresh_binding_per_iteration() {
    assert_eq!(
        eval(
            "let f = [];
             for (let x of [1, 2, 3]) { f[x - 1] = function() { return x; }; }
             f[0]() + ':' + f[1]() + ':' + f[2]();"
        ),
        Ok(Value::String("1:2:3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let f = [];
             for (const x of [1, 2, 3]) { f[x - 1] = function() { return x; }; }
             f[0]() + ':' + f[1]() + ':' + f[2]();"
        ),
        Ok(Value::String("1:2:3".to_owned().into()))
    );
}

#[test]
fn for_of_lexical_head_capture_scope_does_not_leak_after_loop() {
    assert_eq!(
        eval(
            "(function() {
               return eval('var out = []; function readLater() { f; } \
                 for (let f of [0]) {{ function f() {} }} \
                 try { readLater(); out.push(\"captured\"); } \
                 catch (error) { out.push(error.name); } \
                 try { (function() { f; })(); out.push(\"post-loop\"); } \
                 catch (error) { out.push(error.name); } \
                 out.push(typeof f); out.join(\":\");');
             }());"
        ),
        Ok(Value::String(
            "ReferenceError:ReferenceError:undefined".to_owned().into()
        ))
    );
}

#[test]
fn nested_for_of_destructuring_assignments_capture_later_script_lexicals_tdz() {
    assert_eq!(
        eval(
            "var result = '';
             try { (function() { for ([x] of [[]]) { result = 'body'; } result = 'after'; })(); }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             let x;
             result;"
        ),
        Ok(Value::String("reference".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var x;
             var result = '';
             try { (function() { for ([x = y] of [[]]) { result = 'body'; } result = 'after'; })(); }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             let y;
             result;"
        ),
        Ok(Value::String("reference".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var result = '';
             try { (function() { for ([...x] of [[]]) { result = 'body'; } result = 'after'; })(); }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             let x;
             result;"
        ),
        Ok(Value::String("reference".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var result = '';
             try { (function() { for ({x} of [{}]) { result = 'body'; } result = 'after'; })(); }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             let x;
             result;"
        ),
        Ok(Value::String("reference".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var x;
             var result = '';
             try { (function() { for ({x = y} of [{}]) { result = 'body'; } result = 'after'; })(); }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             let y;
             result;"
        ),
        Ok(Value::String("reference".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var result = '';
             try { (function() { for ({a: x} of [{}]) { result = 'body'; } result = 'after'; })(); }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             let x;
             result;"
        ),
        Ok(Value::String("reference".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var x;
             var result = '';
             try { (function() { for ({a: x = y} of [{}]) { result = 'body'; } result = 'after'; })(); }
             catch (error) { result = error instanceof ReferenceError ? 'reference' : error.name; }
             let y;
             result;"
        ),
        Ok(Value::String("reference".to_owned().into()))
    );
}

#[test]
fn for_of_closes_iterator_on_abrupt_exits() {
    let source_prefix = "
        function makeIterable(log) {
          var iterable = {};
          iterable[Symbol.iterator] = function() {
            var n = 0;
            return {
              next: function() { n += 1; log.push('next' + n); return { value: n, done: n > 5 }; },
              return: function() { log.push('return'); return {}; }
            };
          };
          return iterable;
        }";
    assert_eq!(
        eval(&format!(
            "{source_prefix}
             var log = []; for (var v of makeIterable(log)) {{ if (v === 2) break; }} log.join(',');"
        )),
        Ok(Value::String("next1,next2,return".to_owned().into()))
    );
    assert_eq!(
        eval(&format!(
            "{source_prefix}
             var log = [];
             try {{ for (var v of makeIterable(log)) {{ if (v === 2) throw new Error('x'); }} }}
             catch (error) {{}}
             log.join(',');"
        )),
        Ok(Value::String("next1,next2,return".to_owned().into()))
    );
    assert_eq!(
        eval(&format!(
            "{source_prefix}
             var log = [];
             (function() {{ for (var v of makeIterable(log)) {{ if (v === 2) return; }} }})();
             log.join(',');"
        )),
        Ok(Value::String("next1,next2,return".to_owned().into()))
    );
    assert_eq!(
        eval(&format!(
            "{source_prefix}
             var log = [];
             outer: for (var a of makeIterable(log)) {{
               for (var b of makeIterable(log)) {{ break outer; }}
             }}
             log.join(',');"
        )),
        Ok(Value::String("next1,next1,return,return".to_owned().into()))
    );
    assert_eq!(
        eval(&format!(
            "{source_prefix}
             var log = []; var sum = 0;
             for (var v of makeIterable(log)) {{ if (v === 2) continue; sum += v; }}
             sum + '|' + log.join(',');"
        )),
        Ok(Value::String(
            "13|next1,next2,next3,next4,next5,next6".to_owned().into()
        ))
    );
}

#[test]
fn evaluates_catch_parameter_patterns() {
    assert_eq!(
        eval(
            "var out = '';
             try { throw {code: 7, extra: 1}; }
             catch ({code, missing = 'none', ...rest}) {
               out = code + ':' + missing + ':' + Object.keys(rest).join('|');
             }
             out;"
        ),
        Ok(Value::String("7:none:extra".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var out = 0;
             try { throw [1, [2, 3]]; }
             catch ([a, [b, c = 9]]) { out = a + b + c; }
             out;"
        ),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval(
            "var caught = '';
             try { try { throw null; } catch ({a}) {} }
             catch (error) { caught = 'rethrown'; }
             caught;"
        ),
        Ok(Value::String("rethrown".to_owned().into()))
    );
}

#[test]
fn finally_override_supersedes_pending_completion() {
    // An inner `finally { throw }` overriding the exception it interrupted must
    // not leave the original exception pending to be re-raised by an outer
    // finally; the catch sees the override and execution continues.
    assert_eq!(
        eval(
            "var log = [];
             try { try { throw 'ex2'; } finally { throw 'ex3'; } }
             catch (e) { log.push('caught:' + e); }
             finally { log.push('finally'); }
             log.push('after'); log.join(',');"
        ),
        Ok(Value::String("caught:ex3,finally,after".to_owned().into()))
    );
    // A finally-throw overriding a `break` does not corrupt the value stack.
    assert_eq!(
        eval(
            "var r = [];
             for (var i = 0; i < 2; i++) {
               try { try { r.push('t'); break; } finally { throw 'f'; } }
               catch (e) { r.push('c' + e); }
             }
             r.join(',');"
        ),
        Ok(Value::String("t,cf,t,cf".to_owned().into()))
    );
}
