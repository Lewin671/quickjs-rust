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

#[test]
fn evaluates_native_error_builtins() {
    for name in [
        "EvalError",
        "RangeError",
        "ReferenceError",
        "SyntaxError",
        "TypeError",
        "URIError",
    ] {
        assert_eq!(
            eval(&format!("typeof {name};")),
            Ok(Value::String("function".to_owned()))
        );
        assert_eq!(eval(&format!("{name}.length;")), Ok(Value::Number(1.0)));
        assert_eq!(
            eval(&format!("{name}.name;")),
            Ok(Value::String(name.to_owned()))
        );
        assert_eq!(
            eval(&format!("{name}.prototype.name;")),
            Ok(Value::String(name.to_owned()))
        );
        assert_eq!(
            eval(&format!("{name}.prototype.message;")),
            Ok(Value::String(String::new()))
        );
        assert_eq!(
            eval(&format!("{name}.prototype.constructor === {name};")),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(&format!("let error = new {name}('boom'); error.message;")),
            Ok(Value::String("boom".to_owned()))
        );
        assert_eq!(
            eval(&format!("new {name}('boom') instanceof {name};")),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(&format!("new {name}('boom') instanceof Error;")),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(&format!("{name}('boom').toString();")),
            Ok(Value::String(format!("{name}: boom")))
        );
        assert_eq!(
            eval(&format!(
                "Object.prototype.toString.call(new {name}('boom'));"
            )),
            Ok(Value::String("[object Error]".to_owned()))
        );
    }
}

#[test]
fn evaluates_aggregate_error_builtin() {
    assert_eq!(
        eval("typeof AggregateError;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("AggregateError.length;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("AggregateError.prototype.name;"),
        Ok(Value::String("AggregateError".to_owned()))
    );
    assert_eq!(
        eval("AggregateError.prototype.constructor === AggregateError;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let error = new AggregateError([1, 2], 'boom'); error.message;"),
        Ok(Value::String("boom".to_owned()))
    );
    assert_eq!(
        eval(
            "let error = new AggregateError([1, 2], 'boom'); error.errors.length + ':' + error.errors[0] + ':' + error.errors[1];"
        ),
        Ok(Value::String("2:1:2".to_owned()))
    );
    assert_eq!(
        eval("new AggregateError([], 'boom') instanceof AggregateError;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("new AggregateError([], 'boom') instanceof Error;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("new AggregateError([], 'boom').toString();"),
        Ok(Value::String("AggregateError: boom".to_owned()))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new AggregateError([], 'boom'));"),
        Ok(Value::String("[object Error]".to_owned()))
    );
}
