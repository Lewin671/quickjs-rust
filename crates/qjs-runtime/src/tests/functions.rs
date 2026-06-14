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
            "[object Arguments]:undefined:[object Arguments]".to_owned()
        ))
    );
    assert_eq!(
        eval(
            "var after; eval('if (true) function f() { return \"declaration\"; } else function _f() {} after = f;'); after();"
        ),
        Ok(Value::String("declaration".to_owned()))
    );
    assert_eq!(
        eval(
            "var after = 'unchanged'; eval('if (false) function f() { return \"no\"; } else function _f() { return \"alternate\"; } after = _f;'); after();"
        ),
        Ok(Value::String("alternate".to_owned()))
    );
    assert_eq!(
        eval(
            "var after; eval('switch (1) { default: function f() { return \"switch\"; } } after = f;'); after();"
        ),
        Ok(Value::String("switch".to_owned()))
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
        Ok(Value::String("0|1|2".to_owned()))
    );
    assert_eq!(
        eval(
            "function values() { let seen = ''; for (var value of arguments) { seen += value; } return seen; } values('a', 'b', 'c');"
        ),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval(
            "function values() { let descriptor = Object.getOwnPropertyDescriptor(arguments, Symbol.iterator); return (typeof descriptor.value) + ':' + descriptor.enumerable + ':' + descriptor.writable + ':' + descriptor.configurable; } values(1);"
        ),
        Ok(Value::String("function:false:true:true".to_owned()))
    );
    assert_eq!(
        eval(
            "function values() { let seen = ''; for (var value of arguments) { seen += value; arguments[1] = 'z'; } return seen; } values('a', 'b');"
        ),
        Ok(Value::String("az".to_owned()))
    );
    assert_eq!(
        eval(
            "function values(a, b, c) { let seen = ''; for (var value of arguments) { a = b; b = c; c = 1; seen += value; } return seen; } values(1, 2, 3);"
        ),
        Ok(Value::String("131".to_owned()))
    );
    assert_eq!(
        eval(
            "function values(a, b) { arguments[0] = 'x'; return a + ':' + arguments[0]; } values('a', 'b');"
        ),
        Ok(Value::String("x:x".to_owned()))
    );
    assert_eq!(
        eval("let args = (function(a) { arguments[0] = 'x'; return arguments; })('a'); args[0];"),
        Ok(Value::String("x".to_owned()))
    );
    assert_eq!(
        eval("let args = (function(a) { return arguments; })('a'); args[0] = 'x'; args[0];"),
        Ok(Value::String("x".to_owned()))
    );
    assert_eq!(
        eval(
            "function makeCounter() { let index = 0; return function() { index = index + 1; return index; }; } let next = makeCounter(); next() + ':' + next();"
        ),
        Ok(Value::String("1:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let calls = 0; function callback() { calls = calls + 1; return calls; } function helper(fn) { return fn(); } helper(function() { return callback(); }) + ':' + calls;"
        ),
        Ok(Value::String("1:1".to_owned()))
    );
    assert_eq!(
        eval(
            "function makePair() { let index = 0; return [function() { index = index + 1; return index; }, function() { index = index + 1; return index; }]; } let pair = makePair(); pair[0]() + ':' + pair[1]() + ':' + pair[0]();"
        ),
        Ok(Value::String("1:2:3".to_owned()))
    );
    assert_eq!(
        eval(
            "function values(a, a) { arguments[0] = 'x'; arguments[1] = 'y'; return arguments[0] + ':' + a; } values('a', 'b');"
        ),
        Ok(Value::String("x:y".to_owned()))
    );
    assert_eq!(
        eval("function none() { return arguments.length; } none();"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let left = function() {}; let right = function() {}; (left === left) + ':' + (left === right);"
        ),
        Ok(Value::String("true:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let target = function() {}; function values(a, b) { arguments[2] = function() {}; return Array.prototype.lastIndexOf.call(arguments, target) + ':' + Array.prototype.lastIndexOf.call(arguments, arguments[2]); } values(0, target);"
        ),
        Ok(Value::String("1:-1".to_owned()))
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
        Ok(Value::String("a:b|c".to_owned()))
    );
    assert_eq!(
        eval(
            "function collect(...rest) { return rest.length + ':' + Array.isArray(rest); } collect();"
        ),
        Ok(Value::String("0:true".to_owned()))
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
        Ok(Value::String("ac".to_owned()))
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
        Ok(Value::String("true:true:false:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let a = Object.getOwnPropertyDescriptor(Function.prototype, 'arguments'); let c = Object.getOwnPropertyDescriptor(Function.prototype, 'caller'); (typeof a.get) + ':' + (a.get === a.set) + ':' + (a.get === c.get) + ':' + a.enumerable + ':' + a.configurable;"
        ),
        Ok(Value::String("function:true:true:false:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let got = false; let set = false; try { Function.prototype.arguments; } catch (error) { got = error instanceof TypeError; } try { Function.prototype.arguments = 1; } catch (error) { set = error instanceof TypeError; } got + ':' + set;"
        ),
        Ok(Value::String("true:true".to_owned()))
    );
    assert_eq!(
        eval("Function.prototype.toString.call(Array.isArray).includes('[native code]');"),
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
        Ok(Value::String("pair:false:false:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Array.isArray, 'name'); d.value + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("isArray:false:false:true".to_owned()))
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
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(
        eval("let f = function named() { return named === f; }; f();"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let f = function hidden() { return 1; }; typeof hidden;"),
        Ok(Value::String("undefined".to_owned()))
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
        Ok(Value::String("c".to_owned()))
    );
    assert_eq!(
        eval(
            "function count(a, b, c) { return arguments.length + ':' + (a === undefined) + ':' + b + ':' + (c === undefined); } count.apply(null, [, 2, ,]);"
        ),
        Ok(Value::String("3:true:2:true".to_owned()))
    );
    assert_eq!(
        eval(
            "function fn() {} var caught = ''; for (var i = 0; i < 4; i = i + 1) { var value = i === 0 ? true : (i === 1 ? NaN : (i === 2 ? '1,2,3' : Symbol())); try { fn.apply(null, value); caught = caught + '0'; } catch (error) { caught = caught + (error instanceof TypeError ? '1' : '2'); } } caught;"
        ),
        Ok(Value::String("1111".to_owned()))
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
        Ok(Value::String("ctx:1:2:3|pre:a:b:c|true".to_owned()))
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
        Ok(Value::String("bound join".to_owned()))
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
    assert_eq!(
        eval(
            "function f() {} let data = 'data'; Object.defineProperty(Function.prototype, 'prop', { get: function() { return data; }, set: function(value) { data = value; }, configurable: true }); let bound = f.bind({}); bound.prop = 'overrideData'; let result = bound.hasOwnProperty('prop') + ':' + bound.prop + ':' + data; delete Function.prototype.prop; result;"
        ),
        Ok(Value::String("false:overrideData:overrideData".to_owned()))
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
        Ok(Value::String("function:false:false:false".to_owned()))
    );
    assert_eq!(
        eval("let f = Function.prototype[Symbol.hasInstance]; f.length + ':' + f.name;"),
        Ok(Value::String("1:[Symbol.hasInstance]".to_owned()))
    );
    assert_eq!(
        eval(
            "function C() {} let instance = new C(); Function.prototype[Symbol.hasInstance].call(C, instance) + ':' + Function.prototype[Symbol.hasInstance].call(C, {});"
        ),
        Ok(Value::String("true:false".to_owned()))
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
        Ok(Value::String("true:1:true:false".to_owned()))
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
        Ok(Value::String("1|2".to_owned()))
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
        Ok(Value::String("2:5".to_owned()))
    );
    assert_eq!(
        eval("Function('return 1;').name;"),
        Ok(Value::String("anonymous".to_owned()))
    );
    assert_eq!(
        eval(
            "var i = 0; var p = { toString: function() { return 'a' + (++i); } }; let f = new Function(p, p, p, 'return a1 + a2 + a3;'); f('a', 'b', 'c');"
        ),
        Ok(Value::String("abc".to_owned()))
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
    assert!(eval("Function('a +', 'return a;');").is_err());
    assert!(eval("Function('break;');").is_err());
}

#[test]
fn evaluates_spread_call_arguments() {
    assert_eq!(
        eval(
            "function collect(a, b, c, d) { return '' + a + b + c + d; } collect(0, ...[1, 2], 3);"
        ),
        Ok(Value::String("0123".to_owned()))
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
        Ok(Value::String("1:true:true:true".to_owned()))
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
        Ok(Value::String("1:1:2".to_owned()))
    );
}

#[test]
fn evaluates_arrow_functions_with_lexical_this() {
    assert_eq!(
        eval(
            "this.marker = 'global'; let receiver = { marker: 'receiver' }; let read = () => this.marker; read.call(receiver);"
        ),
        Ok(Value::String("global".to_owned()))
    );
    assert_eq!(
        eval(
            "let receiver = { marker: 'receiver' }; let read = function() { return this.marker; }; read.call(receiver);"
        ),
        Ok(Value::String("receiver".to_owned()))
    );
    assert_eq!(
        eval(
            "this.marker = 'global'; let receiver = { marker: 'receiver' }; [1].map(() => this.marker, receiver)[0];"
        ),
        Ok(Value::String("global".to_owned()))
    );
    assert_eq!(
        eval(
            "this.marker = 'global'; let receiver = { marker: 'receiver' }; let seen; new Set([1]).forEach(() => { seen = this.marker; }, receiver); seen;"
        ),
        Ok(Value::String("global".to_owned()))
    );
}

#[test]
fn evaluates_arrow_functions_with_lexical_arguments() {
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
        Ok(Value::String("outer:2:arrow".to_owned()))
    );
    assert_eq!(
        eval(
            "function outer() { let read = () => { return function() { return arguments[0]; }('inner'); }; return read(); } outer('outer');"
        ),
        Ok(Value::String("inner".to_owned()))
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
        Ok(Value::String("function:12".to_owned()))
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
fn evaluates_destructured_parameters() {
    assert_eq!(
        eval(
            "function pick({x, y: {z} = {z: 9}}, [p = 5]) { return x + z + p; } pick({x: 1}, []);"
        ),
        Ok(Value::Number(15.0))
    );
    assert_eq!(
        eval("let sum = ([a, b], {c}) => a + b + c; sum([1, 2], {c: 3});"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("let method = { add([a, b]) { return a + b; } }; method.add([4, 5]);"),
        Ok(Value::Number(9.0))
    );
}

#[test]
fn evaluates_rest_parameter_patterns() {
    assert_eq!(
        eval("function tail(a, ...[b, c]) { return a + b + c; } tail(1, 2, 3, 4);"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("function size(...{length}) { return length; } size(1, 2, 3);"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_binding_pattern_rest_elements() {
    assert_eq!(
        eval(
            "function f([first, ...others]) { return first + ':' + others.join('|'); } f([1, 2, 3]);"
        ),
        Ok(Value::String("1:2|3".to_owned()))
    );
    assert_eq!(
        eval(
            "function f({a, ...rest}) { return a + ':' + Object.keys(rest).join('|') + ':' + rest.b; } f({a: 1, b: 2, c: 3});"
        ),
        Ok(Value::String("1:b|c:2".to_owned()))
    );
}

#[test]
fn parameter_defaults_apply_only_to_undefined() {
    assert_eq!(
        eval("function f(x = 5) { return x; } f(null) === null;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function f(x = 5, y = x + 1) { return x + y; } f();"),
        Ok(Value::Number(11.0))
    );
    assert_eq!(
        eval(
            "var log = []; function t(v) { log.push(v); return v; } function f(a = t(1), {b} = {b: t(2)}, c = t(3)) {} f(); log.join(',');"
        ),
        Ok(Value::String("1,2,3".to_owned()))
    );
}

#[test]
fn destructured_parameters_iterate_iterables() {
    assert_eq!(
        eval("function f([a, b]) { return a + b; } f('xy');"),
        Ok(Value::String("xy".to_owned()))
    );
    assert_eq!(
        eval("function f([[k, v]]) { return k + '=' + v; } f(new Map([['a', 1]]));"),
        Ok(Value::String("a=1".to_owned()))
    );
    // A hand-rolled iterable stands in for a generator until generator
    // evaluation lands in T010 S2.
    assert_eq!(
        eval(
            "function range() {
               var n = 0;
               return { [Symbol.iterator]() { return this; },
                        next() { n = n + 1; return { value: n, done: n > 3 }; } };
             }
             function f([head, ...tail]) { return head + ':' + tail.join('|'); } f(range());"
        ),
        Ok(Value::String("1:2|3".to_owned()))
    );
}

#[test]
fn destructured_parameter_coercion_errors_are_type_errors() {
    assert_eq!(
        eval("try { (function({x}) {})(undefined); } catch (e) { e instanceof TypeError; }"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("try { (function({}) {})(null); } catch (e) { e instanceof TypeError; }"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("try { (function([a]) {})(5); } catch (e) { e instanceof TypeError; }"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn non_simple_parameter_lists_unmap_arguments() {
    assert_eq!(
        eval("function f(a) { a = 99; return arguments[0]; } f(1);"),
        Ok(Value::Number(99.0))
    );
    assert_eq!(
        eval("function f(a, b = 2) { a = 99; return arguments[0]; } f(1);"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "function f([a], {b}) { return arguments.length + ':' + arguments[0][0] + ':' + arguments[1].b; } f([7], {b: 8});"
        ),
        Ok(Value::String("2:7:8".to_owned()))
    );
}

#[test]
fn destructuring_temporaries_stay_frame_local() {
    assert_eq!(
        eval(
            "function g({p} = {p: 0}, {q} = {q: 0}) { return p + q; } function f([a] = [g({p: 1}, {q: 2})], [b]) { return a + ':' + b; } f(undefined, [33]);"
        ),
        Ok(Value::String("3:33".to_owned()))
    );
    assert_eq!(
        eval(
            "function g([p, q]) { return p + q; } function f([a = g([1, 2]), b]) { return a + ':' + b; } f([undefined, 7]);"
        ),
        Ok(Value::String("3:7".to_owned()))
    );
}

#[test]
fn destructured_parameter_function_length_skips_defaults() {
    assert_eq!(
        eval("(function({a}, [b], c = 1) {}).length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("(function(...rest) {}).length;"),
        Ok(Value::Number(0.0))
    );
}
