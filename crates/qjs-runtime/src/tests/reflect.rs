use crate::{Value, eval};

#[test]
fn evaluates_reflect_prototype_builtins() {
    assert_eq!(
        eval("typeof Reflect;"),
        Ok(Value::String("object".to_owned()))
    );
    assert_eq!(
        eval("typeof Reflect.getPrototypeOf;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(
        eval("Reflect.getPrototypeOf.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Reflect.setPrototypeOf.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("Reflect.getPrototypeOf({}) === Object.prototype;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Reflect.getPrototypeOf(Object.create(null));"),
        Ok(Value::Null)
    );
    assert_eq!(
        eval("Reflect.getPrototypeOf([]) === Array.prototype;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Reflect.getPrototypeOf(function f() {}) === Function.prototype;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let proto = { marker: 7 }; let object = {}; Reflect.setPrototypeOf(object, proto); object.marker;"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let proto = { marker: 11 }; let array = []; Reflect.setPrototypeOf(array, proto); array.marker;"
        ),
        Ok(Value::Number(11.0))
    );
    assert_eq!(
        eval(
            "let proto = { marker: 13 }; function f() {} Reflect.setPrototypeOf(f, proto); f.marker;"
        ),
        Ok(Value::Number(13.0))
    );
    assert_eq!(
        eval(
            "let object = {}; Reflect.setPrototypeOf(object, null) && Reflect.getPrototypeOf(object) === null;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.preventExtensions(object); Reflect.setPrototypeOf(object, null);"
        ),
        Ok(Value::Boolean(false))
    );
    assert!(eval("Reflect.getPrototypeOf(1);").is_err());
    assert!(eval("Reflect.setPrototypeOf(1, null);").is_err());
    assert!(eval("Reflect.setPrototypeOf({}, 1);").is_err());
}
