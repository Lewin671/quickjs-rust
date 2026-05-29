use crate::{Value, eval};

#[test]
fn evaluates_string_member_access() {
    assert_eq!(eval("'abc'.length;"), Ok(Value::Number(3.0)));
    assert_eq!(eval("''.length;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("'abc'[0];"), Ok(Value::String("a".to_owned())));
    assert_eq!(eval("'abc'['1'];"), Ok(Value::String("b".to_owned())));
    assert_eq!(eval("'abc'[3];"), Ok(Value::Undefined));
    assert_eq!(eval("'abc'['01'];"), Ok(Value::Undefined));
}

#[test]
fn evaluates_string_builtins() {
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
        eval("String.fromCodePoint(65, 128512, 67);"),
        Ok(Value::String("A😀C".to_owned()))
    );
    assert_eq!(
        eval("String.fromCodePoint();"),
        Ok(Value::String(String::new()))
    );
    assert_eq!(eval("String.fromCodePoint.length;"), Ok(Value::Number(1.0)));
    assert!(eval("String.fromCodePoint(-1);").is_err());
    assert!(eval("String.fromCodePoint(1.5);").is_err());
    assert!(eval("String.fromCodePoint(1114112);").is_err());
    assert_eq!(
        eval("String.prototype.constructor === String;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("String.prototype.at.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'abc'.at(1);"), Ok(Value::String("b".to_owned())));
    assert_eq!(eval("'abc'.at(-1);"), Ok(Value::String("c".to_owned())));
    assert_eq!(eval("'abc'.at(3);"), Ok(Value::Undefined));
    assert_eq!(eval("'abc'.at(-4);"), Ok(Value::Undefined));
    assert_eq!(eval("'abc'.at();"), Ok(Value::String("a".to_owned())));
    assert_eq!(eval("'abc'.at(1.9);"), Ok(Value::String("b".to_owned())));
    assert_eq!(
        eval("String.prototype.charAt.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("'abc'.charAt(1);"), Ok(Value::String("b".to_owned())));
    assert_eq!(eval("'abc'.charAt(9);"), Ok(Value::String(String::new())));
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
        eval("String.prototype.codePointAt.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("'abc'.codePointAt(1);"), Ok(Value::Number(98.0)));
    assert_eq!(eval("'abc'.codePointAt(-1);"), Ok(Value::Undefined));
    assert_eq!(eval("'abc'.codePointAt(3);"), Ok(Value::Undefined));
    assert_eq!(eval("'😀'.codePointAt(0);"), Ok(Value::Number(128_512.0)));
    assert_eq!(eval("'😀'.codePointAt(1);"), Ok(Value::Number(56_832.0)));
    assert_eq!(
        eval("'a'.concat('b', 3, true);"),
        Ok(Value::String("ab3true".to_owned()))
    );
    assert_eq!(eval("'abc'.startsWith('ab');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'abc'.startsWith('bc', 1);"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("'abc'.startsWith('bc', 2);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("'abc'.endsWith('bc');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'abc'.endsWith('ab', 2);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'abc'.endsWith('bc', 2);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("'abc'.indexOf('b');"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'abc'.indexOf('b', 2);"), Ok(Value::Number(-1.0)));
    assert_eq!(eval("'abc'.includes('b');"), Ok(Value::Boolean(true)));
    assert_eq!(eval("'abc'.includes('b', 2);"), Ok(Value::Boolean(false)));
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
        Ok(Value::String("defdabc".to_owned()))
    );
    assert_eq!(
        eval("'abc'.padEnd(7, 'def');"),
        Ok(Value::String("abcdefd".to_owned()))
    );
    assert_eq!(
        eval("'abc'.padStart(5);"),
        Ok(Value::String("  abc".to_owned()))
    );
    assert_eq!(
        eval("'abc'.padEnd(5);"),
        Ok(Value::String("abc  ".to_owned()))
    );
    assert_eq!(
        eval("'abc'.padStart(5, '');"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval("'abc'.padEnd(2, '*');"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval("'ab'.repeat(3);"),
        Ok(Value::String("ababab".to_owned()))
    );
    assert_eq!(eval("'ab'.repeat(0);"), Ok(Value::String(String::new())));
    assert_eq!(
        eval("'ab'.repeat(2.8);"),
        Ok(Value::String("abab".to_owned()))
    );
    assert!(eval("'ab'.repeat(-1);").is_err());
    assert!(eval("'ab'.repeat(Infinity);").is_err());
    assert_eq!(
        eval("'abcdef'.slice(1, 4);"),
        Ok(Value::String("bcd".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.slice(-3);"),
        Ok(Value::String("def".to_owned()))
    );
    assert_eq!(
        eval("String.prototype.split.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("'hello'.split('l').join('|');"),
        Ok(Value::String("he||o".to_owned()))
    );
    assert_eq!(
        eval("'hello'.split('l', 2).join('|');"),
        Ok(Value::String("he|".to_owned()))
    );
    assert_eq!(
        eval("'hello'.split(undefined).join('|');"),
        Ok(Value::String("hello".to_owned()))
    );
    assert_eq!(
        eval("'abc'.split('', 2).join('|');"),
        Ok(Value::String("a|b".to_owned()))
    );
    assert_eq!(eval("'abc'.split('x').length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'abc'.split('b', 0).length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("String.prototype.lastIndexOf.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("'canal'.lastIndexOf('a');"), Ok(Value::Number(3.0)));
    assert_eq!(eval("'canal'.lastIndexOf('a', 2);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'canal'.lastIndexOf('x');"), Ok(Value::Number(-1.0)));
    assert_eq!(eval("'abc'.lastIndexOf('', 1);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("'abc'.lastIndexOf('', 99);"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("String.prototype.substring.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("'abcdef'.substring(1, 4);"),
        Ok(Value::String("bcd".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substring(4, 1);"),
        Ok(Value::String("bcd".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substring(-3, 2);"),
        Ok(Value::String("ab".to_owned()))
    );
    assert_eq!(
        eval("'abcdef'.substring(3);"),
        Ok(Value::String("def".to_owned()))
    );
    assert_eq!(
        eval("'abc'.toString();"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval("'abc'.valueOf();"),
        Ok(Value::String("abc".to_owned()))
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
    assert!(eval("new String.prototype.charAt();").is_err());
}

#[test]
fn evaluates_string_objects() {
    assert_eq!(
        eval("typeof new String('abc');"),
        Ok(Value::String("object".to_owned()))
    );
    assert_eq!(
        eval("let s = new String('abc'); s.constructor === String;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let s = new String('abc'); s.valueOf();"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval("let s = new String('abc'); s.toString();"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(eval("new String('abc').length;"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("let s = new String('abc'); s[1];"),
        Ok(Value::String("b".to_owned()))
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
        Ok(Value::String("[object String]".to_owned()))
    );
    assert_eq!(
        eval("new String('abc').charAt(2);"),
        Ok(Value::String("c".to_owned()))
    );
}
