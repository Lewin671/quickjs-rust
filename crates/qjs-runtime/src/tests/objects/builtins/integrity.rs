use crate::{Value, eval};

#[test]
fn evaluates_object_integrity_builtins() {
    assert_eq!(eval("Object.isExtensible.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Object.isExtensible({});"), Ok(Value::Boolean(true)));
    assert_eq!(
        eval("let object = {}; Object.preventExtensions(object); Object.isExtensible(object);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("Object.isExtensible(1);"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("Object.preventExtensions.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("let object = {}; Object.preventExtensions(object) === object;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let object = {}; Object.preventExtensions(object); object.added = 1; object.added;"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("let array = [1]; Object.preventExtensions(array); array[1] = 2; array.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("let array = [1]; Object.preventExtensions(array); Object.isExtensible(array);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("function fn() {} Object.preventExtensions(fn); fn.added = 1; fn.added;"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("function fn() {} Object.preventExtensions(fn); Object.isExtensible(fn);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let object = { value: 1 }; Object.preventExtensions(object); object.value = 2; object.value;"
        ),
        Ok(Value::Number(2.0))
    );
    assert!(
        eval("let object = {}; Object.preventExtensions(object); Object.defineProperty(object, 'value', { value: 1 });").is_err()
    );
    assert_eq!(eval("Object.preventExtensions(1);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Object.seal.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("typeof Object.seal;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Object.isSealed.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Object.isSealed({});"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("let object = {}; Object.seal(object) === object;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let object = {}; Object.seal(object); Object.isExtensible(object);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let object = { value: 1 }; Object.seal(object); Object.isSealed(object);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = { value: 1 }; Object.seal(object); Object.getOwnPropertyDescriptor(object, 'value').configurable;"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let object = { value: 1 }; Object.seal(object); object.value = 2; object.value;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("let object = { value: 1 }; Object.seal(object); delete object.value; object.value;"),
        Ok(Value::Number(1.0))
    );
    assert!(
        eval("let object = { value: 1 }; Object.seal(object); Object.defineProperty(object, 'value', { value: 2, configurable: true });").is_err()
    );
    assert_eq!(
        eval("let array = [1]; Object.seal(array); Object.isSealed(array);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let array = [1]; Object.seal(array); Object.getOwnPropertyDescriptor(array, '0').configurable;"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("function fn() {} Object.seal(fn); Object.isSealed(fn);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function fn() {} Object.seal(fn); Object.getOwnPropertyDescriptor(fn, 'length').configurable;"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("Object.isSealed(1);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Object.seal(1);"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Object.freeze.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("typeof Object.freeze;"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(eval("Object.isFrozen.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Object.isFrozen({});"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("let object = {}; Object.freeze(object) === object;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let object = {}; Object.freeze(object); Object.isExtensible(object);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let object = { value: 1 }; Object.freeze(object); Object.isSealed(object);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let object = { value: 1 }; Object.freeze(object); Object.isFrozen(object);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = { value: 1 }; Object.freeze(object); Object.getOwnPropertyDescriptor(object, 'value').configurable;"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let object = { value: 1 }; Object.freeze(object); Object.getOwnPropertyDescriptor(object, 'value').writable;"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let object = { value: 1 }; Object.freeze(object); object.value = 2; object.value;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let object = { value: 1 }; Object.freeze(object); delete object.value; object.value;"
        ),
        Ok(Value::Number(1.0))
    );
    assert!(
        eval("let object = { value: 1 }; Object.freeze(object); Object.defineProperty(object, 'value', { value: 2, writable: true });").is_err()
    );
    assert_eq!(
        eval("let array = [1]; Object.freeze(array); Object.isFrozen(array);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let array = [1]; Object.freeze(array); array[0] = 2; array[0];"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("let array = [1]; Object.freeze(array); array.length = 0; array.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let array = [1]; Object.freeze(array); Object.getOwnPropertyDescriptor(array, '0').writable;"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("function fn() {} Object.freeze(fn); Object.isFrozen(fn);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function fn() {} fn.value = 1; Object.freeze(fn); fn.value = 2; fn.value;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "function fn(a) {} Object.freeze(fn); Object.getOwnPropertyDescriptor(fn, 'length').configurable;"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(eval("Object.isFrozen(1);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Object.freeze(1);"), Ok(Value::Number(1.0)));
}

#[test]
fn object_seal_and_freeze_route_through_proxy_traps() {
    // SetIntegrityLevel on a Proxy throws when [[PreventExtensions]] reports
    // false, and propagates an abrupt completion from the trap.
    assert!(
        eval("Object.seal(new Proxy({}, { preventExtensions() { return false; } }));").is_err()
    );
    assert!(
        eval(
            "Object.freeze(new Proxy({}, { preventExtensions() { throw new TypeError('x'); } }));"
        )
        .is_err()
    );
    // Freeze drives the defineProperty trap with the frozen-data descriptor.
    assert_eq!(
        eval(
            "let seen = []; let t = { a: 1 }; \
             let p = new Proxy(t, { \
                 defineProperty(tt, k, d) { seen.push(k + ':' + d.writable + ':' + d.configurable); return Reflect.defineProperty(tt, k, d); } \
             }); \
             Object.freeze(p); \
             seen.join('|') + '#' + Object.getOwnPropertyDescriptor(t, 'a').writable;"
        ),
        Ok(Value::String("a:false:false#false".to_owned()))
    );
    // Seal makes own keys non-configurable but leaves writability untouched.
    assert_eq!(
        eval(
            "let t = { a: 1 }; Object.seal(new Proxy(t, {})); \
             let d = Object.getOwnPropertyDescriptor(t, 'a'); d.configurable + ':' + d.writable;"
        ),
        Ok(Value::String("false:true".to_owned()))
    );
}
