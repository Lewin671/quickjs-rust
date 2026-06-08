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
}
