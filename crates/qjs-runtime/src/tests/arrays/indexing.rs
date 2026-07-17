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
}
