use crate::{Value, eval};

#[test]
fn evaluates_member_assignment() {
    assert_eq!(
        eval("let o = {}; o.answer = 42; o.answer;"),
        Ok(Value::Number(42.0))
    );
    assert_eq!(
        eval("let key = 'answer'; let o = {}; o[key] = 7; o.answer;"),
        Ok(Value::Number(7.0))
    );
    assert_eq!(
        eval("let key = Symbol('answer'); let o = {}; o[key] = 8; o[key];"),
        Ok(Value::Number(8.0))
    );
    assert_eq!(
        eval(
            "function read() {
                 let calls = '';
                 let key = { toString: function() { calls += 'k'; return 'answer'; } };
                 let object = { answer: 9 };
                 return object[key] + ':' + calls;
             }
             read();"
        ),
        Ok(Value::String("9:k".to_owned().into()))
    );
    assert_eq!(
        eval("let seen = 0; let o = { set answer(value) { seen = value; } }; o.answer = 9; seen;"),
        Ok(Value::Number(9.0))
    );
    assert_eq!(
        eval("this.answer = 42; this.answer;"),
        Ok(Value::Number(42.0))
    );
    assert_eq!(eval("this === this;"), Ok(Value::Boolean(true)));
}

#[test]
fn repeated_named_own_data_writes_preserve_the_final_state() {
    assert_eq!(
        eval(
            "var object = { a: 0, b: 0, c: 0 }; var checksum = 0;
             for (var i = 0; i < 1000; i++) {
                 object.a = object.c + 1;
                 object.b = object.a + 1;
                 object.c = object.b - 1;
                 checksum += object.c;
             }
             checksum + ':' + object.a + ':' + object.b + ':' + object.c;"
        ),
        Ok(Value::String("500500:1000:1001:1000".to_owned().into()))
    );
}

#[test]
fn cached_named_compound_writes_revalidate_accessor_receivers() {
    assert_eq!(
        eval(
            "function Box(value) { this.value = value; } \
             function bump(box) { return ++box.value; } \
             var first = new Box(0); var second = new Box(10); \
             var out = [bump(first), bump(second), bump(first)]; \
             var intercepted = 20; \
             Object.defineProperty(second, 'value', { configurable: true, \
               get: function() { return intercepted; }, \
               set: function(value) { intercepted = value; } }); \
             out.push(bump(second)); out.join(':') + ':' + intercepted;"
        ),
        Ok(Value::String("1:11:2:21:21".to_owned().into()))
    );
}

#[test]
fn named_member_assignment_keeps_the_reference_selected_before_the_rhs() {
    assert_eq!(
        eval(
            "var original = { value: 0 };
             var selected = original;
             function rhs() { selected = { value: 10 }; return 7; }
             selected.value = rhs();
             original.value + ':' + selected.value;"
        ),
        Ok(Value::String("7:10".to_owned().into()))
    );
}

#[test]
fn ordinary_set_honors_non_writable_data_properties() {
    // Sloppy assignment to an own non-writable data property is a silent no-op.
    assert_eq!(
        eval(
            "var o = {}; Object.defineProperty(o, 'p', { value: 10, writable: false, \
             configurable: true }); o.p = 20; o.p;"
        ),
        Ok(Value::Number(10.0))
    );
    // Strict assignment to the same property throws a TypeError.
    assert!(
        eval(
            "'use strict'; var o = {}; Object.defineProperty(o, 'p', { value: 10, \
             writable: false }); o.p = 20;"
        )
        .is_err()
    );
    // Strict compound assignment is likewise rejected.
    assert!(
        eval(
            "'use strict'; var o = {}; Object.defineProperty(o, 'p', { value: 10, \
             writable: false }); o.p *= 2;"
        )
        .is_err()
    );
    // An inherited non-writable data property blocks creating an own property.
    assert_eq!(
        eval(
            "function F() {} Object.defineProperty(F.prototype, 'p', { value: 1 }); \
             var o = new F(); o.p = 2; o.hasOwnProperty('p');"
        ),
        Ok(Value::Boolean(false))
    );
    assert_eq!(
        eval(
            "function F() {} Object.defineProperty(F.prototype, 'p', { value: 1 }); \
             var o = new F(); o.p = 2; o.p;"
        ),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn ordinary_missing_own_data_write_preserves_prototype_semantics() {
    assert_eq!(
        eval(
            "var prototype = { p: 1 };
             var object = Object.create(prototype);
             object.p = 2;
             var descriptor = Object.getOwnPropertyDescriptor(object, 'p');
             [object.p, prototype.p, descriptor.writable, descriptor.enumerable,
              descriptor.configurable].join(':');"
        ),
        Ok(Value::String("2:1:true:true:true".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var seen = 0;
             var prototype = { set p(value) { seen = value; } };
             var object = Object.create(prototype);
             object.p = 3;
             seen + ':' + object.hasOwnProperty('p');"
        ),
        Ok(Value::String("3:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var calls = 0;
             var prototype = new Proxy({}, { set() { calls += 1; return true; } });
             var object = Object.create(prototype);
             object.p = 4;
             calls + ':' + object.hasOwnProperty('p');"
        ),
        Ok(Value::String("1:false".to_owned().into()))
    );
    assert_eq!(
        eval(
            "var object = Object.preventExtensions({});
             object.p = 5;
             object.hasOwnProperty('p');"
        ),
        Ok(Value::Boolean(false))
    );
}

#[test]
fn ordinary_set_runs_setter_in_strict_mode() {
    // A successful accessor setter must not throw in strict mode.
    assert_eq!(
        eval("'use strict'; var seen = 0; var o = { set p(v) { seen = v; } }; o.p = 5; seen;"),
        Ok(Value::Number(5.0))
    );
    // Writing through a getter-only accessor fails: silent when sloppy, throws
    // when strict.
    assert_eq!(
        eval("var o = { get p() { return 1; } }; o.p = 5; o.p;"),
        Ok(Value::Number(1.0))
    );
    assert!(eval("'use strict'; var o = { get p() { return 1; } }; o.p = 5;").is_err());
}

#[test]
fn put_value_on_primitive_base_routes_through_wrapper_prototype() {
    // PutValue with a primitive base coerces to the wrapper object and runs
    // [[Set]], so a setter installed on the wrapper prototype chain fires. A
    // Proxy in the chain is consulted via its `set` trap (number/string/
    // boolean/bigint/symbol all box through ToObject).
    assert_eq!(
        eval(
            "var n = 0;
             Object.setPrototypeOf(Number.prototype, new Proxy({}, { set() { n += 1; return true; } }));
             (5).foo = 1;
             n;"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "var n = 0;
             Object.setPrototypeOf(Symbol.prototype, new Proxy({}, { set() { n += 1; return true; } }));
             Symbol().foo = 1;
             n;"
        ),
        Ok(Value::Number(1.0))
    );
    // A plain data write onto a primitive is unobservable: silent in sloppy
    // mode, a TypeError in strict mode.
    assert_eq!(eval("(5).foo = 1; (5).foo;"), Ok(Value::Undefined));
    assert!(eval("'use strict'; (5).foo = 1;").is_err());
    assert!(eval("'use strict'; Symbol().foo = 1;").is_err());
}

#[test]
fn cached_property_reads_track_writes_deletes_and_descriptor_changes() {
    // A named-read site caches an own-property slot once it observes that the
    // field it reads is also written. The slot must follow later value writes,
    // and must be abandoned when the property table's layout changes.
    assert_eq!(
        eval(
            "function Point(x, y) { this.x = x; this.y = y; } \
             function run(point, n) { \
               var total = 0; \
               for (var i = 0; i < n; i++) { point.x = point.x + 1; total += point.x + point.y; } \
               return total; \
             } \
             run(new Point(0, 10), 4);"
        ),
        Ok(Value::Number(50.0))
    );
    // Deleting and re-adding a property shifts storage slots.
    assert_eq!(
        eval(
            "var object = { a: 1, b: 2 }; \
             function read(target) { return target.b; } \
             var out = [read(object), read(object)]; \
             object.b = 20; \
             out.push(read(object)); \
             delete object.a; \
             out.push(read(object)); \
             object.c = 3; \
             out.push(read(object)); \
             out.join(',');"
        ),
        Ok(Value::String("2,2,20,20,20".to_owned().into()))
    );
    // Replacing a data property with an accessor must stop the slot read.
    assert_eq!(
        eval(
            "var object = { v: 1 }; \
             function read(target) { return target.v; } \
             var out = [read(object)]; \
             object.v = 2; \
             out.push(read(object)); \
             Object.defineProperty(object, 'v', { get: function() { return 99; }, configurable: true }); \
             out.push(read(object)); \
             out.join(',');"
        ),
        Ok(Value::String("1,2,99".to_owned().into()))
    );
    // A frozen field keeps reading its last value through the cached slot.
    assert_eq!(
        eval(
            "var object = { v: 5 }; \
             function read(target) { return target.v; } \
             read(object); object.v = 6; read(object); \
             Object.freeze(object); \
             object.v = 7; \
             read(object);"
        ),
        Ok(Value::Number(6.0))
    );
}

#[test]
fn shared_slot_property_cache_revalidates_every_receiver() {
    // One read site sees many objects built the same way. The cache records a
    // storage slot and the interned name held there, so it must still reject a
    // receiver whose layout differs, whose property became an accessor, or that
    // simply lacks the property.
    assert_eq!(
        eval(
            "function Point(x) { this.a = 0; this.b = 1; this.c = x; }\
             var list = []; for (var i = 0; i < 8; i++) list.push(new Point(i));\
             var read = function (o) { return o.c; };\
             var total = 0; for (var i = 0; i < list.length; i++) total += read(list[i]);\
             delete list[3].b;\
             Object.defineProperty(list[4], 'c', { get: function () { return 100; } });\
             var reshaped = { c: 50 };\
             var after = 0;\
             for (var i = 0; i < list.length; i++) after += read(list[i]);\
             [total, after, read(reshaped), read({}) === undefined].join(':');"
        ),
        Ok(Value::String("28:124:50:true".to_owned().into()))
    );
    // A receiver that inherits the name must not read a same-named own slot of
    // an unrelated object, and a later own definition must win.
    assert_eq!(
        eval(
            "function Base() { this.k = 1; }\
             function Derived() {}\
             Derived.prototype = { k: 9 };\
             var read = function (o) { return o.k; };\
             var out = [];\
             for (var i = 0; i < 4; i++) { out.push(read(new Base())); out.push(read(new Derived())); }\
             var d = new Derived(); d.k = 7; out.push(read(d));\
             out.join(':');"
        ),
        Ok(Value::String("1:9:1:9:1:9:1:9:7".to_owned().into()))
    );
}

// The computed string-key read and write both try a borrowed lookup before
// building an owned `PropertyKey`, because building one copies the string and
// a dictionary loop pays that per access. These pin the semantics that borrow
// must not change: it is only allowed to answer the cases the owned path would
// have answered identically.

#[test]
fn a_computed_string_write_still_honours_a_non_writable_property() {
    assert_eq!(
        eval(
            "var o = {}; Object.defineProperty(o, 'x', { value: 1, writable: false }); \
             var k = 'x'; o[k] = 2; o.x;"
        ),
        Ok(Value::Number(1.0))
    );
    assert_eq!(
        eval(
            "'use strict'; var o = {}; \
             Object.defineProperty(o, 'x', { value: 1, writable: false }); \
             var k = 'x'; try { o[k] = 2; 'no throw'; } catch (e) { e instanceof TypeError; }"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn a_computed_string_write_still_runs_an_own_setter() {
    assert_eq!(
        eval(
            "var log = []; var o = { set x(v) { log.push(v); } }; \
             var k = 'x'; o[k] = 7; o[k] = 8; log.join(',');"
        ),
        Ok(Value::String("7,8".into()))
    );
}

#[test]
fn a_computed_string_write_still_runs_an_inherited_setter() {
    assert_eq!(
        eval(
            "var seen = 0; var proto = { set x(v) { seen = v; } }; \
             var o = Object.create(proto); var k = 'x'; o[k] = 41; seen + (o.hasOwnProperty('x') ? 0 : 1);"
        ),
        Ok(Value::Number(42.0))
    );
}

#[test]
fn a_computed_string_write_still_creates_a_missing_property() {
    assert_eq!(
        eval("var o = {}; var k = 'fresh'; o[k] = 5; o.fresh + Object.keys(o).length;"),
        Ok(Value::Number(6.0))
    );
}

#[test]
fn a_computed_string_write_is_still_refused_by_a_frozen_object() {
    assert_eq!(
        eval("var o = Object.freeze({ x: 1 }); var k = 'x'; o[k] = 2; o.x;"),
        Ok(Value::Number(1.0))
    );
}

#[test]
fn a_computed_string_write_still_reaches_a_proxy_trap() {
    assert_eq!(
        eval(
            "var log = []; \
             var p = new Proxy({}, { set(t, k, v) { log.push(k + '=' + v); t[k] = v; return true; } }); \
             var k = 'x'; p[k] = 3; log.join(',') + '|' + p.x;"
        ),
        Ok(Value::String("x=3|3".into()))
    );
}

#[test]
fn a_computed_string_read_still_runs_getters_and_walks_the_prototype() {
    assert_eq!(
        eval(
            "var calls = 0; var proto = { get g() { calls++; return 10; }, inherited: 5 }; \
             var o = Object.create(proto); o.own = 1; \
             var a = 'g', b = 'inherited', c = 'own', d = 'missing'; \
             [o[a], o[b], o[c], o[d], calls].join(',');"
        ),
        Ok(Value::String("10,5,1,,1".into()))
    );
}

#[test]
fn a_computed_string_write_to_the_global_object_still_updates_the_binding() {
    assert_eq!(
        eval("var g = (0, eval)('this'); var k = 'sentinelGlobal'; g[k] = 9; sentinelGlobal;"),
        Ok(Value::Number(9.0))
    );
}
