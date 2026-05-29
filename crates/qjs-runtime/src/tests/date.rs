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
    assert_eq!(
        eval("Date.prototype.getUTCFullYear.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Date.prototype.toJSON.length;"),
        Ok(Value::Number(1.0))
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
    assert_eq!(
        eval("new Date('1970-01-02T03:04:05.006Z').toUTCString();"),
        Ok(Value::String("Fri, 02 Jan 1970 03:04:05 GMT".to_owned()))
    );
    assert_eq!(
        eval("new Date('1970-01-02T03:04:05.006Z').toJSON();"),
        Ok(Value::String("1970-01-02T03:04:05.006Z".to_owned()))
    );
    assert_eq!(
        eval("new Date('0020-01-01T00:00:00Z').toUTCString();"),
        Ok(Value::String("Wed, 01 Jan 0020 00:00:00 GMT".to_owned()))
    );
    assert_eq!(
        eval("new Date(NaN).toUTCString();"),
        Ok(Value::String("Invalid Date".to_owned()))
    );
    assert_eq!(eval("new Date(NaN).toJSON();"), Ok(Value::Null));
    assert_eq!(
        eval("JSON.stringify(new Date('1970-01-02T03:04:05.006Z'));"),
        Ok(Value::String("\"1970-01-02T03:04:05.006Z\"".to_owned()))
    );
    assert_eq!(
        eval("JSON.stringify({when: new Date('1970-01-02T03:04:05.006Z')});"),
        Ok(Value::String(
            "{\"when\":\"1970-01-02T03:04:05.006Z\"}".to_owned()
        ))
    );
    assert_eq!(
        eval("JSON.stringify(new Date(NaN));"),
        Ok(Value::String("null".to_owned()))
    );
    assert_eq!(
        eval(
            "var d = new Date('1970-01-02T03:04:05.006Z'); d.toISOString = function () { return 'custom'; }; d.toJSON();"
        ),
        Ok(Value::String("custom".to_owned()))
    );
    assert_eq!(
        eval(
            "[new Date('2016-12-31T23:59:59.999Z').getUTCFullYear(), new Date('2016-12-31T23:59:59.999Z').getUTCMonth(), new Date('2016-12-31T23:59:59.999Z').getUTCDate(), new Date('2016-12-31T23:59:59.999Z').getUTCDay(), new Date('2016-12-31T23:59:59.999Z').getUTCHours(), new Date('2016-12-31T23:59:59.999Z').getUTCMinutes(), new Date('2016-12-31T23:59:59.999Z').getUTCSeconds(), new Date('2016-12-31T23:59:59.999Z').getUTCMilliseconds()].join('|');"
        ),
        Ok(Value::String("2016|11|31|6|23|59|59|999".to_owned()))
    );
    assert_eq!(
        eval("Number.isNaN(new Date(NaN).getUTCFullYear());"),
        Ok(Value::Boolean(true))
    );
    assert!(eval("Date.prototype.getTime.call({});").is_err());
    assert!(eval("Date.prototype.getUTCFullYear.call({});").is_err());
    assert!(eval("Date.prototype.toJSON.call({});").is_err());
    assert!(eval("Date.prototype.toUTCString.call({});").is_err());
}
