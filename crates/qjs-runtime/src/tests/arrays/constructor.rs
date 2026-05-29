use crate::{Value, eval};

#[test]
fn evaluates_array_of_static_constructor() {
    assert_eq!(eval("Array.of.length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval(
            "let values = Array.of(1, 'x', true, null, undefined); values.length + ':' + values[0] + ':' + values[1] + ':' + values[2] + ':' + (values[3] === null) + ':' + (values[4] === undefined);"
        ),
        Ok(Value::String("5:1:x:true:true:true".to_owned()))
    );
    assert_eq!(eval("Array.of(3).length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Array.of(3)[0];"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("Array.isArray(Array.of(1, 2));"),
        Ok(Value::Boolean(true))
    );
}
