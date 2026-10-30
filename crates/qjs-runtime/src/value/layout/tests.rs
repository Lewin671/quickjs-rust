use super::*;

fn owner() -> PropertyOwner {
    PropertyOwner::new(AtomDomain::new())
}
fn data(n: f64) -> Property {
    Property::enumerable(Value::Number(n))
}

#[test]
fn physical_budgets() {
    assert_eq!(std::mem::size_of::<KeyId>(), 4);
    assert_eq!(std::mem::size_of::<KeyLease>(), 8);
    assert_eq!(std::mem::size_of::<ShapeEntry>(), 16);
    assert_eq!(std::mem::size_of::<NamedStore>(), 48);
    assert_eq!(std::mem::size_of::<RefCell<NamedStore>>(), 56);
}

#[test]
fn repeated_creation_reuses_layout_after_objects_die() {
    let owner = owner();
    let a = owner.domain().intern("a");
    let b = owner.domain().intern("b");
    let weak = {
        let mut store = owner.empty_store();
        store.define(&a, data(1.0)).unwrap();
        store.define(&b, data(2.0)).unwrap();
        Rc::downgrade(store.layout())
    };
    let mut again = owner.empty_store();
    again.define(&a, data(3.0)).unwrap();
    again.define(&b, data(4.0)).unwrap();
    assert!(Rc::ptr_eq(&weak.upgrade().unwrap(), again.layout()));
    assert_eq!(again.get(&a).unwrap().unwrap().value, Value::Number(3.0));
}

#[test]
fn data_overwrite_preserves_validity_and_reads_live_values() {
    let owner = owner();
    let key = owner.domain().intern("x");
    let mut store = owner.empty_store();
    store.define(&key, data(1.0)).unwrap();
    let layout = store.layout().clone();
    let epoch = store.epoch();
    let slot = store.data_slot(&key).unwrap().unwrap();
    assert_eq!(store.replace_data(slot, Value::Number(9.0)), Some(true));
    store.define(&key, data(11.0)).unwrap();
    assert!(Rc::ptr_eq(&layout, store.layout()));
    assert_eq!(epoch, store.epoch());
    assert_eq!(store.load_data(slot), Some(Value::Number(11.0)));
}

#[test]
fn reconfiguration_moves_accessor_payload_out_of_data_slots() {
    let owner = owner();
    let key = owner.domain().intern("x");
    let mut store = owner.empty_store();
    store.define(&key, data(1.0)).unwrap();
    let original = store.layout().clone();
    store
        .define(
            &key,
            Property::accessor(Some(Value::Number(2.0)), None, false, true),
        )
        .unwrap();
    assert!(!Rc::ptr_eq(&original, store.layout()));
    assert_eq!(store.load_data(0), None);
    let p = store.get(&key).unwrap().unwrap();
    assert!(p.is_accessor());
    assert_eq!(p.getter(), Some(&Value::Number(2.0)));
    store
        .define(&key, Property::data(Value::Number(3.0), true, false, false))
        .unwrap();
    assert!(store.accessors.is_none());
    assert_eq!(store.replace_data(0, Value::Number(4.0)), Some(false));
    assert_eq!(store.load_data(0), Some(Value::Number(3.0)));
}

#[test]
fn order_classifies_indices_and_appends_deleted_keys() {
    let owner = owner();
    let mut store = owner.empty_store();
    let keys: Vec<_> = ["01", "4294967295", "9", "2", "x"]
        .iter()
        .map(|text| owner.domain().intern(text))
        .collect();
    for key in &keys {
        store.define(key, data(1.0)).unwrap();
    }
    store.define(&keys[0], data(2.0)).unwrap();
    assert_eq!(
        store
            .keys()
            .iter()
            .map(KeyLease::as_str)
            .collect::<Vec<_>>(),
        ["2", "9", "01", "4294967295", "x"]
    );
    store.remove(&keys[0]).unwrap();
    store.define(&keys[0], data(3.0)).unwrap();
    assert_eq!(
        store
            .keys()
            .iter()
            .map(KeyLease::as_str)
            .collect::<Vec<_>>(),
        ["2", "9", "4294967295", "x", "01"]
    );
}

#[test]
fn dictionary_churn_has_no_tombstones_and_invalidates_slots() {
    let owner = owner();
    let a = owner.domain().intern("a");
    let b = owner.domain().intern("b");
    let mut store = owner.empty_store();
    store.define(&a, data(1.0)).unwrap();
    store.define(&b, data(2.0)).unwrap();
    for _ in 0..1000 {
        let epoch = store.epoch();
        store.remove(&a).unwrap().unwrap();
        assert!(store.epoch() > epoch);
        store.define(&a, data(1.0)).unwrap();
        assert_eq!(store.len(), 2);
        assert_eq!(store.values.len(), 2);
    }
    assert!(store.layout.dictionary);
    assert!(store.layout.transitions.borrow().is_empty());
    assert_eq!(store.keys(), [b, a]);
}

#[test]
fn wide_dictionary_is_private_and_data_writes_preserve_version() {
    let owner = owner();
    let mut store = owner.empty_store();
    for n in 0..2000 {
        let key = owner.domain().intern(&format!("key{n}"));
        store.define(&key, data(n as f64)).unwrap();
    }
    assert!(store.layout.dictionary);
    let epoch = store.epoch();
    let key = owner.domain().intern("key1999");
    let slot = store.data_slot(&key).unwrap().unwrap();
    assert_eq!(store.replace_data(slot, Value::Number(7.0)), Some(true));
    assert_eq!(store.epoch(), epoch);
    assert_eq!(store.get(&key).unwrap().unwrap().value, Value::Number(7.0));
}

#[test]
fn same_integer_in_foreign_domain_cannot_address_a_slot() {
    let first = owner();
    let second = owner();
    let a = first.domain().intern("a");
    let b = second.domain().intern("b");
    assert_eq!(a.id(), b.id());
    let mut store = first.empty_store();
    store.define(&a, data(1.0)).unwrap();
    assert_eq!(store.get(&b).unwrap_err(), ForeignKey);
    assert_eq!(store.define(&b, data(2.0)), Err(ForeignKey));
}

#[test]
fn live_shapes_exceed_cache_bound_and_edges_remain_bounded() {
    let owner = owner();
    let mut live = Vec::new();
    for n in 0..5100 {
        let key = owner.domain().intern(&format!("key{n}"));
        let mut store = owner.empty_store();
        store.define(&key, data(n as f64)).unwrap();
        live.push((store, key));
    }
    assert_eq!(owner.retained_nodes(), MAX_CACHED_NODES);
    assert!(owner.0.root.transitions.borrow().len() <= MAX_EDGES);
    for (n, (store, key)) in live.iter().enumerate() {
        assert_eq!(
            store.get(key).unwrap().unwrap().value,
            Value::Number(n as f64)
        );
    }
}

#[test]
fn escaping_store_retains_domain_but_does_not_pin_owner_cache() {
    let owner = owner();
    let weak_owner = Rc::downgrade(&owner.0);
    let weak_domain = owner.domain().downgrade();
    let key = owner.domain().intern("escape");
    let mut store = owner.empty_store();
    store.define(&key, data(1.0)).unwrap();
    drop(owner);
    assert!(weak_owner.upgrade().is_none());
    assert!(weak_domain.is_live());
    let another = store.domain().intern("after owner");
    store.define(&another, data(2.0)).unwrap();
    assert_eq!(store.get(&key).unwrap().unwrap().value, Value::Number(1.0));
    drop(store);
    assert!(!weak_domain.is_live());
}

#[cfg(feature = "perf-counters")]
#[test]
fn append_hit_has_one_probe_zero_interns_and_zero_field_copies() {
    let owner = owner();
    let a = owner.domain().intern("a");
    let b = owner.domain().intern("b");
    let mut first = owner.empty_store();
    first.define(&a, data(1.0)).unwrap();
    first.define(&b, data(2.0)).unwrap();
    let mut second = owner.empty_store();
    second.define(&a, data(3.0)).unwrap();
    let before = crate::diagnostics::counters().unwrap();
    second.lookup(&b).unwrap().define(data(4.0));
    let after = crate::diagnostics::counters().unwrap();
    assert_eq!(after.property_id_probes - before.property_id_probes, 1);
    assert_eq!(
        after.property_transition_hits - before.property_transition_hits,
        1
    );
    assert_eq!(
        after.property_shape_field_copies - before.property_shape_field_copies,
        0
    );
    assert_eq!(after.property_key_interns - before.property_key_interns, 0);
    assert_eq!(
        after.property_shape_nodes_created - before.property_shape_nodes_created,
        0
    );
}

#[test]
fn symbols_follow_strings_and_duplicate_identity_keeps_first_order() {
    let owner = owner();
    let first = crate::ObjectRef::new(HashMap::new(), owner.domain());
    let second = crate::ObjectRef::new(HashMap::new(), owner.domain());
    let a = owner.domain().intern_symbol(&first);
    let b = owner.domain().intern_symbol(&second);
    let text = owner.domain().intern("text");
    let index = owner.domain().intern("0");
    let mut store = owner.empty_store();
    for key in [&a, &text, &b, &index] {
        store.define(key, data(1.0)).unwrap();
    }
    store.define(&a, data(2.0)).unwrap();
    assert_eq!(
        store.keys(),
        [index.clone(), text.clone(), a.clone(), b.clone()]
    );
    store.remove(&a).unwrap().unwrap();
    store.define(&a, data(3.0)).unwrap();
    assert_eq!(store.keys(), [index, text, b, a]);
}

#[test]
fn exhausted_epoch_publishes_a_new_identity_before_restarting_version() {
    let owner = owner();
    let key = owner.domain().intern("x");
    let mut store = owner.empty_store();
    store.define(&key, data(1.0)).unwrap();
    let old = store.layout().clone();
    store.epoch = u64::MAX;
    store.remove(&key).unwrap().unwrap();
    assert!(!Rc::ptr_eq(&old, store.layout()));
    assert_eq!(store.epoch(), 0);
    assert!(store.layout.dictionary);
}

#[test]
fn owner_attachment_shares_one_cache_per_live_domain() {
    let domain = AtomDomain::new();
    let first = PropertyOwner::new(domain.clone());
    let second = PropertyOwner::new(domain.clone());
    assert!(Rc::ptr_eq(&first.0, &second.0));
    let weak = Rc::downgrade(&first.0);
    drop(first);
    drop(second);
    assert!(weak.upgrade().is_none());
    let fresh = PropertyOwner::new(domain);
    assert!(fresh.0.root.entries.is_empty());
}

#[test]
fn width_boundary_enters_dictionary_on_the_first_property_over_limit() {
    let owner = owner();
    let mut store = owner.empty_store();
    for n in 0..DICTIONARY_WIDTH {
        let key = owner.domain().intern(&format!("key{n}"));
        store.define(&key, data(n as f64)).unwrap();
    }
    assert!(!store.layout.dictionary);
    let next = owner.domain().intern("over limit");
    store.define(&next, data(9.0)).unwrap();
    assert!(store.layout.dictionary);
}

#[test]
fn dictionary_swap_removal_moves_accessor_slots_but_preserves_key_order() {
    let owner = owner();
    let keys: Vec<_> = (0..129)
        .map(|n| owner.domain().intern(&format!("key{n}")))
        .collect();
    let mut store = owner.empty_store();
    for key in &keys[..128] {
        store.define(key, data(1.0)).unwrap();
    }
    store
        .define(
            &keys[128],
            Property::accessor(
                Some(Value::Number(7.0)),
                Some(Value::Number(8.0)),
                true,
                true,
            ),
        )
        .unwrap();
    store.remove(&keys[10]).unwrap().unwrap();
    assert_eq!(store.slot_key(10), Some(&keys[128]));
    let accessor = store.get(&keys[128]).unwrap().unwrap();
    assert_eq!(accessor.getter(), Some(&Value::Number(7.0)));
    assert_eq!(accessor.setter(), Some(&Value::Number(8.0)));
    assert_eq!(
        store.keys(),
        keys.iter()
            .enumerate()
            .filter(|(n, _)| *n != 10)
            .map(|(_, k)| k.clone())
            .collect::<Vec<_>>()
    );
    store.remove(&keys[128]).unwrap().unwrap();
    assert!(store.accessors.is_none());
    store.define(&keys[10], data(9.0)).unwrap();
    assert_eq!(store.keys().last(), Some(&keys[10]));
}

#[test]
fn dictionary_order_exhaustion_compacts_live_order_without_changing_slots() {
    let owner = owner();
    let mut store = owner.empty_store();
    let keys: Vec<_> = (0..129)
        .map(|n| owner.domain().intern(&format!("key{n}")))
        .collect();
    for (n, key) in keys.iter().enumerate() {
        store.define(key, data(n as f64)).unwrap();
    }
    store.remove(&keys[60]).unwrap().unwrap();
    Rc::get_mut(&mut store.layout).unwrap().next_order = u32::MAX;
    let fresh = owner.domain().intern("fresh");
    store.define(&fresh, data(1000.0)).unwrap();
    let expected: Vec<_> = keys
        .iter()
        .enumerate()
        .filter(|(n, _)| *n != 60)
        .map(|(_, key)| key.clone())
        .chain([fresh])
        .collect();
    assert_eq!(store.keys(), expected);
    for (n, key) in keys.iter().enumerate().filter(|(n, _)| *n != 60) {
        assert_eq!(
            store.get(key).unwrap().unwrap().value,
            Value::Number(n as f64)
        );
    }
}

#[cfg(feature = "perf-counters")]
#[test]
fn dictionary_insertion_consumes_one_probe_even_when_index_rebuilds() {
    let owner = owner();
    let mut store = owner.empty_store();
    for n in 0..255 {
        let key = owner.domain().intern(&format!("key{n}"));
        store.define(&key, data(n as f64)).unwrap();
    }
    let key = owner.domain().intern("triggers growth");
    let before = crate::diagnostics::counters().unwrap();
    store.lookup(&key).unwrap().define(data(256.0));
    let after = crate::diagnostics::counters().unwrap();
    assert_eq!(after.property_id_probes - before.property_id_probes, 1);
    assert_eq!(
        after.property_id_index_rebuilds - before.property_id_index_rebuilds,
        1
    );
    assert_eq!(
        after.property_id_index_rebuilt_entries - before.property_id_index_rebuilt_entries,
        256
    );
    assert_eq!(
        after.property_shape_field_copies - before.property_shape_field_copies,
        0
    );
    assert_eq!(after.property_key_interns - before.property_key_interns, 0);
}

#[cfg(feature = "perf-counters")]
#[test]
fn retention_gauge_releases_cache_ownership_while_escaping_layout_stays_live() {
    let before = crate::diagnostics::counters()
        .unwrap()
        .property_cached_shape_nodes;
    let owner = owner();
    let key = owner.domain().intern("kept");
    let mut store = owner.empty_store();
    store.define(&key, data(1.0)).unwrap();
    assert_eq!(
        crate::diagnostics::counters()
            .unwrap()
            .property_cached_shape_nodes,
        before + 2
    );
    drop(owner);
    assert_eq!(
        crate::diagnostics::counters()
            .unwrap()
            .property_cached_shape_nodes,
        before
    );
    assert_eq!(store.get(&key).unwrap().unwrap().value, Value::Number(1.0));
}

#[test]
fn shapes_and_dictionaries_pin_ids_until_every_layout_owner_is_gone() {
    for dictionary in [false, true] {
        let domain = AtomDomain::limited_for_test(1);
        let owner = PropertyOwner::new(domain.clone());
        let key = domain.intern("old");
        let id = key.id();
        let mut store = owner.empty_store();
        store.define(&key, data(1.0)).unwrap();
        if dictionary {
            for n in 0..33 {
                store
                    .define(
                        &key,
                        Property::data(Value::Number(1.0), n % 2 == 0, true, true),
                    )
                    .unwrap();
            }
            assert!(store.layout.dictionary);
        }
        let guard = Rc::downgrade(store.layout());
        drop(key);
        assert!(
            domain.try_intern("new").is_err(),
            "live layouts must pin the lease"
        );
        drop(store);
        assert!(
            domain.try_intern("new").is_err(),
            "cached shapes must also pin the lease"
        );
        drop(owner);
        assert!(
            guard.upgrade().is_none(),
            "expired layout guards cannot validate a new key"
        );
        let new = domain.intern("new");
        assert_eq!(new.id(), id);
        let new_owner = PropertyOwner::new(domain);
        let mut fresh = new_owner.empty_store();
        fresh.define(&new, data(2.0)).unwrap();
        assert!(!guard.ptr_eq(&Rc::downgrade(fresh.layout())));
        assert_eq!(fresh.get(&new).unwrap().unwrap().value, Value::Number(2.0));
    }
}

#[test]
fn reconfiguration_at_width_limit_uses_the_edit_budget_not_append_threshold() {
    let owner = owner();
    let mut store = owner.empty_store();
    let keys: Vec<_> = (0..128)
        .map(|n| owner.domain().intern(&format!("k{n}")))
        .collect();
    for key in &keys {
        store.define(key, data(1.0)).unwrap();
    }
    store
        .define(
            &keys[0],
            Property::data(Value::Number(1.0), false, true, true),
        )
        .unwrap();
    assert!(!store.is_dictionary());
    for n in 0..31 {
        store
            .define(
                &keys[0],
                Property::data(Value::Number(1.0), n % 2 == 0, true, true),
            )
            .unwrap();
    }
    assert!(store.is_dictionary());
}
