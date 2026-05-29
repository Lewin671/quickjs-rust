use crate::{Value, eval};

#[test]
fn evaluates_json_builtins() {
    assert_eq!(eval("typeof JSON;"), Ok(Value::String("object".to_owned())));
    assert_eq!(eval("JSON.parse.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("JSON.stringify.length;"), Ok(Value::Number(3.0)));
    assert_eq!(eval("JSON.parse('null');"), Ok(Value::Null));
    assert_eq!(eval("JSON.parse('true');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("JSON.parse('-12.5e2');"), Ok(Value::Number(-1250.0)));
    assert_eq!(
        eval("JSON.parse('\"text\"');"),
        Ok(Value::String("text".to_owned()))
    );
    assert_eq!(
        eval("JSON.parse('\"line\\\\nfeed\"');"),
        Ok(Value::String("line\nfeed".to_owned()))
    );
    assert_eq!(
        eval("JSON.parse('[1, true, null]')[1];"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let value = JSON.parse('{\"a\":1,\"b\":[2]}'); value.b[0];"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("JSON.stringify({a: 1, b: [true, null], c: undefined});"),
        Ok(Value::String("{\"a\":1,\"b\":[true,null]}".to_owned()))
    );
    assert_eq!(
        eval("JSON.stringify(['x', undefined, NaN, Infinity]);"),
        Ok(Value::String("[\"x\",null,null,null]".to_owned()))
    );
    assert_eq!(eval("JSON.stringify(undefined);"), Ok(Value::Undefined));
    assert!(eval("JSON.parse('{bad');").is_err());
}
