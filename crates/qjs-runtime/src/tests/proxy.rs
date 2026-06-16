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
    // Absent traps forward to a Proxy target's own internal methods.
    assert_eq!(
        eval(
            "let calls = 0; \
             let target = new Proxy({}, { isExtensible() { calls++; return false; }, preventExtensions(t) { Object.preventExtensions(t); return true; } }); \
             Object.preventExtensions(target); \
             let p = new Proxy(target, {}); \
             Object.isExtensible(p) + ':' + (calls > 0);"
        ),
        Ok(Value::String("false:true".to_owned()))
    );
    // deleteProperty forwards through a Proxy target: a configurable property is
    // dropped, a non-configurable one reports false.
    assert_eq!(
        eval(
            "let t = {}; Object.defineProperty(t, 'a', { value: 1, configurable: true }); \
             let p = new Proxy(new Proxy(t, {}), {}); (delete p.a) + ':' + ('a' in t);"
        ),
        Ok(Value::String("true:false".to_owned()))
    );
    assert_eq!(
        eval(
            "let t = {}; Object.defineProperty(t, 'b', { value: 1, configurable: false }); \
             let p = new Proxy(new Proxy(t, {}), {}); Reflect.deleteProperty(p, 'b') + ':' + ('b' in t);"
        ),
        Ok(Value::String("false:true".to_owned()))
    );
}

#[test]
fn evaluates_proxy_prototype_traps() {
    // getPrototypeOf trap result is returned for Object.getPrototypeOf.
    assert_eq!(
        eval(
            "let proto = { tag: 1 }; let p = new Proxy({}, { getPrototypeOf: function() { return proto; } }); Object.getPrototypeOf(p) === proto;"
        ),
        Ok(Value::Boolean(true))
    );
    // A non-object, non-null trap result is a TypeError.
    assert_eq!(
        eval(
            "let p = new Proxy({}, { getPrototypeOf: function() { return 5; } }); let caught = false; try { Object.getPrototypeOf(p); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // getPrototypeOf on a non-extensible target must report the real prototype.
    assert_eq!(
        eval(
            "let t = Object.preventExtensions({}); let p = new Proxy(t, { getPrototypeOf: function() { return { x: 1 }; } }); let caught = false; try { Object.getPrototypeOf(p); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // setPrototypeOf trap success is reflected through Reflect.setPrototypeOf.
    assert_eq!(
        eval(
            "let seen; let p = new Proxy({}, { setPrototypeOf: function(target, proto) { seen = proto; return true; } }); let r = Reflect.setPrototypeOf(p, null); r + ':' + (seen === null);"
        ),
        Ok(Value::String("true:true".to_owned()))
    );
    // Changing the prototype of a non-extensible target is a TypeError.
    assert_eq!(
        eval(
            "let t = Object.preventExtensions({}); let p = new Proxy(t, { setPrototypeOf: function() { return true; } }); let caught = false; try { Object.setPrototypeOf(p, {}); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // An absent setPrototypeOf trap forwards to the target; a Proxy target runs
    // its own trap.
    assert_eq!(
        eval(
            "let seen; let target = new Proxy({}, { setPrototypeOf(t, v) { seen = v; return true; } }); \
             let proto = {}; Object.setPrototypeOf(new Proxy(target, {}), proto); seen === proto;"
        ),
        Ok(Value::Boolean(true))
    );
    // The non-extensible invariant runs the target's own [[IsExtensible]]: a
    // proxy target whose isExtensible trap throws propagates that completion.
    assert!(
        eval(
            "let target = new Proxy({}, { isExtensible() { throw new TypeError('x'); } }); \
             let p = new Proxy(target, { setPrototypeOf() { return true; } }); Object.setPrototypeOf(p, {});"
        )
        .is_err()
    );
    // getPrototypeOf with an absent trap forwards through a Proxy target's trap.
    assert_eq!(
        eval(
            "let proto = { tag: 7 }; let target = new Proxy({}, { getPrototypeOf() { return proto; } }); \
             Object.getPrototypeOf(new Proxy(target, {})) === proto;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn evaluates_proxy_own_keys_trap() {
    // The trap result order is preserved through Reflect.ownKeys.
    assert_eq!(
        eval(
            "let p = new Proxy({}, { ownKeys: function() { return ['b', 'a']; } }); Reflect.ownKeys(p).join(',');"
        ),
        Ok(Value::String("b,a".to_owned()))
    );
    // Duplicate keys in the trap result are a TypeError.
    assert_eq!(
        eval(
            "let p = new Proxy({}, { ownKeys: function() { return ['a', 'a']; } }); let caught = false; try { Reflect.ownKeys(p); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // Non-string/symbol elements are a TypeError.
    assert_eq!(
        eval(
            "let p = new Proxy({}, { ownKeys: function() { return [1]; } }); let caught = false; try { Reflect.ownKeys(p); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // Omitting a non-configurable target key is a TypeError.
    assert_eq!(
        eval(
            "let t = {}; Object.defineProperty(t, 'a', { value: 1, configurable: false }); let p = new Proxy(t, { ownKeys: function() { return []; } }); let caught = false; try { Reflect.ownKeys(p); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
    // A non-extensible target requires its keys exactly.
    assert_eq!(
        eval(
            "let t = { a: 1 }; Object.preventExtensions(t); let p = new Proxy(t, { ownKeys: function() { return ['a', 'b']; } }); let caught = false; try { Reflect.ownKeys(p); } catch (error) { caught = error instanceof TypeError; } caught;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn proxy_traps_enforce_target_consistency_invariants() {
    // get must return a non-configurable non-writable property's exact value.
    assert!(
        eval(
            "let t = {}; Object.defineProperty(t, 'a', { value: 1, writable: false, configurable: false }); \
             let p = new Proxy(t, { get() { return 2; } }); p.a;"
        )
        .is_err()
    );
    // get must return undefined for a non-configurable accessor with no getter.
    assert!(
        eval(
            "let t = {}; Object.defineProperty(t, 'a', { set() {}, configurable: false }); \
             let p = new Proxy(t, { get() { return 2; } }); p.a;"
        )
        .is_err()
    );
    // has(false) may not hide a property of a non-extensible target.
    assert!(
        eval(
            "let t = { a: 1 }; Object.preventExtensions(t); \
             let p = new Proxy(t, { has() { return false; } }); 'a' in p;"
        )
        .is_err()
    );
    // set(true) may not contradict a non-configurable non-writable property.
    assert!(
        eval(
            "let t = {}; Object.defineProperty(t, 'a', { value: 1, writable: false, configurable: false }); \
             let p = new Proxy(t, { set() { return true; } }); p.a = 2;"
        )
        .is_err()
    );
    // deleteProperty(true) may not report dropping a non-configurable property.
    assert!(
        eval(
            "let t = {}; Object.defineProperty(t, 'a', { value: 1, configurable: false }); \
             let p = new Proxy(t, { deleteProperty() { return true; } }); delete p.a;"
        )
        .is_err()
    );
    // The well-behaved cases still succeed.
    assert_eq!(
        eval(
            "let t = { a: 1 }; let p = new Proxy(t, { get() { return 9; }, has() { return false; }, set() { return true; }, deleteProperty() { return true; } }); \
             p.a + ':' + ('a' in p) + ':' + ((p.a = 5), true) + ':' + (delete p.a);"
        ),
        Ok(Value::String("9:false:true:true".to_owned()))
    );
}

#[test]
fn dispatches_proxy_traps_through_the_prototype_chain() {
    // A `get` on an ordinary object whose prototype is a Proxy must invoke the
    // proxy `get` trap with the original receiver.
    assert_eq!(
        eval(
            "let p = new Proxy({}, { get(target, key, receiver) { return key === 'foo' ? 3 : undefined; } }); \
             let o = Object.create(p); o.foo;"
        ),
        Ok(Value::Number(3.0))
    );
    // A `set` walking the prototype chain reaches the proxy `set` trap, which
    // receives the target, key, value, and the original receiver.
    assert_eq!(
        eval(
            "let log = ''; let target = {}; \
             let handler = { set(t, prop, value, receiver) { log = (t === target) + ':' + prop + ':' + value + ':' + (receiver === o); return true; } }; \
             let p = new Proxy(target, handler); var o = Object.create(p); o.prop = 'v'; log;"
        ),
        Ok(Value::String("true:prop:v:true".to_owned()))
    );
    // An absent trap forwards through a proxy target that is itself a proxy.
    assert_eq!(
        eval(
            "let inner = new Proxy({}, { get(t, k) { return k === 'x' ? 42 : undefined; } }); \
             let outer = new Proxy(inner, { get: null }); Object.create(outer).x;"
        ),
        Ok(Value::Number(42.0))
    );
}

#[test]
fn compares_proxy_values_by_identity() {
    assert_eq!(
        eval("let p = new Proxy({}, {}); p === p;"),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval(
            "let p = new Proxy({}, {}); let o = Object.create(p); Object.getPrototypeOf(o) === p;"
        ),
        Ok(Value::Boolean(true))
    );
    assert_eq!(
        eval("new Proxy({}, {}) === new Proxy({}, {});"),
        Ok(Value::Boolean(false))
    );
}

#[test]
fn in_operator_propagates_proxy_has_trap_throw() {
    assert_eq!(
        eval(
            "let thrown = false; let p = new Proxy({}, { has() { throw new TypeError('boom'); } }); \
             try { 'attr' in p; } catch (e) { thrown = e instanceof TypeError; } thrown;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn has_own_property_and_property_is_enumerable_dispatch_proxy_trap() {
    // hasOwnProperty runs the proxy's getOwnPropertyDescriptor trap (its
    // [[GetOwnProperty]]), including through a proxy whose target is a proxy.
    assert_eq!(
        eval(
            "let inner = new Proxy({}, { getOwnPropertyDescriptor(t, k) { return k === 'foo' ? { value: 1, configurable: true } : undefined; } }); \
             let p = new Proxy(inner, { getOwnPropertyDescriptor: null }); \
             p.hasOwnProperty('foo') + ':' + p.hasOwnProperty('bar');"
        ),
        Ok(Value::String("true:false".to_owned()))
    );
    // propertyIsEnumerable reads the trap-reported enumerable flag.
    assert_eq!(
        eval(
            "let p = new Proxy({}, { getOwnPropertyDescriptor(t, k) { return { value: 1, enumerable: true, configurable: true }; } }); \
             p.propertyIsEnumerable('x');"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn instanceof_dispatches_proxy_get_prototype_of_trap() {
    // `instanceof` walks the operand's [[GetPrototypeOf]] chain, so a proxy's
    // getPrototypeOf trap decides the result.
    assert_eq!(
        eval(
            "function Custom() {} \
             let p = new Proxy({}, { getPrototypeOf() { return Custom.prototype; } }); \
             p instanceof Custom;"
        ),
        Ok(Value::Boolean(true))
    );
    // A getPrototypeOf trap that contradicts a non-extensible target's prototype
    // violates the invariant, and `instanceof` propagates that TypeError.
    assert_eq!(
        eval(
            "function Custom() {} \
             let target = {}; \
             let p = new Proxy(target, { getPrototypeOf() { return Custom.prototype; } }); \
             Object.preventExtensions(target); \
             let threw = false; \
             try { p instanceof Custom; } catch (e) { threw = e instanceof TypeError; } threw;"
        ),
        Ok(Value::Boolean(true))
    );
    // Ordinary instanceof is unaffected.
    assert_eq!(
        eval("([] instanceof Array) + ':' + ({} instanceof Object) + ':' + (1 instanceof Object);"),
        Ok(Value::String("true:true:false".to_owned()))
    );
}

#[test]
fn revocation_function_is_anonymous() {
    assert_eq!(
        eval("Proxy.revocable({}, {}).revoke.name;"),
        Ok(Value::String(String::new()))
    );
    assert_eq!(
        eval(
            "let d = Object.getOwnPropertyDescriptor(Proxy.revocable({}, {}).revoke, 'name'); \
             d.writable + ':' + d.enumerable + ':' + d.configurable;"
        ),
        Ok(Value::String("false:false:true".to_owned()))
    );
}

#[test]
fn object_create_accepts_an_array_prototype() {
    assert_eq!(
        eval("let o = Object.create([7, 8, 9]); o.length + ':' + o[1];"),
        Ok(Value::String("3:8".to_owned()))
    );
}
