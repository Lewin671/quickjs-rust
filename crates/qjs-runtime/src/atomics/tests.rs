use crate::{Value, eval};

#[test]
fn atomics_object_surface_and_lock_free() {
    assert_eq!(
        eval(
            "Object.prototype.toString.call(Atomics) + ':' + \
             Atomics.add.length + ':' + Atomics.compareExchange.length + ':' + \
             Atomics.isLockFree(1) + ':' + Atomics.isLockFree(3);"
        ),
        Ok(Value::String("[object Atomics]:3:4:true:false".to_owned()))
    );
    assert_eq!(
        eval(
            "try { new Atomics.add(new Int32Array(1), 0, 1); false; } \
             catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn atomics_read_modify_write_numeric_views() {
    assert_eq!(
        eval(
            "let a = new Int32Array(new SharedArrayBuffer(16)); \
             a[2] = 6; \
             [Atomics.add(a, 2, 4), Atomics.load(a, 2), Atomics.sub(a, 2, 3), \
              Atomics.and(a, 2, 6), Atomics.or(a, 2, 8), Atomics.xor(a, 2, 3), \
              Atomics.exchange(a, 2, -1), Atomics.store(a, 2, Math.PI), a[2]].join(',');"
        ),
        Ok(Value::String("6,10,10,7,6,14,13,3,3".to_owned()))
    );
}

#[test]
fn atomics_store_normalizes_negative_zero_return_value() {
    assert_eq!(
        eval(
            "let a = new Int32Array(new SharedArrayBuffer(4)); \
             let stored = Atomics.store(a, 0, -0); \
             Object.is(stored, 0) + ':' + Object.is(stored, -0) + ':' + Object.is(a[0], 0);"
        ),
        Ok(Value::String("true:false:true".to_owned()))
    );
}

#[test]
fn atomics_pause_surface() {
    assert_eq!(
        eval("Atomics.pause() === undefined && Atomics.pause(42) === undefined;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(eval("Atomics.pause.length;"), Ok(Value::Number(0.0)));
    assert!(eval("new Atomics.pause();").is_err());
    assert!(eval("Atomics.pause(42.5);").is_err());
    assert!(eval("Atomics.pause('42');").is_err());
}

#[test]
fn atomics_notify_validates_and_returns_zero_without_waiters() {
    assert_eq!(
        eval(
            "let i32 = new Int32Array(new SharedArrayBuffer(8)); \
             let i64 = new BigInt64Array(new SharedArrayBuffer(16)); \
             [Atomics.notify.length, Atomics.notify(i32, 0), Atomics.notify(i64, 0, 1)].join(':');"
        ),
        Ok(Value::String("3:0:0".to_owned()))
    );
    assert_eq!(
        eval("let i32 = new Int32Array(new ArrayBuffer(8)); Atomics.notify(i32, 0, '33');"),
        Ok(Value::Number(0.0))
    );
    assert_eq!(
        eval(
            "let index = { valueOf() { throw new Error('index'); } }; \
             try { Atomics.notify(new Uint8Array(new SharedArrayBuffer(8)), index, 0); false; } \
             catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let count = { valueOf() { throw new Error('count'); } }; \
             try { Atomics.notify(new Int32Array(new SharedArrayBuffer(8)), 9, count); false; } \
             catch (e) { e instanceof RangeError; }"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn atomics_read_modify_write_bigint_views() {
    assert_eq!(
        eval(
            "let a = new BigInt64Array(new SharedArrayBuffer(64)); \
             a[3] = 0x33333333n; \
             let old = Atomics.xor(a, 3, 0x55555555n); \
             let after = Atomics.load(a, 3); \
             let stored = Atomics.store(a, 3, -5n); \
             [old, after, stored, a[3]].join(',');"
        ),
        Ok(Value::String("858993459,1717986918,-5,-5".to_owned()))
    );
}

#[test]
fn atomics_compare_exchange_updates_only_on_match() {
    assert_eq!(
        eval(
            "let a = new Uint8Array(new ArrayBuffer(4)); \
             a[0] = 7; \
             [Atomics.compareExchange(a, 0, 1, 9), a[0], \
              Atomics.compareExchange(a, 0, 7, 9), a[0]].join(',');"
        ),
        Ok(Value::String("7,7,7,9".to_owned()))
    );
}

#[test]
fn atomics_validation_order_and_receivers() {
    assert_eq!(
        eval(
            "let index = { valueOf() { throw new Error('index'); } }; \
             try { Atomics.add(new Float32Array(new SharedArrayBuffer(8)), index, 0); false; } \
             catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let value = { valueOf() { throw new Error('value'); } }; \
             try { Atomics.xor(new BigInt64Array(new SharedArrayBuffer(16)), 99, value); false; } \
             catch (e) { e instanceof RangeError; }"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let calls = ''; \
             let a = new Int32Array(new ArrayBuffer(4).transferToImmutable()); \
             let index = { valueOf() { calls += 'index'; return 0; } }; \
             let value = { valueOf() { calls += 'value'; return 1; } }; \
             try { Atomics.store(a, index, value); false; } \
             catch (e) { calls + ':' + (e instanceof TypeError); }"
        ),
        Ok(Value::String(":true".to_owned()))
    );
}
