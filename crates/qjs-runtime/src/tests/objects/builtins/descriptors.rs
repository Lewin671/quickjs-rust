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
        eval("Object.getOwnPropertyDescriptor([1, 2], 'length').value;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyDescriptor({}, 'missing');"),
        Ok(Value::Undefined)
    );
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
        Ok(Value::String("own".to_owned()))
    );
    assert_eq!(
        eval(
            "let constructors = [Object, Function, Array, String, Boolean, Number, Date, RegExp, Error, EvalError, RangeError, ReferenceError, SyntaxError, TypeError, URIError]; constructors.every(function (ctor) { let d = Object.getOwnPropertyDescriptor(ctor, 'prototype'); return d.value === ctor.prototype && !d.writable && !d.enumerable && !d.configurable; });"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let descriptors = Object.getOwnPropertyDescriptors('ab'); descriptors.length.value + ':' + descriptors[0].value + ':' + descriptors[0].writable + ':' + descriptors[0].configurable;"
        ),
        Ok(Value::String("2:a:false:false".to_owned()))
    );
    assert_eq!(
        eval("Object.keys(Object.getOwnPropertyDescriptors(0)).length;"),
        Ok(Value::Number(0.0))
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
    assert_eq!(
        eval(
            "let object = {}; let data = 'data'; Object.defineProperty(object, 'value', { set: function(value) { data = value; } }); object.value = 'updated'; data;"
        ),
        Ok(Value::String("updated".to_owned()))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, Infinity, {}); Object.defineProperty(object, -0, {}); Object.hasOwn(object, 'Infinity') && Object.hasOwn(object, '0');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, [1, 2], { value: 9 }); object['1,2'];"
        ),
        Ok(Value::Number(9.0))
    );
    assert_eq!(
        eval(
            "let object = {}; let toStringAccessed = false; let valueOfAccessed = false; let key = { toString: function() { toStringAccessed = true; return {}; }, valueOf: function() { valueOfAccessed = true; return 'abc'; } }; Object.defineProperty(object, key, {}); Object.hasOwn(object, 'abc') && toStringAccessed && valueOfAccessed;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = { abc: 1 }; let toStringAccessed = false; let valueOfAccessed = false; let key = { toString: function() { toStringAccessed = true; return {}; }, valueOf: function() { valueOfAccessed = true; return 'abc'; } }; let descriptor = Object.getOwnPropertyDescriptor(object, key); descriptor.value === 1 && toStringAccessed && valueOfAccessed;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = {}; let descriptor = function() {}; descriptor.value = 7; Object.defineProperty(object, 'value', descriptor); object.value;"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let object = {}; let descriptor = []; descriptor.value = 8; Reflect.defineProperty(object, 'value', descriptor) && object.value;"
        ),
        Ok(Value::Number(8.0))
    );
    assert_eq!(
        eval(
            "let array = [0, 1, 2]; Object.defineProperty(array, '2', { configurable: false }); Object.getOwnPropertyDescriptor(array, '2').configurable;"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "let array = [0, 1, 2]; let caught = false; Object.defineProperty(array, '2', { configurable: false }); try { Object.defineProperty(array, 'length', { value: 1 }); } catch (error) { caught = error instanceof TypeError; } caught + ':' + array.length;"
        ),
        Ok(Value::String("true:3".to_owned()))
    );
    assert_eq!(
        eval(
            "let array = [0, 1, 2]; let caught = false; Object.defineProperty(array, '1', { configurable: false }); try { Object.defineProperty(array, 'length', { value: 0, writable: false }); } catch (error) { caught = error instanceof TypeError; } caught + ':' + array.length + ':' + Object.getOwnPropertyDescriptor(array, 'length').writable;"
        ),
        Ok(Value::String("true:2:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let object = {}; let getter = function() { return 1; }; Object.defineProperty(object, 'value', { get: getter }); Object.defineProperty(object, 'value', { set: undefined }); let descriptor = Object.getOwnPropertyDescriptor(object, 'value'); descriptor.get === getter && descriptor.set === undefined;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'value', { get: undefined, set: undefined, configurable: true }); let before = Object.getOwnPropertyDescriptor(object, 'value'); Object.defineProperty(object, 'value', { value: 1001 }); let after = Object.getOwnPropertyDescriptor(object, 'value'); before.hasOwnProperty('get') && !before.hasOwnProperty('value') && after.hasOwnProperty('value');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let array = []; let valueOfAccessed = false; let toStringAccessed = false; Object.defineProperty(array, 'length', { value: { valueOf: function() { valueOfAccessed = true; return {}; }, toString: function() { toStringAccessed = true; return '2'; } } }); array.length + ':' + valueOfAccessed + ':' + toStringAccessed;"
        ),
        Ok(Value::String("2:true:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let array = []; Object.defineProperty(array, 'length', { writable: false }); Object.defineProperty(array, 'length', { value: 0 }); array.length;"
        ),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let array = [1, 2, 3]; Object.defineProperty(array, 'length', { writable: false }); let caught = false; try { Object.defineProperty(array, '3', { value: 'abc' }); } catch (error) { caught = error instanceof TypeError; } caught + ':' + array.length;"
        ),
        Ok(Value::String("true:3".to_owned()))
    );
    assert_eq!(
        eval(
            "let array = [0, 1, 2]; Object.defineProperty(array, '1', { configurable: false }); Object.defineProperty(array, '2', { configurable: true }); let caught = false; try { Object.defineProperty(array, 'length', { value: 1 }); } catch (error) { caught = error instanceof TypeError; } caught + ':' + array.length + ':' + array.hasOwnProperty('2');"
        ),
        Ok(Value::String("true:2:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let array = []; Object.defineProperty(array, '0', { value: 2010, writable: true, enumerable: true, configurable: false }); array[0] = 1001; array[0];"
        ),
        Ok(Value::Number(1001.0))
    );
    assert_eq!(
        eval(
            "let caught = false; try { Object.defineProperty([], 'length', { value: undefined }); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; try { Object.defineProperty([], 'length', { value: -1 }); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "function f() {} let data = 'data'; Object.defineProperty(Function.prototype, 'prop', { get: function() { return data; }, set: function(value) { data = value; }, enumerable: true, configurable: true }); let object = f.bind({}); object.prop = 'overrideData'; let result = !object.hasOwnProperty('prop') && object.prop === 'overrideData'; delete Function.prototype.prop; result;"
        ),
        Ok(Value::Boolean(true))
    );
    assert!(eval("Object.getOwnPropertyDescriptors(null);").is_err());
    assert!(eval("Object.getOwnPropertyDescriptors(undefined);").is_err());
    assert!(eval("Object.getOwnPropertyDescriptor(null, 'value');").is_err());
    assert!(eval("Object.getOwnPropertyDescriptor(undefined, 'value');").is_err());
}
