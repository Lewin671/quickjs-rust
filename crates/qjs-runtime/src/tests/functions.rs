use crate::{Value, eval};

#[test]
fn evaluates_function_declarations_and_calls() {
    assert_eq!(
        eval("function add(a, b) { return a + b; } add(2, 3);"),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval(
            "let result = callBeforeDeclaration(); function callBeforeDeclaration() { return 11; } result;"
        ),
        Ok(Value::Number(11.0))
    );
    assert_eq!(
        eval("function outer() { return inner(); function inner() { return 13; } } outer();"),
        Ok(Value::Number(13.0))
    );
    assert_eq!(
        eval("let result; { result = inside(); function inside() { return 17; } } result;"),
        Ok(Value::Number(17.0))
    );
    assert_eq!(
        eval(
            "function g() { { function f() { return 1; } { function f() { return 2; } } } return f(); } g();"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "(function() { let before = arguments.toString(); let during; { during = arguments(); function arguments() {} } return before + ':' + during + ':' + arguments.toString(); }());"
        ),
        Ok(Value::String(
            "[object Arguments]:undefined:[object Arguments]"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(
        eval("function deleted() { return delete arguments; } deleted(1, 2);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("function shadow(arguments) { return arguments; } shadow(42);"),
        Ok(Value::Number(42.0))
    );
    assert_eq!(
        eval("function shadow(arguments) { return typeof arguments; } shadow();"),
        Ok(Value::String("undefined".to_owned().into()))
    );
    assert_eq!(
        eval("function shadow({ arguments }) { return arguments; } shadow({ arguments: 42 });"),
        Ok(Value::Number(42.0))
    );
    assert_eq!(
        eval(
            "var after; eval('if (true) function f() { return \"declaration\"; } else function _f() {} after = f;'); after();"
        ),
        Ok(Value::String("declaration".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var after = 'unchanged'; eval('if (false) function f() { return \"no\"; } else function _f() { return \"alternate\"; } after = _f;'); after();"
        ),
        Ok(Value::String("alternate".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var after; eval('switch (1) { default: function f() { return \"switch\"; } } after = f;'); after();"
        ),
        Ok(Value::String("switch".to_owned().into()))
    );
    assert_eq!(
        eval("function first(a) { return a; } first();"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("function first(a) { return a; } first(1, 2);"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("function arg(index) { return arguments[index]; } arg(1, 2, 3);"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("function count() { return arguments.length; } count(1, 2, 3);"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "function countEval() { return eval('arguments.length + arguments[1]'); } countEval(1, 2, 3);"
        ),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval("function keys() { return Object.keys(arguments).join('|'); } keys(1, 2, 3);"),
        Ok(Value::String("0|1|2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function values() { let seen = ''; for (var value of arguments) { seen += value; } return seen; } values('a', 'b', 'c');"
        ),
        Ok(Value::String("abc".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function values() { let descriptor = Object.getOwnPropertyDescriptor(arguments, Symbol.iterator); return (typeof descriptor.value) + ':' + descriptor.enumerable + ':' + descriptor.writable + ':' + descriptor.configurable; } values(1);"
        ),
        Ok(Value::String("function:false:true:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function values() { let seen = ''; for (var value of arguments) { seen += value; arguments[1] = 'z'; } return seen; } values('a', 'b');"
        ),
        Ok(Value::String("az".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function values(a, b, c) { let seen = ''; for (var value of arguments) { a = b; b = c; c = 1; seen += value; } return seen; } values(1, 2, 3);"
        ),
        Ok(Value::String("131".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function values(a, b) { arguments[0] = 'x'; return a + ':' + arguments[0]; } values('a', 'b');"
        ),
        Ok(Value::String("x:x".to_owned().into()))
    );
    assert_eq!(
        eval("let args = (function(a) { arguments[0] = 'x'; return arguments; })('a'); args[0];"),
        Ok(Value::String("x".to_owned().into()))
    );
    assert_eq!(
        eval("let args = (function(a) { return arguments; })('a'); args[0] = 'x'; args[0];"),
        Ok(Value::String("x".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function makeCounter() { let index = 0; return function() { index = index + 1; return index; }; } let next = makeCounter(); next() + ':' + next();"
        ),
        Ok(Value::String("1:2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let calls = 0; function callback() { calls = calls + 1; return calls; } function helper(fn) { return fn(); } helper(function() { return callback(); }) + ':' + calls;"
        ),
        Ok(Value::String("1:1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function makePair() { let index = 0; return [function() { index = index + 1; return index; }, function() { index = index + 1; return index; }]; } let pair = makePair(); pair[0]() + ':' + pair[1]() + ':' + pair[0]();"
        ),
        Ok(Value::String("1:2:3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function values(a, a) { arguments[0] = 'x'; arguments[1] = 'y'; return arguments[0] + ':' + a; } values('a', 'b');"
        ),
        Ok(Value::String("x:y".to_owned().into()))
    );
    assert_eq!(
        eval("function none() { return arguments.length; } none();"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let left = function() {}; let right = function() {}; (left === left) + ':' + (left === right);"
        ),
        Ok(Value::String("true:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let target = function() {}; function values(a, b) { arguments[2] = function() {}; return Array.prototype.lastIndexOf.call(arguments, target) + ':' + Array.prototype.lastIndexOf.call(arguments, arguments[2]); } values(0, target);"
        ),
        Ok(Value::String("1:-1".to_owned().into()))
    );
    assert_eq!(
        eval("function pair(a, b) { return b; } pair(1);"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("function pair(a, b) { return arguments[2]; } pair(1, 2, 3);"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("function pair(a, b) {} pair.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "function collect(first, ...rest) { return first + ':' + rest.join('|'); } collect('a', 'b', 'c');"
        ),
        Ok(Value::String("a:b|c".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function collect(...rest) { return rest.length + ':' + Array.isArray(rest); } collect();"
        ),
        Ok(Value::String("0:true".to_owned().into()))
    );
    assert_eq!(
        eval("function collect(first, ...rest) {} collect.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("function pick(a, b = 4) { return a + b; } pick(3);"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("function pick(a, b = 4) { return a + b; } pick(3, 5);"),
        Ok(Value::Number(8.0))
    );
    assert_eq!(
        eval("function pick(a, b = a + 1) { return b; } pick(3);"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("function pick(a, b = 4) {} pick.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("function pick(a = 1) { arguments[0] = 9; return a; } pick(undefined);"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("((first, ...rest) => first + rest[1])('a', 'b', 'c');"),
        Ok(Value::String("ac".to_owned().into()))
    );
    assert_eq!(eval("((a, b = a + 1,) => b)(3);"), Ok(Value::Number(4.0)));
    assert_eq!(
        eval("((a, b = 4) => a + b)(3, undefined);"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(eval("((a, b = 4) => a + b)(3, 5);"), Ok(Value::Number(8.0)));
    assert_eq!(
        eval("Function.prototype.toString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Function.prototype, 'toString'); (d.value === Function.prototype.toString) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let a = Object.getOwnPropertyDescriptor(Function.prototype, 'arguments'); let c = Object.getOwnPropertyDescriptor(Function.prototype, 'caller'); (typeof a.get) + ':' + (a.get === a.set) + ':' + (a.get === c.get) + ':' + a.enumerable + ':' + a.configurable;"
        ),
        Ok(Value::String(
            "function:true:true:false:true".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let got = false; let set = false; try { Function.prototype.arguments; } catch (error) { got = error instanceof TypeError; } try { Function.prototype.arguments = 1; } catch (error) { set = error instanceof TypeError; } got + ':' + set;"
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
    assert_eq!(
        eval("Function.prototype.toString.call(Array.isArray).includes('[native code]');"),
        Ok(Value::Boolean(true))
    );
    // A callable Proxy is an acceptable toString receiver and uses a
    // NativeFunction-shaped representation instead of exposing target source.
    assert_eq!(
        eval(
            "let p = new Proxy(function foo() {}, {}); \
             let source = Function.prototype.toString.call(p); \
             source.includes('[native code]') && source.startsWith('function');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "Function.prototype.toString.call(new Proxy(Math.max, {})).includes('[native code]');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let p = new Proxy(function() {}, {}); \
             let source = '' + p; \
             source.includes('[native code]') && source.startsWith('function');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let fn = function() { return 'ok'; }; \
             let p = new Proxy(fn, {}); \
             let object = { [Symbol.toPrimitive]: p }; \
             String(object);"
        ),
        Ok(Value::String("ok".to_owned().into()))
    );
    // A non-callable receiver still throws a TypeError.
    assert_eq!(
        eval(
            "try { Function.prototype.toString.call({}); 'no-throw'; } \
             catch (error) { error.constructor.name; }"
        ),
        Ok(Value::String("TypeError".to_owned().into()))
    );
    // A user function returns its original source text verbatim.
    assert_eq!(
        eval("function foo(a, b) { return a + b; } foo.toString();"),
        Ok(Value::String(
            "function foo(a, b) { return a + b; }".to_owned().into()
        ))
    );
    assert_eq!(
        eval("var f = (x) => x * 2; f.toString();"),
        Ok(Value::String("(x) => x * 2".to_owned().into()))
    );
    // Source is retained through nested function compilation, and generators
    // reproduce their `function*` form.
    assert_eq!(
        eval(
            "function outer() { function inner(z) { return z; } return inner; } outer().toString();"
        ),
        Ok(Value::String(
            "function inner(z) { return z; }".to_owned().into()
        ))
    );
    assert_eq!(
        eval("function* gen() { yield 1; } gen.toString();"),
        Ok(Value::String(
            "function* gen() { yield 1; }".to_owned().into()
        ))
    );
    // CR and CRLF line terminators in the source are retained verbatim.
    assert_eq!(
        eval("eval('var f = function () {\\r\\n  return 1;\\r};'); f.toString();"),
        Ok(Value::String(
            "function () {\r\n  return 1;\r}".to_owned().into()
        ))
    );
    assert_eq!(
        eval("eval('var f = function () {\\r  return 1;\\r};'); f.toString();"),
        Ok(Value::String(
            "function () {\r  return 1;\r}".to_owned().into()
        ))
    );
    // A plain `Function(...)` call inside a constructor ignores the ambient
    // new.target: the created function's prototype is %Function.prototype%, so
    // it has the usual callable methods.
    assert_eq!(
        eval(
            "let applyType; \
             function F() { applyType = typeof Function('return 1;').apply; } \
             new F(); applyType;"
        ),
        Ok(Value::String("function".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function F() { this.g = Function('return 1;'); } \
             Object.getPrototypeOf(new F().g) === Function.prototype;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("delete decodeURI.length; decodeURI.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "function pair(a, b) {} let d = Object.getOwnPropertyDescriptor(pair, 'name'); d.value + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("pair:false:false:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Array.isArray, 'name'); d.value + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("isArray:false:false:true".to_owned().into()))
    );
    assert_eq!(
        eval("function noReturn() { 1 + 2; } noReturn();"),
        Ok(Value::Undefined)
    );
    assert_eq!(eval("1 + 2;"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval(
            "function make(value) { return function() { return value; }; } let get = make(7); get();"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let value = 100; function make(value) { return function() { return value; }; } let get = make(7); get();"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("let value = 1; function read() { return value; } value = 2; read();"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let changed = false; function outer() { function inner() { changed = true; } inner(); } outer(); changed;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let changed = false; function call(callback) { callback(); } call(function() { changed = true; }); changed;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let changed = false; function call(callback) { try { callback(); } catch (error) {} } call(function() { changed = true; throw 1; }); changed;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let called = 0; function C() {} function callback() { called = called + 1; new C(); throw {}; } try { callback(); } catch (error) {} called;"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("let add = function(a, b) { return a + b; }; add(2, 3);"),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval("let f = function named() { return typeof named; }; f();"),
        Ok(Value::String("function".to_owned().into()))
    );
    assert_eq!(
        eval("let f = function named() { return named === f; }; f();"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let f = function hidden() { return 1; }; typeof hidden;"),
        Ok(Value::String("undefined".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let factorial = function fact(n) { return n <= 1 ? 1 : n * fact(n - 1); }; factorial(5);"
        ),
        Ok(Value::Number(120.0))
    );
    assert_eq!(
        eval("(function(value) { return value + 1; })(2);"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(eval("(() => {})();"), Ok(Value::Undefined));
    assert_eq!(eval("(value => value + 1)(2);"), Ok(Value::Number(3.0)));
    assert_eq!(eval("((a, b) => a + b)(2, 3);"), Ok(Value::Number(5.0)));
    assert!(eval("new (() => {})();").is_err());
    assert_eq!(
        eval("function getThis() { return this; } getThis() === this;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function getThis() { 'use strict'; return this; } getThis() === undefined;"),
        Ok(Value::Boolean(true))
    );
    // A "use strict" directive written with a line continuation or an escape
    // computes to the string "use strict" but is NOT a Use Strict Directive, so
    // the function stays sloppy and `this` coerces to the global object.
    assert_eq!(
        eval("function f() { 'use str\\\nict'; return this !== undefined; } f.call(undefined);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function f() { 'use\\u0020strict'; return this !== undefined; } f.call(undefined);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function getThis() { return this; } let o = {}; o.getThis = getThis; o.getThis() === o;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function getGlobal() { return this; } function method() { return getGlobal(); } let o = {}; o.method = method; o.method() === this;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let o = { method: function() { return this.value; }, value: 7 }; o.method();"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "function add(a, b) { return this.base + a + b; } let context = { base: 4 }; add.call(context, 2, 3);"
        ),
        Ok(Value::Number(9.0))
    );
    assert_eq!(
        eval(
            "function add(a, b) { return this.base + a + b; } let context = { base: 4 }; add.apply(context, [2, 3]);"
        ),
        Ok(Value::Number(9.0))
    );
    assert_eq!(
        eval("function count() { return arguments.length; } count.apply(null, undefined);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "function add(a, b) { return a + b; } function caller() { return add.apply(null, arguments); } caller(2, 3);"
        ),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval(
            "var i = 0; var p = { toString: function() { return 'a' + (++i); } }; var obj = {}; new Function(p, p, p, 'this.shifted = a3;').apply(obj, (function() { return arguments; })('a', 'b', 'c')); obj.shifted;"
        ),
        Ok(Value::String("c".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function count(a, b, c) { return arguments.length + ':' + (a === undefined) + ':' + b + ':' + (c === undefined); } count.apply(null, [, 2, ,]);"
        ),
        Ok(Value::String("3:true:2:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let values = [65, 0x1F600, 67]; String.fromCodePoint.apply(null, values).length + ':' + String.fromCodePoint.apply(null, values).charCodeAt(1);"
        ),
        Ok(Value::String("4:55357".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let calls = 0; let values = [{ valueOf() { calls = calls + 1; return 65; } }]; String.fromCodePoint.apply(null, values) + ':' + calls;"
        ),
        Ok(Value::String("A:1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let caught = false; try { String.fromCodePoint.apply(null, [0x110000]); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function fn() {} var caught = ''; for (var i = 0; i < 4; i = i + 1) { var value = i === 0 ? true : (i === 1 ? NaN : (i === 2 ? '1,2,3' : Symbol())); try { fn.apply(null, value); caught = caught + '0'; } catch (error) { caught = caught + (error instanceof TypeError ? '1' : '2'); } } caught;"
        ),
        Ok(Value::String("1111".to_owned().into()))
    );
    assert_eq!(
        eval("function getThis() { return this; } getThis.call(undefined) === this;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function touch() { this.touched = true; return this instanceof Number && this.touched; } touch.call(1);"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function touch() { this.touched = true; return this instanceof Boolean && this.touched; } touch.call(true);"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function touch() { this.touched = true; return this instanceof String && this.touched; } touch.call('x');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function getThis() { 'use strict'; return this; } getThis.call(undefined) === undefined;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function getThis() { 'use strict'; return this; } getThis.call(1) === 1;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function collect(a, b, c) { return this.tag + ':' + a + ':' + b + ':' + c; } \
             let uncurried = Function.prototype.call.bind(collect); \
             let prebound = Function.prototype.call.bind(collect, { tag: 'pre' }, 'a'); \
             let hasOwn = Function.prototype.call.bind(Object.prototype.hasOwnProperty); \
             uncurried({ tag: 'ctx' }, 1, 2, 3) + '|' + prebound('b', 'c') + '|' + hasOwn({ x: 1 }, 'x');"
        ),
        Ok(Value::String("ctx:1:2:3|pre:a:b:c|true".to_owned().into()))
    );
    assert_eq!(
        eval("'use strict'; let getThis = function() { return this; }; getThis() === undefined;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("'use strict'; function getThis() { return this; } getThis() === undefined;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "'use strict'; function outer() { return function() { return this; }; } outer()() === undefined;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function add(a, b) { return a + b; } add.call.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("function add(a, b) { return a + b; } add.apply.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "function add(a, b) { return this.base + a + b; } let context = { base: 4 }; let bound = add.bind(context, 2); bound(3);"
        ),
        Ok(Value::Number(9.0))
    );
    assert_eq!(
        eval("function join(a, b, c) { return '' + a + b + c; } join.bind(null, 'a').length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("function join(a, b, c) { return '' + a + b + c; } join.bind(null, 'a').name;"),
        Ok(Value::String("bound join".to_owned().into()))
    );
    assert_eq!(
        eval("function add(a, b) { return a + b; } add.bind(null, 2).bind(null, 3)();"),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval(
            "function Point(x, y) { this.x = x; this.y = y; } let Bound = Point.bind({ ignored: true }, 2); let point = new Bound(3); point.x + point.y;"
        ),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval(
            "function Point(x) { this.x = x; } let Bound = Point.bind(null, 2); let point = new Bound(); point instanceof Point;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function add(a, b) { return a + b; } Object.hasOwn(add.bind(null), 'prototype');"),
        Ok(Value::Boolean(false))
    );
    // A function's own `prototype` is writable and non-enumerable but
    // non-configurable, so a (sloppy) delete is a no-op.
    assert_eq!(
        eval(
            "function f() {} let d = Object.getOwnPropertyDescriptor(f, 'prototype'); \
             d.writable + ':' + d.enumerable + ':' + d.configurable + ':' + (delete f.prototype) + ':' + ('prototype' in f);"
        ),
        Ok(Value::String(
            "true:false:false:false:true".to_owned().into()
        ))
    );
    // `Proxy` is constructable yet exposes no own `prototype`.
    assert_eq!(
        eval("Object.hasOwn(Proxy, 'prototype');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "function f() {} let data = 'data'; Object.defineProperty(Function.prototype, 'prop', { get: function() { return data; }, set: function(value) { data = value; }, configurable: true }); let bound = f.bind({}); bound.prop = 'overrideData'; let result = bound.hasOwnProperty('prop') + ':' + bound.prop + ':' + data; delete Function.prototype.prop; result;"
        ),
        Ok(Value::String(
            "false:overrideData:overrideData".to_owned().into()
        ))
    );
    assert_eq!(
        eval("function add(a, b) { return a + b; } add.call.propertyIsEnumerable('length');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Function.prototype.constructor === Function;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getPrototypeOf(function() {}) === Function.prototype;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Function.prototype.isPrototypeOf(function() {});"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Function.prototype, Symbol.hasInstance); typeof d.value + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String(
            "function:false:false:false".to_owned().into()
        ))
    );
    assert_eq!(
        eval("let f = Function.prototype[Symbol.hasInstance]; f.length + ':' + f.name;"),
        Ok(Value::String("1:[Symbol.hasInstance]".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function C() {} let instance = new C(); Function.prototype[Symbol.hasInstance].call(C, instance) + ':' + Function.prototype[Symbol.hasInstance].call(C, {});"
        ),
        Ok(Value::String("true:false".to_owned().into()))
    );
    assert_eq!(
        eval("Function.prototype[Symbol.hasInstance].call({}, {});"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "function C() {} let Bound = C.bind(null); let instance = new C(); Function.prototype[Symbol.hasInstance].call(Bound, instance);"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function F() {} let s = Symbol(); F[s] = 1; let before = Object.getOwnPropertySymbols(F)[0] === s; let descriptor = Object.getOwnPropertyDescriptor(F, s); delete F[s]; before + ':' + descriptor.value + ':' + descriptor.enumerable + ':' + Object.hasOwn(F, s);"
        ),
        Ok(Value::String("true:1:true:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function make(value) { return function() { return ({ value: value }).hasOwnProperty('value'); }; } make(1)();"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function make(value) { return function() { return [value, 2].join('|'); }; } make(1)();"
        ),
        Ok(Value::String("1|2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function outer(value) { return function inner() { function nested() { return value + 1; } return nested(); }; } outer(4)();"
        ),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval("let add = Function('a', 'b', 'return a + b;'); add(2, 3);"),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval("let add = new Function('a,b', 'return a + b;'); add.length + ':' + add(2, 3);"),
        Ok(Value::String("2:5".to_owned().into()))
    );
    assert_eq!(
        eval("Function('return 1;').name;"),
        Ok(Value::String("anonymous".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var i = 0; var p = { toString: function() { return 'a' + (++i); } }; let f = new Function(p, p, p, 'return a1 + a2 + a3;'); f('a', 'b', 'c');"
        ),
        Ok(Value::String("abc".to_owned().into()))
    );
    assert_eq!(
        eval("let f = Function('this.value = 7;'); let o = {}; f.call(o); o.value;"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("let C = Function('x', 'this.x = x;'); let c = new C(9); c.x;"),
        Ok(Value::Number(9.0))
    );
    assert_eq!(eval("Function('<!--'); 1;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Function('-->'); 1;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Function('<!--', ''); 1;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Function('\\n-->', ''); 1;"), Ok(Value::Number(1.0)));
    assert!(eval("Function('-->', '');").is_err());
    assert_eq!(
        eval(
            "let caught = false; try { Function('-->', ''); } catch (error) { caught = error instanceof SyntaxError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { Function('#!\\n_', ''); } catch (error) { caught = error instanceof SyntaxError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { Function('#!\\n_'); } catch (error) { caught = error instanceof SyntaxError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let ctors = [Function, (async function(){}).constructor, (function*(){}).constructor, (async function*(){}).constructor]; \
             ctors.every(function(ctor) { \
               try { ctor('#!\\n_', ''); return false; } catch (error) { return error instanceof SyntaxError; } \
             });"
        ),
        Ok(Value::Boolean(true))
    );
    assert!(eval("Function('a +', 'return a;');").is_err());
    assert!(eval("Function('break;');").is_err());
}

#[test]
fn sloppy_function_this_keeps_internal_global_identity() {
    assert_eq!(
        eval(
            "var originalGlobal = this; \
             globalThis = { replacement: true }; \
             function getThis() { return this; } \
             getThis() === originalGlobal && getThis() !== globalThis;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn direct_leaf_call_does_not_inherit_caller_catch_binding() {
    assert_eq!(
        eval(
            "var error = 'global';
             function readError() { return error; }
             try { throw 'caught'; } catch (error) { readError(); }"
        ),
        Ok(Value::String("global".to_owned().into()))
    );
}

#[test]
fn direct_leaf_vm_call_preserves_caller_slot_and_global_write() {
    assert_eq!(
        eval(
            "var total = 1; \
             function leaf(value) { total = total + value; value = 99; return value; } \
             function caller() { \
               let value = 3; \
               let returned = leaf(value); \
               return value + ':' + returned + ':' + total; \
             } \
             caller();"
        ),
        Ok(Value::String("3:99:4".to_owned().into()))
    );
}

#[test]
fn numeric_leaf_falls_back_for_coercion_without_duplicate_effects() {
    assert_eq!(
        eval(
            "var coercions = 0; \
             function addOne(value) { return value + 1; } \
             var object = { valueOf: function() { coercions++; return 4; } }; \
             addOne(2) + ':' + addOne('x') + ':' + addOne(object) + ':' + coercions;"
        ),
        Ok(Value::String("3:x1:5:1".to_owned().into()))
    );
}

#[test]
fn numeric_leaf_preserves_direct_arithmetic_and_comparisons() {
    assert_eq!(
        eval(
            "function sub(a, b) { return a - b; } \
             function mul(a, b) { return a * b; } \
             function div(a, b) { return a / b; } \
             function rem(a, b) { return a % b; } \
             function lt(a, b) { return a < b; } \
             function strictEq(a, b) { return a === b; } \
             sub(7, 2) + ':' + mul(3, 4) + ':' + div(9, 2) + ':' + \
               rem(9, 4) + ':' + lt(2, 3) + ':' + strictEq(NaN, NaN);"
        ),
        Ok(Value::String("5:12:4.5:1:true:false".to_owned().into()))
    );
}

#[test]
fn numeric_leaf_falls_back_for_duplicate_parameter_slots() {
    assert_eq!(
        eval("function duplicate(value, value) { return value + 1; } duplicate(2, 4);"),
        Ok(Value::Number(5.0))
    );
}

#[test]
fn numeric_leaf_falls_back_when_scratch_stack_is_too_deep() {
    assert_eq!(
        eval(
            "function deep(value) { \
               return value + (1 + (2 + (3 + (4 + (5 + (6 + (7 + (8 + (9 + \
                 (10 + (11 + (12 + (13 + (14 + (15 + (16 + 17)))))))))))))))); \
             } \
             deep(0);"
        ),
        Ok(Value::Number(153.0))
    );
}

#[test]
fn numeric_leaf_commits_captured_writes_before_later_fallbacks() {
    assert_eq!(
        eval(
            "function makeCounter() { \
               var captured = 0; \
               return function(value) { captured += value; return captured; }; \
             } \
             var counter = makeCounter(); \
             counter(2) + ':' + counter(3) + ':' + counter('x') + ':' + counter(1);"
        ),
        Ok(Value::String("2:5:5x:5x1".to_owned().into()))
    );
}

#[test]
fn numeric_leaf_rolls_back_scratch_writes_before_opcode_fallback() {
    assert_eq!(
        eval(
            "function makeCounter() { \
               var captured = 0; \
               return function() { captured += 1; return Math.abs(captured); }; \
             } \
             var counter = makeCounter(); \
             counter() + ':' + counter();"
        ),
        Ok(Value::String("1:2".to_owned().into()))
    );
}

#[test]
fn arrow_captures_new_target_at_creation() {
    assert_eq!(
        eval(
            "var calls = 0; \
             function F() { \
               if ((() => new.target)() === F) calls++; \
               return () => new.target; \
             } \
             var plain = F(); \
             var constructed; \
             function Capture() { constructed = F.call(this); } \
             Reflect.construct(F, [], F); \
             calls + ':' + (plain() === undefined);"
        ),
        Ok(Value::String("1:true".to_owned().into()))
    );
}

#[test]
fn evaluates_spread_call_arguments() {
    assert_eq!(
        eval(
            "function collect(a, b, c, d) { return '' + a + b + c + d; } collect(0, ...[1, 2], 3);"
        ),
        Ok(Value::String("0123".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let receiver = { base: 5, add: function(a, b) { return this.base + a + b; } }; receiver.add(...[2, 3]);"
        ),
        Ok(Value::Number(10.0))
    );
    assert_eq!(
        eval("function Pair(a, b) { this.sum = a + b; } new Pair(...[4, 6]).sum;"),
        Ok(Value::Number(10.0))
    );
}

#[test]
fn evaluates_sloppy_undeclared_global_assignment() {
    assert_eq!(
        eval("function set() { sloppyGlobal = 7; } set(); sloppyGlobal + this.sloppyGlobal;"),
        Ok(Value::Number(14.0))
    );
    assert_eq!(
        eval("let set = () => { arrowGlobal = 3; }; set(); arrowGlobal + this.arrowGlobal;"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval(
            "function set() { descriptorGlobal = 1; } set(); let d = Object.getOwnPropertyDescriptor(this, 'descriptorGlobal'); d.value + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("1:true:true:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function set() { 'use strict'; strictMissing = 1; } let caught = false; try { set(); } catch (error) { caught = error instanceof ReferenceError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function makeCounter() { let index = 0; return function() { index = index + 1; return index; }; } let left = makeCounter(); let right = makeCounter(); left() + ':' + right() + ':' + left();"
        ),
        Ok(Value::String("1:1:2".to_owned().into()))
    );
}

#[test]
fn evaluates_arrow_functions_with_lexical_this() {
    assert_eq!(
        eval(
            "var same = __quickjsRustAssertSameValue; \
             class C { \
               *#m() { return 42; } \
               get ref() { return this.#m; } \
               constructor() { \
                 same(this.#m, (() => this)().#m); \
                 var result = this.#m().next(); \
               } \
             } \
             var instance = new C(); \
             var result = instance.ref().next(); \
             result.value;"
        ),
        Ok(Value::Number(42.0))
    );
    assert_eq!(
        eval(
            "this.marker = 'global'; let receiver = { marker: 'receiver' }; let read = () => this.marker; read.call(receiver);"
        ),
        Ok(Value::String("global".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let receiver = { marker: 'receiver' }; let read = function() { return this.marker; }; read.call(receiver);"
        ),
        Ok(Value::String("receiver".to_owned().into()))
    );
    assert_eq!(
        eval(
            "this.marker = 'global'; let receiver = { marker: 'receiver' }; [1].map(() => this.marker, receiver)[0];"
        ),
        Ok(Value::String("global".to_owned().into()))
    );
    assert_eq!(
        eval(
            "this.marker = 'global'; let receiver = { marker: 'receiver' }; let seen; new Set([1]).forEach(() => { seen = this.marker; }, receiver); seen;"
        ),
        Ok(Value::String("global".to_owned().into()))
    );
}

#[test]
fn direct_eval_keeps_function_context_in_leaf_calls() {
    assert_eq!(
        eval(
            "let receiver = { marker: 7 }; \
             function direct() { return eval('this.marker'); } \
             direct.call(receiver);"
        ),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn leaf_calls_seed_this_parameters_and_received_upvalues_without_named_bindings() {
    assert_eq!(
        eval(
            "let outer = 5; \
             function leaf(first, missing) { \
               return (this.marker + first + outer) + ':' + typeof missing; \
             } \
             leaf.call({ marker: 2 }, 3);"
        ),
        Ok(Value::String("10:undefined".to_owned().into()))
    );
}

#[test]
fn evaluates_arrow_functions_with_lexical_arguments() {
    assert_eq!(
        eval(
            "function outer(...outerValues) { \
               let marker = 1; \
               return function inner(...innerValues) { \
                 return marker + ':' + innerValues[0] + ':' + outerValues[0]; \
               }; \
             } \
             outer('outer')('inner');"
        ),
        Ok(Value::String("1:inner:outer".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function outer() { let args = arguments; let read = () => arguments; return read() === args; } outer(1, 2);"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function outer() { let read = (value) => arguments[0] + ':' + arguments.length + ':' + value; return read('arrow'); } outer('outer', 'second');"
        ),
        Ok(Value::String("outer:2:arrow".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function outer() { let read = () => { return function() { return arguments[0]; }('inner'); }; return read(); } outer('outer');"
        ),
        Ok(Value::String("inner".to_owned().into()))
    );
}

#[test]
fn bound_function_construct_substitutes_new_target() {
    // `new B()` where `B = A.bind()` constructs `A` with `new.target` set to
    // `A` (the bound function is the new.target, so step 4 substitutes the
    // target), including through chained binds.
    assert_eq!(
        eval("var nt; function A() { nt = new.target; } var B = A.bind(); new B(); nt === A;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "var nt; function A() { nt = new.target; } var C = A.bind().bind(); new C(); nt === A;"
        ),
        Ok(Value::Boolean(true))
    );
    // An explicit `new.target` via Reflect.construct is preserved, not replaced.
    assert_eq!(
        eval(
            "var nt; function A() { nt = new.target; } var B = A.bind(); \
             Reflect.construct(B, [], Object); nt === Object;"
        ),
        Ok(Value::Boolean(true))
    );
    // The instance's prototype comes from the target and bound arguments are
    // still prepended.
    assert_eq!(
        eval(
            "function A(x, y) { this.sum = x + y; } var B = A.bind(null, 10); \
             var o = new B(5); o.sum + ':' + (o instanceof A);"
        ),
        Ok(Value::String("15:true".to_owned().into()))
    );
}

#[test]
fn construct_falls_back_to_marked_object_realm_prototype() {
    assert_eq!(
        eval(
            "let realmPrototype = { realm: 'other-object' }; \
             function C() { return this; } \
             Object.defineProperty(C, '__quickjsRustRealmObjectPrototype', { value: realmPrototype }); \
             C.prototype = null; \
             Object.getPrototypeOf(new C()) === realmPrototype;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn dynamic_function_reads_live_global_bindings() {
    assert_eq!(
        eval(
            "var f = Function.call(this, 'return planet;'); \
             var before = f(); \
             var planet = 'mars'; \
             before + ':' + f();"
        ),
        Ok(Value::String("undefined:mars".to_owned().into()))
    );
}

#[test]
fn dynamic_function_construct_uses_marked_function_realm_prototype() {
    assert_eq!(
        eval(
            "let realmFunctionPrototype = function() {}; \
             function C() {} \
             Object.defineProperty(C, '__quickjsRustRealmFunctionPrototype', { value: realmFunctionPrototype }); \
             C.prototype = null; \
             Object.getPrototypeOf(Reflect.construct(Function, [], C)) === realmFunctionPrototype;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn inherited_primitive_accessors_preserve_receiver() {
    assert_eq!(
        eval(
            "Object.defineProperty(Object.prototype, 'sloppyReceiver', { \
                 configurable: true, \
                 get: function() { return this; } \
             }); \
             let value = (5).sloppyReceiver; \
             let result = typeof value + ':' + (value == 5) + ':' + (value === 5); \
             delete Object.prototype.sloppyReceiver; \
             result;"
        ),
        Ok(Value::String("object:true:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "Object.defineProperty(Object.prototype, 'strictReceiver', { \
                 configurable: true, \
                 get: function() { 'use strict'; return this; } \
             }); \
             let value = (5).strictReceiver; \
             let result = typeof value + ':' + (value === 5); \
             delete Object.prototype.strictReceiver; \
             result;"
        ),
        Ok(Value::String("number:true".to_owned().into()))
    );
}

#[test]
fn evaluates_new_expressions() {
    assert_eq!(
        eval(
            "function Point(x, y) { this.x = x; this.y = y; } let p = new Point(2, 3); p.x + p.y;"
        ),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval("function Empty() { this.value = 9; } let p = new Empty; p.value;"),
        Ok(Value::Number(9.0))
    );
    assert_eq!(
        eval(
            "function Box() { this.value = 1; return { value: 4 }; } let box = new Box(); box.value;"
        ),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("function Box() { this.value = 6; return 1; } let box = new Box(); box.value;"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval(
            "function Args() { this.count = arguments.length; } let args = new Args(1, 2, 3); args.count;"
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("function F() { return new.target; } F() === undefined;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function C() { this.ok = new.target === C; } new C().ok;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function C() {} C.prototype.value = 4; let instance = new C(); instance.value;"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval(
            "function C() { this.value = 9; } C.prototype.value = 4; let instance = new C(); instance.value;"
        ),
        Ok(Value::Number(9.0))
    );
    assert_eq!(
        eval("function C() {} C.prototype = { value: 8 }; let instance = new C(); instance.value;"),
        Ok(Value::Number(8.0))
    );
    assert_eq!(
        eval(
            "let proto = Function(); proto.value = 12; function C() {} C.prototype = proto; let instance = new C(); typeof instance.apply + ':' + instance.value;"
        ),
        Ok(Value::String("function:12".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let proto = Function(); function C() {} C.prototype = proto; let instance = new C(); let caught = false; try { instance.apply(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "C.prototype = { value: 10 }; function C() {} let instance = new C(); instance.value;"
        ),
        Ok(Value::Number(10.0))
    );
    assert_eq!(
        eval("function C() {} C.prototype.value = 4; let instance = new C(); 'value' in instance;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function C() {} C.prototype.constructor === C;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function C() {} let instance = new C(); instance.constructor === C;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let C = function Named() {}; C.prototype.constructor === C;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function C() {} C.prototype = { value: 1 }; let instance = new C(); instance.constructor === Object;"
        ),
        Ok(Value::Boolean(true))
    );
    assert!(eval("new 1;").is_err());
}

#[test]
fn function_prototype_has_empty_name_after_length() {
    // %Function.prototype% exposes an empty, non-writable, non-enumerable,
    // configurable `name`, ordered immediately after `length`.
    assert_eq!(
        eval("JSON.stringify(Object.getOwnPropertyDescriptor(Function.prototype, 'name'));"),
        Ok(Value::String(
            r#"{"value":"","writable":false,"enumerable":false,"configurable":true}"#
                .to_owned()
                .into()
        ))
    );
    assert_eq!(
        eval(
            "var n = Object.getOwnPropertyNames(Function.prototype); n.indexOf('name') === n.indexOf('length') + 1;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn function_prototype_is_callable_function_object() {
    assert_eq!(
        eval(
            "[
               Object.prototype.toString.call(Function.prototype),
               Function.prototype(),
               Function.prototype(null, void 0),
               Object.getPrototypeOf(Function.prototype) === Object.prototype,
               Object.getOwnPropertyDescriptor(Function, 'prototype').writable,
               Object.prototype.hasOwnProperty.call(Function.prototype, Symbol.hasInstance)
             ].join('|');"
        ),
        Ok(Value::String(
            "[object Function]|||true|false|true".to_owned().into()
        ))
    );
}

#[test]
fn bound_function_length_and_name_follow_spec() {
    // Length is derived from the target's `length` via ToIntegerOrInfinity,
    // minus bound args, clamped to >= 0; +Infinity is preserved.
    assert_eq!(
        eval("(function (a, b, c) {}).bind().length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        // bind(0, 0): the first 0 is thisArg, the second is one bound arg, so
        // length is 3 - 1 = 2.
        eval("(function (a, b, c) {}).bind(0, 0).length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "var f = function () {}; Object.defineProperty(f, 'length', { value: Infinity }); f.bind(0, 0).length;"
        ),
        Ok(Value::Number(f64::INFINITY))
    );
    assert_eq!(
        eval(
            "var f = function () {}; Object.defineProperty(f, 'length', { value: NaN }); f.bind().length;"
        ),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "var f = function () {}; Object.defineProperty(f, 'length', { value: 3.66 }); f.bind().length;"
        ),
        Ok(Value::Number(3.0))
    );
    // When the target has no *own* `length` (only an inherited one), the bound
    // length is 0 rather than the inherited value (HasOwnProperty check).
    assert_eq!(
        eval(
            "function bar() {} Object.setPrototypeOf(bar, { length: 42 }); delete bar.length;
             Function.prototype.bind.call(bar, null, 1).length;"
        ),
        Ok(Value::Number(0.0))
    );
    // Name is "bound " + target name; a throwing name getter propagates.
    assert_eq!(
        eval("function foo() {} foo.bind().name;"),
        Ok(Value::String("bound foo".to_owned().into()))
    );
    assert!(
        eval(
            "var t = Object.defineProperty(function () {}, 'name', { get() { throw new TypeError('x'); } });
             t.bind();"
        )
        .is_err()
    );
}

#[test]
fn named_function_expression_name_binding_is_immutable() {
    // A named function expression's own name is an immutable inner binding:
    // assigning to it is a silent no-op in sloppy mode, so the name still
    // refers to the function.
    assert_eq!(
        eval("var ref = function f() { f = 1; return f; }; ref() === ref;"),
        Ok(Value::Boolean(true))
    );
    // In strict mode the assignment is a TypeError.
    assert_eq!(
        eval(
            "'use strict'; var ref = function f() { f = 1; return f; }; \
             try { ref(); 'no throw'; } catch (e) { e.name; }"
        ),
        Ok(Value::String("TypeError".to_owned().into()))
    );
    // A function declaration's name is an ordinary mutable binding.
    assert_eq!(
        eval("function g() { g = 1; return g; } g();"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("'use strict'; function g() { g = 1; return g; } g();"),
        Ok(Value::Number(1.0))
    );
    // A parameter or var that shadows the name restores a mutable binding.
    assert_eq!(
        eval("var ref = function f(f) { f = 1; return f; }; ref(5);"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("var ref = function f() { var f; f = 1; return f; }; ref();"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("let ref = function f() { (() => { f = 1; })(); return f; }; ref() === ref;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let ref = function f() { eval('f = 1'); return f; }; ref() === ref;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "'use strict'; let ref = function f() { try { (() => { f = 1; })(); } catch (error) { return error.name; } }; ref();"
        ),
        Ok(Value::String("TypeError".to_owned().into()))
    );
    assert_eq!(
        eval(
            "'use strict'; let ref = function f() { try { eval('f = 1'); } catch (error) { return error.name; } }; ref();"
        ),
        Ok(Value::String("TypeError".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var f = 'outside'; var probeParams, probeBody; var func = function f(_ = (probeParams = function() { return f; })) { probeBody = function() { return f; }; }; func(); (probeParams() === func) + ':' + (probeBody() === func);"
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
    assert_eq!(
        eval("var ref = function f() { var f; return f; }; ref();"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "var f = 'outside'; var probeBody, setBody; var func = function f(_ = 0) { probeBody = function() { return f; }; setBody = function() { f = null; return f; }; }; func(); (probeBody() === func) + ':' + (setBody() === func) + ':' + (probeBody() === func);"
        ),
        Ok(Value::String("true:true:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "'use strict'; var result; var func = function f(_ = 0) { (function() { try { f = null; result = 'no throw'; } catch (error) { result = error.name; } })(); }; func(); result;"
        ),
        Ok(Value::String("TypeError".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var g = 'outside'; var probeBody, setBody; var func = function* g(_ = 0) { probeBody = function() { return g; }; setBody = function() { g = null; return g; }; }; func().next(); (probeBody() === func) + ':' + (setBody() === func) + ':' + (probeBody() === func);"
        ),
        Ok(Value::String("true:true:true".to_owned().into()))
    );
}
