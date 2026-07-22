use crate::{Value, eval};

#[test]
fn evaluates_reflect_prototype_builtins() {
    assert_eq!(
        eval("typeof Reflect;"),
        Ok(Value::String("object".to_owned().into()))
    );
    assert_eq!(
        eval("typeof Reflect.getPrototypeOf;"),
        Ok(Value::String("function".to_owned().into()))
    );
    assert_eq!(eval("Reflect.apply.length;"), Ok(Value::Number(3.0)));
    assert_eq!(eval("Reflect.construct.length;"), Ok(Value::Number(2.0)));
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
        eval(
            "function join(a, b) { return a + ':' + b + ':' + arguments.length; } Reflect.apply(join, null, { 0: 'x', 1: 'y', length: 2 });"
        ),
        Ok(Value::String("x:y:2".to_owned().into()))
    );
    assert_eq!(
        eval(
            "function count() { return arguments.length + ':' + arguments[0]; } let args = function() {}; Object.defineProperty(args, 'length', { get: function() { return 1; } }); Reflect.apply(count, null, args);"
        ),
        Ok(Value::String("1:undefined".to_owned().into()))
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
        eval(
            "function C(a, b) { this.sum = a + b; } let value = Reflect.construct(C, [2, 5]); value instanceof C && value.sum;"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "function C() { this.proto = Object.getPrototypeOf(this); } let value = Reflect.construct(C, [], Array); Object.getPrototypeOf(value) === Array.prototype && value.proto === Array.prototype;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("function C() { return { marker: 11 }; } Reflect.construct(C, []).marker;"),
        Ok(Value::Number(11.0))
    );
    assert_eq!(
        eval(
            "function C(a, b) { this.sum = a + b; } let args = { 0: 3, 1: 4, length: 2 }; Reflect.construct(C, args).sum;"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("Reflect.get({ value: 5 }, 'value');"),
        Ok(Value::Number(5.0))
    );
    assert_eq!(
        eval("Reflect.get(Object.create({ inherited: 7 }), 'inherited');"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let target = {}; let receiver = { value: 17 }; Object.defineProperty(target, 'x', { get: function() { return this.value; } }); Reflect.get(target, 'x', receiver);"
        ),
        Ok(Value::Number(17.0))
    );
    assert_eq!(
        eval(
            "let proto = {}; let receiver = { value: 19 }; Object.defineProperty(proto, 'x', { get: function() { return this.value; } }); Reflect.get(Object.create(proto), 'x', receiver);"
        ),
        Ok(Value::Number(19.0))
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
        eval("let symbol = Symbol(); let object = { [symbol]: 13 }; Reflect.get(object, symbol);"),
        Ok(Value::Number(13.0))
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
        eval(
            "let array = [1, 2, 3]; let hints = []; let length = {}; length[Symbol.toPrimitive] = function(hint) { hints.push(hint); Object.defineProperty(array, 'length', { writable: false }); return 0; }; Reflect.set(array, 'length', length) + ':' + hints.join() + ':' + array.length;"
        ),
        Ok(Value::String("false:number,number:3".to_owned().into()))
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
            "let target = {}; let seen = 0; Object.defineProperty(target, 'value', { set: function(value) { seen = value; } }); Reflect.set(target, 'value', 41) && seen;"
        ),
        Ok(Value::Number(41.0))
    );
    assert_eq!(
        eval(
            "let symbol = Symbol(); let object = {}; Reflect.set(object, symbol, 59) && object[symbol];"
        ),
        Ok(Value::Number(59.0))
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
        eval(
            "let log = []; let proto = new Proxy({}, { has: function(target, key) { log.push(key); return key === 'marker'; } }); let object = {}; Reflect.setPrototypeOf(object, proto); Reflect.has(object, 'marker') + ':' + log.join('|');"
        ),
        Ok(Value::String("true:marker".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let log = []; let proto = new Proxy({ marker: 1 }, { get: function(target, key, receiver) { log.push(key); return Reflect.get(target, key, receiver); } }); let array = []; Reflect.setPrototypeOf(array, proto); Reflect.getPrototypeOf(array).marker + ':' + log.join('|');"
        ),
        Ok(Value::String("1:marker".to_owned().into()))
    );
    assert_eq!(
        eval("Reflect.has(function f() {}, 'call');"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let symbol = Symbol(); let other = Symbol(); let object = { [symbol]: 1 }; Reflect.has(object, symbol) && !Reflect.has(object, other);"
        ),
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
        eval(
            "Object.getOwnPropertyNames(Reflect.getOwnPropertyDescriptor({ value: 1 }, 'value')).join(',');"
        ),
        Ok(Value::String(
            "value,writable,enumerable,configurable".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'value', { get: function() {}, configurable: true }); Object.getOwnPropertyNames(Reflect.getOwnPropertyDescriptor(object, 'value')).join(',');"
        ),
        Ok(Value::String(
            "get,set,enumerable,configurable".to_owned().into()
        ))
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
        Ok(Value::String("0:23".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let symbol = Symbol(); let object = {}; Reflect.defineProperty(object, symbol, { value: 17, enumerable: true, writable: true, configurable: true }) && object[symbol];"
        ),
        Ok(Value::Number(17.0))
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
            "let symbol = Symbol(); let object = { [symbol]: 19 }; Reflect.deleteProperty(object, symbol) && !object.hasOwnProperty(symbol) && object[symbol] === undefined;"
        ),
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
        Ok(Value::String("a,b".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = {}; Object.defineProperty(object, 'hidden', { value: 1 }); object.shown = 2; Reflect.ownKeys(object).join();"
        ),
        Ok(Value::String("hidden,shown".to_owned().into()))
    );
    assert_eq!(
        eval("Reflect.ownKeys([1, 2]).join();"),
        Ok(Value::String("0,1,length".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let a = Symbol('a'); let b = Symbol('b'); let object = {}; object[a] = 1; Object.defineProperty(object, b, { value: 2 }); let keys = Reflect.ownKeys(object); keys.length + ':' + (keys[0] === a) + ':' + (keys[1] === b);"
        ),
        Ok(Value::String("2:true:true".to_owned().into()))
    );
    assert!(eval("Reflect.apply(1, null, []);").is_err());
    assert!(eval("Reflect.apply(function f() {}, null, 1);").is_err());
    assert_eq!(
        eval(
            "let caught = false; try { Reflect.apply(function() {}, null, Symbol()); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert!(eval("Reflect.construct(1, []);").is_err());
    assert!(eval("Reflect.construct(function f() {}, 1);").is_err());
    assert!(eval("Reflect.construct(function f() {}, [], Reflect.apply);").is_err());
    assert!(eval("Reflect.getPrototypeOf(1);").is_err());
    assert!(eval("Reflect.getPrototypeOf(Symbol('target'));").is_err());
    assert!(eval("Reflect.defineProperty(1, 'value', { value: 1 });").is_err());
    assert!(eval("Reflect.deleteProperty(1, 'value');").is_err());
    assert!(eval("Reflect.get(1, 'value');").is_err());
    assert!(eval("Reflect.getOwnPropertyDescriptor(1, 'toString');").is_err());
    assert!(eval("Reflect.has(1, 'toString');").is_err());
    assert!(eval("Reflect.isExtensible(1);").is_err());
    assert!(eval("Reflect.ownKeys(1);").is_err());
    assert!(eval("Reflect.preventExtensions(1);").is_err());
    assert!(eval("Reflect.set(1, 'value', 1);").is_err());
    assert_eq!(
        eval(
            "let names = ['defineProperty', 'deleteProperty', 'get', 'getOwnPropertyDescriptor', 'has', 'isExtensible', 'ownKeys', 'preventExtensions', 'set']; names.every(function(name) { try { Reflect[name](Symbol('target'), 'key', {}); } catch (error) { return error instanceof TypeError; } return false; });"
        ),
        Ok(Value::Boolean(true))
    );
    assert!(eval("Reflect.setPrototypeOf(1, null);").is_err());
    assert!(eval("Reflect.setPrototypeOf({}, 1);").is_err());
    assert!(eval("Reflect.setPrototypeOf(Symbol('target'), null);").is_err());
    assert!(eval("Reflect.setPrototypeOf({}, Symbol('proto'));").is_err());
}

#[test]
fn reflect_set_honors_array_receiver_descriptors() {
    assert_eq!(
        eval(
            "let calls = 0; \
             const target = {}; \
             Object.defineProperty(target, '0', { value: 1, writable: true }); \
             const accessor = []; \
             Object.defineProperty(accessor, '0', { \
                 get: function() { return 3; }, \
                 set: function(value) { calls += value; }, \
                 configurable: true \
             }); \
             const accessorResult = Reflect.set(target, '0', 7, accessor); \
             const accessorDescriptor = Object.getOwnPropertyDescriptor(accessor, '0'); \
             const blocked = []; \
             Object.defineProperty(blocked, '0', { \
                 value: 4, writable: false, configurable: true \
             }); \
             const blockedResult = Reflect.set(target, '0', 8, blocked); \
             const blockedDescriptor = Object.getOwnPropertyDescriptor(blocked, '0'); \
             accessorResult + ':' + calls + ':' + accessor[0] + ':' \
                 + (typeof accessorDescriptor.set) + ':' + blockedResult + ':' \
                 + blocked[0] + ':' + blockedDescriptor.writable;"
        ),
        Ok(Value::String(
            "false:0:3:function:false:4:false".to_owned().into()
        ))
    );
}

#[test]
fn reflect_set_symbol_dispatches_proxy_receiver_internal_methods() {
    assert_eq!(
        eval(
            "const key = Symbol('key'); \
             const target = {}; \
             Object.defineProperty(target, key, { value: 1, writable: true }); \
             const backing = {}; \
             Object.defineProperty(backing, key, { \
                 value: 2, writable: true, configurable: true \
             }); \
             const log = []; \
             const receiver = new Proxy(backing, { \
                 getOwnPropertyDescriptor: function(object, property) { \
                     log.push('get:' + (property === key)); \
                     return Reflect.getOwnPropertyDescriptor(object, property); \
                 }, \
                 defineProperty: function(object, property, descriptor) { \
                     log.push('define:' + (property === key) + ':' \
                         + Object.keys(descriptor).join(',')); \
                     return Reflect.defineProperty(object, property, descriptor); \
                 } \
             }); \
             const success = Reflect.set(target, key, 7, receiver); \
             let getThrew = false; \
             try { \
                 Reflect.set(target, key, 8, new Proxy({}, { \
                     getOwnPropertyDescriptor: function() { throw 41; } \
                 })); \
             } catch (error) { getThrew = error === 41; } \
             let defineThrew = false; \
             try { \
                 Reflect.set(target, key, 9, new Proxy({}, { \
                     getOwnPropertyDescriptor: function() { return undefined; }, \
                     defineProperty: function() { throw 43; } \
                 })); \
             } catch (error) { defineThrew = error === 43; } \
             success + ':' + target[key] + ':' + backing[key] + ':' \
                 + log.join('|') + ':' + getThrew + ':' + defineThrew;"
        ),
        Ok(Value::String(
            "true:1:7:get:true|define:true:value:true:true"
                .to_owned()
                .into()
        ))
    );
}
