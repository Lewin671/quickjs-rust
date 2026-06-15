use crate::{Value, eval};

#[test]
fn evaluates_number_builtins() {
    assert_eq!(
        eval("typeof Number;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Number.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Number();"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Number(undefined) === Number(undefined);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("Number('10');"), Ok(Value::Number(10.0)));
    assert_eq!(eval("Number('0b11');"), Ok(Value::Number(3.0)));
    assert_eq!(eval("Number('0B010');"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Number('0o77');"), Ok(Value::Number(63.0)));
    assert_eq!(eval("Number('0O010');"), Ok(Value::Number(8.0)));
    assert_eq!(eval("Number(true);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Number(null);"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Number.prototype.constructor === Number;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Number.prototype.toString.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Number.prototype.toLocaleString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Number.prototype.toFixed.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Number.prototype.toExponential.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Number.prototype.toPrecision.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Number.prototype.valueOf.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("let n = 10; n.toString();"),
        Ok(Value::String("10".to_owned()))
    );
    assert_eq!(
        eval("(10).toLocaleString();"),
        Ok(Value::String("10".to_owned()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Number.prototype, 'toLocaleString'); (d.value === Number.prototype.toLocaleString) + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("true:true:false:true".to_owned()))
    );
    assert_eq!(
        eval("let n = 255; n.toString(16);"),
        Ok(Value::String("ff".to_owned()))
    );
    assert_eq!(
        eval(
            "let caught = false; try { (3).toString(1); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { (3).toString(37); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let n = 1e21; n.toString();"),
        Ok(Value::String("1e+21".to_owned()))
    );
    assert_eq!(
        eval("let n = 1e-7; n.toString();"),
        Ok(Value::String("1e-7".to_owned()))
    );
    assert_eq!(
        eval("String.prototype.trim.call(1000000000000000000000);"),
        Ok(Value::String("1e+21".to_owned()))
    );
    assert_eq!(eval("let n = 10; n.valueOf();"), Ok(Value::Number(10.0)));
    assert_eq!(
        eval("(new Number(7)).toString();"),
        Ok(Value::String("7".to_owned()))
    );
    assert_eq!(
        eval("(1000000000000000128).toString();"),
        Ok(Value::String("1000000000000000100".to_owned()))
    );
    assert_eq!(
        eval("(1.25).toFixed(1);"),
        Ok(Value::String("1.3".to_owned()))
    );
    assert_eq!(
        eval("(1.25).toFixed(2);"),
        Ok(Value::String("1.25".to_owned()))
    );
    assert_eq!(
        eval("(1000000000000000128).toFixed(0);"),
        Ok(Value::String("1000000000000000128".to_owned()))
    );
    assert_eq!(
        eval("(new Number(7)).toFixed(2);"),
        Ok(Value::String("7.00".to_owned()))
    );
    assert_eq!(
        eval("Number.prototype.toFixed.call(-0, 2);"),
        Ok(Value::String("0.00".to_owned()))
    );
    assert_eq!(
        eval("(1e21).toFixed(2);"),
        Ok(Value::String("1e+21".to_owned()))
    );
    assert_eq!(eval("NaN.toFixed(2);"), Ok(Value::String("NaN".to_owned())));
    assert_eq!(
        eval("(12.345).toExponential();"),
        Ok(Value::String("1.2345e+1".to_owned()))
    );
    assert_eq!(
        eval("(12.345).toExponential(2);"),
        Ok(Value::String("1.23e+1".to_owned()))
    );
    assert_eq!(
        eval("(123.456).toExponential([2]);"),
        Ok(Value::String("1.23e+2".to_owned()))
    );
    assert_eq!(
        eval("(1).toExponential(0);"),
        Ok(Value::String("1e+0".to_owned()))
    );
    assert_eq!(
        eval("(25).toExponential(0);"),
        Ok(Value::String("3e+1".to_owned()))
    );
    assert_eq!(
        eval("(12345).toExponential(3);"),
        Ok(Value::String("1.235e+4".to_owned()))
    );
    assert_eq!(
        eval("Number.prototype.toExponential.call(-0, 2);"),
        Ok(Value::String("0.00e+0".to_owned()))
    );
    assert_eq!(
        eval("(new Number(7)).toExponential(1);"),
        Ok(Value::String("7.0e+0".to_owned()))
    );
    assert_eq!(
        eval("NaN.toExponential(101);"),
        Ok(Value::String("NaN".to_owned()))
    );
    assert_eq!(
        eval("Infinity.toExponential(101);"),
        Ok(Value::String("Infinity".to_owned()))
    );
    assert_eq!(
        eval(
            "let calls = 0; NaN.toExponential({ valueOf: function() { calls++; return 1; } }); calls;"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("(123.456).toPrecision();"),
        Ok(Value::String("123.456".to_owned()))
    );
    assert_eq!(
        eval("(123.456).toPrecision(5);"),
        Ok(Value::String("123.46".to_owned()))
    );
    assert_eq!(
        eval("(123.456).toPrecision(2);"),
        Ok(Value::String("1.2e+2".to_owned()))
    );
    assert_eq!(
        eval("(123.456).toPrecision([2]);"),
        Ok(Value::String("1.2e+2".to_owned()))
    );
    assert_eq!(
        eval("(0.0001234).toPrecision(5);"),
        Ok(Value::String("0.00012340".to_owned()))
    );
    assert_eq!(
        eval("(1e-7).toPrecision(2);"),
        Ok(Value::String("1.0e-7".to_owned()))
    );
    assert_eq!(
        eval("(new Number(7)).toPrecision(3);"),
        Ok(Value::String("7.00".to_owned()))
    );
    assert_eq!(
        eval("NaN.toPrecision(101);"),
        Ok(Value::String("NaN".to_owned()))
    );
    assert_eq!(
        eval(
            "let calls = 0; NaN.toPrecision({ valueOf: function() { calls++; return 1 / 0; } }); calls;"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("(new Number(7)).valueOf();"), Ok(Value::Number(7.0)));
    assert_eq!(
        eval("let n = new Number(7); n.tag = Object.prototype.toString; n.tag();"),
        Ok(Value::String("[object Number]".to_owned()))
    );
    assert!(eval("let o = Object.create(Number.prototype); o.valueOf();").is_err());
    assert!(eval("let o = Object.create(Number.prototype); o.toFixed();").is_err());
    assert!(eval("let o = Object.create(Number.prototype); o.toExponential();").is_err());
    assert!(eval("let o = Object.create(Number.prototype); o.toPrecision();").is_err());
    assert_eq!(
        eval(
            "let caught = false; try { (3).toFixed(101); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { (3).toExponential(101); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { (3).toPrecision(0); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Number('abc') === Number('abc');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Number('0b2') === Number('0b2');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Number('+0o10') === Number('+0o10');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Number.NaN === Number.NaN;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Number.POSITIVE_INFINITY === Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Number.NEGATIVE_INFINITY === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Number.MAX_SAFE_INTEGER;"),
        Ok(Value::Number(9_007_199_254_740_991.0))
    );
    assert_eq!(
        eval("Number.MIN_SAFE_INTEGER;"),
        Ok(Value::Number(-9_007_199_254_740_991.0))
    );
    assert_eq!(
        eval(
            "Number.MIN_VALUE > 0 && Number.MIN_VALUE < Number.EPSILON && Number.MIN_VALUE / 2 === 0;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Number.isFinite.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Number.isInteger.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Number.isNaN.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Number.isSafeInteger.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Number.isFinite(10);"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Number.isFinite(Infinity);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("Number.isFinite('10');"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Number.isNaN(NaN);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Number.isNaN('NaN');"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Number.isInteger(10);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Number.isInteger(10.5);"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("Number.isSafeInteger(9007199254740991);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Number.isSafeInteger(9007199254740992);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(Number, 'NaN').writable;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("parseInt.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("parseFloat.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Number.parseInt.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Number.parseFloat.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Number.parseInt === parseInt;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("isFinite.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("isNaN.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("isFinite(10);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("isFinite('10');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("isFinite(null);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("isFinite(Infinity);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("isFinite(undefined);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("isNaN(NaN);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("isNaN('abc');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("isNaN('10');"), Ok(Value::Boolean(false)));
    assert_eq!(eval("isNaN(null);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("parseInt('15px');"), Ok(Value::Number(15.0)));
    assert_eq!(eval("parseInt('0x10');"), Ok(Value::Number(16.0)));
    assert_eq!(eval("parseInt('10', 2);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("parseInt('-10', 10);"), Ok(Value::Number(-10.0)));
    assert_eq!(eval("parseInt('z', 36);"), Ok(Value::Number(35.0)));
    assert_eq!(eval("parseInt('1Z\\u0660', 36);"), Ok(Value::Number(71.0)));
    assert_eq!(
        eval(
            "let hits = []; parseInt({ toString() { hits.push('string'); return '11'; } }, { valueOf() { hits.push('radix'); return 2; } }) + ':' + hits.join(',');"
        ),
        Ok(Value::String("3:string,radix".to_owned()))
    );
    assert_eq!(
        eval("parseInt('10', 37) === NaN;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("parseFloat('3.5px');"), Ok(Value::Number(3.5)));
    assert_eq!(eval("parseFloat('-1.25e2x');"), Ok(Value::Number(-125.0)));
    assert_eq!(
        eval(
            "let hits = []; parseFloat({ toString() { hits.push('string'); return '1.5'; } }) + ':' + hits.join(',');"
        ),
        Ok(Value::String("1.5:string".to_owned()))
    );
    assert_eq!(
        eval("parseFloat('Infinity');"),
        Ok(Value::Number(f64::INFINITY))
    );
    assert_eq!(eval("parseFloat('x') === NaN;"), Ok(Value::Boolean(false)));
    assert!(eval("new Number.isNaN(NaN);").is_err());
    assert!(eval("new parseInt('10');").is_err());
    assert!(eval("new isNaN(1);").is_err());
}
