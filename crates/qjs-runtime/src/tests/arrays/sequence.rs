use crate::{Value, eval};

#[test]
fn evaluates_array_sequence_builtins() {
    assert_eq!(
        eval("[1, 'x', true].join();"),
        Ok(Value::String("1,x,true".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].join('|');"),
        Ok(Value::String("1|2|3".to_owned()))
    );
    assert_eq!(
        eval("[1, null, undefined, 4].join('-');"),
        Ok(Value::String("1---4".to_owned()))
    );
    assert_eq!(
        eval("[1, 'x', true].toString();"),
        Ok(Value::String("1,x,true".to_owned()))
    );
    assert_eq!(
        eval("[1, [2, 3], 4].join(';');"),
        Ok(Value::String("1;2,3;4".to_owned()))
    );
    assert_eq!(
        eval("[0, 1, 2, 3, 4].slice(1, 4).join();"),
        Ok(Value::String("1,2,3".to_owned()))
    );
    assert_eq!(
        eval("[0, 1, 2, 3, 4].slice(2).join('|');"),
        Ok(Value::String("2|3|4".to_owned()))
    );
    assert_eq!(
        eval("[0, 1, 2, 3, 4].slice(-3, -1).join();"),
        Ok(Value::String("2,3".to_owned()))
    );
    assert_eq!(eval("[0, 1, 2].slice(5).length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("let copy = [1, 2].slice(); Array.isArray(copy) && copy[1] === 2;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("[0].concat([1, 2], 3, [4]).join();"),
        Ok(Value::String("0,1,2,3,4".to_owned()))
    );
    assert_eq!(
        eval("[].concat([0, 1], [2, 3]).length;"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(eval("[0].concat('x', true)[2];"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval(
            "let xs = [1, 2, 3, 4, 5]; let result = xs.copyWithin(0, 3); result === xs && xs.join();"
        ),
        Ok(Value::String("4,5,3,4,5".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3, 4, 5].copyWithin(1, 3, 4).join();"),
        Ok(Value::String("1,4,3,4,5".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3, 4, 5].copyWithin(-2, 0, 2).join();"),
        Ok(Value::String("1,2,3,1,2".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3, 4].copyWithin(1, 0, 3).join();"),
        Ok(Value::String("1,1,2,3".to_owned()))
    );
}

#[test]
fn evaluates_array_to_reversed() {
    assert_eq!(
        eval(
            "let xs = [1, 2, 3]; let out = xs.toReversed(); out.join() + ':' + xs.join() + ':' + (out === xs);"
        ),
        Ok(Value::String("3,2,1:1,2,3:false".to_owned()))
    );
    assert_eq!(eval("[].toReversed().length;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("[7].toReversed()[0];"), Ok(Value::Number(7.0)));
    assert_eq!(
        eval("Array.prototype.toReversed.call({ length: 3, 0: 'a', 2: 'c' }).join('|');"),
        Ok(Value::String("c||a".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.toReversed.call('abc').join('');"),
        Ok(Value::String("cba".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.toReversed.length;"),
        Ok(Value::Number(0.0))
    );
    assert!(eval("Array.prototype.toReversed.call(null);").is_err());
}

#[test]
fn evaluates_array_with() {
    assert_eq!(
        eval(
            "let xs = [1, 2, 3]; let out = xs.with(1, 9); out.join() + ':' + xs.join() + ':' + (out === xs);"
        ),
        Ok(Value::String("1,9,3:1,2,3:false".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].with(-1, 9).join();"),
        Ok(Value::String("1,2,9".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].with(undefined, 9).join();"),
        Ok(Value::String("9,2,3".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].with(1).join();"),
        Ok(Value::String("1,,3".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.with.call({ length: 3, 0: 'a', 2: 'c' }, 1, 'b').join('|');"),
        Ok(Value::String("a|b|c".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.with.call('abc', -2, 'x').join('');"),
        Ok(Value::String("axc".to_owned()))
    );
    assert_eq!(eval("Array.prototype.with.length;"), Ok(Value::Number(2.0)));
    assert!(eval("[].with(0, 1);").is_err());
    assert!(eval("[1].with(1, 2);").is_err());
    assert!(eval("Array.prototype.with.call(null, 0, 1);").is_err());
}
