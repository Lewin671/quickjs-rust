use crate::{Value, eval};

#[test]
fn evaluates_object_descriptor_queries() {
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
        eval(
            "['Object','Function','Array','String','Boolean','Number','Date','RegExp','Error','EvalError','RangeError','ReferenceError','SyntaxError','TypeError','URIError','Map','Set','WeakMap','WeakSet','Promise','Symbol'].every(function(name) { let d = Object.getOwnPropertyDescriptor(this[name], 'prototype'); return d.writable === false && d.enumerable === false && d.configurable === false && !d.hasOwnProperty('get') && !d.hasOwnProperty('set'); });"
        ),
        Ok(Value::Boolean(true))
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
        eval("Object.getOwnPropertyDescriptor(0, 'missing');"),
        Ok(Value::Undefined)
    );
    assert!(eval("Object.getOwnPropertyDescriptor(null, 'missing');").is_err());
    assert!(eval("Object.getOwnPropertyDescriptor(undefined, 'missing');").is_err());
    assert_eq!(
        eval("Object.getOwnPropertyDescriptors.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Object.getPrototypeOf(Object.getOwnPropertyDescriptors({})) === Object.prototype;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let descriptors = Object.getOwnPropertyDescriptors({ value: 1 }); descriptors.value.value;"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptors({ value: 1 }).value.enumerable;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'hidden', { value: 2 }); Object.getOwnPropertyDescriptors(object).hidden.enumerable;"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let object = Object.create({ inherited: 1 }, { own: { value: 2, enumerable: true } }); Object.keys(Object.getOwnPropertyDescriptors(object)).join();"
        ),
        Ok(Value::String("own".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let descriptors = Object.getOwnPropertyDescriptors('ab'); descriptors.length.value + ':' + descriptors[0].value + ':' + descriptors[0].writable + ':' + descriptors[0].configurable;"
        ),
        Ok(Value::String("2:a:false:false".to_owned().into()))
    );
    assert_eq!(
        eval("Object.keys(Object.getOwnPropertyDescriptors(0)).length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let object = {}; let value = {}; let hidden = Symbol('hidden'); let shown = Symbol('shown'); object[shown] = value; Object.defineProperty(object, hidden, { value: value, enumerable: false, writable: true, configurable: true }); let descriptors = Object.getOwnPropertyDescriptors(object); let symbols = Object.getOwnPropertySymbols(descriptors); symbols.length + ':' + (symbols[0] === shown) + ':' + (symbols[1] === hidden) + ':' + descriptors[shown].enumerable + ':' + descriptors[hidden].enumerable + ':' + (descriptors[hidden].value === value);"
        ),
        Ok(Value::String(
            "2:true:true:true:false:true".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'value', { get: function() { return 9; }, enumerable: true, configurable: true }); object.value;"
        ),
        Ok(Value::Number(9.0))
    );
    assert_eq!(
        eval(
            "let object = {}; let getter = function() { return 9; }; Object.defineProperty(object, 'value', { get: getter, enumerable: true, configurable: true }); let descriptor = Object.getOwnPropertyDescriptor(object, 'value'); descriptor.get === getter && descriptor.set === undefined && descriptor.enumerable && descriptor.configurable;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'value', { set: function(_) {}, configurable: true }); object.value;"
        ),
        Ok(Value::Undefined)
    );
    assert!(eval("Object.getOwnPropertyDescriptors(null);").is_err());
    assert!(eval("Object.getOwnPropertyDescriptors(undefined);").is_err());
    // getOwnPropertyDescriptors drives a Proxy's ownKeys once, then its
    // getOwnPropertyDescriptor per key, in [[OwnPropertyKeys]] order.
    assert_eq!(
        eval(
            "let log = ''; let t = { a: 1, b: 2, c: 3 }; \
             let p = new Proxy(t, { ownKeys(o) { log += 'K'; return Reflect.ownKeys(o); }, getOwnPropertyDescriptor(o, k) { log += k; return Reflect.getOwnPropertyDescriptor(o, k); } }); \
             let d = Object.getOwnPropertyDescriptors(p); log + ':' + Object.keys(d).join(',') + ':' + d.b.value;"
        ),
        Ok(Value::String("Kabc:a,b,c:2".to_owned().into()))
    );
}

#[test]
fn define_property_rejects_symbol_descriptor() {
    // A Symbol is a primitive, not a valid property descriptor object.
    assert!(eval("Object.defineProperty({}, 'a', Symbol());").is_err());
}

#[test]
fn strict_delete_of_non_configurable_property_throws() {
    // Strict-mode `delete` of a non-configurable property is a TypeError; the
    // sloppy form silently returns false.
    assert!(
        eval("'use strict'; let o = {}; Object.defineProperty(o, 'x', { value: 1 }); delete o.x;")
            .is_err()
    );
    assert_eq!(
        eval("let o = {}; Object.defineProperty(o, 'x', { value: 1 }); delete o.x;"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "'use strict'; let s = Symbol(); let o = {}; Object.defineProperty(o, s, { value: 1 }); let threw = false; try { delete o[s]; } catch (e) { threw = e instanceof TypeError; } threw;"
        ),
        Ok(Value::Boolean(true))
    );
    // Deleting a configurable property still succeeds in strict mode.
    assert_eq!(
        eval("'use strict'; let o = { x: 1 }; delete o.x;"),
        Ok(Value::Boolean(true))
    );
}
