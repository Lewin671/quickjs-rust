//! Function value representation: allocation sharing, lazy own properties and
//! the identity of prototypes and names.

use std::{mem, rc::Rc};

use super::{Function, FunctionData};
use crate::{NativeFunction, Value, eval};

#[test]
fn cloning_function_reuses_the_backing_allocation() {
    let function = Function::new_native(
        &crate::property::atom::AtomDomain::test_domain(),
        Some("shared"),
        0,
        NativeFunction::UninitializedLexical,
        false,
    );
    let cloned = function.clone();

    assert!(Rc::ptr_eq(&function.0, &cloned.0));
    assert!(function.ptr_eq(&cloned));
    assert_eq!(
        mem::size_of::<Function>(),
        mem::size_of::<Rc<FunctionData>>()
    );
    assert!(function.auxiliary.cold.borrow().is_none());
}

#[test]
fn ordinary_function_header_keeps_cold_identity_state_out_of_line() {
    let auxiliary_size = mem::size_of::<super::FunctionAuxiliaryState>();
    let function_data_size = mem::size_of::<FunctionData>();
    assert_eq!(
        mem::size_of::<super::NativeContext>(),
        mem::size_of::<Rc<()>>(),
        "lazy native context must remain pointer-sized"
    );
    assert_eq!(
        mem::size_of::<super::ModuleImports>(),
        mem::size_of::<Rc<()>>(),
        "shared module imports must remain pointer-sized"
    );
    assert!(
        mem::size_of::<super::LazyFunctionProperties>() <= 16,
        "lazy function property header must stay within two machine words"
    );
    assert!(
        auxiliary_size <= 48,
        "function auxiliary header grew to {auxiliary_size} bytes"
    );
    assert!(
        function_data_size <= 296,
        "function object grew to {function_data_size} bytes"
    );
}

#[test]
fn user_functions_keep_empty_function_maps_unallocated() {
    let value = eval("function make() { return function value() {}; } make();")
        .expect("function expression should evaluate");
    let Value::Function(function) = value else {
        panic!("expected function value");
    };
    assert!(!function.native_context.is_allocated());
    assert!(!function.properties.is_allocated());

    let mut native = Function::new_native(
        &crate::property::atom::AtomDomain::test_domain(),
        Some("capturing"),
        0,
        NativeFunction::UninitializedLexical,
        false,
    );
    assert!(!native.native_context.is_allocated());
    native.insert_native_context("captured".to_owned(), Value::Number(1.0));
    assert!(native.native_context.is_allocated());
    assert_eq!(
        native.native_context.get("captured"),
        Some(&Value::Number(1.0))
    );
    native.define_property(
        "explicit".to_owned(),
        crate::Property::enumerable(Value::Number(2.0)),
    );
    assert!(native.properties.is_allocated());
}

#[test]
fn cloned_function_handles_keep_identity_state_and_source_shared() {
    let function = Function::new_native(
        &crate::property::atom::AtomDomain::test_domain(),
        Some("shared"),
        0,
        NativeFunction::UninitializedLexical,
        false,
    );
    let cloned = function.clone();
    cloned.define_property(
        "shared".to_owned(),
        crate::Property::enumerable(Value::Number(1.0)),
    );
    assert!(Rc::ptr_eq(&function.0, &cloned.0));
    assert!(function.ptr_eq(&cloned));
    assert_eq!(
        function
            .own_property("shared")
            .map(|property| property.value),
        Some(Value::Number(1.0))
    );

    let Value::Function(function) = eval("(function shared() {})")
        .expect("function expression with retained source should evaluate")
    else {
        panic!("expected function value");
    };
    let cloned = function.clone();
    assert!(Rc::ptr_eq(&function.0, &cloned.0));
    assert_eq!(
        cloned.source_text().as_deref(),
        Some("function shared() {}")
    );
}

#[test]
fn default_name_and_length_stay_implicit_until_descriptor_mutation() {
    let function = Function::new_native(
        &crate::property::atom::AtomDomain::test_domain(),
        Some("shared"),
        2,
        NativeFunction::UninitializedLexical,
        false,
    );

    assert!(function.properties.borrow().is_empty());
    assert_eq!(
        function
            .own_property("length")
            .map(|property| property.value),
        Some(Value::Number(2.0))
    );
    assert_eq!(
        function.own_property("name").map(|property| property.value),
        Some(Value::String("shared".to_owned().into()))
    );
    assert_eq!(function.own_property_names(), ["length", "name"]);

    function.define_property(
        "name".to_owned(),
        crate::Property::data(
            Value::String("renamed".to_owned().into()),
            false,
            false,
            true,
        ),
    );
    assert_eq!(function.properties.borrow().len(), 2);
    assert_eq!(function.own_property_names(), ["length", "name"]);
}

#[test]
fn deleting_and_redefining_an_implicit_default_moves_it_to_the_end() {
    assert_eq!(
        eval(
            "var f = function value(a) {}; delete f.name; Object.defineProperty(f, 'name', { value: 'again', configurable: true }); Object.getOwnPropertyNames(f).join('|');"
        ),
        Ok(Value::String("length|prototype|name".to_owned().into()))
    );
}

#[test]
fn install_time_override_and_implicit_default_are_one_property_slot() {
    let function = Function::new_native(
        &crate::property::atom::AtomDomain::test_domain(),
        Some("initial"),
        0,
        NativeFunction::UninitializedLexical,
        false,
    );
    function.properties.borrow_mut().insert_in(
        &crate::property::atom::AtomDomain::test_domain(),
        "name",
        crate::Property::data(
            Value::String("installed".to_owned().into()),
            false,
            false,
            true,
        ),
    );

    assert_eq!(
        function.own_property("name").map(|property| property.value),
        Some(Value::String("installed".to_owned().into()))
    );
    assert_eq!(
        function
            .own_property_names()
            .into_iter()
            .filter(|key| key == "name")
            .count(),
        1
    );
    assert!(function.delete_own_property("name"));
    assert!(function.own_property("name").is_none());
}

#[test]
fn compiled_function_materializes_default_prototype_on_observation() {
    assert_eq!(
        eval(
            "var f = function (value) { return value; }; f.prototype.constructor === f && Object.getOwnPropertyNames(f).includes('prototype') && !delete f.prototype;"
        ),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn compiled_function_keeps_default_prototype_order_implicit_until_observation() {
    let function = eval("function make() { return function value() {}; } make();")
        .expect("function expression should evaluate");
    let Value::Function(function) = function else {
        panic!("expected function value");
    };

    assert!(function.auxiliary.lazy_default_prototype.get());
    assert!(function.properties.borrow().is_empty());
    assert_eq!(
        function.own_property_names(),
        ["length", "name", "prototype"]
    );
}

#[test]
fn later_properties_follow_an_unobserved_default_prototype_slot() {
    assert_eq!(
        eval(
            "var f = function value() {}; f.assigned = 1; Object.defineProperty(f, 'defined', { value: 2, configurable: true }); Object.getOwnPropertyNames(f).join('|');"
        ),
        Ok(Value::String(
            "length|name|prototype|assigned|defined".to_owned().into()
        ))
    );
}

#[test]
fn tmp_probe_table_after_prototype_read() {
    let Value::Function(function) = eval("function N(v){} N.prototype; N").unwrap() else { panic!() };
    let table = function.properties.borrow();
    eprintln!("TMPPROBE len={} keys={:?} enc={:?}", table.len(), table.keys().collect::<Vec<_>>(), table.domain().is_some());
}
