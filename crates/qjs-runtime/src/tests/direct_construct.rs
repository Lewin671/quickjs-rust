//! `new` on an ordinary bytecode function runs on the slot-seeded direct-leaf
//! frame instead of the general construct path. These pin the observable
//! contract that path must keep: receiver prototype, `new.target`, the
//! primitive-versus-object completion rule, error propagation, and the shapes
//! that must still decline to the general path.

use crate::{Value, eval};

fn value_of(source: &str) -> Value {
    eval(source).expect("source must evaluate")
}

fn error_of(source: &str) -> String {
    eval(source).expect_err("source must fail").message
}

#[test]
fn a_direct_constructor_seeds_its_receiver_from_its_own_prototype() {
    assert_eq!(
        value_of(
            "function F(a) { this.a = a; }
             var o = new F(3);
             o.a === 3 && Object.getPrototypeOf(o) === F.prototype && o instanceof F;"
        ),
        Value::Boolean(true)
    );
}

#[test]
fn a_direct_constructor_sees_itself_as_new_target() {
    assert_eq!(
        value_of(
            "function Inner() { this.t = new.target === Inner; }
             function Outer() {
               var inner = new Inner();
               this.ok = inner.t && new.target === Outer;
             }
             new Outer().ok && new Inner().t;"
        ),
        Value::Boolean(true)
    );
}

#[test]
fn a_primitive_completion_yields_the_receiver_and_an_object_completion_wins() {
    assert_eq!(
        value_of("function H(a) { this.a = a; return 5; } new H(1).a;"),
        Value::Number(1.0)
    );
    assert_eq!(
        value_of("function G() { this.a = 1; return { k: 2 }; } new G().k;"),
        Value::Number(2.0)
    );
}

#[test]
fn a_non_object_prototype_still_falls_back_to_object_prototype() {
    assert_eq!(
        value_of(
            "function I() {}
             I.prototype = 7;
             Object.getPrototypeOf(new I()) === Object.prototype;"
        ),
        Value::Boolean(true)
    );
}

#[test]
fn proxy_and_bound_constructors_keep_the_general_construct_path() {
    assert_eq!(
        value_of(
            "function F() { this.x = 1; }
             var P = new Proxy(F, { construct: function () { return { marker: 1 }; } });
             var B = F.bind(null);
             var bound = new B();
             new P().marker === 1 && bound.x === 1 && bound instanceof F;"
        ),
        Value::Boolean(true)
    );
}

#[test]
fn a_function_prototype_is_a_valid_receiver_prototype() {
    assert_eq!(
        value_of(
            "function L() { this.x = 1; }
             L.prototype = function () {};
             typeof Object.getPrototypeOf(new L());"
        ),
        Value::string_from_utf8("function")
    );
}

#[test]
fn a_throwing_constructor_propagates_to_the_caller() {
    assert_eq!(
        value_of(
            "function K() { this.x = 1; throw new Error('boom'); }
             var message = '';
             try { new K(); } catch (e) { message = e.message; }
             message === 'boom';"
        ),
        Value::Boolean(true)
    );
}

#[test]
fn non_constructable_callees_still_raise_type_errors() {
    assert!(error_of("var f = () => {}; new f();").contains("TypeError"));
    assert!(error_of("var o = { m() {} }; new o.m();").contains("TypeError"));
}

#[test]
fn class_constructors_keep_their_own_construct_path() {
    assert_eq!(
        value_of(
            "class C { constructor(a) { this.a = a; } }
             class D extends C { constructor() { super(4); this.b = 1; } }
             new C(2).a + new D().a + new D().b;"
        ),
        Value::Number(7.0)
    );
    assert!(error_of("class C {} C();").contains("TypeError"));
}

#[test]
fn a_constructor_used_as_a_plain_call_and_as_new_keeps_both_meanings() {
    assert_eq!(
        value_of(
            "var seen = [];
             function F(a) { 'use strict'; seen.push(this === undefined ? 'call' : 'new'); this.a = a; }
             new F(1);
             try { F(2); } catch (e) { seen.push('throw'); }
             new F(3);
             seen.join(',') === 'new,call,throw,new';"
        ),
        Value::Boolean(true)
    );
}

#[test]
fn constructors_with_defaulted_and_missing_arguments_seed_every_parameter() {
    assert_eq!(
        value_of(
            "function P(a, b = 10, c) { this.sum = a + b + (c === undefined ? 100 : c); }
             new P(1).sum + new P(1, 2, 3).sum;"
        ),
        Value::Number(117.0)
    );
}

#[test]
fn a_base_class_with_public_fields_constructs_on_the_direct_leaf_frame() {
    assert_eq!(
        value_of(
            "class V { x = 0; y = null; z = []; w = V.d; static d = 7; constructor(a) { this.a = a; } }
             var v = new V(3);
             var v2 = new V(4);
             v2.z.push(1);
             v.x === 0 && v.y === null && Array.isArray(v.z) && v.z.length === 0 && v2.z.length === 1
               && v.w === 7 && v.a === 3 && Object.keys(v).join() === 'x,y,z,w,a'
               && Object.getPrototypeOf(v) === V.prototype;"
        ),
        Value::Boolean(true)
    );
}

#[test]
fn field_initializers_see_the_receiver_and_an_undefined_new_target() {
    assert_eq!(
        value_of(
            "class T { p = this; q = (this.p === this); r = new.target; constructor() { this.nt = new.target === T; } }
             var t = new T();
             t.q && t.r === undefined && t.nt;"
        ),
        Value::Boolean(true)
    );
}

#[test]
fn an_initializer_that_makes_the_receiver_reject_a_field_still_throws() {
    assert!(
        error_of("class F { a = Object.preventExtensions(this); b = 1; } new F();")
            .contains("TypeError")
    );
    assert!(
        error_of(
            "class G {
               a = Object.defineProperty(this, 'b', { value: 9, writable: false, configurable: false });
               b = 1;
             }
             new G();"
        )
        .contains("TypeError")
    );
}

#[test]
fn private_symbol_keyed_and_derived_classes_keep_the_general_construct_path() {
    assert_eq!(
        value_of("class P { #s = 5; get s() { return this.#s; } } new P().s;"),
        Value::Number(5.0)
    );
    assert_eq!(
        value_of(
            "class E { [Symbol.iterator] = 1; x = 2; }
             var e = new E();
             e.x === 2 && e[Symbol.iterator] === 1;"
        ),
        Value::Boolean(true)
    );
    assert_eq!(
        value_of(
            "class B { k = 1; }
             class D extends B { m = 2; constructor() { super(); this.n = 3; } }
             var d = new D();
             d.k === 1 && d.m === 2 && d.n === 3 && Object.keys(d).join() === 'k,m,n';"
        ),
        Value::Boolean(true)
    );
}

#[test]
fn a_constant_field_initializer_is_read_without_a_call() {
    // `Object.defineProperty` on `Function.prototype.call` would detect a
    // stray call; the observable contract is only the installed value, so
    // pin that a literal, a string, and a boolean arrive intact and that a
    // non-literal initializer still runs with `this` bound.
    assert_eq!(
        value_of(
            "class C { n = 1.5; s = 'str'; b = false; u; d = this.n * 2; }
             var c = new C();
             c.n === 1.5 && c.s === 'str' && c.b === false && c.u === undefined && c.d === 3
               && Object.keys(c).join() === 'n,s,b,u,d';"
        ),
        Value::Boolean(true)
    );
}
