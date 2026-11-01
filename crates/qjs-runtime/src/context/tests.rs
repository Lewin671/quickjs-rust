//! Service borrows preserve semantic owners/cells without an owned facade.
use super::*;
use crate::{
    Function, NativeFunction, ObjectRef, Value,
    binding::{BindingCell, DynamicBindings},
};
use std::{collections::HashMap, rc::Rc};

#[test]
fn native_borrow_preserves_derived_cell_and_does_not_inherit_construct_target() {
    let mut root = HostRoot::new(new_realm(HashMap::new()));
    let cell = BindingCell::with_metadata(
        crate::binding::BindingMetaRef::dynamic("this".into(), true, false),
        None,
    );
    let token = Value::Object(ObjectRef::new(
        HashMap::new(),
        root.activation.realm().atoms(),
    ));
    let mut owner = CallOwner::root(Value::Undefined, OwnerKind::Function);
    owner.this = ThisBinding::Derived(cell.clone());
    owner.new_target = Some(token.clone());
    root.activation.set_call_owner(owner);
    let native = Function::new_native(
        None,
        0,
        NativeFunction::MathAbs,
        false,
        root.activation.realm(),
    );
    let receiver = Value::Number(9.0);
    let info = crate::native::CallInfo {
        callee: &native,
        this_value: &receiver,
        is_construct: false,
    };
    let mut caller = root.context();
    {
        let child = caller.with_native(&info, None, None);
        assert!(child.caller().derived_this_cell().unwrap().ptr_eq(&cell));
        assert!(child.new_target().is_none());
        assert!(child.caller().this_value().is_err());
        cell.initialize(token.clone()).unwrap();
        assert_eq!(child.caller().this_value(), Ok(token.clone()));
    }
    assert_eq!(caller.caller().this_value(), Ok(token.clone()));
    assert!(
        caller
            .caller()
            .lexical_new_target()
            .unwrap()
            .same_value(&token)
    );
}

#[test]
fn initialized_undefined_receiver_never_becomes_derived_on_native_entry() {
    let mut root = HostRoot::new(new_realm(HashMap::new()));
    root.activation
        .set_call_owner(CallOwner::root(Value::Undefined, OwnerKind::Module));
    let native = Function::new_native(
        None,
        0,
        NativeFunction::MathAbs,
        false,
        root.activation.realm(),
    );
    let receiver = Value::Number(9.0);
    let info = crate::native::CallInfo {
        callee: &native,
        this_value: &receiver,
        is_construct: false,
    };
    let mut caller = root.context();
    let child = caller.with_native(&info, None, None);
    assert!(child.caller().derived_this_cell().is_none());
    assert_eq!(child.caller().this_value(), Ok(Value::Undefined));
    assert_eq!(child.owner_kind(), Some(OwnerKind::Module));
}

#[test]
fn independent_activations_retain_one_import_target_after_creation_route_mutation() {
    let creation = CreationContext::root(new_realm(HashMap::new()));
    let mut first = ActivationContext::new(creation.clone(), ExecutionMode::Module);
    let second = ActivationContext::new(creation, ExecutionMode::Module);
    assert!(Rc::ptr_eq(
        &first.creation.imports().clone(),
        &second.creation.imports().clone()
    ));
    let exports = DynamicBindings::new();
    let cell = BindingCell::new(Value::Number(3.0));
    exports.insert_cell("exported".into(), cell.clone());
    first.set_module_import("local".into(), exports, "exported".into());
    assert!(first.module_import_cell("local").unwrap().ptr_eq(&cell));
    assert!(!second.has_module_import("local"));
}

#[test]
fn root_cell_routes_survive_class_key_mode_without_crossing_to_a_child_function() {
    let mut root = HostRoot::new(new_realm(HashMap::new()));
    let cells = DynamicBindings::new();
    let cell = BindingCell::new(Value::Number(3.0));
    cells.insert_cell("value".into(), cell.clone());
    root.activation.set_module_live_bindings(cells);
    root.activation
        .set_execution_state(ExecutionMode::ClassKey, true);
    assert!(
        root.context()
            .module_live_binding_cell("value")
            .unwrap()
            .ptr_eq(&cell)
    );
    let child = ActivationContext::new(root.activation.creation.clone(), ExecutionMode::Function);
    assert!(child.module_live_binding_cell("value").is_none());
}

#[test]
fn typed_call_metadata_keeps_indexed_binding_authority() {
    let mut root = HostRoot::new(new_realm(HashMap::new()));
    root.activation
        .set_call_owner(CallOwner::root(Value::Undefined, OwnerKind::Function));
    assert!(root.activation.permits_owned_slots());
    assert_eq!(root.context().caller().this_value(), Ok(Value::Undefined));
}

#[test]
fn marked_global_cache_tracks_initial_replacement_and_deletion() {
    let atoms = crate::property::atom::AtomDomain::new();
    let first = ObjectRef::new(HashMap::new(), &atoms);
    let key = "__quickjsRustDynamicFunctionRealm";
    let realm = new_realm_sharing_atoms(
        HashMap::from([(key.into(), Value::Object(first.clone()))]),
        atoms.clone(),
    );
    assert!(
        realm
            .dynamic_function_realm_global()
            .unwrap()
            .ptr_eq(&first)
    );
    realm.insert_value(key.into(), Value::Undefined);
    realm.refresh_dynamic_function_realm_global();
    assert!(realm.dynamic_function_realm_global().is_none());
    let second = ObjectRef::new(HashMap::new(), &atoms);
    let mut root = HostRoot::new(realm.clone());
    root.context()
        .insert_realm(key.into(), Value::Object(second.clone()));
    assert!(
        realm
            .dynamic_function_realm_global()
            .unwrap()
            .ptr_eq(&second)
    );
    root.context().remove_realm(key);
    assert!(realm.dynamic_function_realm_global().is_none());
}

#[cfg(feature = "perf-counters")]
#[test]
fn native_calls_mint_no_activation_identity_and_use_no_retired_env_boundary() {
    let mut root = HostRoot::script();
    let native = Value::Function(Function::new_native(
        None,
        1,
        NativeFunction::MathAbs,
        false,
        root.activation.realm(),
    ));
    let mut context = root.context();
    crate::reset_counters();
    for _ in 0..128 {
        assert_eq!(
            crate::function::call_function_slice(
                &native,
                Value::Undefined,
                &[Value::Number(-3.0)],
                &mut context
            ),
            Ok(Value::Number(3.0))
        );
    }
    let counters = crate::counters().unwrap();
    assert_eq!(counters.native_calls, 128);
    assert_eq!(counters.typed_variable_identities, 0);
    assert_eq!(counters.call_env_constructions, 0);
    assert_eq!(counters.call_env_clones, 0);
    assert_eq!(counters.call_env_equality_proofs, 0);
}

#[test]
fn native_realm_target_and_parked_caller_are_distinct_borrows() {
    let atoms = crate::property::atom::AtomDomain::new();
    let caller_realm = new_realm_sharing_atoms(HashMap::new(), atoms.clone());
    let native_realm = new_realm_sharing_atoms(HashMap::new(), atoms);
    let native = Function::new_native(None, 1, NativeFunction::Boolean, true, &native_realm);
    let target = Value::Function(Function::new_native(
        None,
        1,
        NativeFunction::Object,
        true,
        &caller_realm,
    ));
    let lexical = Value::Function(Function::new_native(
        None,
        1,
        NativeFunction::Array,
        true,
        &caller_realm,
    ));
    let mut root = HostRoot::new(caller_realm.clone());
    let mut owner = CallOwner::root(Value::Undefined, OwnerKind::Function);
    owner.new_target = Some(lexical.clone());
    root.activation.set_call_owner(owner);
    let identity = root.activation.variable_identity();
    let receiver = Value::Undefined;
    let info = crate::native::CallInfo {
        callee: &native,
        this_value: &receiver,
        is_construct: true,
    };
    let realm_owners = Rc::strong_count(&native_realm);
    let creation_owners = Rc::strong_count(native.creation_context());
    let mut caller = root.context();
    {
        let site = caller.with_native(&info, Some(&target), None);
        assert!(Rc::ptr_eq(site.realm(), &native_realm));
        assert!(Rc::ptr_eq(site.caller().realm(), &caller_realm));
        assert!(site.new_target().unwrap().same_value(&target));
        assert_eq!(site.caller().variable_identity(), identity);
        assert_eq!(Rc::strong_count(&native_realm), realm_owners);
        assert_eq!(Rc::strong_count(native.creation_context()), creation_owners);
    }
    assert!(
        caller
            .caller()
            .lexical_new_target()
            .unwrap()
            .same_value(&lexical)
    );
}

#[test]
fn independent_derived_activations_never_share_their_this_cell() {
    let creation = CreationContext::root(new_realm(HashMap::new()));
    let first_cell = BindingCell::with_metadata(
        crate::binding::BindingMetaRef::dynamic("this".into(), true, false),
        None,
    );
    let second_cell = first_cell.fresh_uninitialized();
    let mut first = ActivationContext::new(creation.clone(), ExecutionMode::Function);
    let mut second = ActivationContext::new(creation, ExecutionMode::Function);
    for (activation, cell) in [(&mut first, &first_cell), (&mut second, &second_cell)] {
        let mut owner = CallOwner::root(Value::Undefined, OwnerKind::Function);
        owner.this = ThisBinding::Derived(cell.clone());
        activation.set_call_owner(owner);
    }
    first_cell.initialize(Value::Number(7.0)).unwrap();
    assert_eq!(first.this_value(), Ok(Value::Number(7.0)));
    assert!(second.this_value().is_err());
    assert!(
        !first
            .derived_this_cell()
            .unwrap()
            .ptr_eq(&second.derived_this_cell().unwrap())
    );
}

#[test]
fn native_internal_errors_keep_the_callee_realm_across_the_borrow_boundary() {
    let mut foreign = HostRoot::script();
    let expected = foreign
        .activation
        .realm()
        .error_prototype(ErrorKind::RangeError)
        .unwrap();
    let number = foreign.context().get("Number").unwrap();
    let prototype = crate::property_value(number, "prototype", &mut foreign.context()).unwrap();
    let method = crate::property_value(prototype, "toFixed", &mut foreign.context()).unwrap();
    let mut caller = HostRoot::script();
    let error = crate::call_function(
        method,
        Value::Number(1.0),
        vec![Value::Number(101.0)],
        &mut caller.context(),
        false,
    )
    .unwrap_err();
    let Value::Object(error) = crate::error::runtime_error_to_value(error, &caller.context())
    else {
        panic!("error object");
    };
    assert!(
        matches!(error.prototype_slot(), Some(crate::Prototype::Object(prototype)) if prototype.ptr_eq(&expected))
    );
}

#[test]
fn javascript_internal_errors_keep_the_defining_realm_after_returning() {
    let mut foreign = HostRoot::script();
    let expected = foreign
        .activation
        .realm()
        .error_prototype(ErrorKind::TypeError)
        .unwrap();
    let constructor = foreign.context().get("Function").unwrap();
    let function = crate::call_function(
        constructor,
        Value::Undefined,
        vec![Value::String("return null.missing;".into())],
        &mut foreign.context(),
        false,
    )
    .unwrap();
    let mut caller = HostRoot::script();
    let error = crate::call_function(
        function,
        Value::Undefined,
        Vec::new(),
        &mut caller.context(),
        false,
    )
    .unwrap_err();
    let Value::Object(error) = crate::error::runtime_error_to_value(error, &caller.context())
    else {
        panic!("error object");
    };
    assert!(
        matches!(error.prototype_slot(), Some(crate::Prototype::Object(prototype)) if prototype.ptr_eq(&expected))
    );
}

#[test]
fn specialized_native_errors_keep_the_foreign_realm_inside_a_caller_body() {
    let mut foreign = HostRoot::script();
    let expected = foreign
        .activation
        .realm()
        .error_prototype(ErrorKind::UriError)
        .unwrap();
    let decode = foreign.context().get("decodeURI").unwrap();
    let mut caller = HostRoot::script();
    let Value::Object(global) = caller.activation.global_this().unwrap() else {
        panic!("global");
    };
    global.set("foreignDecode".into(), decode.clone());
    caller
        .context()
        .insert_realm("foreignDecode".into(), decode);
    let constructor = caller.context().get("Function").unwrap();
    let function = crate::call_function(
        constructor,
        Value::Undefined,
        vec![Value::String("return foreignDecode('%');".into())],
        &mut caller.context(),
        false,
    )
    .unwrap();
    let error = crate::call_function(
        function,
        Value::Undefined,
        Vec::new(),
        &mut caller.context(),
        false,
    )
    .unwrap_err();
    let Value::Object(error) = crate::error::runtime_error_to_value(error, &caller.context())
    else {
        panic!("error object");
    };
    assert!(
        matches!(error.prototype_slot(), Some(crate::Prototype::Object(prototype)) if prototype.ptr_eq(&expected))
    );
}

#[cfg(feature = "perf-counters")]
#[test]
fn callback_forwarding_counts_each_javascript_entry_once() {
    crate::reset_counters();
    assert_eq!(
        crate::eval("(function() { return [1, 2, 3].map(value => value + 1).join(':'); })();"),
        Ok(Value::String("2:3:4".into()))
    );
    let counters = crate::counters().unwrap();
    assert_eq!(counters.ordinary_call_attempts, 4);
    assert_eq!(counters.ordinary_call_forwardings, 0);
    let function = crate::eval("value => value + 1;").unwrap();
    let mut root = HostRoot::script();
    crate::reset_counters();
    assert_eq!(
        crate::call_function(
            function,
            Value::Undefined,
            vec![Value::Number(2.0)],
            &mut root.context(),
            false
        ),
        Ok(Value::Number(3.0))
    );
    let counters = crate::counters().unwrap();
    assert_eq!(counters.ordinary_call_attempts, 1);
    assert_eq!(counters.ordinary_call_forwardings, 1);
    assert_eq!(counters.call_env_constructions, 0);
    assert_eq!(counters.call_env_clones, 0);
    assert_eq!(counters.legacy_applyback_calls, 0);
    assert_eq!(counters.native_family_probes, 0);
    assert_eq!(counters.argument_vec_native_pop, 0);
}

#[cfg(feature = "perf-counters")]
#[test]
fn wide_numeric_roots_count_their_own_entry_separately_from_retained_children() {
    crate::reset_counters();
    assert_eq!(
        crate::eval(
            "function calc(value) { return value * value + 1; } function wrapper(object) { return calc(object.x); } wrapper({x: 3});"
        ),
        Ok(Value::Number(10.0))
    );
    let counts = crate::counters().unwrap();
    assert!(counts.numeric_plan_roots > 0);
    assert_eq!(counts.ordinary_call_attempts, 2);
    // Identities are minted on exposure; neither call exposes a scope.
    assert_eq!(counts.typed_variable_identities, 0);
}

#[test]
fn unrelated_closures_do_not_inherit_an_unused_or_fixed_self_scope() {
    let Value::Function(factory) =
        crate::eval("(function factory() { return function(value) { return value + 1; }; });")
            .unwrap()
    else {
        panic!("factory");
    };
    assert!(!factory.has_name_binding);
    let mut root = HostRoot::script();
    let Value::Function(child) = crate::call_function(
        Value::Function(factory),
        Value::Undefined,
        Vec::new(),
        &mut root.context(),
        false,
    )
    .unwrap() else {
        panic!("child");
    };
    assert!(child.lexical_scope().is_none());
    let Value::Object(result) = crate::eval(
        "(function factory() { return { self: factory, child: function(value) { return value + 1; } }; })();"
    ).unwrap() else { panic!("result"); };
    let Value::Function(child) = result.own_property("child").unwrap().value else {
        panic!("child");
    };
    assert!(child.lexical_scope().is_none());
    assert_eq!(
        crate::call_function(
            Value::Function(child),
            Value::Undefined,
            vec![Value::Number(4.0)],
            &mut root.context(),
            false
        ),
        Ok(Value::Number(5.0))
    );
}

#[test]
fn a_descendant_free_self_reference_retains_the_original_name_cell() {
    let Value::Function(factory) =
        crate::eval("(function factory() { return function() { return factory; }; });").unwrap()
    else {
        panic!("factory");
    };
    assert!(factory.has_name_binding);
    let mut root = HostRoot::script();
    let child = crate::call_function(
        Value::Function(factory.clone()),
        Value::Undefined,
        Vec::new(),
        &mut root.context(),
        false,
    )
    .unwrap();
    assert_eq!(
        crate::call_function(
            child,
            Value::Undefined,
            Vec::new(),
            &mut root.context(),
            false
        ),
        Ok(Value::Function(factory))
    );
}

#[test]
fn an_inactive_shadow_does_not_hide_a_descendant_free_self_reference() {
    assert_eq!(
        crate::eval(
            "(function factory() { return function() { const result = factory; { let factory = 1; } return typeof result; }; })()();"
        ),
        Ok(Value::String("function".into()))
    );
}

#[test]
fn declarative_deletion_keeps_self_captures_and_tdz_as_present_bindings() {
    for source in [
        "(function named() { return delete named; })();",
        "(function() { let value = 1; return () => delete value; })()();",
        "(function() { return delete value; let value = 1; })();",
        "(function named() { return () => delete named; })()();",
    ] {
        assert_eq!(crate::eval(source), Ok(Value::Boolean(false)), "{source}");
    }
    assert_eq!(
        crate::eval("(function() { return delete neverThere; })();"),
        Ok(Value::Boolean(true))
    );
}

#[test]
fn class_consumers_retain_only_the_fixed_names_their_code_can_select() {
    let Value::Function(constructor) = crate::eval("(function factory() { const self = factory; return class { method(value) { return value + 1; } }; })();").unwrap() else { panic!("constructor"); };
    assert!(constructor.lexical_scope().is_none());
    let Value::Object(prototype) = constructor.own_property("prototype").unwrap().value else {
        panic!("prototype");
    };
    let Value::Function(method) = prototype.own_property("method").unwrap().value else {
        panic!("method");
    };
    assert!(method.lexical_scope().is_none());
    assert_eq!(
        crate::eval(
            "(function factory() { const C = class { method() { return factory; } }; return new C().method() === factory; })();"
        ),
        Ok(Value::Boolean(true))
    );
}
