use crate::{Value, eval};

#[test]
fn indexed_write_routes_through_per_kind_conversion() {
    // Direct `ta[i] = v` writes apply the per-kind numeric conversion and
    // persist through the backing buffer (IntegerIndexedElementSet).
    assert_eq!(
        eval(
            "let a = new Uint8Array(3); a[0] = 257; a[1] = -1; a[2] = 3.9; \
             a[0] + ',' + a[1] + ',' + a[2] + '|' \
             + Array.prototype.join.call(new Uint8Array(a.buffer));"
        ),
        Ok(Value::String("1,255,3|1,255,3".to_owned().into()))
    );
    assert_eq!(
        eval("let c = new Uint8ClampedArray(1); c[0] = 300; c[0];"),
        Ok(Value::Number(255.0))
    );
    assert_eq!(
        eval("let b = new BigInt64Array(1); b[0] = 5n; typeof b[0] + ':' + b[0];"),
        Ok(Value::String("bigint:5".to_owned().into()))
    );
    assert_eq!(
        eval("let a = new Uint8Array([1]); Reflect.set(a, 0, 257) + ':' + a[0];"),
        Ok(Value::String("true:1".to_owned().into()))
    );
}

#[test]
fn immutable_typed_array_indexed_assignment_rejects_before_value_coercion() {
    assert_eq!(
        eval(
            "let a = new Uint8Array(new Uint8Array([3]).buffer.transferToImmutable()); \
             let sloppy = (a[0] = 9); \
             let strict = false; \
             try { (function () { 'use strict'; a[0] = 10; })(); } \
             catch (error) { strict = error instanceof TypeError; } \
             sloppy + ':' + strict + ':' + a[0];"
        ),
        Ok(Value::String("9:true:3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array(new ArrayBuffer(1).transferToImmutable()); \
             let calls = 0, poison = { valueOf() { calls++; throw 'coerced'; } }; \
             let integerResult = (a[0] = poison); \
             let key = '0', stringResult = (a[key] = poison); \
             (integerResult === poison) + ':' + (stringResult === poison) + ':' + calls + ':' + a[0];"
        ),
        Ok(Value::String("true:true:0:0".to_owned().into()))
    );
}

#[test]
fn reflect_set_immutable_typed_array_canonical_keys_return_false_without_coercion() {
    assert_eq!(
        eval(
            "let a = new Uint8Array(new ArrayBuffer(2).transferToImmutable()); \
             let calls = 0, poison = { valueOf() { calls++; throw 'coerced'; } }; \
             let results = ['0', '99', '1.5', '-0'].map(key => Reflect.set(a, key, poison)); \
             results.join(':') + ':' + calls + ':' + a[0];"
        ),
        Ok(Value::String(
            "false:false:false:false:0:0".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array(new ArrayBuffer(1).transferToImmutable()); \
             Reflect.set(a, '01', 7) + ':' + Reflect.set(a, 'foo', 8) + ':' + a['01'] + ':' + a.foo;"
        ),
        Ok(Value::String("true:true:7:8".to_owned().into()))
    );
}

#[test]
fn mutable_typed_array_invalid_index_sets_still_coerce_and_succeed() {
    assert_eq!(
        eval(
            "let a = new Uint8Array(1), calls = 0; \
             let value = { valueOf() { calls++; return 7; } }; \
             let out = Reflect.set(a, '99', value); \
             let fractional = Reflect.set(a, '1.5', value); \
             a.buffer.transfer(); \
             let detached = Reflect.set(a, '0', value); \
             out + ':' + fractional + ':' + detached + ':' + calls;"
        ),
        Ok(Value::String("true:true:true:3".to_owned().into()))
    );
}

#[test]
fn reflect_set_rejects_immutable_typed_array_receiver_before_coercion() {
    assert_eq!(
        eval(
            "let target = new Uint8Array([1]); \
             let receiver = new Uint8Array(new Uint8Array([2]).buffer.transferToImmutable()); \
             let calls = 0, poison = { valueOf() { calls++; throw 'coerced'; } }; \
             Reflect.set(target, '0', poison, receiver) + ':' + calls + ':' + target[0] + ':' + receiver[0];"
        ),
        Ok(Value::String("false:0:1:2".to_owned().into()))
    );
}

#[test]
fn immutable_typed_array_define_accepts_only_compatible_descriptors() {
    assert_eq!(
        eval(
            "let a = new Uint8Array(new Uint8Array([3]).buffer.transferToImmutable()); \
             let same = Object.defineProperty(a, '0', {}) === a; \
             let compatible = Reflect.defineProperty(a, '0', { value: 3, enumerable: true, writable: false, configurable: false }); \
             let changed = Reflect.defineProperty(a, '0', { value: 4 }); \
             let writable = Reflect.defineProperty(a, '0', { writable: true }); \
             let configurable = Reflect.defineProperty(a, '0', { configurable: true }); \
             let hidden = Reflect.defineProperty(a, '0', { enumerable: false }); \
             let descriptor = Object.getOwnPropertyDescriptor(a, '0'); \
             same + ':' + compatible + ':' + changed + ':' + writable + ':' + configurable + ':' + hidden + '|' \
             + descriptor.value + ':' + descriptor.enumerable + ':' + descriptor.writable + ':' + descriptor.configurable;"
        ),
        Ok(Value::String(
            "true:true:false:false:false:false|3:true:false:false"
                .to_owned()
                .into()
        ))
    );
    assert!(
        eval(
            "let a = new Uint8Array(new Uint8Array([3]).buffer.transferToImmutable()); Object.defineProperty(a, '0', { value: 4 });"
        )
        .is_err()
    );
}

#[test]
fn immutable_array_buffer_state_is_not_a_javascript_property() {
    assert_eq!(
        eval(
            "let buffer = new Uint8Array([3]).buffer.transferToImmutable(); \
             let key = '\\0ArrayBufferImmutable'; \
             let hidden = Object.getOwnPropertyNames(buffer).indexOf(key) === -1; \
             buffer[key] = false; delete buffer[key]; \
             Object.defineProperty(buffer, key, { value: false, configurable: true }); \
             let view = new Uint8Array(buffer); view[0] = 9; \
             hidden + ':' + buffer.immutable + ':' + view[0];"
        ),
        Ok(Value::String("true:true:3".to_owned().into()))
    );
}

#[test]
fn immutable_typed_arrays_freeze_directly_and_through_proxy() {
    assert_eq!(
        eval(
            "function immutableView(value) { \
                 return new Uint8Array(new Uint8Array([value]).buffer.transferToImmutable()); \
             } \
             let direct = immutableView(3); \
             let target = immutableView(4), proxy = new Proxy(target, {}); \
             let directResult = Object.freeze(direct) === direct; \
             let proxyResult = Object.freeze(proxy) === proxy; \
             [directResult, Object.isFrozen(direct), proxyResult, Object.isFrozen(proxy), Object.isFrozen(target), direct[0], proxy[0]].join(':');"
        ),
        Ok(Value::String(
            "true:true:true:true:true:3:4".to_owned().into()
        ))
    );
    assert!(eval("Object.freeze(new Uint8Array([1]));").is_err());
}

#[test]
fn mutable_typed_array_integrity_accounts_for_virtual_indices_after_failure() {
    assert_eq!(
        eval(
            "function state(value) { return [Object.isExtensible(value), Object.isSealed(value), Object.isFrozen(value)].join(':'); } \
             let sealed = new Uint8Array([1]), sealThrew = false; \
             try { Object.seal(sealed); } catch (_) { sealThrew = true; } \
             sealed[0] = 7; \
             let frozen = new Uint8Array([2]), freezeThrew = false; \
             try { Object.freeze(frozen); } catch (_) { freezeThrew = true; } \
             frozen[0] = 8; \
             let prevented = new Uint8Array([3]); Object.preventExtensions(prevented); \
             let sealTarget = new Uint8Array([4]), sealProxy = new Proxy(sealTarget, {}), proxySealThrew = false; \
             try { Object.seal(sealProxy); } catch (_) { proxySealThrew = true; } \
             let freezeTarget = new Uint8Array([5]), freezeProxy = new Proxy(freezeTarget, {}), proxyFreezeThrew = false; \
             try { Object.freeze(freezeProxy); } catch (_) { proxyFreezeThrew = true; } \
             [sealThrew, state(sealed), sealed[0], freezeThrew, state(frozen), frozen[0], state(prevented), \
              proxySealThrew, state(sealTarget), Object.isSealed(sealProxy), Object.isFrozen(sealProxy), \
              proxyFreezeThrew, state(freezeTarget), Object.isSealed(freezeProxy), Object.isFrozen(freezeProxy)].join('|');"
        ),
        Ok(Value::String(
            "true|false:false:false|7|true|false:false:false|8|false:false:false|true|false:false:false|false|false|true|false:false:false|false|false"
                .to_owned()
                .into()
        ))
    );
}

#[test]
fn typed_array_integrity_tracks_empty_and_detached_views() {
    assert_eq!(
        eval(
            "function state(value) { return [Object.isExtensible(value), Object.isSealed(value), Object.isFrozen(value)].join(':'); } \
             let emptySeal = new Uint8Array(0); Object.seal(emptySeal); \
             let emptyFreeze = new Uint8Array(0); Object.freeze(emptyFreeze); \
             let emptyPrevent = new Uint8Array(0); Object.preventExtensions(emptyPrevent); \
             let detachedSeal = new Uint8Array(2); __quickjsRustDetachArrayBuffer(detachedSeal.buffer); Object.seal(detachedSeal); \
             let detachedFreeze = new Uint8Array(2); __quickjsRustDetachArrayBuffer(detachedFreeze.buffer); Object.freeze(detachedFreeze); \
             let detachedPrevent = new Uint8Array(2); __quickjsRustDetachArrayBuffer(detachedPrevent.buffer); Object.preventExtensions(detachedPrevent); \
             [state(emptySeal), state(emptyFreeze), state(emptyPrevent), state(detachedSeal), state(detachedFreeze), state(detachedPrevent)].join('|');"
        ),
        Ok(Value::String(
            "false:true:true|false:true:true|false:true:true|false:true:true|false:true:true|false:true:true"
                .to_owned()
                .into()
        ))
    );
}

#[test]
fn variable_length_typed_arrays_reject_integrity_without_side_effects() {
    assert_eq!(
        eval(
            "function probe(make) { \
                 let objectTarget = make(), objectThrew = false; \
                 try { Object.preventExtensions(objectTarget); } catch (_) { objectThrew = true; } \
                 let reflectTarget = make(), reflectResult = Reflect.preventExtensions(reflectTarget); \
                 let sealTarget = make(), sealThrew = false; \
                 try { Object.seal(sealTarget); } catch (_) { sealThrew = true; } \
                 let freezeTarget = make(), freezeThrew = false; \
                 try { Object.freeze(freezeTarget); } catch (_) { freezeThrew = true; } \
                 return [objectThrew, Object.isExtensible(objectTarget), reflectResult, Object.isExtensible(reflectTarget), \
                         sealThrew, Object.isExtensible(sealTarget), Object.isSealed(sealTarget), Object.isFrozen(sealTarget), \
                         freezeThrew, Object.isExtensible(freezeTarget), Object.isSealed(freezeTarget), Object.isFrozen(freezeTarget)].join(':'); \
             } \
             function rabFixedZero() { let buffer = new ArrayBuffer(4, { maxByteLength: 8 }); return new Uint8Array(buffer, 0, 0); } \
             function rabTracking() { let buffer = new ArrayBuffer(4, { maxByteLength: 8 }); return new Uint8Array(buffer); } \
             function gsabFixedZero() { let buffer = new SharedArrayBuffer(0, { maxByteLength: 8 }); return new Uint8Array(buffer, 0, 0); } \
             function gsabTracking() { let buffer = new SharedArrayBuffer(0, { maxByteLength: 8 }); return new Uint8Array(buffer); } \
             [probe(rabFixedZero), probe(rabTracking), probe(gsabFixedZero), probe(gsabTracking)].join('|');"
        ),
        Ok(Value::String(
            "true:true:false:true:true:true:false:false:true:true:false:false|true:true:false:true:true:true:false:false:true:true:false:false|false:false:true:false:false:false:true:true:false:false:true:true|true:true:false:true:true:true:false:false:true:true:false:false"
                .to_owned()
                .into()
        ))
    );
}

#[test]
fn immutable_typed_arrays_seal_directly_and_through_proxy() {
    assert_eq!(
        eval(
            "function immutableView(value) { return new Uint8Array(new Uint8Array([value]).buffer.transferToImmutable()); } \
             let direct = immutableView(3), target = immutableView(4), proxy = new Proxy(target, {}), prevented = immutableView(5); \
             let directResult = Object.seal(direct) === direct, proxyResult = Object.seal(proxy) === proxy; \
             Object.preventExtensions(prevented); \
             [directResult, Object.isSealed(direct), Object.isFrozen(direct), proxyResult, Object.isSealed(proxy), Object.isFrozen(proxy), Object.isSealed(target), Object.isFrozen(target), Object.isSealed(prevented), Object.isFrozen(prevented)].join(':');"
        ),
        Ok(Value::String(
            "true:true:true:true:true:true:true:true:true:true"
                .to_owned()
                .into()
        ))
    );
}

#[test]
fn numeric_index_loop_writes_through_backing_buffer() {
    assert_eq!(
        eval(
            "let a = new Uint8Array(4); let calls = 0; \
             for (let i = 0; i < a.length; i++) { \
               a[i] = { valueOf() { calls++; return i + 256; } }; \
             } \
             calls + ':' + Array.prototype.join.call(new Uint8Array(a.buffer));"
        ),
        Ok(Value::String("4:0,1,2,3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let a = new Uint8Array(1); a[3] = { valueOf() { return 9; } }; \
             a[3] === undefined && a.length === 1;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn numeric_index_loops_observe_overridden_length_accessor() {
    assert_eq!(
        eval(
            "let proto = Object.getPrototypeOf(Uint8Array.prototype); \
             let descriptor = Object.getOwnPropertyDescriptor(proto, 'length'); \
             let calls = 0; \
             Object.defineProperty(proto, 'length', { get() { calls++; return 2; }, configurable: true }); \
             let seen = 0; \
             try { let a = new Uint8Array(4); for (let i = 0; i < a.length; i++) { seen++; } } \
             finally { Object.defineProperty(proto, 'length', descriptor); } \
             calls + ':' + seen;"
        ),
        Ok(Value::String("3:2".to_owned().into()))
    );
}

#[test]
fn reflect_set_typed_array_index_uses_receiver_define_semantics() {
    assert_eq!(
        eval(
            "let target = new Uint8Array([0]); let receiver = {}; \
             let value = { valueOf() { throw 'coerced'; } }; \
             Reflect.set(target, 0, value, receiver) + ':' + target[0] + ':' + (receiver[0] === value);"
        ),
        Ok(Value::String("true:0:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let target = new Uint8Array([0]); let receiver = new Uint8Array([1]); \
             Reflect.set(target, 0, 257, receiver) + ':' + target[0] + ':' + receiver[0];"
        ),
        Ok(Value::String("true:0:1".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let target = new Uint8Array([0, 0]); let receiver = new Uint8Array([1]); \
             let calls = 0; let value = { valueOf() { calls += 1; return 2; } }; \
             Reflect.set(target, 1, value, receiver) + ':' + target[1] + ':' \
             + receiver.hasOwnProperty(1) + ':' + calls;"
        ),
        Ok(Value::String("false:0:false:0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let target = new BigInt64Array([0n]); let receiver = new BigInt64Array([1n]); \
             Reflect.set(target, 0, Object(2n), receiver) + ':' + target[0] + ':' + receiver[0];"
        ),
        Ok(Value::String("true:0:2".to_owned().into()))
    );
}

#[test]
fn indexed_write_drops_out_of_range_and_canonical_indices() {
    // Out-of-range and non-integer canonical numeric indices never create a
    // property, but coercion side effects still run.
    assert_eq!(
        eval("let a = new Uint8Array(2); a[5] = 9; a[5] === undefined && a.length === 2;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("let a = new Uint8Array(2); a['1.5'] = 7; a['1.5'];"),
        Ok(Value::Undefined)
    );
    // ToNumber side effects fire even for an out-of-range index.
    assert_eq!(
        eval(
            "let log = []; let a = new Uint8Array(1); \
             a[3] = { valueOf() { log.push('x'); return 0; } }; log.join(',');"
        ),
        Ok(Value::String("x".to_owned().into()))
    );
}

#[test]
fn non_numeric_property_writes_still_work() {
    assert_eq!(
        eval("let a = new Uint8Array(1); a.foo = 'bar'; a.foo;"),
        Ok(Value::String("bar".to_owned().into()))
    );
}

#[test]
fn detached_typed_array_numeric_delete_succeeds() {
    assert_eq!(
        eval(
            "let a = new Uint8Array([1]); a.buffer.transfer(); \
             delete a[0] && delete a['-0'] && delete a['1.1'];"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let a = new BigInt64Array([1n]); a.buffer.transfer(); \
             Reflect.deleteProperty(a, 0) + ':' + Reflect.deleteProperty(a, '-0');"
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let a = new BigInt64Array([1n]); a.extra = 1; a.buffer.transfer(); \
             delete a.extra + ':' + a.hasOwnProperty('extra');"
        ),
        Ok(Value::String("true:false".to_owned().into()))
    );
}

#[test]
fn typed_array_numeric_delete_uses_integer_indexed_semantics() {
    assert_eq!(
        eval(
            "let a = new Uint8Array([3]); \
             Object.defineProperty(Uint8Array.prototype, '0', { \
               get() { throw 'prototype getter should not run'; }, configurable: true \
             }); \
             let valid = delete a[0]; \
             let out = delete a[1]; \
             let minusZero = delete a['-0']; \
             delete Uint8Array.prototype[0]; \
             valid + ':' + out + ':' + minusZero + ':' + a[0];"
        ),
        Ok(Value::String("false:true:true:3".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let a = new BigInt64Array([3n]); \
             let valid = Reflect.deleteProperty(a, '0'); \
             let out = Reflect.deleteProperty(a, '1'); \
             valid + ':' + out + ':' + a[0];"
        ),
        Ok(Value::String("false:true:3".to_owned().into()))
    );
}

#[test]
fn typed_array_prototype_chain_set_writes_receiver_property() {
    assert_eq!(
        eval(
            "let calls = 0; \
             let value = { valueOf() { calls += 1; return 7; } }; \
             Object.defineProperty(Uint8Array.prototype, '0', { \
               get() { throw 'getter should not run'; }, \
               set() { throw 'setter should not run'; }, \
               configurable: true \
             }); \
             let target = new Uint8Array([0]); \
             let receiver = Object.create(target); \
             receiver[0] = value; \
             let arrayReceiver = Object.setPrototypeOf([], target); \
             arrayReceiver[0] = value; \
             delete Uint8Array.prototype[0]; \
             target[0] + ':' + (receiver[0] === value) + ':' \
             + (arrayReceiver[0] === value) + ':' + arrayReceiver.length + ':' + calls;"
        ),
        Ok(Value::String("0:true:true:1:0".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let calls = 0; \
             let value = { valueOf() { calls += 1; return 7n; } }; \
             Object.defineProperty(BigInt64Array.prototype, '0', { \
               get() { throw 'getter should not run'; }, \
               set() { throw 'setter should not run'; }, \
               configurable: true \
             }); \
             let target = new BigInt64Array([0n]); \
             let receiver = Object.create(target); \
             receiver[0] = value; \
             let arrayReceiver = Object.setPrototypeOf([], target); \
             arrayReceiver[0] = value; \
             delete BigInt64Array.prototype[0]; \
             target[0] + ':' + (receiver[0] === value) + ':' \
             + (arrayReceiver[0] === value) + ':' + arrayReceiver.length + ':' + calls;"
        ),
        Ok(Value::String("0:true:true:1:0".to_owned().into()))
    );
}

#[test]
fn own_property_descriptor_uses_current_indexed_element_state() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(2, { maxByteLength: 4 }); \
             let a = new Uint8Array(b); a[1] = 7; \
             let before = Object.getOwnPropertyDescriptor(a, 1); \
             b.resize(3); a[2] = 9; \
             let grown = Object.getOwnPropertyDescriptor(a, 2); \
             b.transfer(); \
             let detached = Object.getOwnPropertyDescriptor(a, 1); \
             before.value + ':' + before.enumerable + ':' + before.writable + ':' \
             + before.configurable + '|' + grown.value + '|' + (detached === undefined);"
        ),
        Ok(Value::String("7:true:true:true|9|true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let a = new BigInt64Array([1n]); a.buffer.transfer(); \
             Object.getOwnPropertyDescriptor(a, 0) === undefined;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn indexed_reads_use_current_resizable_buffer_bounds() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 8 }); \
             let fixed = new Uint8Array(b, 0, 4); \
             let tracking = new Uint8Array(b, 1); \
             fixed.set([10, 20, 30, 40]); \
             b.resize(2); \
             let shrunk = (fixed[0] === undefined) + ':' + (fixed[2] === undefined) + ':' \
               + tracking.length + ':' + tracking[0] + ':' + (tracking[1] === undefined); \
             b.resize(5); \
             shrunk + '|' + fixed.length + ':' + fixed[2] + ':' \
               + tracking.length + ':' + tracking[3];"
        ),
        Ok(Value::String(
            "true:true:1:20:true|4:0:4:0".to_owned().into()
        ))
    );
}

#[test]
fn define_own_property_uses_integer_indexed_semantics() {
    assert_eq!(
        eval(
            "let a = new Uint8Array([1, 2]); \
             let ok = Reflect.defineProperty(a, '0', { \
               value: 257, configurable: true, enumerable: true, writable: true \
             }); \
             let rejectWritable = Reflect.defineProperty(a, '1', { \
               value: 9, configurable: true, enumerable: true, writable: false \
             }); \
             let rejectOutOfRange = Reflect.defineProperty(a, '2', { \
               value: 9, configurable: true, enumerable: true, writable: true \
             }); \
             let rejectFractional = Reflect.defineProperty(a, '0.5', { \
               value: 9, configurable: true, enumerable: true, writable: true \
             }); \
             ok + ':' + a[0] + ':' + rejectWritable + ':' + a[1] + ':' \
             + rejectOutOfRange + ':' + (a[2] === undefined) + ':' \
             + rejectFractional + ':' + (a['0.5'] === undefined);"
        ),
        Ok(Value::String(
            "true:1:false:2:false:true:false:true".to_owned().into()
        ))
    );
    assert_eq!(
        eval(
            "let a = new BigInt64Array([1n]); \
             Reflect.defineProperty(a, '0', { value: Object(2n) }) + ':' + a[0];"
        ),
        Ok(Value::String("true:2".to_owned().into()))
    );
}

#[test]
fn object_define_property_throws_for_rejected_typed_array_indices() {
    assert_eq!(
        eval(
            "let a = new Uint8Array([1]); let descriptor = Object.getOwnPropertyDescriptor(a, '0'); \
             let badIndex = false; let badAttrs = false; \
             try { Object.defineProperty(a, '-0', descriptor); } \
             catch (e) { badIndex = e instanceof TypeError; } \
             try { Object.defineProperty(a, '0', { configurable: false }); } \
             catch (e) { badAttrs = e instanceof TypeError; } \
             badIndex + ':' + badAttrs + ':' + a[0];"
        ),
        Ok(Value::String("true:true:1".to_owned().into()))
    );
}

#[test]
fn typed_array_define_property_coercion_order_matches_integer_indexed_set() {
    assert_eq!(
        eval(
            "let calls = 0; let a = new Uint8Array([1]); \
             let value = { valueOf() { calls += 1; return 258; } }; \
             let ok = Reflect.defineProperty(a, '0', { value }); \
             let rejected = Reflect.defineProperty(a, '1', { value }); \
             ok + ':' + rejected + ':' + calls + ':' + a[0] + ':' + (a[1] === undefined);"
        ),
        Ok(Value::String("true:false:1:2:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let a = new BigInt64Array([17n]); \
             let ok = Reflect.defineProperty(a, '0', { \
               value: { valueOf() { a.buffer.transfer(); return 42n; } } \
             }); \
             ok + ':' + (a[0] === undefined);"
        ),
        Ok(Value::String("true:true".to_owned().into()))
    );
}

#[test]
fn detached_typed_array_for_in_skips_stale_index_descriptors() {
    assert_eq!(
        eval(
            "let a = new Uint8Array(4); let before = 0; \
             for (let key in a) { before += 1; } \
             a.buffer.transfer(); let after = 0; \
             for (let key in a) { after += 1; } \
             before + ':' + after;"
        ),
        Ok(Value::String("4:0".to_owned().into()))
    );
}

#[test]
fn own_keys_track_resizable_typed_array_length() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 6 }); \
             let a = new Uint8Array(b, 1); \
             Reflect.ownKeys(a).join(',') + '|'; \
             b.resize(6); let grown = Reflect.ownKeys(a).join(','); \
             b.resize(3); let shrunk = Reflect.ownKeys(a).join(','); \
             b.resize(1); let boundary = Reflect.ownKeys(a).join(','); \
             grown + '|' + shrunk + '|' + boundary;"
        ),
        Ok(Value::String("0,1,2,3,4|0,1|".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 6 }); \
             let a = new Uint8Array(b, 1, 2); \
             let initial = Reflect.ownKeys(a).join(','); \
             b.resize(6); let grown = Reflect.ownKeys(a).join(','); \
             b.resize(2); let out = Reflect.ownKeys(a).join(','); \
             initial + '|' + grown + '|' + out;"
        ),
        Ok(Value::String("0,1|0,1|".to_owned().into()))
    );
}

#[test]
fn own_keys_keep_non_index_properties_after_dynamic_indices() {
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(4, { maxByteLength: 6 }); \
             let a = new Uint8Array(b); a.extra = 1; \
             b.resize(2); \
             Object.keys(a).join(',') + '|' + Object.getOwnPropertyNames(a).join(',');"
        ),
        Ok(Value::String("0,1,extra|0,1,extra".to_owned().into()))
    );
    assert_eq!(
        eval(
            "let b = new ArrayBuffer(2, { maxByteLength: 4 }); \
             let a = new Uint8Array(b); let s = Symbol('s'); a[s] = 1; \
             b.resize(3); let keys = Reflect.ownKeys(a); \
             keys.length + ':' + keys.slice(0, 3).join(',') + ':' + (keys[3] === s);"
        ),
        Ok(Value::String("4:0,1,2:true".to_owned().into()))
    );
}

#[test]
fn integer_index_read_fast_path_matches_spec() {
    // Integer-indexed reads are owned by the typed-array exotic [[Get]]:
    // in-range returns the element, out-of-range is undefined, and a
    // prototype getter at an integer key never fires.
    assert_eq!(
        eval("var ta = new Uint8Array(3); ta[1] = 7; ta[1];"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("var ta = new Uint8Array(3); typeof ta[5];"),
        Ok(Value::String("undefined".to_owned().into()))
    );
    assert_eq!(
        eval(
            "Object.defineProperty(Uint8Array.prototype, '0', { get() { return 999; }, configurable: true });
             var ta = new Uint8Array(1); ta[0] = 5; var r = ta[0]; delete Uint8Array.prototype[0]; r;"
        ),
        Ok(Value::Number(5.0))
    );
    // A non-canonical numeric-looking key is an ordinary property.
    assert_eq!(
        eval("var ta = new Uint8Array(2); ta['01'] = 'x'; ta['01'];"),
        Ok(Value::String("x".to_owned().into()))
    );
}

#[test]
fn indexed_elements_are_observable_without_materialized_properties() {
    assert_eq!(
        eval(
            "let a = new Uint8Array([7]); \
             a.hasOwnProperty('0') + ':' \
             + Object.prototype.propertyIsEnumerable.call(a, '0') + ':' \
             + Object.getOwnPropertyDescriptor(a, '0').value + ':' \
             + Reflect.ownKeys(a)[0];"
        ),
        Ok(Value::String("true:true:7:0".to_owned().into()))
    );
}
