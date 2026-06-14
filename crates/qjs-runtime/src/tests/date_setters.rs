use crate::{Value, eval};

#[test]
fn evaluates_date_utc_time_setters() {
    assert_eq!(
        eval(
            "var d = new Date('1970-01-02T03:04:05.006Z'); var result = d.setUTCHours(10); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "122645006|1970-01-02T10:04:05.006Z".to_owned()
        ))
    );
    assert_eq!(
        eval(
            "var d = new Date('1970-01-02T03:04:05.006Z'); var result = d.setUTCHours(10, 11, 12, 13); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "123072013|1970-01-02T10:11:12.013Z".to_owned()
        ))
    );
    assert_eq!(
        eval(
            "var d = new Date('1970-01-02T03:04:05.006Z'); var result = d.setUTCHours(24); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "173045006|1970-01-03T00:04:05.006Z".to_owned()
        ))
    );
    assert_eq!(
        eval(
            "var d = new Date('1970-01-02T03:04:05.006Z'); var result = d.setUTCMinutes(30); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "99005006|1970-01-02T03:30:05.006Z".to_owned()
        ))
    );
    assert_eq!(
        eval(
            "var d = new Date('1970-01-02T03:04:05.006Z'); var result = d.setUTCMinutes(30, 31, 32); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "99031032|1970-01-02T03:30:31.032Z".to_owned()
        ))
    );
    assert_eq!(
        eval(
            "var d = new Date('1970-01-02T03:04:05.006Z'); var result = d.setUTCSeconds(40); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "97480006|1970-01-02T03:04:40.006Z".to_owned()
        ))
    );
    assert_eq!(
        eval(
            "var d = new Date('1970-01-02T03:04:05.006Z'); var result = d.setUTCSeconds(40, 41); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "97480041|1970-01-02T03:04:40.041Z".to_owned()
        ))
    );
    assert_eq!(
        eval(
            "var d = new Date('1970-01-02T03:04:05.006Z'); var result = d.setUTCMilliseconds(500); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "97445500|1970-01-02T03:04:05.500Z".to_owned()
        ))
    );
    assert_eq!(
        eval(
            "var d = new Date(NaN); Number.isNaN(d.setUTCHours(1, 2, 3, 4)) && Number.isNaN(d.getTime());"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "var d = new Date(0); Number.isNaN(d.setUTCSeconds(undefined, 1)) && Number.isNaN(d.getTime());"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "var d = new Date(0); Number.isNaN(d.setUTCMilliseconds()) && Number.isNaN(d.getTime());"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn date_setters_coerce_arguments_before_invalid_date_return() {
    assert_eq!(
        eval(
            "var d = new Date(NaN); var calls = 0; \
             var value = { valueOf() { calls++; d.setTime(0); return 1; } }; \
             var result = d.setDate(value); \
             calls + ':' + Number.isNaN(result) + ':' + d.getTime();"
        ),
        Ok(Value::String("1:true:0".to_owned()))
    );
    assert_eq!(
        eval(
            "var d = new Date(NaN); var effects = []; \
             var month = { valueOf() { effects.push('month'); return 0; } }; \
             var date = { valueOf() { effects.push('date'); d.setTime(0); return 1; } }; \
             var result = d.setMonth(month, date); \
             effects.join(',') + ':' + Number.isNaN(result) + ':' + d.getTime();"
        ),
        Ok(Value::String("month,date:true:0".to_owned()))
    );
    assert_eq!(
        eval(
            "var d = new Date(NaN); var calls = 0; \
             var value = { valueOf() { calls++; d.setTime(0); return 1; } }; \
             var result = d.setUTCHours(value); \
             calls + ':' + Number.isNaN(result) + ':' + d.getTime();"
        ),
        Ok(Value::String("1:true:0".to_owned()))
    );
}

#[test]
fn date_setters_coerce_optional_arguments_with_environment() {
    assert_eq!(
        eval(
            "var d = new Date('2016-07-07T11:36:23.002Z'); var calls = 0; \
             var month = { valueOf() { calls++; return 2; } }; \
             d.setFullYear(2016, month); calls + ':' + d.toISOString();"
        ),
        Ok(Value::String("1:2016-03-07T11:36:23.002Z".to_owned()))
    );
    assert_eq!(
        eval(
            "var d = new Date('2016-07-07T11:36:23.002Z'); var calls = 0; \
             var day = { valueOf() { calls++; return 2; } }; \
             d.setUTCMonth(6, day); calls + ':' + d.toISOString();"
        ),
        Ok(Value::String("1:2016-07-02T11:36:23.002Z".to_owned()))
    );
}
