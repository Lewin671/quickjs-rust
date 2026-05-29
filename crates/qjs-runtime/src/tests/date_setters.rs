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
