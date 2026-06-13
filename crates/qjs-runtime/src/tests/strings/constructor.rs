use crate::{Value, eval};

#[test]
fn evaluates_string_constructor_and_statics() {
    assert_eq!(
        eval("typeof String;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("String.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("String();"), Ok(Value::String(String::new())));
    assert_eq!(eval("String(123);"), Ok(Value::String("123".to_owned())));
    assert_eq!(eval("String(null);"), Ok(Value::String("null".to_owned())));
    assert_eq!(
        eval("String.fromCharCode(65, 66, 67);"),
        Ok(Value::String("ABC".to_owned()))
    );
    assert_eq!(
        eval("String.fromCharCode(0x00a0, 0x2028, 0xffff).length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("String.fromCharCode(0x00a0, 0x2028, 0xffff).charCodeAt(2);"),
        Ok(Value::Number(65535.0))
    );
    assert_eq!(
        eval("String.fromCharCode(0xd800, 0xdc00).charCodeAt(0);"),
        Ok(Value::Number(55296.0))
    );
    assert_eq!(
        eval("String.fromCharCode(0xd800, 0xdc00).charCodeAt(1);"),
        Ok(Value::Number(56320.0))
    );
    assert_eq!(
        eval(
            "let value = String.fromCodePoint(65, 128512, 67); value.length + ':' + value.charCodeAt(0) + ':' + value.charCodeAt(1) + ':' + value.charCodeAt(2) + ':' + value.charCodeAt(3);"
        ),
        Ok(Value::String("4:65:55357:56832:67".to_owned()))
    );
    assert_eq!(
        eval("String.fromCodePoint();"),
        Ok(Value::String(String::new()))
    );
    assert_eq!(eval("String.fromCodePoint.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval(
            "let caught = false; try { String.fromCodePoint(-1); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { String.fromCodePoint(1.5); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { String.fromCodePoint(1114112); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("String.raw.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("String.raw({ raw: ['a', 'b', 'c'] }, 1, 2);"),
        Ok(Value::String("a1b2c".to_owned()))
    );
    assert_eq!(
        eval("String.raw({ raw: { 0: 'x', 1: 'y', 2: 'z', length: 3 } }, 'A');"),
        Ok(Value::String("xAyz".to_owned()))
    );
    assert_eq!(
        eval("String.raw({ raw: { length: 0 } });"),
        Ok(Value::String(String::new()))
    );
    assert!(eval("String.raw(null);").is_err());
    assert!(eval("String.raw({ raw: null });").is_err());
    assert_eq!(
        eval("String.prototype.constructor === String;"),
        Ok(Value::Boolean(true))
    );
}
