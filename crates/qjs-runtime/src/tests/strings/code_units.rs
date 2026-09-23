use crate::{Value, eval};

#[test]
fn evaluates_string_code_unit_builtins() {
    assert_eq!(eval("String.prototype.at.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("'abc'.at(1);"),
        Ok(Value::String("b".to_owned().into()))
    );
    assert_eq!(
        eval("'abc'.at(-1);"),
        Ok(Value::String("c".to_owned().into()))
    );
    assert_eq!(eval("'abc'.at(3);"), Ok(Value::Undefined));
    assert_eq!(eval("'abc'.at(-4);"), Ok(Value::Undefined));
    assert_eq!(
        eval("'😀'.at(0).charCodeAt(0);"),
        Ok(Value::Number(55_357.0))
    );
    assert_eq!(
        eval("'😀'.at(1).charCodeAt(0);"),
        Ok(Value::Number(56_832.0))
    );
    assert_eq!(
        eval("'abc'.at();"),
        Ok(Value::String("a".to_owned().into()))
    );
    assert_eq!(
        eval("'abc'.at(1.9);"),
        Ok(Value::String("b".to_owned().into()))
    );
    assert_eq!(
        eval("String.prototype.charAt.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("'abc'.charAt(1);"),
        Ok(Value::String("b".to_owned().into()))
    );
    assert_eq!(
        eval("'abc'.charAt(9);"),
        Ok(Value::String(crate::JsString::default()))
    );
    assert_eq!(
        eval("String.prototype.charCodeAt.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("'abc'.charCodeAt(1);"), Ok(Value::Number(98.0)));
    assert_eq!(
        eval("'abc'.charCodeAt(undefined);"),
        Ok(Value::Number(97.0))
    );
    assert_eq!(
        eval("let x = 'abc'.charCodeAt(9); x !== x;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let x = 'abc'.charCodeAt(-1); x !== x;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("'😀'.charCodeAt(0);"), Ok(Value::Number(55_357.0)));
    assert_eq!(eval("'😀'.charCodeAt(1);"), Ok(Value::Number(56_832.0)));
    assert_eq!(
        eval(
            "let conversions = 0; let index = { valueOf() { conversions += 1; return 1; } }; 'abc'.charCodeAt(index) === 98 && conversions === 1;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "'abc'.charCodeAt(NaN) === 97 && 'abc'.charCodeAt(1.9, 99) === 98 && Number.isNaN('abc'.charCodeAt(Infinity));"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("'\\uD800\\uDC00'.codePointAt(0);"),
        Ok(Value::Number(65_536.0))
    );
    assert_eq!(
        eval("'\\uD800\\uE000'.codePointAt(0);"),
        Ok(Value::Number(55_296.0))
    );
    assert_eq!(
        eval("'\\uD800'.charCodeAt(0);"),
        Ok(Value::Number(55_296.0))
    );
    assert_eq!(
        eval(
            "let object = new Object(42); object.charAt = String.prototype.charAt; object.charAt(false) + object.charAt(true);"
        ),
        Ok(Value::String("42".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = new Object(42); object.charCodeAt = String.prototype.charCodeAt; object.charCodeAt(0) + object.charCodeAt(1);"
        ),
        Ok(Value::Number(102.0))
    );
    assert_eq!(
        eval(
            "let object = { valueOf: 1, toString: function() { throw 'marker'; }, charAt: String.prototype.charAt }; let caught = false; try { object.charAt(); } catch (error) { caught = error === 'marker'; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("String.prototype.codePointAt.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("'abc'.codePointAt(1);"), Ok(Value::Number(98.0)));
    assert_eq!(eval("'abc'.codePointAt(-1);"), Ok(Value::Undefined));
    assert_eq!(eval("'abc'.codePointAt(3);"), Ok(Value::Undefined));
    assert_eq!(eval("'😀'.codePointAt(0);"), Ok(Value::Number(128_512.0)));
    assert_eq!(eval("'😀'.codePointAt(1);"), Ok(Value::Number(56_832.0)));
    assert_eq!(
        eval("let seen = ''; for (var ch of 'ab') { seen += ch; } seen;"),
        Ok(Value::String("ab".to_owned().into()))
    );
    assert_eq!(
        eval("let seen = ''; for (var ch of 'a\\uD801\\uDC28b') { seen += ch + '|'; } seen;"),
        Ok(Value::String("a|𐐨|b|".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let seen = ''; for (var ch of 'a\\uD801b') { seen += ch.length + ':' + ch.charCodeAt(0) + '|'; } seen;"
        ),
        Ok(Value::String("1:97|1:55297|1:98|".to_owned().into()))
    );
    assert_eq!(
        eval("let iterator = 'x'[Symbol.iterator](); iterator[Symbol.iterator]() === iterator;"),
        Ok(Value::Boolean(true))
    );
    assert!(eval("new String.prototype.charAt();").is_err());
}

#[test]
fn equality_and_order_follow_code_units_across_storage_forms() {
    // A surrogate pair stored whole and the same pair built from two lone
    // halves are equal; order is by UTF-16 code unit, not by code point.
    assert_eq!(
        eval(
            "var joined = '\\uD83D' + '\\uDE00', whole = '\\uD83D\\uDE00';
             [joined === whole, joined == whole, joined !== whole,
              whole < '\\uFFFF', '\\uFFFF' > whole, 'abc' < 'abd', 'ab' < 'abc',
              'é' > 'z', whole === '\\uD83D'].join();"
        ),
        Ok(Value::String(
            "true,true,false,true,true,true,true,true,false"
                .to_owned()
                .into()
        ))
    );
}

#[test]
fn typeof_names_are_ordinary_strings() {
    assert_eq!(
        eval(
            "var names = [typeof 1, typeof 'a', typeof true, typeof undefined, typeof null,
                          typeof {}, typeof function () {}, typeof Symbol(), typeof 1n];
             names.join() + '|' + (typeof 1 === 'number') + (names[0] + 's');"
        ),
        Ok(Value::String(
            "number,string,boolean,undefined,object,object,function,symbol,bigint|truenumbers"
                .to_owned()
                .into()
        ))
    );
}
