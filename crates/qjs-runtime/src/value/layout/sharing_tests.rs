//! Regression proofs for sharing, prefix lookup, retention and integrity edits.
use super::*;
fn data() -> Property {
    Property::enumerable(Value::Number(1.0))
}
fn owner() -> PropertyOwner {
    PropertyOwner::new(AtomDomain::new())
}

#[test]
fn branches_and_prefixes_never_expose_the_shared_tail() {
    let owner = owner();
    let a = owner.domain().intern("a");
    let b = owner.domain().intern("b");
    let c = owner.domain().intern("c");
    let mut prefix = owner.empty_store();
    prefix.define(&a, data()).unwrap();
    let mut longer = owner.empty_store();
    longer.define(&a, data()).unwrap();
    longer.define(&b, data()).unwrap();
    assert!(Rc::ptr_eq(&prefix.layout.data, &longer.layout.data));
    assert!(prefix.get(&b).unwrap().is_none());
    prefix.define(&c, data()).unwrap();
    assert!(!Rc::ptr_eq(&prefix.layout.data, &longer.layout.data));
    assert_eq!(prefix.keys(), [a.clone(), c]);
    assert_eq!(longer.keys(), [a, b]);
}

#[test]
fn bulk_integrity_preserves_accessors_and_shares_shapes() {
    for frozen in [false, true] {
        let owner = owner();
        let a = owner.domain().intern("a");
        let b = owner.domain().intern("b");
        let mut stores = Vec::new();
        for n in 0..3 {
            let mut store = owner.empty_store();
            store.define(&a, data()).unwrap();
            store
                .define(
                    &b,
                    Property::accessor(Some(Value::Number(n as f64)), None, true, true),
                )
                .unwrap();
            store.transform_descriptors(frozen);
            let p = store.get(&a).unwrap().unwrap();
            assert!(!p.configurable);
            assert_eq!(p.writable, !frozen);
            let p = store.get(&b).unwrap().unwrap();
            assert!(!p.configurable);
            assert_eq!(p.getter(), Some(&Value::Number(n as f64)));
            stores.push(store);
        }
        assert!(Rc::ptr_eq(stores[0].layout(), stores[2].layout()));
    }
}

#[test]
fn last_delete_restores_parent_but_other_delete_detaches() {
    let owner = owner();
    let a = owner.domain().intern("a");
    let b = owner.domain().intern("b");
    let mut store = owner.empty_store();
    store.define(&a, data()).unwrap();
    let parent = store.layout.clone();
    store.define(&b, data()).unwrap();
    let child = store.layout.clone();
    store.remove(&b).unwrap();
    assert!(Rc::ptr_eq(&parent, store.layout()));
    store.define(&b, data()).unwrap();
    assert!(Rc::ptr_eq(&child, store.layout()));
    store.remove(&a).unwrap();
    assert!(store.layout.parent().is_none());
    assert_eq!(store.keys(), [b]);
}

#[cfg(feature = "perf-counters")]
#[test]
fn live_fanout_survives_strong_cache_eviction_without_misses() {
    let owner = owner();
    let keys: Vec<_> = (0..2000)
        .map(|n| owner.domain().intern(&format!("first{n}")))
        .collect();
    let b = owner.domain().intern("b");
    let mut live = Vec::new();
    for key in &keys {
        let mut store = owner.empty_store();
        store.define(key, data()).unwrap();
        store.define(&b, data()).unwrap();
        live.push(store);
    }
    assert!(owner.retained_nodes() < live.len());
    let before = crate::diagnostics::counters().unwrap();
    for (key, old) in keys.iter().zip(&live) {
        let mut store = owner.empty_store();
        store.define(key, data()).unwrap();
        store.define(&b, data()).unwrap();
        assert!(Rc::ptr_eq(old.layout(), store.layout()));
    }
    let after = crate::diagnostics::counters().unwrap();
    assert_eq!(
        after.property_transition_misses - before.property_transition_misses,
        0
    );
    assert_eq!(
        after.property_transition_hits - before.property_transition_hits,
        4000
    );
    assert_eq!(
        after.property_shape_field_copies - before.property_shape_field_copies,
        0
    );
}

#[cfg(feature = "perf-counters")]
#[test]
fn distinct_tail_construction_copies_no_prefix_fields() {
    let owner = owner();
    let before = crate::diagnostics::counters().unwrap();
    for n in 0..200 {
        let mut store = owner.empty_store();
        for k in 0..100 {
            let key = owner.domain().intern(&format!("{n}_{k}"));
            store.define(&key, data()).unwrap();
        }
    }
    let after = crate::diagnostics::counters().unwrap();
    assert_eq!(
        after.property_shape_field_copies - before.property_shape_field_copies,
        0
    );
    assert_eq!(
        after.property_shape_nodes_created - before.property_shape_nodes_created,
        20_000
    );
    assert!(owner.0.root.budget.bytes() <= MAX_CACHED_BYTES);
}

#[cfg(feature = "perf-counters")]
#[test]
fn repeated_freeze_copies_one_descriptor_array_and_last_delete_builds_once() {
    let owner = owner();
    let keys: Vec<_> = (0..40)
        .map(|n| owner.domain().intern(&format!("k{n}")))
        .collect();
    let tmp = owner.domain().intern("tmp");
    let before = crate::diagnostics::counters().unwrap();
    for _ in 0..1000 {
        let mut store = owner.empty_store();
        store.define(&tmp, data()).unwrap();
        store.remove(&tmp).unwrap();
        for key in &keys {
            store.define(key, data()).unwrap();
        }
        store.transform_descriptors(true);
    }
    let after = crate::diagnostics::counters().unwrap();
    assert_eq!(
        after.property_shape_field_copies - before.property_shape_field_copies,
        40
    );
    assert_eq!(
        after.property_shape_nodes_created - before.property_shape_nodes_created,
        42
    );
}

#[test]
fn retained_bytes_include_long_keys_and_shared_buffer_growth() {
    let owner = owner();
    for n in 0..1000 {
        let mut store = owner.empty_store();
        let a = owner.domain().intern("a");
        store.define(&a, data()).unwrap();
        let key = owner.domain().intern(&format!("{n}{}", "x".repeat(4096)));
        store.define(&key, data()).unwrap();
    }
    // Weak edges are swept amortized; the strong cache cannot retain historical
    // arrays beyond the byte budget even when a shared tail grows after caching.
    assert!(owner.0.root.budget.bytes() <= MAX_CACHED_BYTES);
}

#[test]
fn dead_children_remove_their_small_and_hashed_edges() {
    for width in [4, 64] {
        let owner = owner();
        for n in 0..width {
            let mut store = owner.empty_store();
            store
                .define(&owner.domain().intern(&format!("k{n}")), data())
                .unwrap();
        }
        owner.0.retained.borrow_mut().nodes.clear();
        assert!(owner.0.root.transitions_empty());
    }
}

#[cfg(feature = "perf-counters")]
#[test]
fn repeated_noop_freeze_does_not_copy_or_change_identity() {
    let owner = owner();
    let key = owner.domain().intern("x");
    let mut store = owner.empty_store();
    store.define(&key, data()).unwrap();
    store.transform_descriptors(true);
    let version = store.version();
    let before = crate::diagnostics::counters().unwrap();
    store.transform_descriptors(true);
    let after = crate::diagnostics::counters().unwrap();
    assert_eq!(version, store.version());
    assert_eq!(
        after.property_shape_field_copies - before.property_shape_field_copies,
        0
    );
    assert_eq!(
        after.property_shape_nodes_created - before.property_shape_nodes_created,
        0
    );
}

#[test]
fn expired_transition_paths_keep_pressure_without_pinning_keys() {
    let owner = owner();
    // Real live metadata exceeds the bounded keep-alive budget. Ephemeral
    // children can then expire between separate constructions.
    let mut anchor = owner.empty_store();
    let long = owner.domain().intern(&"x".repeat(MAX_CACHED_BYTES + 1));
    anchor.define(&long, data()).unwrap();
    let mut expired = Vec::new();
    for i in 0..10 {
        let key = owner.domain().intern(&format!("ephemeral{i}"));
        let mut store = owner.empty_store();
        store.define(&key, data()).unwrap();
        expired.push(Rc::downgrade(store.layout()));
    }
    assert!(expired.iter().all(|layout| layout.strong_count() == 0));
    let key = owner.domain().intern("next");
    let mut next = owner.empty_store();
    next.lookup(&key).unwrap().define_dynamic(data());
    assert!(next.is_dictionary());
    assert_eq!(next.keys(), [key]);
    assert!(anchor.get(&long).unwrap().is_some());
}
