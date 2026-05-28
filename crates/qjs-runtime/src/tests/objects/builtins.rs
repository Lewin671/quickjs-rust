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
        Ok(Value::Number(6.0))
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
    assert!(eval("Object.create(1);").is_err());
    assert!(eval("new Object.create({});").is_err());
    assert!(eval("new Object.prototype.hasOwnProperty('value');").is_err());
}
