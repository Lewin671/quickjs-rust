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
    assert_eq!(eval("Reflect.apply.length;"), Ok(Value::Number(3.0)));
    assert_eq!(
        eval("Reflect.getPrototypeOf.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Reflect.getOwnPropertyDescriptor.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("Reflect.defineProperty.length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval("Reflect.deleteProperty.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(eval("Reflect.get.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Reflect.has.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Reflect.isExtensible.length;"), Ok(Value::Number(1.0)));
    assert_eq!(eval("Reflect.ownKeys.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Reflect.preventExtensions.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(eval("Reflect.set.length;"), Ok(Value::Number(3.0)));
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
            "function add(a, b) { return this.base + a + b; } let context = { base: 4 }; Reflect.apply(add, context, [2, 3]);"
        ),
        Ok(Value::Number(9.0))
    );
    assert_eq!(
        eval("function count() { return arguments.length; } Reflect.apply(count, null, []);"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("function getThis() { return this; } Reflect.apply(getThis, undefined, []) === this;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function value() { return 57; } Reflect.apply(value, null, []);"),
        Ok(Value::Number(57.0))
    );
    assert_eq!(
        eval("Reflect.get({ value: 5 }, 'value');"),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval("Reflect.get(Object.create({ inherited: 7 }), 'inherited');"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(eval("Reflect.get({}, 'missing');"), Ok(Value::Undefined));
    assert_eq!(
        eval("Reflect.get([3, 4], '1') + Reflect.get([3, 4], 'length');"),
        Ok(Value::Number(6.0))
    );
    assert_eq!(
        eval("Reflect.get(function f(a, b) {}, 'length');"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("function f() {} f.value = 11; Reflect.get(f, 'value');"),
        Ok(Value::Number(11.0))
    );
    assert_eq!(
        eval("let object = {}; Reflect.set(object, 'value', 41) && object.value;"),
        Ok(Value::Number(41.0))
    );
    assert_eq!(
        eval(
            "let target = { value: 1 }; let receiver = { value: 2 }; Reflect.set(target, 'value', 43, receiver) && target.value + receiver.value;"
        ),
        Ok(Value::Number(44.0))
    );
    assert_eq!(
        eval(
            "let target = Object.create({ inherited: 5 }); Reflect.set(target, 'inherited', 47) && target.inherited;"
        ),
        Ok(Value::Number(47.0))
    );
    assert_eq!(
        eval("let array = [1]; Reflect.set(array, '1', 2) && array.length + array[1];"),
        Ok(Value::Number(4.0))
    );
    assert_eq!(
        eval("let array = [1, 2, 3]; Reflect.set(array, 'length', 1) && array.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("function f() {} Reflect.set(f, 'value', 53) && f.value;"),
        Ok(Value::Number(53.0))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'fixed', { value: 1 }); Reflect.set(object, 'fixed', 2);"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let object = {}; Object.preventExtensions(object); Reflect.set(object, 'value', 1);"),
        Ok(Value::Boolean(false))
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
    assert_eq!(
        eval("Reflect.has({ own: 1 }, 'own');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Reflect.has(Object.create({ inherited: 1 }), 'inherited');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Reflect.has([1, 2], 'length') && Reflect.has([1, 2], '1');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let proto = { marker: 11 }; let array = []; Object.setPrototypeOf(array, proto); Reflect.has(array, 'marker');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Reflect.has(function f() {}, 'call');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let object = {}; Reflect.isExtensible(object);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let object = {}; Object.preventExtensions(object); Reflect.isExtensible(object);"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let array = []; Reflect.preventExtensions(array) && !Reflect.isExtensible(array);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function f() {} Reflect.preventExtensions(f) && !Reflect.isExtensible(f) && Reflect.preventExtensions(f);"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Reflect.getOwnPropertyDescriptor({ value: 1 }, 'value').value;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Reflect.getOwnPropertyDescriptor([1, 2], 'length').enumerable;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Reflect.getOwnPropertyDescriptor(function f(a, b) {}, 'length').value;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("Reflect.getOwnPropertyDescriptor({}, 'missing');"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval(
            "let object = {}; Reflect.defineProperty(object, 'value', { value: 19, enumerable: true, writable: true, configurable: true }) && object.value;"
        ),
        Ok(Value::Number(19.0))
    );
    assert_eq!(
        eval(
            "let object = {}; Reflect.defineProperty(object, 'hidden', { value: 23 }); Object.keys(object).length + ':' + object.hidden;"
        ),
        Ok(Value::String("0:23".to_owned()))
    );
    assert_eq!(
        eval(
            "function f() {} Reflect.defineProperty(f, 'value', { value: 29, enumerable: true }) && f.value;"
        ),
        Ok(Value::Number(29.0))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.preventExtensions(object); Reflect.defineProperty(object, 'value', { value: 1 });"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'value', { value: 1 }); Reflect.defineProperty(object, 'value', { configurable: true });"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let object = { value: 31 }; Reflect.deleteProperty(object, 'value') && !Reflect.has(object, 'value');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'fixed', { value: 1 }); Reflect.deleteProperty(object, 'fixed');"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("let object = {}; Reflect.deleteProperty(object, 'missing');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function f() {} f.value = 37; Reflect.deleteProperty(f, 'value') && !Reflect.has(f, 'value');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Reflect.deleteProperty(function f() {}, 'length');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let f = function() {}; Reflect.deleteProperty(f, 'length'); Reflect.getOwnPropertyDescriptor(f, 'length');"
        ),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("Reflect.ownKeys({ a: 1, b: 2 }).join();"),
        Ok(Value::String("a,b".to_owned()))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'hidden', { value: 1 }); object.shown = 2; Reflect.ownKeys(object).join();"
        ),
        Ok(Value::String("hidden,shown".to_owned()))
    );
    assert_eq!(
        eval("Reflect.ownKeys([1, 2]).join();"),
        Ok(Value::String("0,1,length".to_owned()))
    );
    assert!(eval("Reflect.apply(1, null, []);").is_err());
    assert!(eval("Reflect.apply(function f() {}, null, 1);").is_err());
    assert!(eval("Reflect.getPrototypeOf(1);").is_err());
    assert!(eval("Reflect.defineProperty(1, 'value', { value: 1 });").is_err());
    assert!(eval("Reflect.deleteProperty(1, 'value');").is_err());
    assert!(eval("Reflect.get(1, 'value');").is_err());
    assert!(eval("Reflect.getOwnPropertyDescriptor(1, 'toString');").is_err());
    assert!(eval("Reflect.has(1, 'toString');").is_err());
    assert!(eval("Reflect.isExtensible(1);").is_err());
    assert!(eval("Reflect.ownKeys(1);").is_err());
    assert!(eval("Reflect.preventExtensions(1);").is_err());
    assert!(eval("Reflect.set(1, 'value', 1);").is_err());
    assert!(eval("Reflect.setPrototypeOf(1, null);").is_err());
    assert!(eval("Reflect.setPrototypeOf({}, 1);").is_err());
}
