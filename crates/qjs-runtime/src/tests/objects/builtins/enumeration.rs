use crate::{Value, eval};

#[test]
fn evaluates_object_enumeration_builtins() {
    assert_eq!(eval("Object.keys.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Object.keys({ value: 1 })[0];"),
        Ok(Value::String("value".to_owned().into()))
    );
    assert_eq!(eval("Object.keys([1, 2]).length;"), Ok(Value::Number(2.0)));
    assert_eq!(
        eval(
            "let xs = [1, , 3, , 5]; Object.defineProperty(xs, 5, { value: 7, enumerable: false, configurable: true }); Object.defineProperty(xs, 10000, { value: 'x', enumerable: true, configurable: true }); Object.keys(xs).join('|');"
        ),
        Ok(Value::String("0|2|4|10000".to_owned().into()))
    );
    assert_eq!(
        eval("Object.keys(Object.create({ value: 1 })).length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(eval("Object.keys(Object).length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval("Object.keys(Object.prototype).length;"),
        Ok(Value::Number(0.0))
    );
    assert!(eval("Object.keys(null);").is_err());
    assert!(eval("Object.keys(undefined);").is_err());
    assert_eq!(eval("Object.values.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval("Object.values({ first: 1, second: 2 }).join();"),
        Ok(Value::String("1,2".to_owned().into()))
    );
    assert_eq!(
        eval("Object.values([1, 2]).join();"),
        Ok(Value::String("1,2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "Object.values(Object.create({ inherited: 1 }, { own: { value: 2, enumerable: true } }))[0];"
        ),
        Ok(Value::Number(2.0))
    );
    assert_eq!(
        eval("Object.values('ab').join();"),
        Ok(Value::String("a,b".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = { a: 'A', get b() { this.c = 'C'; return 'B'; } }; let values = Object.values(object); values.length + ':' + values[0] + ':' + values[1] + ':' + object.c;"
        ),
        Ok(Value::String("2:A:B:C".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let caught = false; try { Object.values({ get a() { throw new RangeError('x'); } }); } catch (error) { caught = error instanceof RangeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Object.values(0).length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval(
            "function fakeObject() { throw 'called'; } fakeObject.values = Object.values; let global = Function('return this;')(); global.Object = fakeObject; Object === fakeObject && Object.values(1).length === 0;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Object.entries.length;"), Ok(Value::Number(1.0)));
    assert_eq!(
        eval(
            "let entries = Object.entries({ first: 1, second: 2 }); entries[0][0] + ':' + entries[0][1] + '|' + entries[1][0] + ':' + entries[1][1];"
        ),
        Ok(Value::String("first:1|second:2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let entries = Object.entries([4, 5]); entries[0][0] + ':' + entries[0][1] + '|' + entries[1][0] + ':' + entries[1][1];"
        ),
        Ok(Value::String("0:4|1:5".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let entries = Object.entries(Object.create({ inherited: 1 }, { own: { value: 2, enumerable: true } })); entries.length + ':' + entries[0][0] + ':' + entries[0][1];"
        ),
        Ok(Value::String("1:own:2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let entries = Object.entries('ab'); entries[0][0] + ':' + entries[0][1] + '|' + entries[1][0] + ':' + entries[1][1];"
        ),
        Ok(Value::String("0:a|1:b".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = { a: 'A', get b() { return 'B'; } }; let entries = Object.entries(object); entries.length + ':' + entries[0][0] + ':' + entries[0][1] + ':' + entries[1][0] + ':' + entries[1][1];"
        ),
        Ok(Value::String("2:a:A:b:B".to_owned().into()))
    );
    assert_eq!(eval("Object.entries(0).length;"), Ok(Value::Number(0.0)));
    assert_eq!(
        eval(
            "let fn = () => {}; fn.a = 1; Object.defineProperty(fn, 'name', { enumerable: true }); Object.entries(fn).map(entry => entry[0]).join('|');"
        ),
        Ok(Value::String("name|a".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function fakeObject() { throw 'called'; } fakeObject.entries = Object.entries; let global = Function('return this;')(); global.Object = fakeObject; Object === fakeObject && Object.entries(1).length === 0;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames({ value: 1 })[0];"),
        Ok(Value::String("value".to_owned().into()))
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames([1, 2]).length;"),
        Ok(Value::Number(3.0))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'a', { value: 1, configurable: true }); object.b = 2; Object.defineProperty(object, 'a', { value: 3 }); Object.getOwnPropertyNames(object).join('|');"
        ),
        Ok(Value::String("a|b".to_owned().into()))
    );
    assert_eq!(
        eval("let array = []; array.a = 1; Object.getOwnPropertyNames(array).join('|');"),
        Ok(Value::String("length|a".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = {}; object.b = 1; object[2] = 2; object[1] = 3; object.a = 4; object['01'] = 5; Object.getOwnPropertyNames(object).join('|');"
        ),
        Ok(Value::String("1|2|b|a|01".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let str = new String('abc'); str[5] = 'de'; Object.getOwnPropertyNames(str).join('|');"
        ),
        Ok(Value::String("0|1|2|5|length".to_owned().into()))
    );
    assert_eq!(
        // constructor, hasOwnProperty, isPrototypeOf, propertyIsEnumerable,
        // toLocaleString, toString, valueOf, define/lookup accessors, plus the
        // `__proto__` accessor.
        eval("Object.getOwnPropertyNames(Object.prototype).length;"),
        Ok(Value::Number(12.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames(Object.prototype)[0];"),
        Ok(Value::String("constructor".to_owned().into()))
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames(Object(1)).length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Object.getOwnPropertyNames(Object(true)).length;"),
        Ok(Value::Number(0.0))
    );
    assert!(eval("Object.getOwnPropertyNames(null);").is_err());
    assert!(eval("Object.getOwnPropertyNames(undefined);").is_err());
    assert_eq!(
        eval("Object.getOwnPropertySymbols.length;"),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "let a = Symbol('a'); let b = Symbol('b'); let object = {}; Object.defineProperty(object, a, { value: 1, enumerable: true, configurable: true }); Object.defineProperty(object, b, { value: 2 }); let symbols = Object.getOwnPropertySymbols(object); symbols.length + ':' + (symbols[0] === a) + ':' + (symbols[1] === b) + ':' + Object.getOwnPropertyDescriptor(object, symbols[0]).value + ':' + Object.hasOwn(object, b) + ':' + Object.getOwnPropertyNames(object).length;"
        ),
        Ok(Value::String("2:true:true:1:true:0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let symbol = Symbol(); let object = {}; Object.defineProperty(object, symbol, { value: 1, configurable: true }); Object.defineProperty(object, symbol, { value: 2, configurable: true }); Object.getOwnPropertySymbols(object).length + ':' + Object.getOwnPropertyDescriptor(object, symbol).value;"
        ),
        Ok(Value::String("1:2".to_owned().into()))
    );
    assert!(eval("Object.getOwnPropertySymbols(null);").is_err());
    // getOwnPropertyNames/Symbols run a Proxy's full [[OwnPropertyKeys]] (with
    // its invariants over both key kinds) before filtering by key type.
    assert_eq!(
        eval(
            "let p = new Proxy({}, { ownKeys() { return ['b', 'a']; }, getOwnPropertyDescriptor() { return { value: 1, enumerable: true, configurable: true }; } }); \
             Object.getOwnPropertyNames(p).join(',');"
        ),
        Ok(Value::String("b,a".to_owned().into()))
    );
    // A symbol-key invariant violation throws even through getOwnPropertyNames.
    assert!(
        eval(
            "let s = Symbol(); Object.getOwnPropertyNames(new Proxy({}, { ownKeys() { return [s, s]; } }));"
        )
        .is_err()
    );
    // A string-key invariant violation throws even through getOwnPropertySymbols.
    assert!(
        eval("Object.getOwnPropertySymbols(new Proxy({}, { ownKeys() { return ['a', 'a']; } }));")
            .is_err()
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
        eval(
            "let sym = Symbol(); let object = {}; object[sym] = 1; let calls = ''; let byPrimitive = {}; byPrimitive[Symbol.toPrimitive] = function() { calls += 'p'; return sym; }; let byString = { toString: function() { calls += 's'; return sym; }, valueOf: function() { throw 'bad'; } }; let byValue = { toString: null, valueOf: function() { calls += 'v'; return sym; } }; Object.hasOwn(object, byPrimitive) + ':' + Object.hasOwn(object, byString) + ':' + Object.hasOwn(object, byValue) + ':' + calls;"
        ),
        Ok(Value::String("true:true:true:psv".to_owned().into()))
    );
}

#[test]
fn object_enumeration_builtins_observe_proxy_traps() {
    assert_eq!(
        eval(
            "let log = ''; let target = { a: 1, b: 2 }; let proxy = new Proxy(target, { ownKeys(t) { log += '|ownKeys'; return ['a', 'b']; }, getOwnPropertyDescriptor(t, k) { log += '|desc:' + k; return Object.getOwnPropertyDescriptor(t, k); }, get(t, k) { log += '|get:' + k; return t[k]; } }); Object.keys(proxy).join(',') + ';' + log;"
        ),
        Ok(Value::String(
            "a,b;|ownKeys|desc:a|desc:b".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let log = ''; let trapKeys = { get length() { log += '|length'; return 2; }, get 0() { log += '|key0'; return 'a'; }, get 1() { log += '|key1'; return 'b'; } }; let proxy = new Proxy({}, { ownKeys() { log += '|ownKeys'; return trapKeys; }, getOwnPropertyDescriptor(t, k) { log += '|desc:' + k; return { value: k, enumerable: k === 'a', configurable: true }; } }); Object.keys(proxy).join(',') + ';' + log;"
        ),
        Ok(Value::String(
            "a;|ownKeys|length|key0|key1|desc:a|desc:b"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(
        eval(
            "let log = ''; let target = { a: 1, b: 2 }; let proxy = new Proxy(target, { ownKeys(t) { log += '|ownKeys'; return ['a', 'b']; }, getOwnPropertyDescriptor(t, k) { log += '|desc:' + k; return Object.getOwnPropertyDescriptor(t, k); }, get(t, k) { log += '|get:' + k; return t[k]; } }); Object.values(proxy).join(',') + ';' + log;"
        ),
        Ok(Value::String(
            "1,2;|ownKeys|desc:a|get:a|desc:b|get:b".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let log = ''; let target = { a: 1, b: 2 }; let proxy = new Proxy(target, { ownKeys(t) { log += '|ownKeys'; return ['a', 'b']; }, getOwnPropertyDescriptor(t, k) { log += '|desc:' + k; return Object.getOwnPropertyDescriptor(t, k); }, get(t, k) { log += '|get:' + k; return t[k]; } }); let entries = Object.entries(proxy); entries[0][0] + ':' + entries[0][1] + ',' + entries[1][0] + ':' + entries[1][1] + ';' + log;"
        ),
        Ok(Value::String(
            "a:1,b:2;|ownKeys|desc:a|get:a|desc:b|get:b"
                .to_owned()
                .into()
        ))
    );
}
