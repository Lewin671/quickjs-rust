use super::{Value, eval};

#[test]
fn evaluates_arithmetic() {
    assert_eq!(eval("1 + 2 * 3;"), Ok(Value::Number(7.0)));
    assert_eq!(eval("true + true;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("true * 2;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("2 ** 3;"), Ok(Value::Number(8.0)));
    assert_eq!(eval("2 ** 3 ** 2;"), Ok(Value::Number(512.0)));
    assert_eq!(eval("3 * 2 ** 3;"), Ok(Value::Number(24.0)));
    assert_eq!(eval("2 ** -1 * 2;"), Ok(Value::Number(1.0)));
}

#[test]
fn evaluates_bitwise_and_shift_expressions() {
    assert_eq!(eval("5 & 3;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("5 | 2;"), Ok(Value::Number(7.0)));
    assert_eq!(eval("5 ^ 3;"), Ok(Value::Number(6.0)));
    assert_eq!(eval("2 << 3;"), Ok(Value::Number(16.0)));
    assert_eq!(eval("-8 >> 1;"), Ok(Value::Number(-4.0)));
    assert_eq!(eval("-1 >>> 0;"), Ok(Value::Number(4_294_967_295.0)));
    assert_eq!(eval("~false;"), Ok(Value::Number(-1.0)));
    assert_eq!(eval("1 + 2 << 3;"), Ok(Value::Number(24.0)));
}

#[test]
fn evaluates_string_addition() {
    assert_eq!(eval("'x' + 1;"), Ok(Value::String("x1".to_owned())));
    assert_eq!(eval("1 + 'x';"), Ok(Value::String("1x".to_owned())));
    assert_eq!(eval("'x' + true;"), Ok(Value::String("xtrue".to_owned())));
    assert_eq!(eval("'x' + null;"), Ok(Value::String("xnull".to_owned())));
    assert_eq!(
        eval("'x' + undefined;"),
        Ok(Value::String("xundefined".to_owned()))
    );
}

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
fn evaluates_boolean_builtins() {
    assert_eq!(
        eval("typeof Boolean;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Boolean.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Boolean();"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Boolean(0);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Boolean(1);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Boolean('');"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Boolean('x');"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Boolean.prototype.constructor === Boolean;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Boolean.prototype.toString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Boolean.prototype.valueOf.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Boolean.prototype.toString();"),
        Ok(Value::String("false".to_owned()))
    );
    assert_eq!(
        eval("Boolean.prototype.valueOf();"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("true.toString();"),
        Ok(Value::String("true".to_owned()))
    );
    assert_eq!(eval("false.valueOf();"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("(new Boolean(true)).toString();"),
        Ok(Value::String("true".to_owned()))
    );
    assert_eq!(
        eval("(new Boolean(0)).valueOf();"),
        Ok(Value::Boolean(false))
    );
    assert!(eval("let o = Object.create(Boolean.prototype); o.valueOf();").is_err());
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
        eval("String.prototype.constructor === String;"),
        Ok(Value::Boolean(true))
    );
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
        eval("'  abc  '.trimEnd();"),
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
        eval("Object.getOwnPropertyDescriptor(String.prototype, 'charAt').enumerable;"),
        Ok(Value::Boolean(false))
    );
    assert!(eval("new String.prototype.charAt();").is_err());
}

#[test]
fn evaluates_comparison_and_equality() {
    assert_eq!(eval("1 + 2 * 3 >= 7;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("1 + 1 === 2;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("1 !== 2;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("function C() {} let instance = new C(); instance instanceof C;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function C() {} function D() {} let instance = new C(); instance instanceof D;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("function C() {} 1 instanceof C;"),
        Ok(Value::Boolean(false))
    );
    assert!(eval("let object = {}; object instanceof {};").is_err());
    assert!(
        eval("function C() {} C.prototype = 1; let object = {}; object instanceof C;").is_err()
    );
}

#[test]
fn evaluates_object_builtins() {
    assert_eq!(
        eval("typeof Object;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Object.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Object.assign.length;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval(
            "let target = { foo: 1 }; let result = Object.assign(target, { a: 2 }); result === target;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let target = { foo: 1 }; Object.assign(target, { a: 2 }); target.a;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let target = { a: 1 }; Object.assign(target, { a: 5 }, { b: 6 }); target.a + target.b;"
        ),
        Ok(Value::Number(11.0))
    );
    assert_eq!(
        eval("let target = {}; Object.assign(target, 'ab', null, undefined); target[1];"),
        Ok(Value::String("b".to_owned()))
    );
    assert_eq!(
        eval(
            "let target = {}; Object.assign(target, Object.create({ inherited: 1 })); Object.keys(target).length;"
        ),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Object.defineProperty.length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'value', { value: 7 }); object.value;"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'value', { value: 7 }); Object.keys(object).length;"
        ),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'value', { value: 7, enumerable: true, writable: true, configurable: true }); Object.keys(object)[0];"
        ),
        Ok(Value::String("value".to_owned()))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'value', { value: 7 }); object.value = 9; object.value;"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'value', { value: 7, writable: true }); object.value = 9; object.value;"
        ),
        Ok(Value::Number(9.0))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'value', { value: 7, configurable: true }); Object.getOwnPropertyDescriptor(object, 'value').configurable;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.defineProperties.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperties(object, { first: { value: 1, enumerable: true }, second: { value: 2 } }); object.first + object.second;"
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperties(object, { first: { value: 1, enumerable: true }, second: { value: 2 } }); Object.keys(object).length;"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "function fn() {} Object.defineProperties(fn, { value: { value: 9, enumerable: true } }); fn.value;"
        ),
        Ok(Value::Number(9.0))
    );
    assert_eq!(eval("Object.create.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("let proto = { value: 7 }; let object = Object.create(proto); object.value;"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let proto = { inherited: 1 }; let object = Object.create(proto, { own: { value: 2, enumerable: true } }); object.inherited + object.own;"
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "let object = Object.create(null, { own: { value: 2, enumerable: true } }); Object.keys(object)[0];"
        ),
        Ok(Value::String("own".to_owned()))
    );
    assert_eq!(
        eval("Object.create({}, undefined) instanceof Object;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = Object.create({}, { hidden: { value: 4 } }); Object.keys(object).length;"
        ),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let proto = {}; let object = Object.create(proto); Object.getPrototypeOf(object) === proto;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getPrototypeOf(Object.create(null));"),
        Ok(Value::Null)
    );
    assert_eq!(eval("({}) instanceof Object;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Object() instanceof Object;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("(new Object()).constructor === Object;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let object = { value: 3 }; Object(object) === object;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let object = { value: 3 }; new Object(object) === object;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("({ value: 1 }).hasOwnProperty('value');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.toString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Object.prototype.toString();"),
        Ok(Value::String("[object Object]".to_owned()))
    );
    assert_eq!(
        eval("({}).toString();"),
        Ok(Value::String("[object Object]".to_owned()))
    );
    assert_eq!(
        eval("Object.prototype.valueOf.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("let object = { value: 1 }; object.valueOf() === object;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.valueOf() === Object.prototype;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Object.keys.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Object.keys({ value: 1 })[0];"),
        Ok(Value::String("value".to_owned()))
    );
    assert_eq!(eval("Object.keys([1, 2]).length;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("Object.keys(Object.create({ value: 1 })).length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(eval("Object.keys(Object).length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Object.keys(Object.prototype).length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames({ value: 1 })[0];"),
        Ok(Value::String("value".to_owned()))
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames([1, 2]).length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames(Object.prototype).length;"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames(Object.prototype)[0];"),
        Ok(Value::String("constructor".to_owned()))
    );
    assert_eq!(eval("Object.hasOwn.length;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("Object.hasOwn({ value: 1 }, 'value');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.hasOwn({ value: 1 }, 'missing');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let proto = { value: 1 }; let object = Object.create(proto); Object.hasOwn(object, 'value');"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let object = Object.create(null, { value: { value: 1 } }); Object.hasOwn(object, 'value');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.hasOwn([1, 2], '1');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Object.hasOwn('ab', '1');"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("let object = { value: 1 }; Object.getOwnPropertyDescriptor(object, 'value').value;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor({ value: 1 }, 'value').enumerable;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(Object.prototype, 'toString').enumerable;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor([1, 2], 'length').value;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor({}, 'missing');"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("Object.keys('ab')[1];"),
        Ok(Value::String("1".to_owned()))
    );
    assert_eq!(eval("Object.keys(0).length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("({ value: 1 }).hasOwnProperty('missing');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let proto = { value: 1 }; let object = Object.create(proto); object.hasOwnProperty('value');"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("[1, 2].hasOwnProperty('1');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("'ab'.hasOwnProperty('1');"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Object.prototype.propertyIsEnumerable.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("({ value: 1 }).propertyIsEnumerable('value');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.propertyIsEnumerable('toString');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Object.prototype.propertyIsEnumerable('propertyIsEnumerable');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let proto = { value: 1 }; let object = Object.create(proto); object.propertyIsEnumerable('value');"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("[1, 2].propertyIsEnumerable('length');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("'ab'.propertyIsEnumerable('1');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.isPrototypeOf.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let proto = { value: 1 }; let object = Object.create(proto); proto.isPrototypeOf(object);"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let proto = { value: 1 }; let object = Object.create(proto); Object.prototype.isPrototypeOf(object);"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.isPrototypeOf({});"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.prototype.isPrototypeOf([1, 2]);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function F() {} Object.prototype.isPrototypeOf(F);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function F() {} F.prototype.isPrototypeOf(F);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Object.prototype.isPrototypeOf(1);"),
        Ok(Value::Boolean(false))
    );
    assert!(eval("Object.create(1);").is_err());
    assert!(eval("new Object.create({});").is_err());
    assert!(eval("new Object.prototype.hasOwnProperty('value');").is_err());
}

#[test]
fn evaluates_array_builtins() {
    assert_eq!(
        eval("typeof Array;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Array.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Array.isArray.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Array.prototype.at.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Array.prototype.concat.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("Array.prototype.fill.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Array.prototype.includes.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.indexOf.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.lastIndexOf.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("Array.prototype.join.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Array.prototype.slice.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(eval("Array.prototype.pop.length;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Array.prototype.push.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Array.prototype.shift.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Array.prototype.reverse.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Array.prototype.unshift.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Array.prototype.toString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(eval("Array().length;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Array(1, 2)[1];"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("let array = new Array('x'); array[0];"),
        Ok(Value::String("x".to_owned()))
    );
    assert_eq!(eval("Array.isArray([]);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Array.isArray({});"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Array.isArray('abc');"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("Array.prototype.constructor === Array;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("[] instanceof Array;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("Array.prototype.isPrototypeOf([]);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getPrototypeOf([]) === Array.prototype;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("[1, 'x', true].join();"),
        Ok(Value::String("1,x,true".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].join('|');"),
        Ok(Value::String("1|2|3".to_owned()))
    );
    assert_eq!(
        eval("[1, null, undefined, 4].join('-');"),
        Ok(Value::String("1---4".to_owned()))
    );
    assert_eq!(
        eval("[1, 'x', true].toString();"),
        Ok(Value::String("1,x,true".to_owned()))
    );
    assert_eq!(
        eval("[1, [2, 3], 4].join(';');"),
        Ok(Value::String("1;2,3;4".to_owned()))
    );
    assert_eq!(eval("[1, 2, 1].indexOf(1);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("[1, 2, 1].indexOf(1, 1);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("[1, 2, 1].indexOf(1, -1);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("[1, 2, 1].indexOf(1, -5);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("[1, 2, 3].indexOf(4);"), Ok(Value::Number(-1.0)));
    assert_eq!(
        eval("[false, 'false'].indexOf(false);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("[false, 'false'].indexOf('false');"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("[1, 2, 1].lastIndexOf(1);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("[1, 2, 1].lastIndexOf(1, 1);"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("[1, 2, 1].lastIndexOf(1, -2);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("[1, 2, 1].lastIndexOf(1, -5);"),
        Ok(Value::Number(-1.0))
    );
    assert_eq!(eval("[1, 2, 3].lastIndexOf(4);"), Ok(Value::Number(-1.0)));
    assert_eq!(
        eval("[false, 'false'].lastIndexOf(false);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("[0, 1, 2, 3, 4].slice(1, 4).join();"),
        Ok(Value::String("1,2,3".to_owned()))
    );
    assert_eq!(
        eval("[0, 1, 2, 3, 4].slice(2).join('|');"),
        Ok(Value::String("2|3|4".to_owned()))
    );
    assert_eq!(
        eval("[0, 1, 2, 3, 4].slice(-3, -1).join();"),
        Ok(Value::String("2,3".to_owned()))
    );
    assert_eq!(eval("[0, 1, 2].slice(5).length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("let copy = [1, 2].slice(); Array.isArray(copy) && copy[1] === 2;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("[0].concat([1, 2], 3, [4]).join();"),
        Ok(Value::String("0,1,2,3,4".to_owned()))
    );
    assert_eq!(
        eval("[].concat([0, 1], [2, 3]).length;"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(eval("[0].concat('x', true)[2];"), Ok(Value::Boolean(true)));
    assert_eq!(eval("[1, 2, 3].at(0);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("[1, 2, 3].at(2);"), Ok(Value::Number(3.0)));
    assert_eq!(eval("[1, 2, 3].at(-1);"), Ok(Value::Number(3.0)));
    assert_eq!(eval("[1, 2, 3].at(-3);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("[1, 2, 3].at(3);"), Ok(Value::Undefined));
    assert_eq!(eval("[1, 2, 3].at(-4);"), Ok(Value::Undefined));
    assert_eq!(eval("[1, 2, 3].at();"), Ok(Value::Number(1.0)));
    assert_eq!(eval("[1, 2, 3].at(1.9);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("[1, 2, 3].at(-1.9);"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("let xs = [1]; xs.push(2, 3) + ':' + xs.length + ':' + xs.join();"),
        Ok(Value::String("3:3:1,2,3".to_owned()))
    );
    assert_eq!(
        eval("let xs = [1, 2]; xs.pop() + ':' + xs.length + ':' + xs.join();"),
        Ok(Value::String("2:1:1".to_owned()))
    );
    assert_eq!(eval("[].pop();"), Ok(Value::Undefined));
    assert_eq!(
        eval("let xs = [1, 2]; xs.shift() + ':' + xs.length + ':' + xs.join();"),
        Ok(Value::String("1:1:2".to_owned()))
    );
    assert_eq!(eval("[].shift();"), Ok(Value::Undefined));
    assert_eq!(
        eval("let xs = [3]; xs.unshift(1, 2) + ':' + xs.join();"),
        Ok(Value::String("3:1,2,3".to_owned()))
    );
    assert_eq!(
        eval("let xs = [1, 2, 3]; let result = xs.reverse(); result === xs && xs.join();"),
        Ok(Value::String("3,2,1".to_owned()))
    );
    assert_eq!(
        eval("let xs = [1, 2, 3]; let result = xs.fill(9); result === xs && xs.join();"),
        Ok(Value::String("9,9,9".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3, 4].fill(0, 1, 3).join();"),
        Ok(Value::String("1,0,0,4".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3, 4].fill(0, -3, -1).join();"),
        Ok(Value::String("1,0,0,4".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].fill(0, 5).join();"),
        Ok(Value::String("1,2,3".to_owned()))
    );
    assert_eq!(
        eval("[1, 2, 3].fill().join();"),
        Ok(Value::String(",,".to_owned()))
    );
    assert_eq!(
        eval("let xs = [1, undefined, 3]; xs.reverse(); xs.length + ':' + xs.join();"),
        Ok(Value::String("3:3,,1".to_owned()))
    );
    assert_eq!(
        eval("let xs = [2]; let ys = xs; ys.unshift(1); xs.shift() + ':' + xs.join();"),
        Ok(Value::String("1:2".to_owned()))
    );
    assert_eq!(
        eval("let xs = [1]; let ys = xs; ys.push(2); xs.join();"),
        Ok(Value::String("1,2".to_owned()))
    );
    assert_eq!(
        eval("let xs = [1]; xs[2] = 3; xs.length + ':' + xs.join();"),
        Ok(Value::String("3:1,,3".to_owned()))
    );
    assert_eq!(
        eval("let xs = [1, 2, 3]; xs.length = 1; xs.join();"),
        Ok(Value::String("1".to_owned()))
    );
    assert_eq!(eval("[1, 2, 3].includes(2);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("[1, 2, 3].includes(4);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("[1, 2, 3].includes(1, 1);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("[1, 2, 3].includes(3, -1);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("[0 / 0].includes(0 / 0);"), Ok(Value::Boolean(true)));
    assert!(eval("Array(3);").is_err());
}

#[test]
fn evaluates_logical_expressions() {
    assert_eq!(eval("0 || 5;"), Ok(Value::Number(5.0)));
    assert_eq!(eval("1 && 7;"), Ok(Value::Number(7.0)));
}

#[test]
fn evaluates_nullish_coalescing_expressions() {
    assert_eq!(eval("null ?? 42;"), Ok(Value::Number(42.0)));
    assert_eq!(eval("undefined ?? 42;"), Ok(Value::Number(42.0)));
    assert_eq!(eval("0 ?? 42;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("false ?? 42;"), Ok(Value::Boolean(false)));
    assert_eq!(eval("42 ?? missing;"), Ok(Value::Number(42.0)));
    assert_eq!(eval("null ?? 0 ?? 1;"), Ok(Value::Number(0.0)));
}

#[test]
fn evaluates_conditional_expressions() {
    assert_eq!(eval("true ? 1 : 2;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("false ? 1 : 2;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("let x = true ? 'yes' : 'no'; x;"),
        Ok(Value::String("yes".to_owned()))
    );
    assert_eq!(eval("true ? 1 : missing;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("false ? missing : 2;"), Ok(Value::Number(2.0)));
}

#[test]
fn evaluates_sequence_expressions() {
    assert_eq!(eval("1, 2;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval("let x = 0; x = 1, x = x + 2, x;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("let x = 0; while ((x = x + 1, x < 3)) { } x;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_variable_declarations() {
    assert_eq!(
        eval("let x = 2; const y = 3; x * y;"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(eval("var missing; missing;"), Ok(Value::Undefined));
    assert_eq!(eval("x; var x;"), Ok(Value::Undefined));
    assert_eq!(eval("x; var x = 1; x;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("if (false) { var x = 1; } x;"), Ok(Value::Undefined));
    assert_eq!(
        eval("function f() { return x; var x = 2; } f();"),
        Ok(Value::Undefined)
    );
    assert!(eval("x; let x;").is_err());
    assert_eq!(
        eval("var x = 1, y = 2, missing; x + y;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_assignment_expressions() {
    assert_eq!(eval("let x = 2; x = x + 3; x;"), Ok(Value::Number(5.0)));
}

#[test]
fn evaluates_update_and_compound_assignment() {
    assert_eq!(eval("let x = 1; x++; x;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("let x = 1; ++x;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("let x = 1; x++;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("let x = false; x++;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("let x = 3; x--; x;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("let x = 1; x += 2; x;"), Ok(Value::Number(3.0)));
    assert_eq!(eval("let x = -3; x **= 3; x;"), Ok(Value::Number(-27.0)));
    assert_eq!(eval("let x = 2; x <<= 3; x;"), Ok(Value::Number(16.0)));
    assert_eq!(eval("let x = -8; x >>= 1; x;"), Ok(Value::Number(-4.0)));
    assert_eq!(
        eval("let x = -1; x >>>= 0; x;"),
        Ok(Value::Number(4_294_967_295.0))
    );
    assert_eq!(eval("let x = 5; x &= 3; x;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("let x = 5; x ^= 3; x;"), Ok(Value::Number(6.0)));
    assert_eq!(eval("let x = 5; x |= 2; x;"), Ok(Value::Number(7.0)));
    assert_eq!(
        eval("let x = 'a'; x += 1; x;"),
        Ok(Value::String("a1".to_owned()))
    );
    assert_eq!(
        eval("let o = { count: 1 }; o.count++; o.count;"),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn evaluates_logical_assignment() {
    assert_eq!(eval("let x = 0; x &&= missing; x;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("let x = 2; x &&= 7; x;"), Ok(Value::Number(7.0)));
    assert_eq!(eval("let x = 0; x ||= 7; x;"), Ok(Value::Number(7.0)));
    assert_eq!(eval("let x = 2; x ||= missing; x;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("let x = null; x ??= 7; x;"), Ok(Value::Number(7.0)));
    assert_eq!(
        eval("let x = undefined; x ??= 8; x;"),
        Ok(Value::Number(8.0))
    );
    assert_eq!(
        eval("let x = false; x ??= missing; x;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let o = { value: 0 }; o.value ||= 3; o.value;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_if_else_statements() {
    assert_eq!(
        eval("let x = 1; if (x > 0) { x = 7; } else { x = 3; } x;"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("let x = 1; if (x < 0) x = 7; else x = 3; x;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_while_statements() {
    assert_eq!(
        eval("let x = 0; while (x < 3) { x = x + 1; } x;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_do_while_statements() {
    assert_eq!(
        eval("let x = 0; do { x = x + 1; } while (false); x;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("let x = 0; do { x++; } while (x < 3); x;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("let x = 0; do { x++; if (x === 2) continue; } while (x < 3); x;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_for_statements() {
    assert_eq!(
        eval("let sum = 0; for (var i = 0; i < 4; i = i + 1) { sum = sum + i; } sum;"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("let i = 0; for (; i < 3; ) i = i + 1; i;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_for_in_statements() {
    assert_eq!(
        eval("let count = 0; for (var key in { a: 1, b: 2 }) { count++; } count;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let total = 0; let item; let values = [1, 2, 3]; for (item in values) { total += values[item]; } total;"
        ),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("let count = 0; for (var key in null) { count++; } count;"),
        Ok(Value::Number(0.0))
    );
}

#[test]
fn evaluates_break_and_continue() {
    assert_eq!(
        eval("let i = 0; while (true) { i = i + 1; if (i === 3) break; } i;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "let sum = 0; for (var i = 0; i < 5; i = i + 1) { if (i === 2) continue; sum = sum + i; } sum;"
        ),
        Ok(Value::Number(8.0))
    );
}

#[test]
fn evaluates_switch_statements() {
    assert_eq!(
        eval(
            "let x = 2; let out = 0; switch (x) { case 1: out = 1; break; case 2: out = 2; break; default: out = 3; } out;"
        ),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let x = 4; let out = 0; switch (x) { case 1: out = 1; break; default: out = 3; } out;"
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "let x = 1; let out = 0; switch (x) { case 1: out += 1; case 2: out += 2; default: out += 4; } out;"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let x = '1'; let out = 0; switch (x) { case 1: out = 1; break; default: out = 2; } out;"
        ),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn evaluates_throw_statement_only_when_reached() {
    assert_eq!(eval("if (false) { throw; } 1;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("if (false) { throw 'no'; } 1;"),
        Ok(Value::Number(1.0))
    );
    let error = eval("throw;").expect_err("throw should fail evaluation");
    assert_eq!(error.message, "throw statement executed: undefined");
    let error = eval("throw 'expected';").expect_err("throw should fail evaluation");
    assert_eq!(error.message, "throw statement executed: expected");
    let error = eval("throw 42;").expect_err("throw should fail evaluation");
    assert_eq!(error.message, "throw statement executed: 42");
}

#[test]
fn evaluates_try_catch_finally_statements() {
    assert_eq!(
        eval("try { throw 'caught'; } catch (error) { error; }"),
        Ok(Value::String("caught".to_owned()))
    );
    assert_eq!(
        eval("let x = 1; try { throw 2; } catch (error) { x = error; } x;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("let x = 1; try { x += 1; } finally { x += 2; } x;"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("let x = 1; try { throw 1; } catch (error) { x += error; } finally { x += 2; } x;"),
        Ok(Value::Number(4.0))
    );
    let error =
        eval("try { throw 'try'; } finally { throw 'finally'; }").expect_err("throw should fail");
    assert_eq!(error.message, "throw statement executed: finally");
    assert_eq!(
        eval("let error = 'outer'; try { throw 'inner'; } catch (error) { error; } error;"),
        Ok(Value::String("outer".to_owned()))
    );
}

#[test]
fn evaluates_debugger_statement_as_noop() {
    assert_eq!(eval("debugger; 1;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("let x = 0; if (true) debugger; x = 2; x;"),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn evaluates_unary_expressions() {
    assert_eq!(eval("-1 + 3;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("!0;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("+true;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("void 0;"), Ok(Value::Undefined));
    assert_eq!(eval("let x = 0; void (x = 1); x;"), Ok(Value::Number(1.0)));
}

#[test]
fn evaluates_typeof_expressions() {
    assert_eq!(
        eval("typeof undefined;"),
        Ok(Value::String("undefined".to_owned()))
    );
    assert_eq!(
        eval("typeof neverDeclared;"),
        Ok(Value::String("undefined".to_owned()))
    );
    assert_eq!(
        eval("typeof true;"),
        Ok(Value::String("boolean".to_owned()))
    );
    assert_eq!(eval("typeof 1;"), Ok(Value::String("number".to_owned())));
    assert_eq!(eval("typeof 'x';"), Ok(Value::String("string".to_owned())));
    assert_eq!(eval("typeof null;"), Ok(Value::String("object".to_owned())));
    assert_eq!(eval("typeof {};"), Ok(Value::String("object".to_owned())));
    assert_eq!(eval("typeof this;"), Ok(Value::String("object".to_owned())));
    assert_eq!(
        eval("function f() { return 1; } typeof f;"),
        Ok(Value::String("function".to_owned()))
    );
}

#[test]
fn evaluates_delete_operator() {
    assert_eq!(eval("let o = {}; delete o.x;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("let o = { red: 1 }; delete o.red; o.red;"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("let o = { 2: 2 }; delete o[2]; o['2'];"),
        Ok(Value::Undefined)
    );
}

#[test]
fn evaluates_in_operator() {
    assert_eq!(
        eval("'answer' in { answer: 42 };"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("'missing' in { answer: 42 };"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let o = {}; o.present = undefined; 'present' in o;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("'length' in [1, 2];"), Ok(Value::Boolean(true)));
}

#[test]
fn evaluates_function_declarations_and_calls() {
    assert_eq!(
        eval("function add(a, b) { return a + b; } add(2, 3);"),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval(
            "let result = callBeforeDeclaration(); function callBeforeDeclaration() { return 11; } result;"
        ),
        Ok(Value::Number(11.0))
    );
    assert_eq!(
        eval("function outer() { return inner(); function inner() { return 13; } } outer();"),
        Ok(Value::Number(13.0))
    );
    assert_eq!(
        eval("let result; { result = inside(); function inside() { return 17; } } result;"),
        Ok(Value::Number(17.0))
    );
    assert_eq!(
        eval("function first(a) { return a; } first();"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("function first(a) { return a; } first(1, 2);"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("function arg(index) { return arguments[index]; } arg(1, 2, 3);"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("function count() { return arguments.length; } count(1, 2, 3);"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("function none() { return arguments.length; } none();"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("function pair(a, b) { return b; } pair(1);"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("function pair(a, b) { return arguments[2]; } pair(1, 2, 3);"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("function pair(a, b) {} pair.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "function make(value) { return function() { return value; }; } let get = make(7); get();"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("let value = 1; function read() { return value; } value = 2; read();"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("let add = function(a, b) { return a + b; }; add(2, 3);"),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval("let f = function named() { return typeof named; }; f();"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(
        eval("let f = function named() { return named === f; }; f();"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let f = function hidden() { return 1; }; typeof hidden;"),
        Ok(Value::String("undefined".to_owned()))
    );
    assert_eq!(
        eval(
            "let factorial = function fact(n) { return n <= 1 ? 1 : n * fact(n - 1); }; factorial(5);"
        ),
        Ok(Value::Number(120.0))
    );
    assert_eq!(
        eval("(function(value) { return value + 1; })(2);"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("function getThis() { return this; } getThis() === this;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function getThis() { return this; } let o = {}; o.getThis = getThis; o.getThis() === o;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function getGlobal() { return this; } function method() { return getGlobal(); } let o = {}; o.method = method; o.method() === this;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let o = { method: function() { return this.value; }, value: 7 }; o.method();"),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn evaluates_new_expressions() {
    assert_eq!(
        eval(
            "function Point(x, y) { this.x = x; this.y = y; } let p = new Point(2, 3); p.x + p.y;"
        ),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval("function Empty() { this.value = 9; } let p = new Empty; p.value;"),
        Ok(Value::Number(9.0))
    );
    assert_eq!(
        eval(
            "function Box() { this.value = 1; return { value: 4 }; } let box = new Box(); box.value;"
        ),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("function Box() { this.value = 6; return 1; } let box = new Box(); box.value;"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval(
            "function Args() { this.count = arguments.length; } let args = new Args(1, 2, 3); args.count;"
        ),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("function C() {} C.prototype.value = 4; let instance = new C(); instance.value;"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval(
            "function C() { this.value = 9; } C.prototype.value = 4; let instance = new C(); instance.value;"
        ),
        Ok(Value::Number(9.0))
    );
    assert_eq!(
        eval("function C() {} C.prototype = { value: 8 }; let instance = new C(); instance.value;"),
        Ok(Value::Number(8.0))
    );
    assert_eq!(
        eval("function C() {} C.prototype.value = 4; let instance = new C(); 'value' in instance;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function C() {} C.prototype.constructor === C;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function C() {} let instance = new C(); instance.constructor === C;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let C = function Named() {}; C.prototype.constructor === C;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function C() {} C.prototype = { value: 1 }; let instance = new C(); instance.constructor === Object;"
        ),
        Ok(Value::Boolean(true))
    );
    assert!(eval("new 1;").is_err());
}

#[test]
fn evaluates_array_literals() {
    assert_eq!(
        eval("let xs = [1, 2 + 3, true]; xs.length + ':' + xs[0] + ':' + xs[1] + ':' + xs[2];"),
        Ok(Value::String("3:1:5:true".to_owned()))
    );
    assert_eq!(eval("[] === [];"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("let xs = []; let ys = xs; xs === ys;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_array_member_access() {
    assert_eq!(eval("let xs = [1, 2 + 3]; xs[1];"), Ok(Value::Number(5.0)));
    assert_eq!(eval("[1, 2, 3].length;"), Ok(Value::Number(3.0)));
}

#[test]
fn evaluates_object_literals_and_member_access() {
    assert_eq!(
        eval("let o = { answer: 40 + 2 }; o.answer;"),
        Ok(Value::Number(42.0))
    );
    assert_eq!(
        eval("let answer = 42; let o = { answer }; o.answer;"),
        Ok(Value::Number(42.0))
    );
    assert_eq!(
        eval(
            "let first = 1; let second = 2; let o = { first, second: first + second }; o.first + o.second;"
        ),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("let key = 'answer'; let o = { [key]: 42 }; o.answer;"),
        Ok(Value::Number(42.0))
    );
    assert_eq!(
        eval("let o = { [1 + 1]: 'two' }; o[2];"),
        Ok(Value::String("two".to_owned()))
    );
    assert_eq!(
        eval("let object = { value: 7, method() { return this.value; } }; object.method();"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("let object = { add(a, b) { return a + b; } }; object.add(2, 3);"),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval("let method = { method() {} }.method; method.prototype;"),
        Ok(Value::Undefined)
    );
    assert!(eval("let method = { method() {} }.method; new method();").is_err());
    assert_eq!(eval("({ 'a': 1 })['a'];"), Ok(Value::Number(1.0)));
    assert_eq!(eval("({ true: 1 }).true;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("({}).missing;"), Ok(Value::Undefined));
}

#[test]
fn evaluates_member_assignment() {
    assert_eq!(
        eval("let o = {}; o.answer = 42; o.answer;"),
        Ok(Value::Number(42.0))
    );
    assert_eq!(
        eval("let key = 'answer'; let o = {}; o[key] = 7; o.answer;"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("this.answer = 42; this.answer;"),
        Ok(Value::Number(42.0))
    );
    assert_eq!(eval("this === this;"), Ok(Value::Boolean(true)));
}

#[test]
fn evaluates_global_undefined_binding() {
    assert_eq!(eval("undefined;"), Ok(Value::Undefined));
    assert_eq!(eval("undefined === undefined;"), Ok(Value::Boolean(true)));
}

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
        eval("Number.prototype.valueOf.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("let n = 10; n.toString();"),
        Ok(Value::String("10".to_owned()))
    );
    assert_eq!(
        eval("let n = 255; n.toString(16);"),
        Ok(Value::String("ff".to_owned()))
    );
    assert_eq!(eval("let n = 10; n.valueOf();"), Ok(Value::Number(10.0)));
    assert_eq!(
        eval("(new Number(7)).toString();"),
        Ok(Value::String("7".to_owned()))
    );
    assert_eq!(eval("(new Number(7)).valueOf();"), Ok(Value::Number(7.0)));
    assert_eq!(
        eval("let n = new Number(7); n.tag = Object.prototype.toString; n.tag();"),
        Ok(Value::String("[object Number]".to_owned()))
    );
    assert!(eval("let o = Object.create(Number.prototype); o.valueOf();").is_err());
    assert_eq!(
        eval("Number('abc') === Number('abc');"),
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
    assert_eq!(
        eval("parseInt('10', 37) === NaN;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("parseFloat('3.5px');"), Ok(Value::Number(3.5)));
    assert_eq!(eval("parseFloat('-1.25e2x');"), Ok(Value::Number(-125.0)));
    assert_eq!(
        eval("parseFloat('Infinity');"),
        Ok(Value::Number(f64::INFINITY))
    );
    assert_eq!(eval("parseFloat('x') === NaN;"), Ok(Value::Boolean(false)));
    assert!(eval("new Number.isNaN(NaN);").is_err());
    assert!(eval("new parseInt('10');").is_err());
    assert!(eval("new isNaN(1);").is_err());
}

#[test]
fn evaluates_math_builtins() {
    assert_eq!(eval("typeof Math;"), Ok(Value::String("object".to_owned())));
    assert_eq!(
        eval("typeof Math.PI;"),
        Ok(Value::String("number".to_owned()))
    );
    assert_eq!(eval("NaN === NaN;"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Infinity === 1 / 0;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Math.abs.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.acos.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.acosh.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.asin.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.asinh.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.atan.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.atan2.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Math.atanh.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.cbrt.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.cos.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.cosh.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.exp.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.expm1.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.fround.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.hypot.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Math.log.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.log1p.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.log10.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.log2.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.max.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Math.min.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Math.pow.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Math.sqrt.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.round.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.sign.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.sin.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.sinh.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.clz32.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.imul.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Math.tan.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.tanh.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.trunc.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.abs(-7);"), Ok(Value::Number(7.0)));
    assert_eq!(
        eval("1 / Math.abs(-0) === Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Math.ceil(1.2);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Math.floor(1.8);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.trunc(-1.8);"), Ok(Value::Number(-1.0)));
    assert_eq!(eval("Math.max(1, 9, 3);"), Ok(Value::Number(9.0)));
    assert_eq!(eval("Math.max() === -Infinity;"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Math.min(1, -9, 3);"), Ok(Value::Number(-9.0)));
    assert_eq!(eval("Math.min() === Infinity;"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("1 / Math.max(-0, 0) === Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("1 / Math.min(-0, 0) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Math.max(1, NaN) === Math.max(1, NaN);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Math.min(1, NaN) === Math.min(1, NaN);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("Math.pow(2, 8);"), Ok(Value::Number(256.0)));
    assert_eq!(
        eval("Math.pow(2, NaN) === Math.pow(2, NaN);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("Math.sqrt(81);"), Ok(Value::Number(9.0)));
    assert_eq!(
        eval("Math.sqrt(-1) === Math.sqrt(-1);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("Math.round(1.5);"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Math.round(-1.5);"), Ok(Value::Number(-1.0)));
    assert_eq!(
        eval("1 / Math.round(-0.4) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Math.sign(-7);"), Ok(Value::Number(-1.0)));
    assert_eq!(eval("Math.sign(7);"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("1 / Math.sign(-0) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Math.sign(NaN) === Math.sign(NaN);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("Math.clz32(0);"), Ok(Value::Number(32.0)));
    assert_eq!(eval("Math.clz32(1);"), Ok(Value::Number(31.0)));
    assert_eq!(eval("Math.clz32(4294967295);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.imul(2, 4);"), Ok(Value::Number(8.0)));
    assert_eq!(eval("Math.imul(-1, 8);"), Ok(Value::Number(-8.0)));
    assert_eq!(eval("Math.imul(4294967295, 5);"), Ok(Value::Number(-5.0)));
    assert_eq!(eval("Math.sin(0);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.cos(0);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.tan(0);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.asin(0);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.atan(0);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.atan2(0, 1);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.cbrt(27);"), Ok(Value::Number(3.0)));
    assert_eq!(eval("Math.exp(0);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.log(1);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.log10(1000);"), Ok(Value::Number(3.0)));
    assert_eq!(eval("Math.log2(8);"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("Math.acos(NaN) === Math.acos(NaN);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Math.log(-1) === Math.log(-1);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Math.log10(0) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Math.log2(0) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Math.acosh(1);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.asinh(0);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.atanh(0);"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Math.atanh(1) === Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Math.cosh(0);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Math.expm1(0);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.fround(1.5);"), Ok(Value::Number(1.5)));
    assert_eq!(
        eval("1 / Math.fround(-0) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Math.hypot(3, 4);"), Ok(Value::Number(5.0)));
    assert_eq!(eval("Math.hypot();"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Math.hypot(Infinity, NaN) === Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Math.log1p(0);"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Math.log1p(-1) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Math.sinh(0);"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Math.tanh(Infinity);"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("1 / Math.tanh(-0) === -Infinity;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Math.propertyIsEnumerable('PI');"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(Math, 'PI').writable;"),
        Ok(Value::Boolean(false))
    );
    assert!(eval("new Math.abs(1);").is_err());
}
