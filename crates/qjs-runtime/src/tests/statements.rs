use crate::{Value, eval};

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
fn evaluates_variable_declaration_rest_destructuring() {
    assert_eq!(
        eval("let [first, ...others] = [1, 2, 3]; first + ':' + others.join('|');"),
        Ok(Value::String("1:2|3".to_owned()))
    );
    assert_eq!(
        eval("const {p, ...rest} = {p: 1, q: 2, r: 3}; p + ':' + Object.keys(rest).join('|');"),
        Ok(Value::String("1:q|r".to_owned()))
    );
    assert_eq!(
        eval("var g = function*() { yield 1; yield 2; }; let [x, y] = g(); x + y;"),
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
        Ok(Value::String("next1,next2,next3,return|1|3".to_owned()))
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
        Ok(Value::String("next1,next2,next3|1|2".to_owned()))
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
        Ok(Value::String("boom:false".to_owned()))
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
        Ok(Value::String("dflt:true".to_owned()))
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
        Ok(Value::String("true:true".to_owned()))
    );
}

#[test]
fn destructuring_defaults_do_not_treat_html_dda_as_undefined() {
    assert_eq!(
        eval(
            "let initCount = 0; const counter = function() { initCount += 1; }; const [x = counter()] = [__quickjsRustIsHTMLDDA]; (x === __quickjsRustIsHTMLDDA) + ':' + initCount;"
        ),
        Ok(Value::String("true:0".to_owned()))
    );
    assert_eq!(
        eval(
            "let initCount = 0; const counter = function() { initCount += 1; }; const {x = counter()} = {x: __quickjsRustIsHTMLDDA}; (x === __quickjsRustIsHTMLDDA) + ':' + initCount;"
        ),
        Ok(Value::String("true:0".to_owned()))
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
        Ok(Value::String("own:inherited:".to_owned()))
    );
    assert_eq!(
        eval(
            "Object.defineProperty(Boolean.prototype, 'visible', { value: 1, enumerable: true, configurable: true }); let seen = false; for (var key in new Boolean()) { if (key === 'visible') seen = true; } delete Boolean.prototype.visible; seen;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("var stored; for (var key = 0 in stored = key, { a: 1 }) {} stored + ':' + key;"),
        Ok(Value::String("0:a".to_owned()))
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
        Ok(Value::String("ab".to_owned()))
    );
    assert_eq!(
        eval(
            "let seen = ''; for (let entry of new Map([['a', 1], ['b', 2]])) { seen = seen + entry[0] + entry[1]; } seen;"
        ),
        Ok(Value::String("a1b2".to_owned()))
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
        Ok(Value::String("caught".to_owned()))
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
        Ok(Value::String("outer".to_owned()))
    );
    assert_eq!(
        eval("try { throw { marker: 7 }; } catch ({ marker }) { marker; }"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let marker = 'outer'; try { throw { marker: 'inner' }; } catch ({ marker }) { marker; } marker;"
        ),
        Ok(Value::String("outer".to_owned()))
    );
    assert_eq!(
        eval("try { throw 'thrown'; } catch (foo) { var foo = 'initializer in catch'; foo; }"),
        Ok(Value::String("initializer in catch".to_owned()))
    );
    assert_eq!(
        eval(
            "function capturedFoo() { return foo; } foo = 'prior to throw'; try { throw new Error(); } catch (foo) { var foo = 'initializer in catch'; } capturedFoo();"
        ),
        Ok(Value::String("prior to throw".to_owned()))
    );
    assert_eq!(
        eval(
            "let before, during, after; try { throw 'exception'; } catch (err) { before = err; for (var err = 'loop initializer'; err !== 'increment'; err = 'increment') { during = err; } after = err; } before + ',' + during + ',' + after;"
        ),
        Ok(Value::String(
            "exception,loop initializer,increment".to_owned()
        ))
    );
    assert_eq!(
        eval(
            "let before, during, after; try { throw 'exception'; } catch (err) { before = err; for (var err in { propertyName: null }) { during = err; } after = err; } before + ',' + during + ',' + after;"
        ),
        Ok(Value::String(
            "exception,propertyName,propertyName".to_owned()
        ))
    );
    assert_eq!(
        eval(
            "let before, during, after; try { throw 'exception'; } catch (err) { before = err; for (var err of [2]) { during = err; } after = err; } before + ',' + during + ',' + after;"
        ),
        Ok(Value::String("exception,2,2".to_owned()))
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
