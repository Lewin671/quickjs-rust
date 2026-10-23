//! Enclosing call context used by separately compiled class expressions.

use crate::{MapResolver, Value, eval, eval_module};

fn number(source: &str, expected: f64) {
    assert_eq!(eval(source), Ok(Value::Number(expected)), "{source}");
}

#[test]
fn deferred_keys_use_static_method_receiver() {
    number(
        r#"class Outer { static #k = "m"; static f() {
            class D { [this.#k]() { return 1; } } return new D().m();
        } } Outer.f();"#,
        1.0,
    );
}

#[test]
fn deferred_keys_use_instance_method_receiver() {
    number(
        r#"class Outer { #k = "m"; f() {
            class D { [this.#k]() { return 2; } } return new D().m();
        } } new Outer().f();"#,
        2.0,
    );
}

#[test]
fn deferred_field_and_accessor_keys_use_enclosing_receiver() {
    number(
        r#"class Outer { #k = "m"; f() {
            class D { [this.#k] = 2; }
            class E { get [this.#k]() { return 3; } }
            return new D().m + new E().m;
        } } new Outer().f();"#,
        5.0,
    );
}

#[test]
fn arrows_inside_deferred_keys_capture_enclosing_receiver() {
    number(
        r#"class Outer { static #k = "m"; static f() {
            class D { [(() => this.#k)()]() { return 12; } } return new D().m();
        } } Outer.f();"#,
        12.0,
    );
}

#[test]
fn escaping_arrows_capture_deferred_key_receiver() {
    number(
        r#"class Outer { #k = "m"; f() { return () => () => {
            class D { [this.#k]() { return 3; } } return new D().m();
        }; } } let read = new Outer().f(); read.call({}).call({});"#,
        3.0,
    );
}

#[test]
fn default_parameter_class_keys_use_call_receiver() {
    number(
        r#"class Outer { static #k = "m";
            static f(C = class { [this.#k]() { return 4; } }) { return new C().m(); }
        } Outer.f();"#,
        4.0,
    );
}

#[test]
fn default_parameter_arrows_capture_deferred_key_receiver() {
    number(
        r#"class Outer { #k = "m"; f(read = () => {
            class D { [this.#k]() { return 5; } } return new D().m();
        }) { return read; } } let read = new Outer().f(); read();"#,
        5.0,
    );
}

#[test]
fn field_initializers_own_deferred_key_receivers() {
    number(
        r#"class Outer { #k = "m"; value = (() => {
            class D { [this.#k]() { return 6; } } return new D().m();
        })(); } new Outer().value;"#,
        6.0,
    );
}

#[test]
fn static_fields_and_blocks_own_deferred_key_receivers() {
    number(
        r#"class Outer { static #k = "m"; static value = (() => {
            class D { [this.#k]() { return 7; } } return new D().m();
        })(); static { this.value += (() => {
            class D { [this.#k]() { return 8; } } return new D().m();
        })(); } } Outer.value;"#,
        15.0,
    );
}

#[test]
fn field_thunks_use_receivers_without_direct_this_loads() {
    number(
        r#"class Outer { #k = "m"; static #s = "m";
            value = new (class { [this.#k] = 13; })().m;
            static value = new (class { [this.#s] = 14; })().m;
        } new Outer().value + Outer.value;"#,
        27.0,
    );
}

#[test]
fn static_blocks_use_receivers_without_direct_this_loads() {
    number(
        r#"let result; class Outer { static #k = "m"; static {
            class D { [this.#k] = 15; } result = new D().m;
        } } result;"#,
        15.0,
    );
}

#[test]
fn deferred_key_arrows_keep_outer_arguments_cells() {
    number(
        r#"class Outer { #k = "m"; f(value) {
            let read = () => { class D { [this.#k]() { return 9; } }
                return new D()[arguments[0]](); }; return read;
        } } let read = new Outer().f("m"); read();"#,
        9.0,
    );
}

#[test]
fn receiver_requirements_stop_at_receiver_owning_children() {
    for source in [
        "(function () { return function () { return this; }; })",
        "(function () { return class { field = this; static field = this; static { this.field; } }; })",
        r#"(function () { return class Outer { #k = "m"; method() {
            class D { [this.#k]() {} } return D;
        } }; })"#,
    ] {
        let Value::Function(function) = eval(source).unwrap() else {
            panic!("function")
        };
        assert!(
            !function.bytecode.as_ref().unwrap().needs_call_receiver(),
            "{source}"
        );
    }
}

#[test]
fn nested_classes_keep_constructor_new_target_and_home_object() {
    number(
        r#"class Parent { get value() { return 10; } }
        class Outer extends Parent { #k = "m"; constructor() { super();
            this.read = () => { class D { [this.#k]() {} }
                return new.target === Outer ? super.value : 0;
            };
        } } let read = new Outer().read; read();"#,
        10.0,
    );
}

#[test]
fn deferred_keys_keep_receiver_in_modules_with_imports() {
    let namespace = eval_module(
        r#"import { value } from 'dep';
        class Outer { static #k = "m"; static f() { return () => {
            class D { [this.#k]() { return value; } } return new D().m();
        }; } } const read = Outer.f(); export const result = read();"#,
        "main",
        Box::new(MapResolver::new().with("dep", "export const value = 11;")),
    )
    .expect("module evaluates");
    let Value::Object(namespace) = namespace else {
        panic!("module namespace")
    };
    assert_eq!(
        namespace.own_property("result").unwrap().value,
        Value::Number(11.0)
    );
}

#[test]
fn deferred_keys_keep_positional_receiver_in_modules() {
    let namespace = eval_module(
        r#"class Outer { static #k = "m"; static f() {
            class D { [this.#k]() { return 16; } } return new D().m();
        } } export const result = Outer.f();"#,
        "main",
        Box::new(MapResolver::new()),
    )
    .expect("module evaluates");
    let Value::Object(namespace) = namespace else {
        panic!("module namespace")
    };
    assert_eq!(
        namespace.own_property("result").unwrap().value,
        Value::Number(16.0)
    );
}
