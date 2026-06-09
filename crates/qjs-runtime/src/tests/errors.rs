use crate::{Value, eval};

#[test]
fn evaluates_error_builtins() {
    assert_eq!(
        eval("typeof Error;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Error.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Error.isError.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Error.isError.name;"),
        Ok(Value::String("isError".to_owned()))
    );
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
    assert!(
        eval("Error(Symbol('boom'));")
            .expect_err("Symbol message conversion should throw")
            .message
            .contains("TypeError")
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
    assert_eq!(
        eval("Error.isError(new Error('boom'));"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Error.isError(Error('boom'));"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Error.isError({});"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Error.isError(Error);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Error.isError();"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Error.isError('boom');"), Ok(Value::Boolean(false)));
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
            eval(&format!("Object.getPrototypeOf({name}) === Error;")),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(&format!("Reflect.getPrototypeOf({name}) === Error;")),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(&format!("{name}.isError === Error.isError;")),
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
        assert_eq!(
            eval(&format!("Error.isError(new {name}('boom'));")),
            Ok(Value::Boolean(true))
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
        eval("Object.getPrototypeOf(AggregateError) === Error;"),
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
    assert_eq!(
        eval("Error.isError(new AggregateError([], 'boom'));"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let cause = { message: 'root' }; let error = new AggregateError([], 'boom', { cause: cause }); let desc = Object.getOwnPropertyDescriptor(error, 'cause'); (desc.value === cause) + ':' + desc.writable + ':' + desc.enumerable + ':' + desc.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned()))
    );
    assert_eq!(
        eval("Object.hasOwn(new AggregateError([], 'boom', { cause: undefined }), 'cause');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.hasOwn(new AggregateError([], 'boom'), 'cause');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let marker = {}; let message = { toString: function() { throw marker; } }; let caught = false; try { new AggregateError([], message); } catch (error) { caught = error === marker; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let source = {}; source[Symbol.iterator] = function() { let index = 0; return { next: function() { index = index + 1; return index > 2 ? { done: true } : { value: index, done: false }; } }; }; let error = new AggregateError(source, 'boom'); error.errors.join();"
        ),
        Ok(Value::String("1,2".to_owned()))
    );
}
