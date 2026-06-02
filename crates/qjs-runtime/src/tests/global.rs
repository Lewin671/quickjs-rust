use crate::{Value, eval};

#[test]
fn evaluates_boolean_builtins() {
    assert_eq!(
        eval("typeof Boolean;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Boolean.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Boolean();"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Boolean(0);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Boolean(1);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Boolean('');"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Boolean('x');"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Boolean.prototype.constructor === Boolean;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Boolean.prototype.toString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Boolean.prototype.valueOf.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Boolean.prototype.toString();"),
        Ok(Value::String("false".to_owned()))
    );
    assert_eq!(
        eval("Boolean.prototype.valueOf();"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("true.toString();"),
        Ok(Value::String("true".to_owned()))
    );
    assert_eq!(eval("false.valueOf();"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("(new Boolean(true)).toString();"),
        Ok(Value::String("true".to_owned()))
    );
    assert_eq!(
        eval("(new Boolean(0)).valueOf();"),
        Ok(Value::Boolean(false))
    );
    assert!(eval("let o = Object.create(Boolean.prototype); o.valueOf();").is_err());
}

#[test]
fn evaluates_global_undefined_binding() {
    assert_eq!(eval("undefined;"), Ok(Value::Undefined));
    assert_eq!(eval("undefined === undefined;"), Ok(Value::Boolean(true)));
}

#[test]
fn evaluates_global_eval_builtin() {
    assert_eq!(
        eval("typeof eval;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("eval.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("this.eval === eval;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("eval(7);"), Ok(Value::Number(7.0)));
    assert_eq!(eval("eval('1 + 2;');"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("let value = 1; eval('value = value + 2;'); value;"),
        Ok(Value::Number(3.0))
    );
}
