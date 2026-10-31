//! Scope identity and declaration ownership without executing JavaScript.
use super::*;
use crate::{Value, value::name_hash::NameMap};
use std::rc::Rc;

fn cursor(parent: Option<Rc<ScopeLease>>, activation: u64) -> ScopeCursor {
    let descriptor = ScopeDescriptor {
        id: ScopeId(0),
        parent: None,
        kind: ScopeKind::FunctionBody,
        declarations: Box::default(),
        lookup: NameMap::default(),
        variable_target: ScopeId(0),
    };
    let root = Rc::new(ScopeLease {
        descriptor: ScopeDescriptorRef {
            owner: Rc::new(ScopeDescriptors(vec![descriptor].into_boxed_slice())),
            id: ScopeId(0),
        },
        parent,
        cells: Box::default(),
        object: None,
        own_vars: Some(Rc::new(VarHolder::new(VariableEnvIdentity {
            activation,
            boundary: ScopeId(0),
        }))),
    });
    ScopeCursor {
        head: root.clone(),
        variable: root,
    }
}

#[test]
fn own_declarations_do_not_select_an_outer_homonym() {
    let outer = cursor(None, 1);
    let outside = outer.declare_var("value");
    outside.assign(Value::Number(1.0), false).unwrap();
    let inner = cursor(Some(outer.head.clone()), 2);
    let inside = inner.declare_var("value");
    inside.assign(Value::Number(2.0), false).unwrap();
    assert!(!outside.ptr_eq(&inside));
    assert_eq!(outside.read(), Ok(Value::Number(1.0)));
    assert_eq!(
        inner.lookup("value").unwrap().read(),
        Ok(Value::Number(2.0))
    );
    assert!(inside.ptr_eq(&inner.declare_var("value")));
}

#[test]
fn parent_additions_remain_live_and_deletion_reveals_the_parent() {
    let outer = cursor(None, 1);
    let inner = cursor(Some(outer.head.clone()), 2);
    assert!(inner.lookup("later").is_none());
    let parent = outer.declare_var("later");
    parent.assign(Value::Number(5.0), false).unwrap();
    assert!(inner.lookup("later").unwrap().cell.ptr_eq(&parent));
    let child = inner.declare_var("later");
    child.assign(Value::Number(2.0), false).unwrap();
    assert_eq!(
        inner.lookup("later").unwrap().read(),
        Ok(Value::Number(2.0))
    );
    assert!(inner.variable.own_vars.as_ref().unwrap().delete("later"));
    assert!(inner.lookup("later").unwrap().cell.ptr_eq(&parent));
    assert_eq!(parent.read(), Ok(Value::Number(5.0)));
}

#[test]
fn a_preselected_cell_survives_deletion_and_never_reselects_the_parent() {
    let outer = cursor(None, 1);
    let parent = outer.declare_var("value");
    parent.assign(Value::Number(1.0), false).unwrap();
    let inner = cursor(Some(outer.head.clone()), 2);
    inner
        .declare_var("value")
        .assign(Value::Number(2.0), false)
        .unwrap();
    let selected = inner.lookup("value").unwrap();
    assert!(inner.variable.own_vars.as_ref().unwrap().delete("value"));
    selected.assign(Value::Number(9.0), false).unwrap();
    assert_eq!(selected.read(), Ok(Value::Number(9.0)));
    assert_eq!(parent.read(), Ok(Value::Number(1.0)));
    assert!(inner.lookup("value").unwrap().cell.ptr_eq(&parent));
}

#[test]
fn import_assignment_does_not_read_the_exporters_tdz() {
    let cell = BindingCell::with_metadata(
        BindingMetaRef::dynamic("exported".into(), true, false),
        None,
    );
    let import = CellReference {
        cell: cell.clone(),
        access: AccessPolicy::Import,
    };
    assert!(
        import
            .assign(Value::Number(2.0), false)
            .unwrap_err()
            .message
            .starts_with("TypeError")
    );
    assert!(!cell.is_initialized());
    assert!(
        import
            .read()
            .unwrap_err()
            .message
            .starts_with("ReferenceError")
    );
    cell.initialize(Value::Number(3.0)).unwrap();
    assert_eq!(import.read(), Ok(Value::Number(3.0)));
    assert!(import.assign(Value::Number(4.0), true).is_err());
    assert_eq!(cell.read(), Ok(Value::Number(3.0)));
}

#[test]
fn self_scope_retains_its_cell_outside_the_call_variable_holder() {
    let plan = FunctionNamePlan::compile("self".into());
    let cell = BindingCell::with_metadata(plan.binding.clone(), None);
    cell.initialize(Value::Number(3.0)).unwrap();
    let name = ScopeCursor::with_function_name(None, cell.clone(), &plan);
    let body = cursor(Some(name.head.clone()), 42);
    let selected = body.lookup("self").unwrap();
    assert!(selected.cell.ptr_eq(&cell));
    let own = body.declare_var("self");
    own.assign(Value::Number(8.0), false).unwrap();
    assert!(!own.ptr_eq(&cell));
    assert!(body.lookup("self").unwrap().cell.ptr_eq(&own));
    selected.assign(Value::Number(9.0), false).unwrap();
    assert_eq!(cell.read(), Ok(Value::Number(3.0)));
    assert!(selected.assign(Value::Number(9.0), true).is_err());
    assert_eq!(own.read(), Ok(Value::Number(8.0)));
    let other = BindingCell::with_metadata(plan.binding.clone(), Some(Value::Number(4.0)));
    let other_scope = ScopeCursor::with_function_name(None, other, &plan);
    assert!(Rc::ptr_eq(
        &name.head.descriptor.owner,
        &other_scope.head.descriptor.owner
    ));
}

#[test]
fn selected_holder_recreates_its_own_deleted_binding_without_selecting_a_parent() {
    let outer = cursor(None, 1);
    let parent = outer.declare_var("value");
    parent.assign(Value::Number(1.0), false).unwrap();
    let inner = cursor(Some(outer.head.clone()), 2);
    let old = inner.declare_var("value");
    old.assign(Value::Number(2.0), false).unwrap();
    let mut services_root =
        crate::context::HostRoot::new(crate::context::new_realm(std::collections::HashMap::new()));
    let mut services = services_root.context();
    let selected = inner.resolve("value", false, &mut services).unwrap();
    assert!(selected.delete(&mut services).unwrap());
    selected
        .assign(Value::Number(9.0), false, &mut services)
        .unwrap();
    let current = inner.declare_var("value");
    assert!(!current.ptr_eq(&old));
    assert_eq!(current.read(), Ok(Value::Number(9.0)));
    assert_eq!(old.read(), Ok(Value::Number(2.0)));
    assert_eq!(parent.read(), Ok(Value::Number(1.0)));
}

#[test]
fn selected_holder_uses_redeclared_own_binding_and_strict_missing_does_not_create() {
    let scope = cursor(None, 1);
    scope
        .declare_var("value")
        .assign(Value::Number(1.0), false)
        .unwrap();
    let mut services_root =
        crate::context::HostRoot::new(crate::context::new_realm(std::collections::HashMap::new()));
    let mut services = services_root.context();
    let selected = scope.resolve("value", false, &mut services).unwrap();
    assert!(selected.delete(&mut services).unwrap());
    assert!(
        selected
            .assign(Value::Number(8.0), true, &mut services)
            .is_err()
    );
    assert!(!scope.variable.own_vars.as_ref().unwrap().contains("value"));
    let replacement = scope.declare_var("value");
    selected
        .assign(Value::Number(9.0), false, &mut services)
        .unwrap();
    assert_eq!(replacement.read(), Ok(Value::Number(9.0)));
    assert_eq!(selected.read(&mut services), Ok(Value::Number(9.0)));
}
