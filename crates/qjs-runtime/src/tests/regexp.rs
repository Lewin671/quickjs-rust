use crate::{Value, eval};

#[test]
fn evaluates_regexp_constructor_identity() {
    assert_eq!(
        eval("typeof RegExp;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("RegExp.length;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("new RegExp() instanceof RegExp;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("/./ instanceof RegExp;"), Ok(Value::Boolean(true)));
    assert!(eval("[].find(/./);").is_err());
    assert_eq!(
        eval("Object.prototype.toString.call(new RegExp());"),
        Ok(Value::String("[object RegExp]".to_owned()))
    );
    assert_eq!(
        eval("new RegExp('test').toString();"),
        Ok(Value::String("/test/".to_owned()))
    );
    assert_eq!(
        eval("/test/.toString();"),
        Ok(Value::String("/test/".to_owned()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(RegExp.prototype, 'test'); d.value === RegExp.prototype.test && !d.enumerable && d.writable && d.configurable;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(RegExp.prototype, 'source'); typeof d.get === 'function' && d.set === undefined && !d.enumerable && d.configurable && !d.hasOwnProperty('writable');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("/test/gim.source;"),
        Ok(Value::String("test".to_owned()))
    );
    assert_eq!(eval("/test/gim.global;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("/test/gim.ignoreCase;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("/test/gim.multiline;"), Ok(Value::Boolean(true)));
}

#[test]
fn evaluates_regexp_exec_literal_match() {
    assert_eq!(
        eval("/test/.exec('a test value')[0];"),
        Ok(Value::String("test".to_owned()))
    );
    assert_eq!(eval("/missing/.exec('a test value');"), Ok(Value::Null));
    assert_eq!(
        eval("/test/.exec('a test value').index;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("/test/.exec('a test value').input;"),
        Ok(Value::String("a test value".to_owned()))
    );
    assert_eq!(
        eval("/test/.test('a test value');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("/missing/.test('a test value');"),
        Ok(Value::Boolean(false))
    );
}

#[test]
fn evaluates_regexp_exec_date_format_shape() {
    assert_eq!(
        eval(
            r#"/^(Sun|Mon|Tue|Wed|Thu|Fri|Sat) (Jan|Feb|Mar|Apr|May|Jun|Jul|Aug|Sep|Oct|Nov|Dec) [0-9]{2} [0-9]{4} [0-9]{2}:[0-9]{2}:[0-9]{2} GMT[+-][0-9]{4}( \(.+\))?$/.exec(new Date(0).toString()) !== null;"#
        ),
        Ok(Value::Boolean(true))
    );
}
