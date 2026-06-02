use crate::{Value, eval};

#[test]
fn evaluates_object_prototype_methods() {
    assert_eq!(
        eval("Object.prototype.toString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Object.prototype.toString();"),
        Ok(Value::String("[object Object]".to_owned()))
    );
    assert_eq!(
        eval("({}).toString();"),
        Ok(Value::String("[object Object]".to_owned()))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new Date(0));"),
        Ok(Value::String("[object Date]".to_owned()))
    );
    assert_eq!(
        eval("Object.prototype.toLocaleString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Object.prototype.toLocaleString();"),
        Ok(Value::String("[object Object]".to_owned()))
    );
    assert_eq!(
        eval(
            "let object = { toString: function() { return 'custom'; } }; object.toLocaleString();"
        ),
        Ok(Value::String("custom".to_owned()))
    );
    assert!(eval("Object.prototype.toLocaleString.call(null);").is_err());
    assert!(eval("Object.prototype.toLocaleString.call(undefined);").is_err());
    assert_eq!(
        eval("Object.prototype.valueOf.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("let object = { value: 1 }; object.valueOf() === object;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.valueOf() === Object.prototype;"),
        Ok(Value::Boolean(true))
    );
}
