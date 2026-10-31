use super::{
    CallEnv, DYNAMIC_FUNCTION_REALM_GLOBAL, DynamicBindings, FrameBindingValue, FrameBindings,
    Upvalue, new_realm,
};
use crate::{ObjectRef, Value};
use std::{collections::HashMap, rc::Rc};

#[test]
fn derived_this_forks_share_only_the_implicit_cell() {
    let domain = crate::property::atom::AtomDomain::new();
    let mut env = CallEnv::detached();
    let this = Upvalue::new(Value::Function(
        crate::Function::uninitialized_lexical_marker(),
    ));
    let local = Upvalue::new(Value::Number(1.0));
    env.insert(
        crate::ACTIVE_CONSTRUCTOR_BINDING.to_owned(),
        Value::Undefined,
    );
    env.insert_frame_cell("this".to_owned(), this.clone());
    env.insert_frame_cell("local".to_owned(), local.clone());
    let clone = env.clone();
    let fork = env.fork_current_frame_values();
    assert!(this.ptr_eq(&clone.derived_this_cell().unwrap()));
    assert!(this.ptr_eq(&fork.derived_this_cell().unwrap()));
    assert!(!local.ptr_eq(&fork.frame_binding_cell("local").unwrap()));
    this.set(Value::Object(ObjectRef::new(HashMap::new(), &domain)));
    assert_eq!(env.get_local("this"), fork.get_local("this"));
    local.set(Value::Number(2.0));
    assert_eq!(fork.get_local("local"), Some(Value::Number(1.0)));
}

#[test]
fn ordinary_undefined_this_is_not_a_derived_cell() {
    let env = CallEnv::detached();
    let this = Upvalue::undefined();
    env.insert_frame_cell("this".to_owned(), this.clone());
    assert!(env.derived_this_cell().is_none());
    let fork = env.fork_current_frame_values();
    assert!(!this.ptr_eq(&fork.frame_binding_cell("this").unwrap()));
    assert_eq!(fork.get_local("this"), Some(Value::Undefined));
}

#[test]
fn derived_this_insert_does_not_write_an_inherited_activations_cell() {
    let domain = crate::property::atom::AtomDomain::new();
    let mut env = CallEnv::detached();
    let this = Upvalue::new(Value::Function(
        crate::Function::uninitialized_lexical_marker(),
    ));
    let other = Upvalue::new(Value::Function(
        crate::Function::uninitialized_lexical_marker(),
    ));
    let bindings = DynamicBindings::new();
    bindings.insert_cell("this".to_owned(), other.clone());
    env.set_deopt_bindings(bindings);
    env.insert_frame(
        crate::ACTIVE_CONSTRUCTOR_BINDING.to_owned(),
        Value::Undefined,
    );
    env.insert_frame_cell("this".to_owned(), this.clone());
    let receiver = Value::Object(ObjectRef::new(HashMap::new(), &domain));
    env.insert("this".to_owned(), receiver.clone());
    assert_eq!(this.get(), receiver);
    assert!(other.with_value(Value::is_uninitialized_lexical_marker));
    assert!(this.ptr_eq(&env.derived_this_cell().unwrap()));
}

#[test]
fn frame_binding_promotes_to_a_cell_only_when_identity_is_requested() {
    let bindings = FrameBindings::default();
    bindings.insert("value".to_owned(), Value::Number(1.0));
    assert!(matches!(
        &bindings.0.borrow()[0].1,
        FrameBindingValue::Direct(Value::Number(1.0))
    ));

    let cell = bindings.cell("value").expect("binding should promote");
    assert!(matches!(
        &bindings.0.borrow()[0].1,
        FrameBindingValue::Cell(_)
    ));
    cell.set(Value::Number(2.0));
    assert!(matches!(bindings.get("value"), Some(Value::Number(2.0))));
    assert!(cell.ptr_eq(&bindings.cell("value").expect("cell is stable")));
}

#[test]
fn function_frame_eval_conflict_sets_are_copy_on_write() {
    let mut caller = CallEnv::new(new_realm(HashMap::new()));
    caller.mark_catch_binding("caller_catch".to_owned());
    caller.mark_direct_eval_var_conflict("caller_lexical".to_owned());

    let mut callee = caller.new_function_frame();
    callee.unmark_catch_binding("caller_catch");
    callee.mark_catch_binding("callee_catch".to_owned());
    callee.clear_direct_eval_var_conflicts();
    callee.mark_direct_eval_var_conflict("callee_lexical".to_owned());

    assert!(caller.is_catch_binding("caller_catch"));
    assert!(!caller.is_catch_binding("callee_catch"));
    assert!(caller.is_direct_eval_var_conflict("caller_lexical"));
    assert!(!caller.is_direct_eval_var_conflict("callee_lexical"));
    assert!(!callee.is_catch_binding("caller_catch"));
    assert!(callee.is_catch_binding("callee_catch"));
    assert!(!callee.is_direct_eval_var_conflict("caller_lexical"));
    assert!(callee.is_direct_eval_var_conflict("callee_lexical"));
}

#[test]
fn direct_leaf_frames_share_empty_metadata_until_mutation() {
    let caller = CallEnv::new(new_realm(HashMap::new()));
    let imports: super::ModuleImports = Rc::new(Default::default());
    let mut first = caller.new_direct_leaf_function_frame(Rc::clone(&imports));
    let second = caller.new_direct_leaf_function_frame(Rc::clone(&imports));

    assert!(first.catch_bindings.is_none() && second.catch_bindings.is_none());
    assert!(first.direct_eval_var_conflicts.is_none());
    assert!(Rc::ptr_eq(&first.module_imports, &imports));
    assert!(Rc::ptr_eq(&second.module_imports, &imports));

    first.mark_catch_binding("caught".to_owned());
    assert!(first.is_catch_binding("caught"));
    assert!(!second.is_catch_binding("caught"));
    assert!(!first.is_direct_eval_var_conflict("caught"));
}

#[test]
fn unrelated_frame_binding_does_not_disable_an_authoritative_slot() {
    let mut env = CallEnv::new(new_realm(HashMap::new()));
    env.insert("this".to_owned(), Value::Undefined);

    assert!(env.slot_is_authoritative("value"));
    assert!(!env.slot_is_authoritative("this"));

    env.set_deopt_bindings(DynamicBindings::new());
    assert!(!env.slot_is_authoritative("value"));
}

#[test]
fn dynamic_cell_overlay_skips_only_the_identical_cell() {
    let bindings = DynamicBindings::new();
    let first = Upvalue::new(Value::Number(1.0));
    assert!(bindings.overlay_cell("value", &first));
    assert!(!bindings.overlay_cell("value", &first));

    first.set(Value::Number(2.0));
    assert!(matches!(bindings.get("value"), Some(Value::Number(2.0))));

    let shadow = Upvalue::new(Value::Number(2.0));
    assert!(bindings.overlay_cell("value", &shadow));
    assert!(
        bindings
            .cell("value")
            .is_some_and(|binding| binding.ptr_eq(&shadow))
    );
    assert!(!bindings.overlay_cell("value", &shadow));
}

#[test]
fn visible_local_scans_skip_shadowed_bindings() {
    let mut env = CallEnv::new(new_realm(HashMap::new()));
    let deopt = DynamicBindings::new();
    deopt.insert("deopt-only".to_owned(), Value::Number(3.0));
    deopt.insert("shadowed".to_owned(), Value::Number(1.0));
    env.set_deopt_bindings(deopt);
    env.frame_bindings
        .push("shadowed".to_owned(), Value::Number(2.0));
    env.frame_bindings
        .push("shadowed".to_owned(), Value::Number(4.0));
    env.frame_bindings
        .push("frame-only".to_owned(), Value::Number(5.0));

    let mut visited = Vec::new();
    env.for_each_visible_local_value(|value| {
        if let Value::Number(number) = value {
            visited.push(*number as i32);
        }
    });
    visited.sort_unstable();

    assert_eq!(visited, vec![3, 4, 5]);

    let names = env.visible_local_names();
    assert_eq!(names.len(), 3);
    assert!(names.contains("deopt-only"));
    assert!(names.contains("shadowed"));
    assert!(names.contains("frame-only"));

    let entries = env
        .visible_local_entries()
        .into_iter()
        .collect::<HashMap<_, _>>();
    assert_eq!(entries.get("deopt-only"), Some(&Value::Number(3.0)));
    assert_eq!(entries.get("shadowed"), Some(&Value::Number(4.0)));
    assert_eq!(entries.get("frame-only"), Some(&Value::Number(5.0)));
}

#[test]
fn module_import_routes_share_until_environment_mutation() {
    let mut env = CallEnv::new(new_realm(HashMap::new()));
    let cloned = env.clone();
    assert!(Rc::ptr_eq(&env.module_imports, &cloned.module_imports));

    env.set_module_import(
        "local".to_owned(),
        DynamicBindings::new(),
        "exported".to_owned(),
    );
    assert!(env.has_module_import("local"));
    assert!(!cloned.has_module_import("local"));
    assert!(!Rc::ptr_eq(&env.module_imports, &cloned.module_imports));
}

#[test]
fn dynamic_function_realm_cache_tracks_bulk_and_incremental_mutations() {
    let domain = crate::property::atom::AtomDomain::new();
    let first = ObjectRef::new(HashMap::new(), &domain);
    let mut bindings = HashMap::new();
    bindings.insert(
        DYNAMIC_FUNCTION_REALM_GLOBAL.to_owned(),
        Value::Object(first.clone()),
    );
    let realm = new_realm(bindings);
    assert!(
        realm
            .dynamic_function_realm_global()
            .is_some_and(|global| global.ptr_eq(&first))
    );

    realm.insert_value(DYNAMIC_FUNCTION_REALM_GLOBAL.to_owned(), Value::Undefined);
    realm.refresh_dynamic_function_realm_global();
    assert!(realm.dynamic_function_realm_global().is_none());

    let second = ObjectRef::new(HashMap::new(), &domain);
    let env = CallEnv::new(Rc::clone(&realm));
    env.insert_realm(
        DYNAMIC_FUNCTION_REALM_GLOBAL.to_owned(),
        Value::Object(second.clone()),
    );
    assert!(
        realm
            .dynamic_function_realm_global()
            .is_some_and(|global| global.ptr_eq(&second))
    );

    env.remove_realm(DYNAMIC_FUNCTION_REALM_GLOBAL);
    assert!(realm.dynamic_function_realm_global().is_none());
}
