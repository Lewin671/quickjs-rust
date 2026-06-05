use crate::{Value, eval};

#[test]
fn evaluates_array_search_builtins() {
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
    assert_eq!(
        eval("[1, 2, 1].lastIndexOf(1, undefined);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(eval("[1, 2, 3].lastIndexOf(4);"), Ok(Value::Number(-1.0)));
    assert_eq!(
        eval("[false, 'false'].lastIndexOf(false);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Array.prototype.indexOf.call({0: 'a', 1: 'b', length: 2}, 'b');"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.lastIndexOf.call({0: 'a', 1: 'b', 2: 'a', length: 3}, 'a');"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("Array.prototype.indexOf.call('abc', 'b');"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.indexOf.call({0: undefined, length: 2}, undefined);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Array.prototype.indexOf.call({length: 2}, undefined);"),
        Ok(Value::Number(-1.0))
    );
    assert_eq!(
        eval(
            "let count = 0; let object = { length: 2 }; Object.defineProperty(object, '1', { get: function() { count++; return 'x'; } }); Array.prototype.indexOf.call(object, 'x') + ':' + count;"
        ),
        Ok(Value::String("1:1".to_owned()))
    );
    assert_eq!(
        eval(
            "let valueOfAccessed = false; let toStringAccessed = false; let fromIndex = { valueOf: function() { valueOfAccessed = true; return {}; }, toString: function() { toStringAccessed = true; return '1'; } }; [0, true].indexOf(true, fromIndex) + ':' + valueOfAccessed + ':' + toStringAccessed;"
        ),
        Ok(Value::String("1:true:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let valueOfAccessed = false; let toStringAccessed = false; let fromIndex = { valueOf: function() { valueOfAccessed = true; return {}; }, toString: function() { toStringAccessed = true; return '1'; } }; [0, true].lastIndexOf(true, fromIndex) + ':' + valueOfAccessed + ':' + toStringAccessed;"
        ),
        Ok(Value::String("1:true:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let stepTwoOccurs = false; let stepFiveOccurs = false; let object = {}; Object.defineProperty(object, 'length', { get: function() { stepTwoOccurs = true; return 2; } }); let fromIndex = { valueOf: function() { stepFiveOccurs = true; return 0; } }; Array.prototype.indexOf.call(object, undefined, fromIndex); stepTwoOccurs + ':' + stepFiveOccurs;"
        ),
        Ok(Value::String("true:true".to_owned()))
    );
    assert_eq!(eval("[1, 2, 3].includes(2);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("[1, 2, 3].includes(4);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("[1, 2, 3].includes(1, 1);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("[1, 2, 3].includes(3, -1);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("[0 / 0].includes(0 / 0);"), Ok(Value::Boolean(true)));
}
