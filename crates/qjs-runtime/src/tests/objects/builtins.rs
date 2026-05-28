use crate::{Value, eval};

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
        eval("Object.prototype.toLocaleString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Object.prototype.toLocaleString();"),
        Ok(Value::String("[object Object]".to_owned()))
    );
    assert_eq!(
        eval(
            "let object = { toString: function() { return 'custom'; } }; object.toLocaleString();"
        ),
        Ok(Value::String("custom".to_owned()))
    );
    assert!(eval("Object.prototype.toLocaleString.call(null);").is_err());
    assert!(eval("Object.prototype.toLocaleString.call(undefined);").is_err());
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
    assert_eq!(eval("Object.values.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Object.values({ first: 1, second: 2 }).join();"),
        Ok(Value::String("1,2".to_owned()))
    );
    assert_eq!(
        eval("Object.values([1, 2]).join();"),
        Ok(Value::String("1,2".to_owned()))
    );
    assert_eq!(
        eval(
            "Object.values(Object.create({ inherited: 1 }, { own: { value: 2, enumerable: true } }))[0];"
        ),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("Object.values('ab').join();"),
        Ok(Value::String("a,b".to_owned()))
    );
    assert_eq!(eval("Object.values(0).length;"), Ok(Value::Number(0.0)));
    assert_eq!(eval("Object.entries.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval(
            "let entries = Object.entries({ first: 1, second: 2 }); entries[0][0] + ':' + entries[0][1] + '|' + entries[1][0] + ':' + entries[1][1];"
        ),
        Ok(Value::String("first:1|second:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let entries = Object.entries([4, 5]); entries[0][0] + ':' + entries[0][1] + '|' + entries[1][0] + ':' + entries[1][1];"
        ),
        Ok(Value::String("0:4|1:5".to_owned()))
    );
    assert_eq!(
        eval(
            "let entries = Object.entries(Object.create({ inherited: 1 }, { own: { value: 2, enumerable: true } })); entries.length + ':' + entries[0][0] + ':' + entries[0][1];"
        ),
        Ok(Value::String("1:own:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let entries = Object.entries('ab'); entries[0][0] + ':' + entries[0][1] + '|' + entries[1][0] + ':' + entries[1][1];"
        ),
        Ok(Value::String("0:a|1:b".to_owned()))
    );
    assert_eq!(eval("Object.entries(0).length;"), Ok(Value::Number(0.0)));
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
        Ok(Value::Number(7.0))
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
    assert_eq!(eval("Object.is.length;"), Ok(Value::Number(2.0)));
    assert_eq!(eval("Object.is(NaN, NaN);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Object.is(+0, -0);"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Object.is(-0, -0);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Object.is(1, 1);"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Object.is(1, '1');"), Ok(Value::Boolean(false)));
    assert_eq!(
        eval("let object = {}; Object.is(object, object);"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Object.is({}, {});"), Ok(Value::Boolean(false)));
    assert_eq!(eval("Object.is();"), Ok(Value::Boolean(true)));
    assert_eq!(eval("Object.is(0);"), Ok(Value::Boolean(false)));
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
            "let descriptors = Object.getOwnPropertyDescriptors('ab'); descriptors.length.value + ':' + descriptors[0].value + ':' + descriptors[0].writable + ':' + descriptors[0].configurable;"
        ),
        Ok(Value::String("2:a:false:false".to_owned()))
    );
    assert_eq!(
        eval("Object.keys(Object.getOwnPropertyDescriptors(0)).length;"),
        Ok(Value::Number(0.0))
    );
    assert!(eval("Object.getOwnPropertyDescriptors(null);").is_err());
    assert!(eval("Object.getOwnPropertyDescriptors(undefined);").is_err());
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
    assert_eq!(
        eval("Object.setPrototypeOf.length;"),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("typeof Object.setPrototypeOf;"),
        Ok(Value::String("function".to_owned()))
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
    assert!(eval("Object.setPrototypeOf(null, null);").is_err());
    assert!(eval("Object.setPrototypeOf(undefined, null);").is_err());
    assert!(eval("Object.setPrototypeOf({}, 1);").is_err());
    assert!(eval("let object = {}; Object.preventExtensions(object); Object.setPrototypeOf(object, null);").is_err());
    assert!(
        eval("let parent = {}; let child = Object.create(parent); Object.setPrototypeOf(parent, child);").is_err()
    );
    assert!(eval("Object.create(1);").is_err());
    assert!(eval("new Object.create({});").is_err());
    assert!(eval("new Object.prototype.hasOwnProperty('value');").is_err());
}
