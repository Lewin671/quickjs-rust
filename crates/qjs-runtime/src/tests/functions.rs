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
        eval("function keys() { return Object.keys(arguments).join('|'); } keys(1, 2, 3);"),
        Ok(Value::String("0|1|2".to_owned()))
    );
    assert_eq!(
        eval("function none() { return arguments.length; } none();"),
        Ok(Value::Number(0.0))
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
    assert!(eval("Function('a +', 'return a;');").is_err());
    assert!(eval("Function('break;');").is_err());
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
