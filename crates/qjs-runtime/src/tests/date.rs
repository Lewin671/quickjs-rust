use crate::{Value, eval};

#[test]
fn evaluates_date_builtins() {
    assert_eq!(
        eval("typeof Date;"),
        Ok(Value::String("function".to_owned().into()))
    );
    assert_eq!(eval("Date.length;"), Ok(Value::Number(7.0)));
    assert_eq!(eval("Date.parse.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Date.UTC.length;"), Ok(Value::Number(7.0)));
    assert_eq!(
        eval("Date.prototype.getTime.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Date.prototype.getFullYear.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Date.prototype.getMonth.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Date.prototype.getDate.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Date.prototype.getDay.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Date.prototype.getHours.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Date.prototype.getMinutes.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Date.prototype.getSeconds.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Date.prototype.getMilliseconds.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Date.prototype.getYear.length;"),
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
    assert_eq!(
        eval("Date.prototype.setTime.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Date.prototype.setYear.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Date.prototype.setFullYear.length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("Date.prototype.setMonth.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("Date.prototype.setDate.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Date.prototype.setHours.length;"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("Date.prototype.setMinutes.length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("Date.prototype.setSeconds.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("Date.prototype.setMilliseconds.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Date.prototype.setUTCDate.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Date.prototype.setUTCFullYear.length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("Date.prototype.setUTCHours.length;"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("Date.prototype.setUTCMilliseconds.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Date.prototype.setUTCMinutes.length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("Date.prototype.setUTCMonth.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("Date.prototype.setUTCSeconds.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(eval("new Date(0).getTime();"), Ok(Value::Number(0.0)));
    assert_eq!(eval("new Date(0).valueOf();"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("new Date(0).toISOString();"),
        Ok(Value::String("1970-01-01T00:00:00.000Z".to_owned().into()))
    );
    assert_eq!(
        eval("new Date(1899, 0).getYear();"),
        Ok(Value::Number(-1.0))
    );
    assert_eq!(eval("new Date(1900, 0).getYear();"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("new Date(1970, 0).getYear();"),
        Ok(Value::Number(70.0))
    );
    assert_eq!(
        eval("new Date(2000, 0).getYear();"),
        Ok(Value::Number(100.0))
    );
    assert_eq!(
        eval("Number.isNaN(new Date(NaN).getYear());"),
        Ok(Value::Boolean(true))
    );
    assert!(eval("Date.prototype.getYear.call({});").is_err());
    assert_eq!(
        eval("new Date(8640000000000000).toISOString();"),
        Ok(Value::String(
            "+275760-09-13T00:00:00.000Z".to_owned().into()
        ))
    );
    assert_eq!(
        eval("new Date(-8640000000000000).toISOString();"),
        Ok(Value::String(
            "-271821-04-20T00:00:00.000Z".to_owned().into()
        ))
    );
    assert_eq!(
        eval("Date.UTC(1970, 0, 2, 3, 4, 5, 6);"),
        Ok(Value::Number(97_445_006.0))
    );
    assert_eq!(eval("Date.UTC(1970);"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Date.UTC(2016, 6, 5, 15, 34, 45, 876);"),
        Ok(Value::Number(1_467_732_885_876.0))
    );
    assert_eq!(
        eval("Date.UTC(-0.999999, 0);"),
        Ok(Value::Number(-2_208_988_800_000.0))
    );
    assert_eq!(eval("Date.UTC(70.999999, 0);"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Date.UTC(99.999999, 0);"),
        Ok(Value::Number(915_148_800_000.0))
    );
    assert_eq!(
        eval(
            "var log = ''; function arg(name, value) { return { valueOf: function () { log += name; return value; } }; } Date.UTC(arg('y', 1970), arg('m', NaN), arg('d', 1), arg('h', 2), arg('i', 3), arg('s', 4), arg('x', 5)); log;"
        ),
        Ok(Value::String("ymdhisx".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var log = ''; function arg(name, value) { return { valueOf: function () { log += name; if (value === 'throw') { throw new TypeError(); } return value; } }; } try { Date.UTC(arg('y', 1970), arg('m', 'throw'), arg('d', 1)); } catch (error) {} log;"
        ),
        Ok(Value::String("ym".to_owned().into()))
    );
    assert_eq!(
        eval("Date.UTC(1970, 0, 1, 80063993375, 29, 1, -288230376151711740);"),
        Ok(Value::Number(29_312.0))
    );
    assert_eq!(
        eval("Date.UTC(1970, 0, 213503982336, 0, 0, 0, -18446744073709552000);"),
        Ok(Value::Number(34_447_360.0))
    );
    assert_eq!(
        eval("Date.parse('1970-01-02T03:04:05.006Z');"),
        Ok(Value::Number(97_445_006.0))
    );
    assert_eq!(
        eval("Date.parse('1970-01-02T03:04:05.006');"),
        Ok(Value::Number(97_445_006.0))
    );
    assert_eq!(eval("Date.parse('1970-01-01');"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Number.isNaN(Date.parse('-000000-03-31T00:45Z'));"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Date.parse('+275760-09-13T00:00:00.000Z');"),
        Ok(Value::Number(8_640_000_000_000_000.0))
    );
    assert_eq!(
        eval("Date.parse('-271821-04-20T00:00:00.000Z');"),
        Ok(Value::Number(-8_640_000_000_000_000.0))
    );
    assert_eq!(
        eval("Number.isNaN(Date.parse('+275760-09-13T00:00:00.001Z'));"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let zero = new Date(0); \
             [Date.parse(zero.toString()), Date.parse(zero.toUTCString()), Date.parse(zero.toISOString())].join('|');"
        ),
        Ok(Value::String("0|0|0".to_owned().into()))
    );
    assert_eq!(
        eval("new Date('1970-01-02T03:04:05.006Z').toISOString();"),
        Ok(Value::String("1970-01-02T03:04:05.006Z".to_owned().into()))
    );
    assert_eq!(
        eval("new Date('1970-01-02T03:04:05.006Z').toUTCString();"),
        Ok(Value::String(
            "Fri, 02 Jan 1970 03:04:05 GMT".to_owned().into()
        ))
    );
    assert_eq!(
        eval("Date.prototype.toGMTString === Date.prototype.toUTCString;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Date.prototype, 'getMonth'); (d.value === Date.prototype.getMonth) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Date.prototype, 'setMonth'); (d.value === Date.prototype.setMonth) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Date.prototype, 'toLocaleString'); (d.value === Date.prototype.toLocaleString) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned().into()))
    );
    assert_eq!(
        eval("new Date('1970-01-02T03:04:05.006Z').toGMTString();"),
        Ok(Value::String(
            "Fri, 02 Jan 1970 03:04:05 GMT".to_owned().into()
        ))
    );
    assert_eq!(
        eval("new Date('1970-01-02T03:04:05.006Z').toJSON();"),
        Ok(Value::String("1970-01-02T03:04:05.006Z".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var d = new Date(0); var result = d.setTime(97445006.9); [result, d.getTime(), d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "97445006|97445006|1970-01-02T03:04:05.006Z"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(
        eval("var d = new Date(0); Number.isNaN(d.setTime(NaN)) && Number.isNaN(d.getTime());"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("var d = new Date(0); Number.isNaN(d.setTime(8640000000000001));"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "var d = new Date(1970, 1, 2, 3, 4, 5); var result = d.setYear(71); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "34311845000|1971-02-02T03:04:05.000Z".to_owned().into()
        ))
    );
    assert_eq!(
        eval("var d = new Date(1970, 0); d.setYear(50.999999); d.getFullYear();"),
        Ok(Value::Number(1950.0))
    );
    assert_eq!(
        eval("var d = new Date(1970, 0); d.setYear(100); d.getFullYear();"),
        Ok(Value::Number(100.0))
    );
    assert_eq!(
        eval(
            "var d = new Date(NaN); var result = d.setYear(71); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "31536000000|1971-01-01T00:00:00.000Z".to_owned().into()
        ))
    );
    assert_eq!(
        eval("var d = new Date(0); Number.isNaN(d.setYear()) && Number.isNaN(d.getTime());"),
        Ok(Value::Boolean(true))
    );
    assert!(eval("Date.prototype.setYear.call({}, 0);").is_err());
    assert_eq!(
        eval(
            "var d = new Date('1970-06-02T03:04:05.006Z'); var result = d.setUTCFullYear(2000); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "959915045006|2000-06-02T03:04:05.006Z".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "var d = new Date('1970-01-02T03:04:05.006Z'); d.setUTCFullYear(1, 1, 3); d.toISOString();"
        ),
        Ok(Value::String("0001-02-03T03:04:05.006Z".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var d = new Date(NaN); var result = d.setUTCFullYear(1); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "-62135596800000|0001-01-01T00:00:00.000Z".to_owned().into()
        ))
    );
    assert_eq!(
        eval("var d = new Date(0); Number.isNaN(d.setUTCFullYear(undefined));"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "var d = new Date('1970-01-31T03:04:05.006Z'); var result = d.setUTCMonth(1); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "5281445006|1970-03-03T03:04:05.006Z".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "var d = new Date('1970-01-31T03:04:05.006Z'); var result = d.setUTCMonth(1, 1); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "2689445006|1970-02-01T03:04:05.006Z".to_owned().into()
        ))
    );
    assert_eq!(
        eval("var d = new Date('1970-01-31T03:04:05.006Z'); d.setUTCMonth(-1); d.toISOString();"),
        Ok(Value::String("1969-12-31T03:04:05.006Z".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var d = new Date(NaN); Number.isNaN(d.setUTCMonth(6, 7)) && Number.isNaN(d.getTime());"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("var d = new Date(0); Number.isNaN(d.setUTCMonth(undefined, 1));"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "var d = new Date('1970-01-31T03:04:05.006Z'); var result = d.setUTCDate(1); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "11045006|1970-01-01T03:04:05.006Z".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "var d = new Date('1970-01-31T03:04:05.006Z'); var result = d.setUTCDate(32); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "2689445006|1970-02-01T03:04:05.006Z".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "var d = new Date('1970-01-31T03:04:05.006Z'); var result = d.setUTCDate(0); [result, d.toISOString()].join('|');"
        ),
        Ok(Value::String(
            "-75354994|1969-12-31T03:04:05.006Z".to_owned().into()
        ))
    );
    assert_eq!(
        eval("var d = new Date(NaN); Number.isNaN(d.setUTCDate(1)) && Number.isNaN(d.getTime());"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "var d = new Date(0); Number.isNaN(d.setUTCDate(undefined)) && Number.isNaN(d.getTime());"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Number.isNaN(new Date(8640000000000001).getTime());"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Number.isNaN(new Date(Infinity).getTime());"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("new Date(97445006.9).getTime();"),
        Ok(Value::Number(97_445_006.0))
    );
    assert_eq!(
        eval("Object.is(new Date(-0).getTime(), 0);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("new Date(2016, 0).getFullYear();"),
        Ok(Value::Number(2016.0))
    );
    assert_eq!(
        eval("new Date(2016, 0, 1, 0, 0, 0, -1).getFullYear();"),
        Ok(Value::Number(2015.0))
    );
    assert_eq!(
        eval("Number.isNaN(new Date(NaN).getFullYear());"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { Date.prototype.getFullYear.call({}); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Number.isNaN(Date.UTC(275760, 8, 13, 0, 0, 0, 1));"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Number.isNaN(new Date(275760, 8, 13, 0, 0, 0, 1).getTime());"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("new Date('0020-01-01T00:00:00Z').toUTCString();"),
        Ok(Value::String(
            "Wed, 01 Jan 0020 00:00:00 GMT".to_owned().into()
        ))
    );
    assert_eq!(
        eval("new Date(NaN).toUTCString();"),
        Ok(Value::String("Invalid Date".to_owned().into()))
    );
    assert_eq!(eval("new Date(NaN).toJSON();"), Ok(Value::Null));
    assert_eq!(
        eval("JSON.stringify(new Date('1970-01-02T03:04:05.006Z'));"),
        Ok(Value::String(
            "\"1970-01-02T03:04:05.006Z\"".to_owned().into()
        ))
    );
    assert_eq!(
        eval("JSON.stringify({when: new Date('1970-01-02T03:04:05.006Z')});"),
        Ok(Value::String(
            "{\"when\":\"1970-01-02T03:04:05.006Z\"}".to_owned().into()
        ))
    );
    assert_eq!(
        eval("JSON.stringify(new Date(NaN));"),
        Ok(Value::String("null".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var d = new Date('1970-01-02T03:04:05.006Z'); d.toISOString = function () { return 'custom'; }; d.toJSON();"
        ),
        Ok(Value::String("custom".to_owned().into()))
    );
    assert_eq!(
        eval(
            "[new Date('2016-12-31T23:59:59.999Z').getUTCFullYear(), new Date('2016-12-31T23:59:59.999Z').getUTCMonth(), new Date('2016-12-31T23:59:59.999Z').getUTCDate(), new Date('2016-12-31T23:59:59.999Z').getUTCDay(), new Date('2016-12-31T23:59:59.999Z').getUTCHours(), new Date('2016-12-31T23:59:59.999Z').getUTCMinutes(), new Date('2016-12-31T23:59:59.999Z').getUTCSeconds(), new Date('2016-12-31T23:59:59.999Z').getUTCMilliseconds()].join('|');"
        ),
        Ok(Value::String("2016|11|31|6|23|59|59|999".to_owned().into()))
    );
    assert_eq!(
        eval("Number.isNaN(new Date(NaN).getUTCFullYear());"),
        Ok(Value::Boolean(true))
    );
    assert!(eval("Date.prototype.getTime.call({});").is_err());
    assert!(eval("Date.prototype.getUTCFullYear.call({});").is_err());
    assert!(eval("Date.prototype.setTime.call({}, 0);").is_err());
    assert!(eval("Date.prototype.toJSON.call({});").is_err());
    assert!(eval("Date.prototype.toUTCString.call({});").is_err());
}

#[test]
fn date_parse_rejects_out_of_range_month_and_day() {
    // Month > 12 or day > 31 is invalid in every ISO form (date-only,
    // year-month, and date+time), not just the date+time path.
    for source in [
        "2020-13-01",
        "2020-01-32",
        "2020-13",
        "2020-13-01T00:00:00Z",
        "2020-01-32T00:00:00Z",
    ] {
        assert_eq!(
            eval(&format!("Number.isNaN(Date.parse('{source}'));")),
            Ok(Value::Boolean(true)),
            "expected NaN for {source}"
        );
    }
    // In-range date-only and year-month values still parse.
    assert_eq!(
        eval("Date.parse('2020-12-31');"),
        Ok(Value::Number(1609372800000.0))
    );
    assert_eq!(
        eval("Date.parse('2020-12');"),
        Ok(Value::Number(1606780800000.0))
    );
}
