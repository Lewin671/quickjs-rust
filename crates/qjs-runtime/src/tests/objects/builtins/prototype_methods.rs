use crate::{Value, eval};

#[test]
fn evaluates_object_prototype_methods() {
    assert_eq!(
        eval("Object.prototype.toString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Object.prototype.toString();"),
        Ok(Value::String("[object Object]".to_owned().into()))
    );
    assert_eq!(
        eval("({}).toString();"),
        Ok(Value::String("[object Object]".to_owned().into()))
    );
    assert_eq!(
        eval("Object.prototype.toString.call(new Date(0));"),
        Ok(Value::String("[object Date]".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = {}; object[Symbol.toStringTag] = 'custom'; Object.prototype.toString.call(object);"
        ),
        Ok(Value::String("[object custom]".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = {}; object[Symbol.toStringTag] = 86; Object.prototype.toString.call(object);"
        ),
        Ok(Value::String("[object Object]".to_owned().into()))
    );
    assert!(
        eval("let object = {}; Object.defineProperty(object, Symbol.toStringTag, { get: function() { throw new Error('tag'); } }); Object.prototype.toString.call(object);").is_err()
    );
    assert_eq!(
        eval(
            "let bigint = BigInt(0); \
             let boxed = Object(bigint); \
             Object.defineProperty(BigInt.prototype, Symbol.toStringTag, { value: undefined, configurable: true }); \
             Object.prototype.toString.call(bigint) + ':' + Object.prototype.toString.call(boxed);"
        ),
        Ok(Value::String(
            "[object Object]:[object Object]".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let toString = Object.prototype.toString; \
             let set = new Set(); \
             delete Set.prototype[Symbol.toStringTag]; \
             let iterator = set[Symbol.iterator](); \
             let prototype = Object.getPrototypeOf(iterator); \
             let initial = toString.call(iterator); \
             Object.defineProperty(prototype, Symbol.toStringTag, { configurable: true, get: function() { return new String('boxed'); } }); \
             let boxed = toString.call(iterator); \
             delete prototype[Symbol.toStringTag]; \
             initial + ':' + boxed + ':' + toString.call(iterator);"
        ),
        Ok(Value::String(
            "[object Set Iterator]:[object Object]:[object Iterator]"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(
        eval(
            "let generatorProxy = new Proxy(function* () {}, {}); \
             let generatorProxyProxy = new Proxy(generatorProxy, {}); \
             delete generatorProxy.constructor.prototype[Symbol.toStringTag]; \
             let asyncProxy = new Proxy(async function() {}, {}); \
             let asyncProxyProxy = new Proxy(asyncProxy, {}); \
             Object.defineProperty(asyncProxy.constructor.prototype, Symbol.toStringTag, { value: undefined, configurable: true }); \
             Object.prototype.toString.call(generatorProxy) + ':' + \
               Object.prototype.toString.call(generatorProxyProxy) + ':' + \
               Object.prototype.toString.call(asyncProxy) + ':' + \
               Object.prototype.toString.call(asyncProxyProxy);"
        ),
        Ok(Value::String(
            "[object Function]:[object Function]:[object Function]:[object Function]"
                .to_owned()
                .into()
        ))
    );
    assert_eq!(
        eval("Object.prototype.toLocaleString.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval("Object.prototype.toLocaleString();"),
        Ok(Value::String("[object Object]".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let object = { toString: function() { return 'custom'; } }; object.toLocaleString();"
        ),
        Ok(Value::String("custom".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let receiver = null; let object = {}; Object.defineProperty(object, 'toString', { get: function() { receiver = this; return function() { return receiver === object ? 'getter' : 'bad'; }; }, configurable: true }); object.toLocaleString();"
        ),
        Ok(Value::String("getter".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let receiver = null; Object.defineProperty(Boolean.prototype, 'toString', { get: function() { 'use strict'; receiver = this; return function() { return receiver === true ? 'primitive' : 'bad'; }; }, configurable: true }); Object.prototype.toLocaleString.call(true);"
        ),
        Ok(Value::String("primitive".to_owned().into()))
    );
    assert!(eval("Object.prototype.toLocaleString.call(null);").is_err());
    assert!(eval("Object.prototype.toLocaleString.call(undefined);").is_err());
    assert_eq!(
        eval("Object.prototype.valueOf.length;"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Object.prototype.valueOf, 'name'); d.value + ':' + d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("valueOf:false:false:true".to_owned().into()))
    );
    assert_eq!(
        eval("let object = { value: 1 }; object.valueOf() === object;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("typeof Object.prototype.valueOf.call(true);"),
        Ok(Value::String("object".to_owned().into()))
    );
    assert_eq!(
        eval("typeof Object.prototype.valueOf.call(false);"),
        Ok(Value::String("object".to_owned().into()))
    );
    assert_eq!(
        eval("Object.prototype.valueOf() === Object.prototype;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn object_prototype_has_immutable_prototype() {
    assert_eq!(
        eval("Reflect.setPrototypeOf(Object.prototype, Object.create(null));"),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval("Reflect.setPrototypeOf(Object.prototype, null);"),
        Ok(Value::Boolean(true))
    );
    assert!(
        eval("Object.setPrototypeOf(Object.prototype, Object.create(null));")
            .expect_err("Object.setPrototypeOf must reject Object.prototype prototype changes")
            .message
            .contains("Object.setPrototypeOf failed")
    );
    assert_eq!(
        eval(
            "let proto = {}; let object = {}; Object.setPrototypeOf(object, proto) === object && Object.getPrototypeOf(object) === proto;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let root = {}; \
             let intermediary = new Proxy(Object.create(root), {}); \
             let leaf = Object.create(intermediary); \
             root.__proto__ = leaf; \
             Object.getPrototypeOf(root) === leaf;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn object_prototype_proto_setter_uses_proxy_set_prototype_trap() {
    assert_eq!(
        eval(
            "function Sentinel() {} \
             let subject = new Proxy({}, { \
               setPrototypeOf: function() { throw new Sentinel(); } \
             }); \
             let caught = false; \
             try { subject.__proto__ = {}; } \
             catch (error) { caught = error instanceof Sentinel; } \
             caught + ':' + (Object.getPrototypeOf(subject) === Object.prototype);"
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn object_prototype_is_prototype_of_order_and_proxy() {
    assert_eq!(
        eval(
            "Object.prototype.isPrototypeOf.call(null, undefined) === false && \
             Object.prototype.isPrototypeOf.call(null, null) === false && \
             Object.prototype.isPrototypeOf.call(null, 10) === false && \
             Object.prototype.isPrototypeOf.call(undefined, true) === false && \
             Object.prototype.isPrototypeOf.call(undefined, 'str') === false && \
             Object.prototype.isPrototypeOf.call(undefined, Symbol('desc')) === false;"
        ),
        Ok(Value::Boolean(true))
    );
    assert!(
        eval("Object.prototype.isPrototypeOf.call(null, {});")
            .expect_err("object argument should require the null receiver to be object-coercible")
            .message
            .contains("isPrototypeOf")
    );
    assert_eq!(
        eval(
            "let proxyProto = []; \
             let log = ''; \
             let proxy = new Proxy({}, { getPrototypeOf: function() { log += 'g'; return proxyProto; } }); \
             proxyProto.isPrototypeOf(proxy) + ':' + log;"
        ),
        Ok(Value::String("true:g".to_owned().into()))
    );
}
