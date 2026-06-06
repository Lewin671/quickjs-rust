use crate::{Value, eval};

#[test]
fn evaluates_array_search_builtins() {
    assert_eq!(eval("[1, 2, 3].at(1);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("[1, 2, 3].at(-1);"), Ok(Value::Number(3.0)));
    assert_eq!(eval("[1, 2, 3].at(5);"), Ok(Value::Undefined));
    assert_eq!(
        eval("Array.prototype.at.call('abc', -2);"),
        Ok(Value::String("b".to_owned()))
    );
    assert_eq!(
        eval("Array.prototype.at.call({ length: 2, 1: 'x' }, 1);"),
        Ok(Value::String("x".to_owned()))
    );
    assert_eq!(
        eval(
            "let log = ''; let object = {}; Object.defineProperty(object, 'length', { get: function() { log += 'l'; return 2; } }); Object.defineProperty(object, '1', { get: function() { log += 'g'; return 7; } }); Array.prototype.at.call(object, { valueOf: function() { log += 'i'; return -1; } }) + ':' + log;"
        ),
        Ok(Value::String("7:lig".to_owned()))
    );
    assert!(eval("Array.prototype.at.call(null, 0);").is_err());
    assert!(eval("Array.prototype.at.call(undefined, 0);").is_err());

    assert_eq!(eval("[1, 2, 1].indexOf(1);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("[1, 2, 1].indexOf(1, 1);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("[1, 2, 1].indexOf(1, -1);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("[1, 2, 1].indexOf(1, -5);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("[1, 2, 3].indexOf(4);"), Ok(Value::Number(-1.0)));
    assert_eq!(
        eval("[false, 'false'].indexOf(false);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("[false, 'false'].indexOf('false');"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("[1, 2, 1].lastIndexOf(1);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("[1, 2, 1].lastIndexOf(1, 1);"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("[1, 2, 1].lastIndexOf(1, -2);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("[1, 2, 1].lastIndexOf(1, -5);"),
        Ok(Value::Number(-1.0))
    );
    assert_eq!(eval("[1, 2, 3].lastIndexOf(4);"), Ok(Value::Number(-1.0)));
    assert_eq!(
        eval("[false, 'false'].lastIndexOf(false);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(eval("[1, 2, 3].includes(2);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("[1, 2, 3].includes(4);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("[1, 2, 3].includes(1, 1);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("[1, 2, 3].includes(3, -1);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("[0 / 0].includes(0 / 0);"), Ok(Value::Boolean(true)));
}
