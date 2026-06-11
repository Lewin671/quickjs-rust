//! NamedEvaluation (ES2023 §8.3.4 and the SetFunctionName call sites): an
//! anonymous function, arrow, generator, async function, or class expression
//! takes its `name` from the binding, assignment, property, default, or field
//! that supplies its value. A named function/class expression keeps its own
//! name, and a value stored through a member reference (`obj.x = fn`) gets none.

use crate::{Value, eval};

fn name_of(source: &str) -> String {
    match eval(source) {
        Ok(Value::String(name)) => name,
        other => panic!("expected a string name, got {other:?}"),
    }
}

// --- Variable declarations -------------------------------------------------

#[test]
fn var_declaration_names_function() {
    assert_eq!(name_of("var f = function () {}; f.name"), "f");
}

#[test]
fn let_declaration_names_function() {
    assert_eq!(name_of("let f = function () {}; f.name"), "f");
}

#[test]
fn const_declaration_names_arrow() {
    assert_eq!(name_of("const f = () => {}; f.name"), "f");
}

#[test]
fn let_declaration_names_class() {
    assert_eq!(name_of("let D = class {}; D.name"), "D");
}

#[test]
fn declaration_names_generator_async_and_async_generator() {
    assert_eq!(name_of("let g = function* () {}; g.name"), "g");
    assert_eq!(name_of("let a = async function () {}; a.name"), "a");
    assert_eq!(name_of("let ag = async function* () {}; ag.name"), "ag");
    assert_eq!(name_of("let af = async () => {}; af.name"), "af");
}

// --- Simple assignment -----------------------------------------------------

#[test]
fn assignment_to_identifier_names_function() {
    assert_eq!(name_of("let f; f = function () {}; f.name"), "f");
}

#[test]
fn assignment_to_identifier_names_class_and_arrow() {
    assert_eq!(name_of("let D; D = class {}; D.name"), "D");
    assert_eq!(name_of("let a; a = () => {}; a.name"), "a");
}

#[test]
fn assignment_to_member_does_not_name() {
    // `obj.x = <anon>` is NOT NamedEvaluation; the name stays empty.
    assert_eq!(name_of("let o = {}; o.x = function () {}; o.x.name"), "");
    assert_eq!(name_of("let o = {}; o.x = class {}; o.x.name"), "");
}

// --- Object literal properties ---------------------------------------------

#[test]
fn object_property_value_names_function_arrow_and_class() {
    assert_eq!(name_of("({ f: function () {} }).f.name"), "f");
    assert_eq!(name_of("({ f: () => {} }).f.name"), "f");
    assert_eq!(name_of("({ f: class {} }).f.name"), "f");
}

#[test]
fn object_method_shorthand_keeps_its_name() {
    assert_eq!(name_of("({ m() {} }).m.name"), "m");
}

#[test]
fn computed_object_property_value_is_unnamed() {
    // Computed-key NamedEvaluation is a known gap (the key is only known at
    // runtime); the value keeps the empty name for now.
    assert_eq!(name_of("let k = 'c'; ({ [k]: function () {} }).c.name"), "");
}

// --- Default values --------------------------------------------------------

#[test]
fn function_parameter_default_names_value() {
    assert_eq!(
        name_of("function g(f = function () {}) { return f.name; } g()"),
        "f"
    );
    assert_eq!(
        name_of("function g(f = () => {}) { return f.name; } g()"),
        "f"
    );
    assert_eq!(
        name_of("function g(f = class {}) { return f.name; } g()"),
        "f"
    );
}

#[test]
fn object_destructuring_default_names_value() {
    assert_eq!(name_of("const { f = function () {} } = {}; f.name"), "f");
    assert_eq!(name_of("const { f = () => {} } = {}; f.name"), "f");
}

#[test]
fn array_destructuring_default_names_value() {
    assert_eq!(name_of("const [f = function () {}] = []; f.name"), "f");
    assert_eq!(name_of("const [f = class {}] = []; f.name"), "f");
}

#[test]
fn assignment_destructuring_default_names_value() {
    assert_eq!(name_of("let f; ({ f = function () {} } = {}); f.name"), "f");
    assert_eq!(name_of("let f; [f = () => {}] = []; f.name"), "f");
}

// --- Class fields ----------------------------------------------------------

#[test]
fn instance_field_names_value() {
    assert_eq!(
        name_of("class C { f = function () {}; } new C().f.name"),
        "f"
    );
    assert_eq!(name_of("class C { f = () => {}; } new C().f.name"), "f");
    assert_eq!(name_of("class C { f = class {}; } new C().f.name"), "f");
}

#[test]
fn static_field_names_value() {
    assert_eq!(
        name_of("class C { static f = function () {}; } C.f.name"),
        "f"
    );
}

#[test]
fn private_field_names_value_with_hash_prefix() {
    assert_eq!(
        name_of("class C { #f = function () {}; name() { return this.#f.name; } } new C().name()"),
        "#f"
    );
}

#[test]
fn computed_field_key_value_is_unnamed() {
    assert_eq!(
        name_of("let k = 'c'; class C { [k] = function () {}; } new C().c.name"),
        ""
    );
}

// --- Logical assignment ----------------------------------------------------

#[test]
fn logical_assignment_names_value() {
    assert_eq!(name_of("let f; f ??= function () {}; f.name"), "f");
    assert_eq!(name_of("let f = 0; f ||= function () {}; f.name"), "f");
    assert_eq!(name_of("let f = 1; f &&= function () {}; f.name"), "f");
}

#[test]
fn arithmetic_compound_assignment_does_not_name() {
    // Only `&&=`, `||=`, and `??=` apply NamedEvaluation; `+=` and friends do
    // not (and would coerce the function to a string anyway).
    assert_eq!(
        name_of("let f = ''; f += function () { return 1; }; typeof f"),
        "string"
    );
}

// --- Counter-cases: named expressions keep their own name ------------------

#[test]
fn named_function_expression_keeps_its_name() {
    assert_eq!(name_of("let f = function bar() {}; f.name"), "bar");
}

#[test]
fn named_class_expression_keeps_its_name() {
    assert_eq!(name_of("let D = class Foo {}; D.name"), "Foo");
}

// --- Name property attributes ----------------------------------------------

#[test]
fn inferred_name_has_spec_attributes() {
    // The `name` property is non-writable, non-enumerable, configurable.
    assert_eq!(
        name_of(
            "let f = function () {}; \
             let d = Object.getOwnPropertyDescriptor(f, 'name'); \
             `${d.writable},${d.enumerable},${d.configurable},${d.value}`"
        ),
        "false,false,true,f"
    );
}

#[test]
fn anonymous_class_name_is_own_configurable_property() {
    assert_eq!(
        name_of(
            "let D = class {}; \
             let d = Object.getOwnPropertyDescriptor(D, 'name'); \
             `${d.writable},${d.enumerable},${d.configurable},${d.value}`"
        ),
        "false,false,true,D"
    );
}
