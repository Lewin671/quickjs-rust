use crate::{Value, eval};

#[test]
fn evaluates_date_builtins() {
    assert_eq!(
        eval("typeof Date;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Date.length;"), Ok(Value::Number(7.0)));
    assert_eq!(eval("Date.parse.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Date.UTC.length;"), Ok(Value::Number(7.0)));
    assert_eq!(
        eval("Date.prototype.getTime.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(eval("new Date(0).getTime();"), Ok(Value::Number(0.0)));
    assert_eq!(eval("new Date(0).valueOf();"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("new Date(0).toISOString();"),
        Ok(Value::String("1970-01-01T00:00:00.000Z".to_owned()))
    );
    assert_eq!(
        eval("Date.UTC(1970, 0, 2, 3, 4, 5, 6);"),
        Ok(Value::Number(97_445_006.0))
    );
    assert_eq!(
        eval("Date.parse('1970-01-02T03:04:05.006Z');"),
        Ok(Value::Number(97_445_006.0))
    );
    assert_eq!(
        eval("new Date('1970-01-02T03:04:05.006Z').toISOString();"),
        Ok(Value::String("1970-01-02T03:04:05.006Z".to_owned()))
    );
    assert!(eval("Date.prototype.getTime.call({});").is_err());
}
