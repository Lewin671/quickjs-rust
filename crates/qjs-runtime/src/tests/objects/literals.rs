use crate::{Value, eval};

#[test]
fn numeric_literal_keys_use_their_canonical_name() {
    // The property name of a numeric literal is `ToString(MV)`, so different
    // notations for the same value name the same property.
    assert_eq!(
        eval("let o = { 0b10: 'a', 0x10: 'b', 1.0: 'c' }; o[2] + o[16] + o[1];"),
        Ok(Value::String("abc".to_owned().into()))
    );
    assert_eq!(
        eval("let o = { 0o17: 'x' }; o['15'];"),
        Ok(Value::String("x".to_owned().into()))
    );
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
        Ok(Value::String("two".to_owned().into()))
    );
    assert_eq!(
        eval("let key = Symbol('key'); let object = { [key]: 42 }; object[key];"),
        Ok(Value::Number(42.0))
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
fn static_data_literal_bulk_path_preserves_order_duplicates_and_evaluation() {
    assert_eq!(
        eval(
            "let log = ''; function take(x) { log += x; return x; } let object = { b: take('b'), a: take('a'), b: take('B') }; log + ':' + Object.keys(object).join(',') + ':' + object.b;"
        ),
        Ok(Value::String("baB:b,a:B".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let proto = { value: 41 }; let object = { method() { return super.value + 1; } }; Object.setPrototypeOf(object, proto); object.method();"
        ),
        Ok(Value::Number(42.0))
    );
    assert_eq!(
        eval(
            "let object = { a: 1, b: 2 }; object.c = 3; delete object.a; object.a = 4; Object.keys(object).join(',') + ':' + object.a + object.b + object.c;"
        ),
        Ok(Value::String("b,c,a:423".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function make(value) { return { x: value, y: value + 1 }; } let first = make(1); let second = make(10); first.x = 3; first.x + ':' + first.y + ':' + second.x + ':' + second.y;"
        ),
        Ok(Value::String("3:2:10:11".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = { 2: 'two', a: 1, 1: 'one' }; Object.defineProperty(object, 'a', { value: 4, enumerable: false }); Object.getOwnPropertyNames(object).join(',') + ':' + Object.keys(object).join(',') + ':' + object.a;"
        ),
        Ok(Value::String("1,2,a:1,2:4".to_owned().into()))
    );
}

#[test]
fn evaluates_object_spread_properties() {
    assert_eq!(
        eval(
            "let source = { x: 1, y: 2 }; let object = { ...source, y: 3 }; object.x + ':' + object.y + ':' + Object.keys(object).join(',');"
        ),
        Ok(Value::String("1:3:x,y".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = { a: 1, ...null, ...undefined, b: 2 }; Object.keys(object).join(',') + ':' + object.a + ':' + object.b;"
        ),
        Ok(Value::String("a,b:1:2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let source = { x: 1 }; let object = { ...source, y: (source.x = 9) }; object.x + ':' + object.y + ':' + source.x;"
        ),
        Ok(Value::String("1:9:9".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let calls = 0; let source = { get x() { calls = calls + 1; return 7; } }; let object = { ...source }; object.x + ':' + calls;"
        ),
        Ok(Value::String("7:1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let key = Symbol('k'); let source = {}; source[key] = 5; let object = { ...source }; object[key];"
        ),
        Ok(Value::Number(5.0))
    );
}

#[test]
fn proto_literal_special_form_sets_prototype() {
    // `{ __proto__: expr }` with a literal key and colon data form sets
    // [[Prototype]] rather than creating an own property.
    assert_eq!(
        eval("let p = { a: 1 }; let o = { __proto__: p, b: 2 }; o.a + o.b;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("let p = { a: 1 }; let o = { __proto__: p }; Object.getPrototypeOf(o) === p;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.keys({ __proto__: { a: 1 }, b: 2 }).join(',');"),
        Ok(Value::String("b".to_owned().into()))
    );
    // A null proto literal yields an object with no prototype.
    assert_eq!(
        eval("Object.getPrototypeOf({ __proto__: null });"),
        Ok(Value::Null)
    );
    // Non-object, non-null proto values are ignored by the special form.
    assert_eq!(
        eval("Object.getPrototypeOf({ __proto__: 42 }) === Object.prototype;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn proto_only_special_for_literal_colon_data_form() {
    // Computed, shorthand, and method `__proto__` forms stay ordinary props.
    assert_eq!(
        eval(
            "let o = { ['__proto__']: 9 }; o.__proto__ === 9 && Object.getPrototypeOf(o) === Object.prototype;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let __proto__ = 5; let o = { __proto__ }; Object.getOwnPropertyDescriptor(o, '__proto__').value;"
        ),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval("let o = { __proto__() { return 7; } }; o.__proto__();"),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn object_prototype_proto_accessor_descriptor() {
    assert_eq!(
        eval("typeof Object.getOwnPropertyDescriptor(Object.prototype, '__proto__').get;"),
        Ok(Value::String("function".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Object.prototype, '__proto__'); d.enumerable === false && d.configurable === true;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(Object.prototype, '__proto__').get.name;"),
        Ok(Value::String("get __proto__".to_owned().into()))
    );
    // Setter on a primitive `this` is a no-op; on null/undefined it throws.
    assert_eq!(
        eval(
            "let s = Object.getOwnPropertyDescriptor(Object.prototype, '__proto__').set; s.call(1, {});"
        ),
        Ok(Value::Undefined)
    );
    assert!(
        eval("Object.getOwnPropertyDescriptor(Object.prototype, '__proto__').get.call(null);")
            .is_err()
    );
}
