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
        Ok(Value::String("3:1:5:true".to_owned()))
    );
    assert_eq!(eval("[] === [];"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("let xs = []; let ys = xs; xs === ys;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let xs = [1, ...[2, 3], ...'ab']; xs.join('|');"),
        Ok(Value::String("1|2|3|a|b".to_owned()))
    );
    assert_eq!(
        eval("let xs = [...[1], , ...[3]]; xs.length + ':' + (1 in xs) + ':' + xs[2];"),
        Ok(Value::String("3:false:3".to_owned()))
    );
    assert_eq!(
        eval(
            "let xs = [...new Set([1, 2]), ...new Map([['a', 3]]), ...{ [Symbol.iterator]: function() { return ['z'][Symbol.iterator](); } }]; xs[0] + ':' + xs[1] + ':' + xs[2][0] + ':' + xs[2][1] + ':' + xs[3];"
        ),
        Ok(Value::String("1:2:a:3:z".to_owned()))
    );
    assert!(eval("[...{}];").is_err());
}

#[test]
fn evaluates_array_member_access() {
    assert_eq!(eval("let xs = [1, 2 + 3]; xs[1];"), Ok(Value::Number(5.0)));
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
        eval(
            "let xs = [0, 1, 2]; xs[4294967294] = 4; xs.length = 2; xs[2] + ':' + xs[4294967294] + ':' + xs.length;"
        ),
        Ok(Value::String("undefined:undefined:2".to_owned()))
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
        Ok(Value::String("1:false:true".to_owned()))
    );
    assert_eq!(
        eval("let xs = []; xs[0] = 1; xs[1] = 2; xs[0] = 3; xs.join(',');"),
        Ok(Value::String("3,2".to_owned()))
    );
    assert_eq!(
        eval("let xs = []; xs[4294967295] = 'x'; xs.length + ':' + xs['4294967295'];"),
        Ok(Value::String("0:x".to_owned()))
    );
}
