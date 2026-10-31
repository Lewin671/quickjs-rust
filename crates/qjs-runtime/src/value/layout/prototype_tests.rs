//! Lazy metadata, graph re-arming, reparenting and shared holder shapes.
use super::*;
use crate::ObjectRef;

fn object(domain: &AtomDomain, parent: Option<ObjectRef>) -> ObjectRef {
    ObjectRef::with_prototype(HashMap::new(), parent, domain)
}

#[test]
fn unused_prototype_edges_allocate_no_roots_states_or_descriptor_copies() {
    let domain = AtomDomain::new();
    let owner = PropertyOwner::new(domain.clone());
    let parent = object(&domain, None);
    parent.set("x".into(), Value::Number(1.0));
    let original = parent.with_named(|s| s.layout.clone());
    let child = object(&domain, Some(parent.clone()));
    let grandchild = object(&domain, Some(child.clone()));
    assert!(owner.0.roots.borrow().is_empty());
    parent.with_named(|s| {
        assert!(Rc::ptr_eq(&s.layout, &original));
        assert!(s.cold.is_unallocated());
        assert!(!s.layout.dictionary);
    });
    // The empty receiver retains its edge inline and shares the neutral seed.
    child.with_named(|s| {
        assert!(s.layout.deferred_seed);
        assert!(s.cold.is_unallocated());
    });
    drop(grandchild);
}

#[test]
fn new_descendant_tokens_rearm_invalid_ancestors_and_track_reparenting() {
    let domain = AtomDomain::new();
    let _owner = PropertyOwner::new(domain.clone());
    let root = object(&domain, None);
    root.set("x".into(), Value::Number(1.0));
    let middle = object(&domain, Some(root.clone()));
    let leaf = object(&domain, Some(middle.clone()));
    let first = leaf.with_named(NamedStore::chain_dependency).unwrap();
    root.set("y".into(), Value::Null);
    assert!(!first.is_valid());
    // Install a previously untouched path under the invalid middle/root.
    let new_middle = object(&domain, Some(middle.clone()));
    let new_leaf = object(&domain, Some(new_middle.clone()));
    let fresh = new_leaf.with_named(NamedStore::chain_dependency).unwrap();
    root.set("z".into(), Value::Null);
    assert!(!fresh.is_valid());
    let before_reparent = leaf.with_named(NamedStore::chain_dependency).unwrap();
    let other = object(&domain, None);
    other.set("x".into(), Value::Number(9.0));
    middle.set_prototype(Some(other.clone())).unwrap();
    assert!(!before_reparent.is_valid());
    let after_reparent = leaf.with_named(NamedStore::chain_dependency).unwrap();
    root.set("x".into(), Value::Null);
    assert!(after_reparent.is_valid());
    other.set("x".into(), Value::Number(10.0));
    assert!(!after_reparent.is_valid());
}

#[test]
fn repeated_reparenting_compacts_obsolete_registrations_even_on_return_to_parent() {
    let a = PrototypeState::new(Weak::new(), true);
    let b = PrototypeState::new(Weak::new(), true);
    let child = PrototypeState::new(Rc::downgrade(&a), true);
    for _ in 0..1_000 {
        PrototypeState::reparent(&child, Rc::downgrade(&b), true);
        PrototypeState::reparent(&child, Rc::downgrade(&a), true);
    }
    assert!(a.users.borrow().len() < 70);
    assert!(b.users.borrow().len() < 70);
    let token = child.token().unwrap();
    PrototypeState::invalidate(&b);
    assert!(token.is_valid());
    PrototypeState::invalidate(&a);
    assert!(!token.is_valid());
}

#[test]
fn shared_append_recipes_invalidate_holders_that_later_become_prototypes() {
    let domain = AtomDomain::new();
    let owner = PropertyOwner::new(domain.clone());
    let mut producer = owner.empty_store();
    let before = producer.version();
    producer.insert_text("x", Property::enumerable(Value::Null));
    let recipe = producer.append_recipe(before, 0).unwrap();
    let holder = object(&domain, None);
    let child = object(&domain, Some(holder.clone()));
    let token = child.with_named(NamedStore::chain_dependency).unwrap();
    assert!(holder.with_named_mut(|s| recipe.apply(s, Value::Null)));
    assert!(!token.is_valid());
    holder.with_named(|s| assert!(!s.layout.dictionary));
}

#[test]
fn first_dependency_and_invalidation_walk_deep_chains_iteratively() {
    let domain = AtomDomain::new();
    let _owner = PropertyOwner::new(domain.clone());
    let root = object(&domain, None);
    let mut head = root.clone();
    for _ in 0..16_000 {
        head = object(&domain, Some(head));
    }
    let token = head.with_named(NamedStore::chain_dependency).unwrap();
    let state = head
        .prototype_slot()
        .unwrap()
        .enter_prototype_mode()
        .unwrap();
    root.set("x".into(), Value::Null);
    assert!(!token.is_valid());
    let visits = state.visits.get();
    for _ in 0..400 {
        root.with_named(NamedStore::invalidate_dependencies);
    }
    assert_eq!(
        state.visits.get(),
        visits,
        "unarmed subtree must be skipped"
    );
    let fresh = head.with_named(NamedStore::chain_dependency).unwrap();
    root.set("y".into(), Value::Null);
    assert!(!fresh.is_valid());
    assert_eq!(state.visits.get(), visits + 1);
}

#[test]
fn publishing_a_deferred_guard_preserves_its_identity_and_retires_it_on_reparent() {
    let domain = AtomDomain::new();
    let _owner = PropertyOwner::new(domain.clone());
    let parent = object(&domain, None);
    let mut store = NamedStore::empty_with_prototype(&domain, Some(Prototype::Object(parent)), 0);
    assert!(store.deferred());
    let guard = store.version();
    assert!(store.matches_version(&guard));
    store.materialize_layout();
    assert!(!store.deferred());
    assert!(store.matches_version(&guard));
    assert!(store.cold.guard().is_none());
    store.set_prototype(Some(Prototype::Object(object(&domain, None))));
    assert!(!store.matches_version(&guard));
}

#[test]
fn deferred_seeds_cannot_match_a_null_prototype_guard_or_another_prototype() {
    let domain = AtomDomain::new();
    let owner = PropertyOwner::new(domain.clone());
    let null_guard = owner.empty_store().version();
    let a = object(&domain, None);
    let b = object(&domain, None);
    let first = NamedStore::empty_with_prototype(&domain, Some(Prototype::Object(a)), 0);
    let second = NamedStore::empty_with_prototype(&domain, Some(Prototype::Object(b)), 0);
    assert!(Rc::ptr_eq(&first.layout, &second.layout));
    assert!(!first.matches_version(&null_guard));
    assert!(!first.matches_shared_identity(&null_guard));
    let first_guard = first.version();
    assert!(!second.matches_version(&first_guard));
    let second_guard = second.version();
    assert!(!first.matches_version(&second_guard));
}
