use crate::{Value, eval};

#[test]
fn instantiates_class_and_reads_field() {
    assert_eq!(
        eval("class C { constructor(x) { this.x = x; } } new C(7).x;"),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn calls_prototype_method() {
    assert_eq!(
        eval(
            "class C { constructor(x) { this.x = x; } twice() { return this.x * 2; } } new C(21).twice();"
        ),
        Ok(Value::Number(42.0))
    );
}

#[test]
fn method_this_binds_to_instance() {
    assert_eq!(
        eval(
            "class Counter { constructor() { this.n = 0; } inc() { this.n += 1; return this.n; } } let c = new Counter(); c.inc(); c.inc();"
        ),
        Ok(Value::Number(2.0))
    );
}

#[test]
fn default_constructor_creates_instance() {
    assert_eq!(
        eval("class C {} typeof new C();"),
        Ok(Value::String("object".to_owned()))
    );
}

#[test]
fn instance_is_instanceof_class() {
    assert_eq!(
        eval("class C {} (new C()) instanceof C;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn prototype_constructor_back_reference() {
    assert_eq!(
        eval("class C {} C.prototype.constructor === C;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn prototype_property_is_non_writable_non_enumerable_non_configurable() {
    assert_eq!(
        eval(
            "class C {} let d = Object.getOwnPropertyDescriptor(C, 'prototype'); [d.writable, d.enumerable, d.configurable].join(',');"
        ),
        Ok(Value::String("false,false,false".to_owned()))
    );
}

#[test]
fn constructor_back_reference_is_writable_non_enumerable_configurable() {
    assert_eq!(
        eval(
            "class C {} let d = Object.getOwnPropertyDescriptor(C.prototype, 'constructor'); [d.writable, d.enumerable, d.configurable].join(',');"
        ),
        Ok(Value::String("true,false,true".to_owned()))
    );
}

#[test]
fn methods_are_non_enumerable_writable_configurable() {
    assert_eq!(
        eval(
            "class C { m() {} } let d = Object.getOwnPropertyDescriptor(C.prototype, 'm'); [d.writable, d.enumerable, d.configurable].join(',');"
        ),
        Ok(Value::String("true,false,true".to_owned()))
    );
}

#[test]
fn methods_are_not_own_enumerable_keys() {
    assert_eq!(
        eval("class C { a() {} b() {} } Object.keys(C.prototype).length;"),
        Ok(Value::Number(0.0))
    );
}

#[test]
fn constructor_name_comes_from_binding() {
    assert_eq!(
        eval("class C {} C.name;"),
        Ok(Value::String("C".to_owned()))
    );
}

#[test]
fn constructor_length_comes_from_constructor_params() {
    assert_eq!(
        eval("class C { constructor(a, b, c) {} } C.length;"),
        Ok(Value::Number(3.0))
    );
}

#[test]
fn default_constructor_has_zero_length() {
    assert_eq!(eval("class C {} C.length;"), Ok(Value::Number(0.0)));
}

#[test]
fn calling_class_without_new_throws_type_error() {
    assert_eq!(
        eval(
            "class C {} try { C(); 'no throw'; } catch (e) { e instanceof TypeError ? 'TypeError' : 'other'; }"
        ),
        Ok(Value::String("TypeError".to_owned()))
    );
}

#[test]
fn method_is_not_constructable() {
    assert_eq!(
        eval(
            "class C { m() {} } let c = new C(); try { new c.m(); 'no throw'; } catch (e) { e instanceof TypeError ? 'TypeError' : 'other'; }"
        ),
        Ok(Value::String("TypeError".to_owned()))
    );
}

#[test]
fn method_has_no_prototype_property() {
    assert_eq!(
        eval("class C { m() {} } C.prototype.m.prototype === undefined;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn class_expression_anonymous_is_constructable() {
    assert_eq!(
        eval("let D = class { value() { return 9; } }; new D().value();"),
        Ok(Value::Number(9.0))
    );
}

#[test]
fn named_class_expression_exposes_name() {
    assert_eq!(
        eval("let x = class Named {}; x.name;"),
        Ok(Value::String("Named".to_owned()))
    );
}

#[test]
fn named_class_expression_name_is_visible_inside_methods() {
    assert_eq!(
        eval("let x = class Named { self() { return Named; } }; let i = new x(); i.self() === x;"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn class_declaration_is_block_scoped() {
    assert_eq!(
        eval("{ class C {} } typeof C;"),
        Ok(Value::String("undefined".to_owned()))
    );
}

#[test]
fn class_declaration_is_not_hoisted_like_var() {
    // Class declarations create lexical bindings, not var-style hoisted ones,
    // so referencing the class before its declaration is a ReferenceError.
    assert_eq!(
        eval(
            "try { let probe = C; 'no throw'; class C {} } catch (e) { e instanceof ReferenceError ? 'ReferenceError' : 'other'; }"
        ),
        Ok(Value::String("ReferenceError".to_owned()))
    );
}

#[test]
fn class_body_is_strict_mode_code() {
    // Assigning to an undeclared name inside a method throws in strict mode.
    assert_eq!(
        eval(
            "class C { m() { undeclaredStrict = 1; } } try { new C().m(); 'no throw'; } catch (e) { e instanceof ReferenceError ? 'ReferenceError' : 'other'; }"
        ),
        Ok(Value::String("ReferenceError".to_owned()))
    );
}

#[test]
fn method_closes_over_enclosing_scope() {
    assert_eq!(
        eval("let base = 100; class C { m() { return base + 1; } } new C().m();"),
        Ok(Value::Number(101.0))
    );
}
