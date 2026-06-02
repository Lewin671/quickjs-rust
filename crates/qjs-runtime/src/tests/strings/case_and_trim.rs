use crate::{Value, eval};

#[test]
fn evaluates_string_case_and_trim_builtins() {
    assert_eq!(
        eval("'abc'.toString();"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval("'abc'.valueOf();"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval("(new String('abc')).toString();"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval("(new String('abc')).valueOf();"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval(
            "let caught = false; try { String.prototype.toString.call(false); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("typeof Symbol;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(
        eval(
            "let caught = false; try { String.prototype.toString.call(Symbol('desc')); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { String.prototype.valueOf.call({ toString: function() { return 'str'; } }); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("String.prototype.toLowerCase.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("String.prototype.toUpperCase.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("'AbC123'.toLowerCase();"),
        Ok(Value::String("abc123".to_owned()))
    );
    assert_eq!(
        eval("'AbC123'.toUpperCase();"),
        Ok(Value::String("ABC123".to_owned()))
    );
    assert_eq!(
        eval("'  abc  '.trim();"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval("'\\uFEFFabc\\uFEFF'.trim();"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval(
            "'\\u0009\\u000A\\u000B\\u000C\\u000D\\u0020\\u00A0\\u1680\\u2000\\u2001\\u2002\\u2003\\u2004\\u2005\\u2006\\u2007\\u2008\\u2009\\u200A\\u2028\\u2029\\u202F\\u205F\\u3000\\uFEFF'.trim();"
        ),
        Ok(Value::String(String::new()))
    );
    assert_eq!(
        eval("String.prototype.trim.call(new RegExp(/test/));"),
        Ok(Value::String("/test/".to_owned()))
    );
    assert_eq!(
        eval("'  abc  '.trimStart();"),
        Ok(Value::String("abc  ".to_owned()))
    );
    assert_eq!(
        eval("'  abc  '.trimLeft();"),
        Ok(Value::String("abc  ".to_owned()))
    );
    assert_eq!(
        eval("'  abc  '.trimEnd();"),
        Ok(Value::String("  abc".to_owned()))
    );
    assert_eq!(
        eval("'  abc  '.trimRight();"),
        Ok(Value::String("  abc".to_owned()))
    );
    assert_eq!(
        eval("String.prototype.trim.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("String.prototype.trimStart.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("String.prototype.trimEnd.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("String.prototype.trimLeft.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("String.prototype.trimRight.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("String.prototype.trimLeft === String.prototype.trimStart;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("String.prototype.trimRight === String.prototype.trimEnd;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(String.prototype, 'charAt').enumerable;"),
        Ok(Value::Boolean(false))
    );
}
