use crate::{Value, eval};

#[test]
fn evaluates_array_indexing() {
    assert_eq!(eval("[1, 2, 3].at(0);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("[1, 2, 3].at(2);"), Ok(Value::Number(3.0)));
    assert_eq!(eval("[1, 2, 3].at(-1);"), Ok(Value::Number(3.0)));
    assert_eq!(eval("[1, 2, 3].at(-3);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("[1, 2, 3].at(3);"), Ok(Value::Undefined));
    assert_eq!(eval("[1, 2, 3].at(-4);"), Ok(Value::Undefined));
    assert_eq!(eval("[1, 2, 3].at();"), Ok(Value::Number(1.0)));
    assert_eq!(eval("[1, 2, 3].at(1.9);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("[1, 2, 3].at(-1.9);"), Ok(Value::Number(3.0)));
}
#[test]
fn evaluates_array_literals() {
    assert_eq!(
        eval("let xs = [1, 2 + 3, true]; xs.length + ':' + xs[0] + ':' + xs[1] + ':' + xs[2];"),
        Ok(Value::String("3:1:5:true".to_owned().into()))
    );
    assert_eq!(eval("[] === [];"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("let xs = []; let ys = xs; xs === ys;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let xs = [1, ...[2, 3], ...'ab']; xs.join('|');"),
        Ok(Value::String("1|2|3|a|b".to_owned().into()))
    );
    assert_eq!(
        eval("let xs = [...[1], , ...[3]]; xs.length + ':' + (1 in xs) + ':' + xs[2];"),
        Ok(Value::String("3:false:3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let xs = [...new Set([1, 2]), ...new Map([['a', 3]]), ...{ [Symbol.iterator]: function() { return ['z'][Symbol.iterator](); } }]; xs[0] + ':' + xs[1] + ':' + xs[2][0] + ':' + xs[2][1] + ':' + xs[3];"
        ),
        Ok(Value::String("1:2:a:3:z".to_owned().into()))
    );
    assert!(eval("[...{}];").is_err());
}

#[test]
fn evaluates_array_member_access() {
    assert_eq!(eval("let xs = [1, 2 + 3]; xs[1];"), Ok(Value::Number(5.0)));
    assert_eq!(
        eval("let xs = [1, undefined, 3]; xs[0] + ':' + xs[1] + ':' + xs[2];"),
        Ok(Value::String("1:undefined:3".to_owned().into()))
    );
    assert_eq!(eval("[1, 2, 3].length;"), Ok(Value::Number(3.0)));
    assert_eq!(eval("[1, , 3].length;"), Ok(Value::Number(3.0)));
    assert_eq!(eval("[1, , 3][1];"), Ok(Value::Undefined));
    assert_eq!(eval("1 in [1, , 3];"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval(
            "let xs = []; Object.defineProperty(xs, '0', { get: function() { return 9; }, configurable: true }); xs[0];"
        ),
        Ok(Value::Number(9.0))
    );
    assert_eq!(
        eval(
            "let xs = []; Object.defineProperty(xs, '0', { set: function(_) {}, configurable: true }); xs[0];"
        ),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("Array.prototype[1] = 13; let xs = [, ,]; xs[1];"),
        Ok(Value::Number(13.0))
    );
    assert_eq!(
        eval("Array.prototype[1] = 13; let xs = [11, 12]; xs[1];"),
        Ok(Value::Number(12.0))
    );
    assert_eq!(
        eval(
            "let proto = { 0: 41 }; let xs = [7]; Object.setPrototypeOf(xs, proto); xs[0] + ':' + xs[1];"
        ),
        Ok(Value::String("7:undefined".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let xs = [0, 1, 2]; xs[4294967294] = 4; xs.length = 2; xs[2] + ':' + xs[4294967294] + ':' + xs.length;"
        ),
        Ok(Value::String("undefined:undefined:2".to_owned().into()))
    );
}

#[test]
fn numeric_literal_index_reads_preserve_generic_property_semantics() {
    assert_eq!(
        eval(
            "let hits = []; \
             let ordinary = Object.create({ 0: 17 }); \
             let accessor = {}; Object.defineProperty(accessor, '1', { get: function() { hits.push('get'); return 23; } }); \
             let proxy = new Proxy({ 2: 31 }, { get: function(target, key, receiver) { hits.push(key); return Reflect.get(target, key, receiver); } }); \
             ordinary[0] + ':' + accessor[1] + ':' + proxy[2] + ':' + hits.join(',');"
        ),
        Ok(Value::String("17:23:31:get,2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "Array.prototype[1] = 41; let holey = [, ,]; \
             let custom = [7]; Object.setPrototypeOf(custom, { 1: 43 }); \
             holey[1] + ':' + custom[0] + ':' + custom[1];"
        ),
        Ok(Value::String("41:7:43".to_owned().into()))
    );
    assert_eq!(
        eval("let values = new Uint8Array([5, 9]); values[0] + ':' + values[1] + ':' + values[2];"),
        Ok(Value::String("5:9:undefined".to_owned().into()))
    );
}

/// The dense `array[i] = x` fast path must stay observably identical to a
/// generic property set: it has to honor an inherited indexed setter on
/// `Array.prototype`, resume direct storage once that setter is removed, and
/// keep an out-of-range "index" as an ordinary property.
#[test]
fn dense_index_store_preserves_prototype_set_semantics() {
    assert_eq!(
        eval(
            "let hits = 0; Object.defineProperty(Array.prototype, '7', { set: function() { hits++; }, configurable: true }); \
             let xs = []; xs[7] = 99; let result = hits + ':' + xs.hasOwnProperty('7'); \
             delete Array.prototype['7']; let ys = []; ys[7] = 5; result + ':' + (ys[7] === 5);"
        ),
        Ok(Value::String("1:false:true".to_owned().into()))
    );
    assert_eq!(
        eval("let xs = []; xs[0] = 1; xs[1] = 2; xs[0] = 3; xs.join(',');"),
        Ok(Value::String("3,2".to_owned().into()))
    );
    assert_eq!(
        eval("let xs = []; xs[4294967295] = 'x'; xs.length + ':' + xs['4294967295'];"),
        Ok(Value::String("0:x".to_owned().into()))
    );
}

#[test]
fn dense_index_store_does_not_treat_holes_as_own_elements() {
    assert_eq!(
        eval(
            "let sloppy = [,]; Object.preventExtensions(sloppy); \
             let sloppyResult = Function('array', 'return (array[0] = 7)')(sloppy); \
             let strict = [,]; Object.preventExtensions(strict); \
             let strictThrew = false; \
             try { \
                 Function('array', '\"use strict\"; return (array[0] = 7)')(strict); \
             } catch (error) { strictThrew = error instanceof TypeError; } \
             sloppyResult + ':' \
                 + Object.prototype.hasOwnProperty.call(sloppy, '0') + ':' \
                 + sloppy[0] + ':' + strictThrew + ':' \
                 + Object.prototype.hasOwnProperty.call(strict, '0') + ':' \
                 + strict[0];"
        ),
        Ok(Value::String(
            "7:false:undefined:true:false:undefined".to_owned().into()
        ))
    );
}

#[test]
fn non_extensible_array_can_grow_writable_length() {
    assert_eq!(
        eval(
            "let reflected = []; Object.preventExtensions(reflected); \
             let reflectedResult = Reflect.set(reflected, 'length', 3); \
             let sloppy = []; Object.preventExtensions(sloppy); sloppy.length = 4; \
             let strict = []; Object.preventExtensions(strict); \
             let strictThrew = false; \
             try { Function('array', '\"use strict\"; array.length = 5')(strict); } \
             catch (error) { strictThrew = error instanceof TypeError; } \
             reflectedResult + ':' + reflected.length + ':' \
                 + Object.keys(reflected).length + ':' + sloppy.length + ':' \
                 + Object.keys(sloppy).length + ':' + strictThrew + ':' \
                 + strict.length + ':' + Object.keys(strict).length;"
        ),
        Ok(Value::String("true:3:0:4:0:false:5:0".to_owned().into()))
    );
}

#[test]
fn dense_index_store_checks_the_complete_prototype_chain() {
    assert_eq!(
        eval(
            "let hits = 0; \
             Object.defineProperty(Object.prototype, '0', { \
                 set: function(value) { hits += value; }, configurable: true \
             }); \
             let first = [], key = 0; first[key] = 7; \
             let result = hits + ':' + first.hasOwnProperty('0'); \
             delete Object.prototype['0']; \
             let restored = []; restored[0] = 9; \
             result + ':' + restored[0];"
        ),
        Ok(Value::String("7:false:9".to_owned().into()))
    );

    assert_eq!(
        eval(
            "let original = Object.getPrototypeOf(Array.prototype), log = ''; \
             let middle = Object.create(original); \
             Object.defineProperty(middle, '1', { \
                 set: function(value) { log += 'middle:' + value; }, configurable: true \
             }); \
             Object.setPrototypeOf(Array.prototype, middle); \
             let throughMiddle = []; throughMiddle[1] = 3; \
             let proxy = new Proxy(original, { \
                 set: function(_target, key, value, receiver) { \
                     log += '|proxy:' + key + ':' + value + ':' + (receiver === throughProxy); \
                     return true; \
                 } \
             }); \
             Object.setPrototypeOf(Array.prototype, proxy); \
             let throughProxy = []; throughProxy[2] = 5; \
             Object.setPrototypeOf(Array.prototype, original); \
             log + ':' + throughMiddle.hasOwnProperty('1') \
                 + ':' + throughProxy.hasOwnProperty('2');"
        ),
        Ok(Value::String(
            "middle:3|proxy:2:5:true:false:false".to_owned().into()
        ))
    );

    assert_eq!(
        eval(
            "Object.defineProperty(Object.prototype, '3', { \
                 value: 11, writable: false, configurable: true \
             }); \
             let blocked = [], strictThrew = false; \
             blocked[3] = 17; \
             try { (function() { 'use strict'; blocked[3] = 19; })(); } \
             catch (error) { strictThrew = error instanceof TypeError; } \
             let result = blocked[3] + ':' + blocked.hasOwnProperty('3') + ':' + strictThrew; \
             delete Object.prototype['3']; result;"
        ),
        Ok(Value::String("11:false:true".to_owned().into()))
    );
}

#[test]
fn indexed_store_walks_a_functions_effective_prototype_chain() {
    assert_eq!(
        eval(
            "let original = Object.getPrototypeOf(Function.prototype), seen = 0, receiverOk = false; \
             let proxy = new Proxy(original, { \
                 set: function(_target, key, value, receiver) { \
                     seen = key + ':' + value; receiverOk = receiver === array; return true; \
                 } \
             }); \
             Object.setPrototypeOf(Function.prototype, proxy); \
             function link() {} \
             let bridge = Object.create(link), array = []; \
             Object.setPrototypeOf(array, bridge); array[0] = 17; \
             Object.setPrototypeOf(Function.prototype, original); \
             seen + ':' + receiverOk + ':' + array.hasOwnProperty('0');"
        ),
        Ok(Value::String("0:17:true:false".to_owned().into()))
    );

    assert_eq!(
        eval(
            "let original = Object.getPrototypeOf(Function.prototype); \
             Object.setPrototypeOf(Function.prototype, new Uint8Array(0)); \
             function link() {} \
             let bridge = Object.create(link), array = []; \
             Object.setPrototypeOf(array, bridge); array[0] = 23; \
             Object.setPrototypeOf(Function.prototype, original); \
             array.hasOwnProperty('0') + ':' + array.length;"
        ),
        Ok(Value::String("false:0".to_owned().into()))
    );
}

#[test]
fn array_prototype_nodes_remain_live_after_installation() {
    assert_eq!(
        eval(
            "let prototype = [], array = [], log = ''; \
             Object.setPrototypeOf(array, prototype); \
             let identity = Object.getPrototypeOf(array) === prototype; \
             Object.defineProperty(prototype, '0', { \
                 set: function() { log += 'first'; }, configurable: true \
             }); \
             array[0] = 1; \
             Object.defineProperty(prototype, '0', { \
                 set: function() { log += ':second'; }, configurable: true \
             }); \
             array[0] = 2; delete prototype[0]; array[0] = 3; \
             identity + ':' + log + ':' + array[0] + ':' + array.hasOwnProperty('0');"
        ),
        Ok(Value::String("true:first:second:3:true".to_owned().into()))
    );
}

#[test]
fn live_array_prototype_preserves_proxy_internal_methods() {
    assert_eq!(
        eval(
            "let original = Object.getPrototypeOf(Array.prototype), log = '', child; \
             let symbol = Symbol('live'); \
             let proxy = new Proxy(original, { \
                 get: function(target, key, receiver) { \
                     let name = key === symbol ? 'symbol' : key; \
                     log += 'get:' + name + ':' + (receiver === child) + ';'; \
                     if (key === 'answer') return 42; \
                     if (key === symbol) return 41; \
                     return Reflect.get(target, key, receiver); \
                 }, \
                 has: function(target, key) { \
                     let name = key === symbol ? 'symbol' : key; \
                     log += 'has:' + name + ';'; \
                     return key === 'answer' || key === symbol || Reflect.has(target, key); \
                 }, \
                 set: function(_target, key, value, receiver) { \
                     let name = key === symbol ? 'symbol' : key; \
                     log += 'set:' + name + ':' + value + ':' + (receiver === child) + ';'; \
                     return true; \
                 } \
             }); \
             Object.setPrototypeOf(Array.prototype, proxy); \
             let bridge = []; child = Object.create(bridge); \
             let result = child.answer + ':' + child[symbol] + ':' \
                 + ('answer' in child) + ':' + (symbol in child); \
             child[0] = 7; child[symbol] = 9; \
             Object.setPrototypeOf(Array.prototype, original); \
             result + ':' + log + ':' \
                 + Object.prototype.hasOwnProperty.call(child, '0') + ':' \
                 + Object.prototype.hasOwnProperty.call(child, symbol);"
        ),
        Ok(Value::String(
            "42:41:true:true:get:answer:true;get:symbol:true;has:answer;has:symbol;set:0:7:true;set:symbol:9:true;:false:false"
                .to_owned()
                .into()
        ))
    );
}

#[test]
fn function_named_get_preserves_live_array_prototype_proxy_get() {
    assert_eq!(
        eval(
            "let prototype = [], functionValue = function() {}, calls = 0, receiverOk = false; \
             Object.setPrototypeOf(functionValue, prototype); \
             Object.setPrototypeOf(prototype, new Proxy({}, { \
                 get: function(target, key, receiver) { \
                     calls += 1; receiverOk = receiver === functionValue; \
                     return key === 'answer' ? 9 : Reflect.get(target, key, receiver); \
                 } \
             })); \
             let typedArray = new Uint8Array([7]), typedPrototype = []; \
             let typedFunction = function() {}; \
             Object.setPrototypeOf(typedPrototype, typedArray); \
             Object.setPrototypeOf(typedFunction, typedPrototype); \
             functionValue.answer + ':' + calls + ':' + receiverOk + ':' \
                 + typedFunction[0] + ':' + typedFunction[1];"
        ),
        Ok(Value::String("9:1:true:7:undefined".to_owned().into()))
    );
}

#[test]
fn indexed_store_walks_native_error_constructor_parent() {
    assert_eq!(
        eval(
            "let seen = 0; \
             Object.defineProperty(Error, '0', { \
                 set: function(value) { seen = value; }, configurable: true \
             }); \
             let array = []; Object.setPrototypeOf(array, TypeError); array[0] = 7; \
             seen + ':' + array.hasOwnProperty('0') + ':' + array[0];"
        ),
        Ok(Value::String("7:false:undefined".to_owned().into()))
    );
}

#[test]
fn typed_array_prototypes_preserve_has_and_canonical_set_semantics() {
    assert_eq!(
        eval(
            "let typedArray = new Uint8Array([7]), array = [], functionValue = function() {}; \
             Object.setPrototypeOf(array, typedArray); \
             Object.setPrototypeOf(functionValue, typedArray); \
             functionValue[1] = 8; \
             array[0] + ':' + ('0' in array) + ':' \
                 + functionValue.hasOwnProperty('1') + ':' + functionValue[1];"
        ),
        Ok(Value::String("7:true:false:undefined".to_owned().into()))
    );
}

#[test]
fn native_error_constructor_explicit_array_prototype_intercepts_set() {
    assert_eq!(
        eval(
            "let hit = 0, prototype = []; \
             Object.defineProperty(prototype, 'x', { \
                 set: function(value) { hit = value; }, configurable: true \
             }); \
             Reflect.setPrototypeOf(TypeError, prototype); \
             TypeError.x = 7; \
             hit + ':' + Object.prototype.hasOwnProperty.call(TypeError, 'x');"
        ),
        Ok(Value::String("7:false".to_owned().into()))
    );
}

/// Computed compound assignments and updates cache a property key after
/// `ToPropertyKey`. Canonical string indices may still use dense storage, but
/// inherited setters and non-index strings must retain ordinary `[[Set]]`
/// behavior.
#[test]
fn dense_compound_index_store_preserves_property_semantics() {
    assert_eq!(
        eval(
            "let xs = new Array(8); xs[7] = 15; xs[7] &= 6; xs[7]++; \
             xs[7] + ':' + xs.length;"
        ),
        Ok(Value::String("7:8".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let hits = 0; \
             Object.defineProperty(Array.prototype, '7', { \
                 get: function() { return 3; }, \
                 set: function(value) { hits += value; }, \
                 configurable: true \
             }); \
             let xs = new Array(8); xs[7] += 4; xs[7]++; \
             let result = hits + ':' + xs.hasOwnProperty('7'); \
             delete Array.prototype['7']; result;"
        ),
        Ok(Value::String("11:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let xs = [10, 20]; xs['1.0'] = 4; xs['1.0'] += 3; \
             xs[1] + ':' + xs['1.0'] + ':' + xs.length;"
        ),
        Ok(Value::String("20:7:2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let calls = 0; \
             let key = { [Symbol.toPrimitive]: function() { calls++; return 0; } }; \
             let xs = [4]; xs[key] += 3; calls + ':' + xs[0];"
        ),
        Ok(Value::String("1:7".to_owned().into()))
    );
}
