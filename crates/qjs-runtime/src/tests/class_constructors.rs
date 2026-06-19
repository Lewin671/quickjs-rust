use crate::{Value, eval};

#[test]
fn default_constructor_creates_instance() {
    assert_eq!(
        eval("class C {} typeof new C();"),
        Ok(Value::String("object".to_owned().into()))
    );
}

#[test]
fn default_derived_constructor_arguments_do_not_shadow_outer_bindings() {
    assert_eq!(
        eval(
            "var args, that; \
             class Base { constructor() { that = this; args = arguments; } } \
             class Derived extends Base {} \
             new Derived(0, 1, 2); \
             args.length + ':' + (that instanceof Derived);"
        ),
        Ok(Value::String("3:true".to_owned().into()))
    );
}

#[test]
fn default_derived_constructor_forwards_arguments_without_spread_iteration() {
    assert_eq!(
        eval(
            "Array.prototype[Symbol.iterator] = function() { throw new Error('iterated'); }; \
             class Base { constructor(value) { this.value = value; } } \
             class Derived extends Base {} \
             new Derived(5).value;"
        ),
        Ok(Value::Number(5.0))
    );
}

#[test]
fn native_super_constructors_use_derived_new_target_prototype() {
    for (source, expected) in [
        (
            "class SubMap extends Map {} \
             var value = new SubMap(); \
             [value instanceof SubMap, value instanceof Map, Object.getPrototypeOf(value) === SubMap.prototype].join(':');",
            "true:true:true",
        ),
        (
            "class SubSet extends Set {} \
             var value = new SubSet(); \
             [value instanceof SubSet, value instanceof Set, Object.getPrototypeOf(value) === SubSet.prototype].join(':');",
            "true:true:true",
        ),
        (
            "class SubWeakMap extends WeakMap {} \
             var value = new SubWeakMap(); \
             [value instanceof SubWeakMap, value instanceof WeakMap, Object.getPrototypeOf(value) === SubWeakMap.prototype].join(':');",
            "true:true:true",
        ),
        (
            "class SubWeakSet extends WeakSet {} \
             var value = new SubWeakSet(); \
             [value instanceof SubWeakSet, value instanceof WeakSet, Object.getPrototypeOf(value) === SubWeakSet.prototype].join(':');",
            "true:true:true",
        ),
        (
            "class SubArrayBuffer extends ArrayBuffer {} \
             var value = new SubArrayBuffer(4); \
             var slice = value.slice(0, 1); \
             [value instanceof SubArrayBuffer, value instanceof ArrayBuffer, Object.getPrototypeOf(value) === SubArrayBuffer.prototype, value.byteLength, slice instanceof SubArrayBuffer, slice instanceof ArrayBuffer, slice.byteLength].join(':');",
            "true:true:true:4:true:true:1",
        ),
        (
            "class SubFunction extends Function {} \
             var value = new SubFunction('return 7;'); \
             [value instanceof SubFunction, value instanceof Function, Object.getPrototypeOf(value) === SubFunction.prototype, value()].join(':');",
            "true:true:true:7",
        ),
        (
            "var GeneratorFunction = Object.getPrototypeOf(function* () {}).constructor; \
             class SubGeneratorFunction extends GeneratorFunction {} \
             var value = new SubGeneratorFunction('yield 7;'); \
             [value instanceof SubGeneratorFunction, value instanceof GeneratorFunction, Object.getPrototypeOf(value) === SubGeneratorFunction.prototype, value().next().value].join(':');",
            "true:true:true:7",
        ),
        (
            "class SubSharedArrayBuffer extends SharedArrayBuffer {} \
             var value = new SubSharedArrayBuffer(); \
             [value instanceof SubSharedArrayBuffer, value instanceof SharedArrayBuffer, Object.getPrototypeOf(value) === SubSharedArrayBuffer.prototype, value.byteLength].join(':');",
            "true:true:true:0",
        ),
        (
            "class SubWeakRef extends WeakRef {} \
             var target = {}; \
             var value = new SubWeakRef(target); \
             [value instanceof SubWeakRef, value instanceof WeakRef, Object.getPrototypeOf(value) === SubWeakRef.prototype, value.deref() === target].join(':');",
            "true:true:true:true",
        ),
    ] {
        assert_eq!(eval(source), Ok(Value::String(expected.to_owned().into())));
    }
}

#[test]
fn array_buffer_constructors_read_bound_new_target_prototype_lazily() {
    assert_eq!(
        eval(
            "let proto = { marker: 'array-buffer' }; \
             let newTarget = (function() {}).bind(null); \
             Object.defineProperty(newTarget, 'prototype', { get() { return proto; } }); \
             Object.getPrototypeOf(Reflect.construct(ArrayBuffer, [8], newTarget)) === proto;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let proto = { marker: 'shared-array-buffer' }; \
             let newTarget = (function() {}).bind(null); \
             Object.defineProperty(newTarget, 'prototype', { get() { return proto; } }); \
             Object.getPrototypeOf(Reflect.construct(SharedArrayBuffer, [8], newTarget)) === proto;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn array_buffer_constructor_reads_new_target_prototype_before_large_allocation() {
    assert_eq!(
        eval(
            "let marker = {}; \
             let newTarget = (function() {}).bind(null); \
             Object.defineProperty(newTarget, 'prototype', { get() { throw marker; } }); \
             let caught = false; \
             try { Reflect.construct(ArrayBuffer, [7 * 1125899906842624], newTarget); } catch (e) { caught = e === marker; } \
             caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let marker = {}; \
             let newTarget = (function() {}).bind(null); \
             Object.defineProperty(newTarget, 'prototype', { get() { throw marker; } }); \
             let caught = false; \
             try { Reflect.construct(ArrayBuffer, [0, { maxByteLength: 7 * 1125899906842624 }], newTarget); } catch (e) { caught = e === marker; } \
             caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn proxy_constructor_has_no_class_heritage_prototype() {
    assert_eq!(
        eval(
            "var result; \
             try { class P extends Proxy {} result = 'ok'; } \
             catch (error) { result = error.name; } \
             [Proxy.hasOwnProperty('prototype'), result].join(':');"
        ),
        Ok(Value::String("false:TypeError".to_owned().into()))
    );
}

#[test]
fn null_extending_class_uses_function_prototype_as_constructor_parent() {
    assert_eq!(
        eval(
            "var reached = 0, after = 0, superError, returnError; \
             class C extends null { \
               constructor(mode) { \
                 if (mode === 'super') { \
                   reached += 1; \
                   try { super(); } catch (error) { superError = error.name; } \
                   after += 1; \
                   return {}; \
                 } \
               } \
             } \
             try { new C('return'); } catch (error) { returnError = error.name; } \
             new C('super'); \
             [Object.getPrototypeOf(C) === Function.prototype, superError, returnError, reached, after].join(':');"
        ),
        Ok(Value::String(
            "true:TypeError:ReferenceError:1:1".to_owned().into()
        ))
    );
}

#[test]
fn explicit_derived_constructor_must_call_super_before_returning() {
    assert!(
        eval("class B {} class C extends B { constructor() {} } new C();").is_err(),
        "derived constructor without super() must throw"
    );
}

#[test]
fn derived_super_property_requires_initialized_this() {
    assert!(
        eval(
            "class B {} \
             class C extends B { constructor() { super.m(); } } \
             new C();"
        )
        .is_err(),
        "super property access before super() must throw"
    );
}

#[test]
fn repeated_super_call_runs_parent_before_reference_error() {
    assert_eq!(
        eval(
            "var calls = 0; \
             class B { constructor() { calls += 1; } } \
             class C extends B { \
               constructor() { \
                 super(); \
                 try { super(); } catch (e) {} \
               } \
             } \
             new C(); calls;"
        ),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn derived_constructor_return_waits_for_lexical_super_in_cleanup() {
    assert_eq!(
        eval(
            "class C extends class {} { \
               constructor() { \
                 var f = () => super(); \
                 try { return; } finally { f(); } \
               } \
             } \
             typeof new C();"
        ),
        Ok(Value::String("object".to_owned().into()))
    );
    assert_eq!(
        eval(
            "class C extends class {} { \
               constructor() { \
                 var f = () => super(); \
                 try { throw null; } catch (e) { return; } finally { f(); } \
               } \
             } \
             typeof new C();"
        ),
        Ok(Value::String("object".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var iter = { \
               [Symbol.iterator]() { return this; }, \
               next() { return { done: false }; }, \
               return() { this.f(); return { done: true }; } \
             }; \
             class C extends class {} { \
               constructor() { \
                 iter.f = () => super(); \
                 for (var k of iter) { return; } \
               } \
             } \
             typeof new C();"
        ),
        Ok(Value::String("object".to_owned().into()))
    );
}
