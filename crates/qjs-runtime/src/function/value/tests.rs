//! Function ownership and representation discriminators.
use std::{mem, rc::Rc};

use super::{Function, FunctionData};
use crate::{NativeFunction, Value, eval};

#[test]
fn cloning_function_reuses_the_backing_allocation() {
    let domain = crate::property::atom::AtomDomain::new();
    let function = Function::new_native_in_domain(
        Some("shared"),
        0,
        NativeFunction::UninitializedLexical,
        false,
        &domain,
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
    eprintln!("S2 sizes: FunctionData={function_data_size} auxiliary={auxiliary_size}");
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
        mem::size_of::<std::cell::RefCell<crate::value::layout::NamedStorage>>() <= 32,
        "lazy function property header must stay within 32 bytes"
    );
    assert!(
        auxiliary_size <= 48,
        "function auxiliary header grew to {auxiliary_size} bytes"
    );
    assert!(
        function_data_size <= 304,
        "function object grew to {function_data_size} bytes"
    );
}

#[test]
fn user_functions_keep_empty_function_maps_unallocated() {
    let domain = crate::property::atom::AtomDomain::new();
    let value = eval("function make() { return function value() {}; } make();")
        .expect("function expression should evaluate");
    let Value::Function(function) = value else {
        panic!("expected function value");
    };
    assert!(!function.native_context.is_allocated());
    assert!(function.named.borrow().store().is_none());

    let mut native = Function::new_native_in_domain(
        Some("capturing"),
        0,
        NativeFunction::UninitializedLexical,
        false,
        &domain,
    );
    assert!(native.native_context.is_allocated());
    assert_eq!(
        native
            .native_context
            .0
            .as_ref()
            .unwrap()
            .bindings
            .capacity(),
        0
    );
    assert!(native.property_domain().ptr_eq(&domain));
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
    assert!(native.named.borrow().store().is_some());
}

#[test]
fn cloned_function_handles_keep_identity_state_and_source_shared() {
    let domain = crate::property::atom::AtomDomain::new();
    let function = Function::new_native_in_domain(
        Some("shared"),
        0,
        NativeFunction::UninitializedLexical,
        false,
        &domain,
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
    let domain = crate::property::atom::AtomDomain::new();
    let function = Function::new_native_in_domain(
        Some("shared"),
        2,
        NativeFunction::UninitializedLexical,
        false,
        &domain,
    );

    assert!(function.named.borrow().store().is_none());
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
    assert_eq!(function.with_named(|s| s.unwrap().len()), 2);
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
    let domain = crate::property::atom::AtomDomain::new();
    let function = Function::new_native_in_domain(
        Some("initial"),
        0,
        NativeFunction::UninitializedLexical,
        false,
        &domain,
    );
    function.define_property(
        "name".to_owned(),
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
    assert!(function.property_order().is_empty());
    assert_eq!(
        function.own_property_names(),
        ["length", "name", "prototype"]
    );
    assert_eq!(function.property_order(), ["length", "name", "prototype"]);
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
fn native_function_retains_its_explicit_domain_without_a_dummy_prototype() {
    let domain = crate::property::atom::AtomDomain::new();
    let weak = domain.downgrade();
    let function = Function::new_native_in_domain(
        Some("owner"),
        0,
        NativeFunction::UninitializedLexical,
        false,
        &domain,
    );
    assert!(function.property_domain().ptr_eq(&domain));
    assert!(function.own_property("prototype").is_none());
    drop(domain);
    assert!(weak.is_live());
    drop(function);
    assert!(!weak.is_live());
}

#[test]
fn empty_native_captures_share_by_domain_and_detach_before_writes() {
    let domain = crate::property::atom::AtomDomain::new();
    let weak = domain.downgrade();
    let mut first =
        Function::new_native_in_domain(Some("a"), 0, NativeFunction::MathAbs, false, &domain);
    let second =
        Function::new_native_in_domain(Some("b"), 0, NativeFunction::MathAbs, false, &domain);
    assert!(Rc::ptr_eq(
        first.native_context.0.as_ref().unwrap(),
        second.native_context.0.as_ref().unwrap()
    ));
    first.insert_native_context("capture".into(), Value::Number(1.0));
    assert_eq!(
        first.native_context.get("capture"),
        Some(&Value::Number(1.0))
    );
    assert!(second.native_context.get("capture").is_none());
    let third = Function::new_native_in_domain(None, 0, NativeFunction::MathAbs, false, &domain);
    assert!(Rc::ptr_eq(
        second.native_context.0.as_ref().unwrap(),
        third.native_context.0.as_ref().unwrap()
    ));
    drop(domain);
    drop(first);
    drop(second);
    drop(third);
    assert!(
        !weak.is_live(),
        "the domain's empty-context attachment must remain weak"
    );
}

#[test]
fn unrelated_probes_leave_function_defaults_and_lazy_prototype_implicit() {
    let Value::Function(function) = eval("(function untouched(a) {})").unwrap() else {
        panic!()
    };
    assert!(function.named.borrow().store().is_none());
    assert!(function.auxiliary.lazy_default_prototype.get());
    let version = function.named_version();
    for _ in 0..1000 {
        let target = crate::value::named::NamedTarget::Function(&function);
        assert_eq!(
            target.probe("call", None),
            crate::value::named::Probe::Missing
        );
        assert_eq!(
            target.probe("absent", None),
            crate::value::named::Probe::Missing
        );
    }
    assert!(function.named.borrow().store().is_none());
    assert!(function.auxiliary.lazy_default_prototype.get());
    assert_eq!(function.named_version(), version);
}
