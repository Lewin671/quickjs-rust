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
