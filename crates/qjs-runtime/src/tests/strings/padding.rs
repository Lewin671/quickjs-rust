use crate::{Value, eval};

#[test]
fn evaluates_string_padding_and_repeat_builtins() {
    assert_eq!(
        eval("String.prototype.padStart.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("String.prototype.padEnd.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("'abc'.padStart(7, 'def');"),
        Ok(Value::String("defdabc".to_owned().into()))
    );
    assert_eq!(
        eval("'abc'.padEnd(7, 'def');"),
        Ok(Value::String("abcdefd".to_owned().into()))
    );
    assert_eq!(
        eval("'abc'.padStart(5);"),
        Ok(Value::String("  abc".to_owned().into()))
    );
    assert_eq!(
        eval("'abc'.padEnd(5);"),
        Ok(Value::String("abc  ".to_owned().into()))
    );
    assert_eq!(
        eval("'abc'.padStart(5, '');"),
        Ok(Value::String("abc".to_owned().into()))
    );
    assert_eq!(
        eval("'abc'.padEnd(2, '*');"),
        Ok(Value::String("abc".to_owned().into()))
    );
    assert_eq!(
        eval("'ab'.repeat(3);"),
        Ok(Value::String("ababab".to_owned().into()))
    );
    assert_eq!(
        eval("'ab'.repeat(0);"),
        Ok(Value::String(::std::rc::Rc::new(String::new())))
    );
    assert_eq!(
        eval("'ab'.repeat(2.8);"),
        Ok(Value::String("abab".to_owned().into()))
    );
    assert_eq!(
        eval("let str = ''; let i = 0; while (i < 10000) { str += '.'; i++; } str.length;"),
        Ok(Value::Number(10000.0))
    );
    assert_eq!(
        eval(
            "var str = ''; var i = 0; while (i < 10000) { str += '.'; i++; } str.length + ':' + globalThis.str.length;"
        ),
        Ok(Value::String("10000:10000".to_owned().into()))
    );
    assert_eq!(
        eval("let value = 1; value += 'x';"),
        Ok(Value::String("1x".to_owned().into()))
    );
    assert!(eval("'ab'.repeat(-1);").is_err());
    assert!(eval("'ab'.repeat(Infinity);").is_err());
    assert_eq!(
        eval(
            r#"
            let log = "";
            function observer(name, string, value) {
                return {
                    toString: function() { log = log + "|toString:" + name; return string; },
                    valueOf: function() { log = log + "|valueOf:" + name; return value; }
                };
            }
            let receiver = observer("receiver", {}, "abc");
            let maxLength = observer("maxLength", 11, {});
            let fillString = observer("fillString", {}, "def");
            let result = String.prototype.padStart.call(receiver, maxLength, fillString);
            result + log;
            "#
        ),
        Ok(Value::String(
            "defdefdeabc|toString:receiver|valueOf:receiver|valueOf:maxLength|toString:maxLength|toString:fillString|valueOf:fillString"
                .to_owned().into()
        ))
    );
}

#[test]
fn pad_beyond_max_length_throws_range_error() {
    // Padding past the 2^30-1 string-length limit throws instead of allocating
    // a multi-gigabyte string.
    assert!(eval("'a'.padStart(1073741824);").is_err());
    assert!(eval("'a'.padEnd(2147483648);").is_err());
}
