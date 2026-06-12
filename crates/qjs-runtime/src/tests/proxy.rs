use crate::{Value, eval};

#[test]
fn evaluates_proxy_constructor_and_basic_traps() {
    assert_eq!(
        eval("typeof Proxy + ':' + Proxy.length;"),
        Ok(Value::String("function:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let p = new Proxy({ value: 1 }, { get: function(target, key) { return key === 'value' ? 7 : target[key]; } }); p.value;"
        ),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval(
            "let p = new Proxy({}, { has: function(target, key) { return key === 'present'; } }); ('present' in p) + ':' + ('missing' in p);"
        ),
        Ok(Value::String("true:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let deleted = ''; let p = new Proxy({ value: 1 }, { deleteProperty: function(target, key) { deleted = key; return true; } }); Reflect.deleteProperty(p, 'value'); deleted;"
        ),
        Ok(Value::String("value".to_owned()))
    );
}

#[test]
fn evaluates_proxy_revocable_and_revoked_operations() {
    assert_eq!(
        eval(
            "let r = Proxy.revocable({ value: 1 }, {}); r.proxy.value + ':' + typeof r.revoke + ':' + Proxy.revocable.length;"
        ),
        Ok(Value::String("1:function:2".to_owned()))
    );
    assert_eq!(
        eval(
            "let r = Proxy.revocable({ value: 1 }, {}); r.revoke(); r.revoke(); let caught = false; try { r.proxy.value; } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let r = Proxy.revocable([], {}); r.revoke(); let caught = false; try { [].concat(r.proxy); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_proxy_apply_trap() {
    assert_eq!(
        eval(
            "let p = new Proxy(function () {}, { apply: function(target, thisArg, args) { return args[0] + args[1]; } }); p(3, 4);"
        ),
        Ok(Value::Number(7.0))
    );
    // Absent apply trap forwards to the callable target.
    assert_eq!(
        eval("let p = new Proxy(function (a, b) { return a * b; }, {}); p(3, 4);"),
        Ok(Value::Number(12.0))
    );
    // A callable proxy reports `typeof` as function.
    assert_eq!(
        eval("typeof new Proxy(function () {}, {});"),
        Ok(Value::String("function".to_owned()))
    );
    assert_eq!(
        eval("typeof new Proxy({}, {});"),
        Ok(Value::String("object".to_owned()))
    );
    // Reflect.apply routes through the apply trap.
    assert_eq!(
        eval(
            "let p = new Proxy(function () {}, { apply: function(t, thisArg, args) { return args.length; } }); Reflect.apply(p, null, [1, 2, 3]);"
        ),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn evaluates_proxy_construct_trap() {
    assert_eq!(
        eval(
            "let p = new Proxy(function () {}, { construct: function(target, args) { return { sum: args[0] + args[1] }; } }); (new p(3, 4)).sum;"
        ),
        Ok(Value::Number(7.0))
    );
    // Absent construct trap forwards to the target constructor.
    assert_eq!(
        eval("function Point(x) { this.x = x; } let p = new Proxy(Point, {}); (new p(9)).x;"),
        Ok(Value::Number(9.0))
    );
    // A construct trap returning a non-object is a TypeError.
    assert_eq!(
        eval(
            "let p = new Proxy(function () {}, { construct: function() { return 5; } }); let caught = false; try { new p(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // Constructing a non-constructor proxy target is a TypeError.
    assert_eq!(
        eval(
            "let p = new Proxy({}, {}); let caught = false; try { new p(); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // Reflect.construct routes through the construct trap with newTarget.
    assert_eq!(
        eval(
            "let nt; let p = new Proxy(function () {}, { construct: function(t, args, newTarget) { nt = newTarget; return {}; } }); Reflect.construct(p, [], Array); nt === Array;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_proxy_traps_used_by_array_operations() {
    assert_eq!(
        eval(
            "let caught = false; let p = new Proxy({ length: 1 }, { has: function() { throw new TypeError('has'); } }); try { Array.prototype.copyWithin.call(p, 0, 0); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let caught = false; let p = new Proxy({ 42: true, length: 43 }, { deleteProperty: function(target, key) { if (key === '42') { throw new TypeError('delete'); } return true; } }); try { Array.prototype.copyWithin.call(p, 42, 0); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let p = new Proxy([], { get: function(target, key) { if (key === 'length') { return Number.MAX_SAFE_INTEGER; } return target[key]; } }); let caught = false; try { [].concat(1, p); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_proxy_define_property_trap() {
    // The trap receives the key and a descriptor object built from the request.
    // The target carries the property so the non-configurable invariant holds.
    assert_eq!(
        eval(
            "let t = {}; Object.defineProperty(t, 'a', { value: 0, writable: true, configurable: false }); let seen; let p = new Proxy(t, { defineProperty: function(target, key, desc) { seen = key + ':' + desc.value + ':' + desc.configurable; return true; } }); Object.defineProperty(p, 'a', { value: 1, configurable: false }); seen;"
        ),
        Ok(Value::String("a:1:false".to_owned()))
    );
    // Defining a property on a non-extensible target through a trap that does
    // not actually add it violates the invariant.
    assert_eq!(
        eval(
            "let t = Object.preventExtensions({}); let p = new Proxy(t, { defineProperty: function() { return true; } }); let caught = false; try { Object.defineProperty(p, 'a', { value: 1 }); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // Reporting a non-configurable definition absent on the target is a
    // TypeError.
    assert_eq!(
        eval(
            "let p = new Proxy({}, { defineProperty: function() { return true; } }); let caught = false; try { Object.defineProperty(p, 'a', { value: 1, configurable: false }); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // A falsy trap result fails the Object.defineProperty.
    assert_eq!(
        eval(
            "let p = new Proxy({}, { defineProperty: function() { return false; } }); let caught = false; try { Object.defineProperty(p, 'a', { value: 1 }); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_proxy_get_own_property_descriptor_trap() {
    // The trap value is reflected in the returned descriptor.
    assert_eq!(
        eval(
            "let p = new Proxy({}, { getOwnPropertyDescriptor: function() { return { value: 9, configurable: true, enumerable: true, writable: true }; } }); Object.getOwnPropertyDescriptor(p, 'a').value;"
        ),
        Ok(Value::Number(9.0))
    );
    // Returning undefined for a non-configurable target property is rejected.
    assert_eq!(
        eval(
            "let t = {}; Object.defineProperty(t, 'a', { value: 1, configurable: false }); let p = new Proxy(t, { getOwnPropertyDescriptor: function() { return undefined; } }); let caught = false; try { Object.getOwnPropertyDescriptor(p, 'a'); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // A non-object, non-undefined trap result is a TypeError.
    assert_eq!(
        eval(
            "let p = new Proxy({}, { getOwnPropertyDescriptor: function() { return 5; } }); let caught = false; try { Object.getOwnPropertyDescriptor(p, 'a'); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // Reporting a property as non-configurable while the target lacks it fails.
    assert_eq!(
        eval(
            "let p = new Proxy({}, { getOwnPropertyDescriptor: function() { return { value: 1, configurable: false }; } }); let caught = false; try { Object.getOwnPropertyDescriptor(p, 'a'); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_proxy_extensibility_traps() {
    // isExtensible trap result must agree with the target.
    assert_eq!(
        eval(
            "let p = new Proxy({}, { isExtensible: function(target) { return Reflect.isExtensible(target); } }); Object.isExtensible(p);"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let p = new Proxy({}, { isExtensible: function() { return false; } }); let caught = false; try { Object.isExtensible(p); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // preventExtensions trap must actually leave the target non-extensible.
    assert_eq!(
        eval(
            "let count = 0; let p = new Proxy({}, { preventExtensions: function(target) { count++; Object.preventExtensions(target); return true; } }); Object.preventExtensions(p); count + ':' + Object.isExtensible(p);"
        ),
        Ok(Value::String("1:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let p = new Proxy({}, { preventExtensions: function() { return true; } }); let caught = false; try { Object.preventExtensions(p); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}
