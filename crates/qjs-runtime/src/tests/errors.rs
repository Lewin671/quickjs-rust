use crate::{Value, eval};

#[test]
fn evaluates_error_builtins() {
    assert_eq!(
        eval("typeof Error;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Error.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Error.prototype.constructor === Error;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Error.prototype.name;"),
        Ok(Value::String("Error".to_owned()))
    );
    assert_eq!(
        eval("Error.prototype.message;"),
        Ok(Value::String(String::new()))
    );
    assert_eq!(
        eval("Error.prototype.toString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("let error = new Error('boom'); error.message;"),
        Ok(Value::String("boom".to_owned()))
    );
    assert_eq!(
        eval("let error = new Error('boom'); error.constructor === Error;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("new Error('boom').toString();"),
        Ok(Value::String("Error: boom".to_owned()))
    );
    assert_eq!(
        eval("Error('boom').toString();"),
        Ok(Value::String("Error: boom".to_owned()))
    );
    assert_eq!(
        eval("new Error().toString();"),
        Ok(Value::String("Error".to_owned()))
    );
    assert_eq!(
        eval("let error = new Error('boom'); error.name = 'Custom'; error.toString();"),
        Ok(Value::String("Custom: boom".to_owned()))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new Error('boom'));"),
        Ok(Value::String("[object Error]".to_owned()))
    );
    assert!(
        eval("throw new Error('boom');")
            .expect_err("throwing an Error should fail evaluation")
            .message
            .contains("Error: boom")
    );
}
