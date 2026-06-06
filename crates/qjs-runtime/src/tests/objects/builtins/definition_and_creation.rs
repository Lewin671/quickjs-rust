use crate::{Value, eval};

#[test]
fn evaluates_object_definition_and_creation_builtins() {
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
        eval(
            "let object = {}; Object.defineProperty(object, [1, 2], {}); object.hasOwnProperty('1,2');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, new String('Hello'), {}); object.hasOwnProperty('Hello');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, new Boolean(false), {}); object.hasOwnProperty('false');"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = {}; let getter = function() { return 1; }; Object.defineProperty(object, 'value', { get: getter, enumerable: false, configurable: false }); try { Object.defineProperty(object, 'value', { get: getter, enumerable: true }); 'not thrown'; } catch (error) { let desc = Object.getOwnPropertyDescriptor(object, 'value'); desc.get === getter && desc.enumerable === false && desc.configurable === false; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'value', { value: 101, configurable: false }); try { Object.defineProperty(object, 'value', { get: function() { return 1; } }); 'not thrown'; } catch (error) { let desc = Object.getOwnPropertyDescriptor(object, 'value'); desc.value === 101 && desc.writable === false && desc.configurable === false; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = {}; let getter = function() { return 1; }; Object.defineProperty(object, 'value', { get: getter, configurable: false }); try { Object.defineProperty(object, 'value', { value: 101 }); 'not thrown'; } catch (error) { let desc = Object.getOwnPropertyDescriptor(object, 'value'); desc.get === getter && desc.configurable === false; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = {}; let getter = function() { return 1; }; Object.defineProperty(object, 'value', { get: getter, configurable: false }); Object.defineProperty(object, 'value', {}); Object.getOwnPropertyDescriptor(object, 'value').get === getter;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; let array = []; try { Object.defineProperty(array, 'length', { value: -1 }); } catch (error) { caught = error instanceof RangeError; } caught + ':' + array.length;"
        ),
        Ok(Value::String("true:0".to_owned()))
    );
    assert_eq!(
        eval(
            "let caught = false; let array = [0, 1, 2]; Object.defineProperty(array, '2', { configurable: false }); try { Object.defineProperty(array, 'length', { value: 1 }); } catch (error) { caught = error instanceof TypeError; } caught + ':' + array.length + ':' + array[2];"
        ),
        Ok(Value::String("true:3:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let caught = false; let array = [0, 1, 2]; Object.defineProperty(array, 'length', { writable: false }); try { Object.defineProperty(array, '3', { value: 3 }); } catch (error) { caught = error instanceof TypeError; } caught + ':' + array.length + ':' + array.hasOwnProperty('3');"
        ),
        Ok(Value::String("true:3:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let array = []; let valueOfAccessed = false; let toStringAccessed = false; Object.defineProperty(array, 'length', { value: { valueOf: function() { valueOfAccessed = true; return 3; }, toString: function() { toStringAccessed = true; return '2'; } } }); array.length + ':' + valueOfAccessed + ':' + toStringAccessed;"
        ),
        Ok(Value::String("3:true:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let array = []; let valueOfAccessed = false; let toStringAccessed = false; Object.defineProperty(array, 'length', { value: { valueOf: function() { valueOfAccessed = true; return {}; }, toString: function() { toStringAccessed = true; return '2'; } } }); array.length + ':' + valueOfAccessed + ':' + toStringAccessed;"
        ),
        Ok(Value::String("2:true:true".to_owned()))
    );
    assert_eq!(
        eval(
            "let array = []; let valueOfAccessed = false; let toStringAccessed = false; let caught = false; try { Object.defineProperty(array, 'length', { value: { valueOf: function() { valueOfAccessed = true; return {}; }, toString: function() { toStringAccessed = true; return {}; } } }); } catch (error) { caught = error instanceof TypeError; } caught + ':' + valueOfAccessed + ':' + toStringAccessed + ':' + array.length;"
        ),
        Ok(Value::String("true:true:true:0".to_owned()))
    );
    assert_eq!(
        eval("Object.defineProperties.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperties(object, false) === object && Object.keys(object).length === 0;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = { value: 1 }; Object.defineProperties(object, -12) === object && object.value === 1;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let object = { value: 1 }; Object.defineProperties(object, '') === object && object.value === 1;"
        ),
        Ok(Value::Boolean(true))
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
    assert_eq!(
        eval(
            "let object = {}; let descriptors = []; descriptors[0] = { value: 7, enumerable: true }; let result = Object.defineProperties(object, descriptors); (result === object) + ':' + object[0] + ':' + Object.keys(object)[0];"
        ),
        Ok(Value::String("true:7:0".to_owned()))
    );
    assert_eq!(
        eval(
            "let object = {}; let descriptors = []; Object.defineProperty(descriptors, 'prop', { get: function() { return { value: 8, enumerable: true }; }, enumerable: true }); Object.defineProperties(object, descriptors); object.prop;"
        ),
        Ok(Value::Number(8.0))
    );
    assert_eq!(eval("Object.create.length;"), Ok(Value::Number(2.0)));
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
            "let desc = {}; Object.defineProperty(desc, 'configurable', { get: function() { return true; } }); let object = Object.create({}, { own: desc }); let before = object.hasOwnProperty('own'); delete object.own; before + ':' + object.hasOwnProperty('own');"
        ),
        Ok(Value::String("true:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let desc = function() {}; desc.enumerable = true; desc.value = 9; let object = Object.create({}, { own: desc }); Object.keys(object)[0] + ':' + object.own;"
        ),
        Ok(Value::String("own:9".to_owned()))
    );
    assert_eq!(
        eval(
            "let object = {}; let desc = {}; Object.defineProperty(desc, 'value', { get: function() { return 11; } }); Reflect.defineProperty(object, 'own', desc); object.own;"
        ),
        Ok(Value::Number(11.0))
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
}
