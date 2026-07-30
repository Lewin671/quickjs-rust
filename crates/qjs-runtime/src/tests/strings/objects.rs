use crate::{Value, eval};

#[test]
fn evaluates_string_objects() {
    assert_eq!(
        eval("typeof new String('abc');"),
        Ok(Value::String("object".to_owned().into()))
    );
    assert_eq!(
        eval("let s = new String('abc'); s.constructor === String;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let s = new String('abc'); s.valueOf();"),
        Ok(Value::String("abc".to_owned().into()))
    );
    assert_eq!(
        eval("let s = new String('abc'); s.toString();"),
        Ok(Value::String("abc".to_owned().into()))
    );
    assert_eq!(eval("new String('abc').length;"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("let s = new String('abc'); s[1];"),
        Ok(Value::String("b".to_owned().into()))
    );
    assert_eq!(
        eval("let s = new String('abc'); try { s.length = 1; } catch (error) {} s.length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("let s = new String('abc'); s == 'abc';"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let s = new String('abc'); s !== 'abc';"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new String('abc'));"),
        Ok(Value::String("[object String]".to_owned().into()))
    );
    assert_eq!(
        eval("new String('abc').charAt(2);"),
        Ok(Value::String("c".to_owned().into()))
    );
}

#[test]
fn string_prototype_is_empty_string_object() {
    assert_eq!(
        eval(
            "[
                String.prototype == '',
                String.prototype.valueOf(),
                String.prototype.length,
                Object.prototype.isPrototypeOf(String.prototype),
                (delete String.prototype.toString,
                 Object.prototype.toString.call(String.prototype))
             ].join('|');"
        ),
        Ok(Value::String(
            "true||0|true|[object String]".to_owned().into()
        ))
    );
}

#[test]
fn boxed_strings_preserve_utf16_properties_and_copy_on_write_values() {
    assert_eq!(
        eval(
            "let source = 'a'; let wrapped = new String(source); source += 'b'; let boxed = Object(source); let index = Object.getOwnPropertyDescriptor(wrapped, '0'); [wrapped.valueOf(), source, boxed.valueOf(), Object.keys(wrapped).join(','), index.writable, index.enumerable, index.configurable].join('|');"
        ),
        Ok(Value::String(
            "a|ab|ab|0|false|true|false".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let value = String.fromCharCode(0xd834, 0xdf06); let wrapped = new String(value); let boxed = Object(value); let first = Object.getOwnPropertyDescriptor(wrapped, '0'); [wrapped.length, wrapped.charCodeAt(0), wrapped.charCodeAt(1), Object.keys(wrapped).join(','), first.writable, first.enumerable, first.configurable, boxed.length, boxed.charCodeAt(0), boxed.charCodeAt(1)].join('|');"
        ),
        Ok(Value::String(
            "2|55348|57094|0,1|false|true|false|2|55348|57094"
                .to_owned()
                .into()
        ))
    );
}
