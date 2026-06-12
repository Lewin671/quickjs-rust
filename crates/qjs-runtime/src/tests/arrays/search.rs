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
    assert_eq!(
        eval("Array.prototype.indexOf.call('abc', 'b');"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.indexOf.call({ length: 3, 2: 'x' }, 'x');"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("[, undefined].indexOf(undefined);"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let calls = 0; [0, 1].indexOf(1, { valueOf: function() { calls += 1; return 1; } }) + ':' + calls;"
        ),
        Ok(Value::String("1:1".to_owned()))
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
    assert_eq!(
        eval("Array.prototype.lastIndexOf.call('abc', 'b');"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.lastIndexOf.call({ length: 3, 2: 'x' }, 'x');"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("[undefined, ,].lastIndexOf(undefined);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let calls = 0; [0, 1, 2].lastIndexOf(1, { valueOf: function() { calls += 1; return 1; } }) + ':' + calls;"
        ),
        Ok(Value::String("1:1".to_owned()))
    );
    assert_eq!(
        eval(
            "let array = [5, undefined, 7]; \
             let log = []; \
             let proxy = new Proxy(Array.prototype, { \
               has: function(target, key) { log.push('has:' + key); return key in target; }, \
               get: function() { throw new Error('unexpected get'); } \
             }); \
             Object.setPrototypeOf(array, proxy); \
             let fromIndex = { valueOf: function() { array.length = 0; return 2; } }; \
             Array.prototype.lastIndexOf.call(array, 100, fromIndex); \
             log.join('|');"
        ),
        Ok(Value::String("has:2|has:1|has:0".to_owned()))
    );
    assert_eq!(eval("[1, 2, 3].includes(2);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("[1, 2, 3].includes(4);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("[1, 2, 3].includes(1, 1);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("[1, 2, 3].includes(3, -1);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("[0 / 0].includes(0 / 0);"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Array.prototype.includes.call('abc', 'b');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Array.prototype.includes.call({ length: 2, 1: 'x' }, 'x');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let calls = 0; let object = { length: 3, 0: 'a' }; Object.defineProperty(object, '1', { get: function() { calls += 1; object[2] = 'z'; return 'b'; } }); Array.prototype.includes.call(object, 'z') + ':' + calls;"
        ),
        Ok(Value::String("true:1".to_owned()))
    );
    assert_eq!(
        eval(
            "let calls = 0; [0, 1].includes(1, { valueOf: function() { calls += 1; return 1; } }) + ':' + calls;"
        ),
        Ok(Value::String("true:1".to_owned()))
    );
    assert!(eval("Array.prototype.includes.call(null, 0);").is_err());
    assert!(eval("Array.prototype.includes.call(undefined, 0);").is_err());
}
