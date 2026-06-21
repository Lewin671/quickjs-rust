use crate::{Value, eval};

#[test]
fn evaluates_object_prototype_mutation_builtins() {
    assert_eq!(
        eval("Object.setPrototypeOf.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("typeof Object.setPrototypeOf;"),
        Ok(Value::String("function".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let proto = { value: 7 }; let object = {}; Object.setPrototypeOf(object, proto) === object;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let proto = { value: 7 }; let object = {}; Object.setPrototypeOf(object, proto); object.value;"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let proto = { value: 7 }; let object = {}; Object.setPrototypeOf(object, proto); Object.getPrototypeOf(object) === proto;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.setPrototypeOf(object, null); Object.getPrototypeOf(object);"
        ),
        Ok(Value::Null)
    );
    assert_eq!(
        eval("Object.setPrototypeOf(1, null);"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("let symbol = Symbol('target'); Object.setPrototypeOf(symbol, null) === symbol;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let proto = { marker: 11 }; let array = []; Object.setPrototypeOf(array, proto) === array;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let proto = { marker: 11 }; let array = []; Object.setPrototypeOf(array, proto); array.marker;"
        ),
        Ok(Value::Number(11.0))
    );
    assert_eq!(
        eval(
            "let proto = {}; let array = []; Object.setPrototypeOf(array, proto); Object.getPrototypeOf(array) === proto;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let array = []; Object.setPrototypeOf(array, null); Object.getPrototypeOf(array);"),
        Ok(Value::Null)
    );
    assert_eq!(
        eval("let proto = { marker: 13 }; function f() {} Object.setPrototypeOf(f, proto) === f;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let proto = { marker: 13 }; function f() {} Object.setPrototypeOf(f, proto); f.marker;"
        ),
        Ok(Value::Number(13.0))
    );
    assert_eq!(
        eval(
            "let proto = {}; function f() {} Object.setPrototypeOf(f, proto); Object.getPrototypeOf(f) === proto;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function f() {} Object.setPrototypeOf(f, null); Object.getPrototypeOf(f);"),
        Ok(Value::Null)
    );
    assert_eq!(
        eval(
            "let caught = false; function f() {} try { Object.setPrototypeOf(f, f); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function f() {} Reflect.setPrototypeOf(f, f);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let caught = false; let object = {}; function f() {} Object.setPrototypeOf(f, object); try { Object.setPrototypeOf(object, f); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = {}; function f() {} Object.setPrototypeOf(f, object); Reflect.setPrototypeOf(object, f);"
        ),
        Ok(Value::Boolean(false))
    );
    assert!(eval("Object.setPrototypeOf(null, null);").is_err());
    assert!(eval("Object.setPrototypeOf(undefined, null);").is_err());
    assert!(eval("Object.setPrototypeOf({}, 1);").is_err());
    assert!(eval("Object.setPrototypeOf({}, Symbol('proto'));").is_err());
    assert!(
        eval(
            "let array = []; Object.preventExtensions(array); Object.setPrototypeOf(array, null);"
        )
        .is_err()
    );
    assert!(
        eval("function f() {} Object.preventExtensions(f); Object.setPrototypeOf(f, null);")
            .is_err()
    );
    assert!(eval("let object = {}; Object.preventExtensions(object); Object.setPrototypeOf(object, null);").is_err());
    assert!(
        eval("let parent = {}; let child = Object.create(parent); Object.setPrototypeOf(parent, child);").is_err()
    );
    assert!(eval("Object.create(1);").is_err());
    assert!(eval("new Object.create({});").is_err());
    assert!(eval("new Object.prototype.hasOwnProperty('value');").is_err());
}
