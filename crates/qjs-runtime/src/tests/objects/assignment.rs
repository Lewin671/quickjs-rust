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
