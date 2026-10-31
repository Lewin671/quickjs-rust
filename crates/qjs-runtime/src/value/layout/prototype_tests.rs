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

#[test]
fn compact_authorities_share_weak_edge_boxes_without_pinning_prototypes() {
    let domain = AtomDomain::new();
    let _owner = PropertyOwner::new(domain.clone());
    let parent = object(&domain, None);
    let weak = parent.downgrade();
    let mut first = NamedStorage::new(&domain, Some(Prototype::Object(parent.clone())), 0x100);
    let second = NamedStorage::new(&domain, Some(Prototype::Object(parent.clone())), 0x100);
    let (
        NamedStorage::Empty {
            prototype: Some(a), ..
        },
        NamedStorage::Empty {
            prototype: Some(b), ..
        },
    ) = (&first, &second)
    else {
        panic!("compact authorities");
    };
    assert!(
        Rc::ptr_eq(a, b),
        "compact instances must reuse their edge allocation"
    );
    first.store_mut();
    let third = NamedStorage::new(&domain, Some(Prototype::Object(parent.clone())), 0x100);
    let (
        NamedStorage::Empty {
            prototype: Some(b), ..
        },
        NamedStorage::Empty {
            prototype: Some(c), ..
        },
    ) = (&second, &third)
    else {
        panic!("compact authorities");
    };
    assert!(Rc::ptr_eq(b, c));
    let guard = second.version().unwrap();
    drop(parent);
    drop(first);
    drop(second);
    drop(third);
    assert!(guard.is_alive(), "binding remains available for reuse");
    assert!(
        weak.upgrade().is_none(),
        "the root memo must not pin the prototype"
    );
    for _ in 0..70 {
        let mut temporary = NamedStore::empty_with_prototype(
            &domain,
            Some(Prototype::Object(object(&domain, None))),
            0x100,
        );
        temporary.materialize_layout();
    }
    assert!(!guard.is_alive(), "dead prototype bindings are swept");
    assert_eq!(std::mem::size_of::<RefCell<NamedStorage>>(), 32);
}

#[test]
fn root_reuse_survives_empty_receiver_churn_without_retaining_dead_bindings() {
    let domain = AtomDomain::new();
    let owner = PropertyOwner::new(domain.clone());
    let parent = object(&domain, None);
    let make =
        || NamedStore::empty_with_prototype(&domain, Some(Prototype::Object(parent.clone())), 0);
    let first = make().version();
    assert!(first.is_alive());
    for _ in 0..1000 {
        let store = NamedStore::empty_with_prototype(
            &domain,
            Some(Prototype::Object(object(&domain, None))),
            0,
        );
        store.version();
    }
    assert_eq!(
        first,
        make().version(),
        "live prototype keeps its canonical root"
    );
    assert!(owner.0.roots.borrow().len() < 100);
    assert!(owner.retained_nodes() < 100);
}

#[test]
fn empty_deferred_proofs_use_only_the_general_guard_until_published() {
    let domain = AtomDomain::new();
    let _owner = PropertyOwner::new(domain.clone());
    let parent = object(&domain, None);
    let mut receiver =
        NamedStore::empty_with_prototype(&domain, Some(Prototype::Object(parent)), 0);
    let proof = receiver.version();
    assert!(receiver.matches_version(&proof));
    assert!(!receiver.matches_shared_identity(&proof));
    assert!(!receiver.matches_own_version(&proof));
    receiver.materialize_layout();
    assert!(receiver.matches_shared_identity(&proof));
    assert!(receiver.matches_own_version(&proof));
}

#[test]
fn dictionary_versions_do_not_allocate_receiver_cold_data() {
    let domain = AtomDomain::new();
    let owner = PropertyOwner::new(domain.clone());
    let mut store = owner.empty_store();
    for i in 0..140 {
        store.insert_text(&format!("p{i}"), Property::enumerable(Value::Null));
    }
    assert!(store.is_dictionary());
    assert!(store.cold.is_unallocated());
    let proof = store.version();
    assert!(store.matches_own_version(&proof));
    store.insert_text("new", Property::enumerable(Value::Null));
    assert!(!store.matches_own_version(&proof));
    assert!(store.cold.is_unallocated());
}

#[test]
fn a_missing_exclusive_probe_does_not_publish_a_deferred_root() {
    let domain = AtomDomain::new();
    let _owner = PropertyOwner::new(domain.clone());
    let parent = object(&domain, None);
    let mut store = NamedStore::empty_with_prototype(&domain, Some(Prototype::Object(parent)), 0);
    let key = domain.intern("x");
    assert!(store.lookup(&key).unwrap().descriptor().is_none());
    assert!(store.deferred());
    assert!(store.cold.is_unallocated());
    store
        .lookup(&key)
        .unwrap()
        .define(Property::enumerable(Value::Null));
    assert!(!store.deferred());
    assert_eq!(store.get(&key).unwrap().unwrap().value, Value::Null);
}

#[test]
fn unobserved_owned_chains_keep_one_small_binding_and_shared_fields() {
    let domain = AtomDomain::new();
    let owner = PropertyOwner::new(domain.clone());
    let mut head = object(&domain, None);
    for i in 0..1000 {
        let next = object(&domain, Some(head.clone()));
        next.set("v".into(), Value::Number(i as f64));
        next.with_named(|s| {
            assert!(s.layout.transient);
            assert!(s.layout.links.get().is_none());
            assert!(s.cold.is_unallocated());
            assert!(!s.layout.data.borrow().index.allocated());
            if i > 0 {
                head.with_named(|p| assert!(Rc::ptr_eq(&s.layout.data, &p.layout.data)));
            }
        });
        head = next;
    }
    assert!(owner.0.roots.borrow().is_empty());
    assert_eq!(owner.0.tiny_data.borrow().len(), 1);
    assert_eq!(std::mem::size_of::<Layout>(), 80);
}

#[test]
fn equal_tiny_fields_share_data_but_never_merge_prototype_guards() {
    let domain = AtomDomain::new();
    let _owner = PropertyOwner::new(domain.clone());
    let a = object(&domain, Some(object(&domain, None)));
    let b = object(&domain, Some(object(&domain, None)));
    for (o, n) in [(&a, 1.0), (&b, 9.0)] {
        o.set("a".into(), Value::Number(n));
        o.set("b".into(), Value::Number(n + 1.0));
    }
    a.with_named(|s| {
        b.with_named(|t| {
            assert!(Rc::ptr_eq(&s.layout.data, &t.layout.data));
            assert!(!Rc::ptr_eq(&s.layout, &t.layout));
            assert!(!s.layout.data.borrow().index.allocated());
            assert!(!s.matches_version(&t.version()));
        })
    });
    assert_eq!(a.get("a"), Some(Value::Number(1.0)));
    assert_eq!(b.get("a"), Some(Value::Number(9.0)));
}

#[test]
fn observing_equal_transient_bindings_publishes_the_canonical_path() {
    let domain = AtomDomain::new();
    let _owner = PropertyOwner::new(domain.clone());
    let parent = object(&domain, None);
    let a = object(&domain, Some(parent.clone()));
    let b = object(&domain, Some(parent));
    a.set("x".into(), Value::Number(2.0));
    b.set("x".into(), Value::Number(3.0));
    for o in [&a, &b] {
        o.with_named_mut(NamedStore::materialize_layout);
    }
    a.with_named(|s| {
        b.with_named(|t| {
            assert!(Rc::ptr_eq(&s.layout, &t.layout));
            assert!(!s.layout.transient);
        })
    });
    assert_eq!(a.get("x"), Some(Value::Number(2.0)));
    assert_eq!(b.get("x"), Some(Value::Number(3.0)));
}

#[test]
fn transient_publication_is_refreshed_after_addition_and_reconfiguration() {
    let domain = AtomDomain::new();
    let _owner = PropertyOwner::new(domain.clone());
    let mut store = NamedStore::empty_with_prototype(
        &domain,
        Some(Prototype::Object(object(&domain, None))),
        0,
    );
    let a = domain.intern("a");
    let b = domain.intern("b");
    store
        .define(&a, Property::enumerable(Value::Number(1.0)))
        .unwrap();
    let old = store.version();
    store
        .define(&b, Property::enumerable(Value::Number(2.0)))
        .unwrap();
    assert!(!store.matches_version(&old));
    let before_attributes = store.version();
    store
        .define(&a, Property::data(Value::Number(4.0), false, false, true))
        .unwrap();
    assert!(!store.matches_version(&before_attributes));
    store.materialize_layout();
    assert_eq!(store.len(), 2);
    let descriptor = store.get(&a).unwrap().unwrap();
    assert_eq!(descriptor.value, Value::Number(4.0));
    assert!(!descriptor.writable && !descriptor.enumerable);
}
