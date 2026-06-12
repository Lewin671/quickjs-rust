use crate::{Value, eval};

#[test]
fn numeric_literal_keys_use_their_canonical_name() {
    // The property name of a numeric literal is `ToString(MV)`, so different
    // notations for the same value name the same property.
    assert_eq!(
        eval("let o = { 0b10: 'a', 0x10: 'b', 1.0: 'c' }; o[2] + o[16] + o[1];"),
        Ok(Value::String("abc".to_owned()))
    );
    assert_eq!(
        eval("let o = { 0o17: 'x' }; o['15'];"),
        Ok(Value::String("x".to_owned()))
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
        Ok(Value::String("two".to_owned()))
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
        Ok(Value::String("b".to_owned()))
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
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Object.prototype, '__proto__'); d.enumerable === false && d.configurable === true;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor(Object.prototype, '__proto__').get.name;"),
        Ok(Value::String("get __proto__".to_owned()))
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
