use crate::{Value, eval};

#[test]
fn evaluates_object_from_entries() {
    assert_eq!(eval("Object.fromEntries.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("let result = Object.fromEntries([['key', 'value']]); result.key;"),
        Ok(Value::String("value".to_owned()))
    );
    assert_eq!(
        eval(
            "let result = Object.fromEntries([['a', 1], ['a', 2], [3, 4]]); result.a + result[3];"
        ),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("let entry = { 0: 'name', 1: 'value' }; Object.fromEntries([entry]).name;"),
        Ok(Value::String("value".to_owned()))
    );
    assert_eq!(
        eval("Object.getPrototypeOf(Object.fromEntries([])) === Object.prototype;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let result = Object.fromEntries([['x', 1]]); let d = Object.getOwnPropertyDescriptor(result, 'x'); d.value + ':' + d.enumerable + ':' + d.writable + ':' + d.configurable;"
        ),
        Ok(Value::String("1:true:true:true".to_owned()))
    );
    assert_eq!(
        eval("let key = Symbol(); let result = Object.fromEntries([[key, 'value']]); result[key];"),
        Ok(Value::String("value".to_owned()))
    );
    assert!(eval("Object.fromEntries();").is_err());
    assert!(eval("Object.fromEntries(['ab']);").is_err());
}
