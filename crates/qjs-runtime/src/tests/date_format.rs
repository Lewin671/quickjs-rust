use crate::{Value, eval};

#[test]
fn evaluates_date_local_format_builtins() {
    assert_eq!(
        eval("Date.prototype.getTimezoneOffset.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Date.prototype.toDateString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Date.prototype.toString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Date.prototype.toTimeString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("new Date(0).getTimezoneOffset();"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("new Date(0).toDateString();"),
        Ok(Value::String("Thu Jan 01 1970".to_owned()))
    );
    assert_eq!(
        eval("new Date(0).toTimeString();"),
        Ok(Value::String("00:00:00 GMT+0000".to_owned()))
    );
    assert_eq!(
        eval("new Date(0).toString();"),
        Ok(Value::String(
            "Thu Jan 01 1970 00:00:00 GMT+0000".to_owned()
        ))
    );
    assert_eq!(
        eval("new Date('0020-01-01T00:00:00Z').toDateString();"),
        Ok(Value::String("Wed Jan 01 0020".to_owned()))
    );
    assert_eq!(
        eval("new Date('-000001-07-01T00:00Z').toDateString();"),
        Ok(Value::String("Thu Jul 01 -0001".to_owned()))
    );
    assert_eq!(
        eval("new Date('-000001-07-01T00:00:00Z').toDateString();"),
        Ok(Value::String("Thu Jul 01 -0001".to_owned()))
    );
    assert_eq!(
        eval("new Date('-000001-07-01T00:00:00Z').toString();"),
        Ok(Value::String(
            "Thu Jul 01 -0001 00:00:00 GMT+0000".to_owned()
        ))
    );
    assert_eq!(
        eval("new Date('-000012-07-01T00:00:00Z').toString();"),
        Ok(Value::String(
            "Fri Jul 01 -0012 00:00:00 GMT+0000".to_owned()
        ))
    );
    assert_eq!(
        eval("new Date(NaN).toString();"),
        Ok(Value::String("Invalid Date".to_owned()))
    );
    assert_eq!(
        eval("new Date(NaN).toDateString();"),
        Ok(Value::String("Invalid Date".to_owned()))
    );
    assert_eq!(
        eval("new Date(NaN).toTimeString();"),
        Ok(Value::String("Invalid Date".to_owned()))
    );
    assert_eq!(
        eval("Number.isNaN(new Date(NaN).getTimezoneOffset());"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { Date.prototype.toString(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert!(eval("Date.prototype.toString.call({});").is_err());
    assert!(eval("Date.prototype.getTimezoneOffset.call({});").is_err());
}
