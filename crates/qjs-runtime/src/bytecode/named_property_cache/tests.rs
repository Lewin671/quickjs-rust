//! Structural proof tests inspect hits as well as observable JavaScript results.
use super::*;
use crate::property::atom::AtomDomain;
use crate::{Function, NativeFunction, Property};
use std::collections::HashMap;

pub(super) fn object(domain: &AtomDomain, name: &str, value: f64) -> ObjectRef {
    let object = ObjectRef::new(HashMap::new(), domain);
    object.set(name.into(), Value::Number(value));
    object
}
#[test]
fn mono_hit_shares_layout_loads_live_values_and_does_not_pin_keys_or_receivers() {
    let domain = AtomDomain::limited_for_test(1);
    let owner = crate::value::layout::PropertyOwner::new(domain.clone());
    let a = object(&domain, "old", 1.0);
    let b = object(&domain, "old", 2.0);
    let id = domain.find("old").unwrap().id();
    let cache = NamedPropertyCache::default();
    cache.update(&a, "old", &Value::Null);
    assert_eq!(cache.get(&b), Some(Value::Number(2.0)));
    b.set("old".into(), Value::Number(3.0));
    assert_eq!(cache.get(&b), Some(Value::Number(3.0)));
    let weak = a.downgrade();
    drop(a);
    drop(b);
    drop(owner);
    assert!(weak.upgrade().is_none());
    let new = domain.try_intern("new").unwrap();
    assert_eq!(new.id(), id);
    let c = object(&domain, "new", 4.0);
    assert!(
        cache.get(&c).is_none(),
        "recycled key ID must not validate the expired layout"
    );
}
#[test]
fn site_promotes_to_four_ways_then_stays_megamorphic() {
    let domain = AtomDomain::new();
    let cache = NamedPropertyCache::default();
    let mut objects = Vec::new();
    for index in 0..5 {
        let o = object(&domain, "x", index as f64);
        for extra in 0..index {
            o.set(format!("extra{extra}"), Value::Null);
        }
        cache.update(&o, "x", &Value::Null);
        objects.push(o);
        if index < 4 {
            for o in &objects {
                assert!(cache.get(o).is_some());
            }
        }
    }
    assert!(matches!(cache.0.state.borrow().entries, Entries::Mega));
    cache.update(&objects[0], "x", &Value::Null);
    assert!(cache.get(&objects[0]).is_none());
}
#[test]
fn inherited_guards_first_edge_and_all_chain_nodes_and_value_writes() {
    let domain = AtomDomain::new();
    let _owner = crate::value::layout::PropertyOwner::new(domain.clone());
    let holder = object(&domain, "x", 1.0);
    let middle = ObjectRef::with_prototype(HashMap::new(), Some(holder.clone()), &domain);
    let receiver = ObjectRef::with_prototype(HashMap::new(), Some(middle.clone()), &domain);
    let other = ObjectRef::with_prototype(HashMap::new(), Some(holder.clone()), &domain);
    assert_ne!(
        receiver.layout_revision(),
        other.layout_revision(),
        "prototype identity belongs to the layout"
    );
    let cache = NamedPropertyCache::default();
    assert_eq!(cache.read_object(&receiver, "x"), Some(Value::Number(1.0)));
    assert!(
        cache.get(&other).is_none(),
        "equal layouts do not prove equal first edges"
    );
    holder.set("x".into(), Value::Number(9.0));
    assert!(cache.get(&receiver).is_none());
    assert_eq!(cache.read_object(&receiver, "x"), Some(Value::Number(9.0)));
    middle.set("x".into(), Value::Number(2.0));
    assert!(cache.get(&receiver).is_none());
    assert_eq!(cache.read_object(&receiver, "x"), Some(Value::Number(2.0)));
    middle.delete_own_property("x");
    assert!(cache.get(&receiver).is_none());
    assert_eq!(cache.read_object(&receiver, "x"), Some(Value::Number(9.0)));
    middle.set_prototype(None).unwrap();
    assert!(cache.get(&receiver).is_none());
}
#[test]
fn dictionary_reconfiguration_and_epoch_rollover_retire_slots_before_reuse() {
    let domain = AtomDomain::new();
    let o = object(&domain, "x", 1.0);
    for i in 0..130 {
        o.set(format!("p{i}"), Value::Null);
    }
    let cache = NamedPropertyCache::default();
    cache.update(&o, "x", &Value::Null);
    o.test_set_layout_epoch(u64::MAX);
    o.delete_own_property("x");
    o.set("other".into(), Value::Number(4.0));
    assert!(cache.get(&o).is_none());
    o.set("x".into(), Value::Number(5.0));
    cache.update(&o, "x", &Value::Null);
    o.define_property(
        "x".into(),
        Property::data(Value::Number(6.0), true, false, true),
    );
    assert!(cache.get(&o).is_none());
    cache.update(&o, "x", &Value::Null);
    assert!(matches!(
        cache.write(&o, "x", &Value::Null),
        Some(OwnDataPropertyWrite::ReadOnly)
    ));
}
#[test]
fn array_and_function_own_named_slots_use_the_same_cache_and_lazy_hooks() {
    let domain = AtomDomain::new();
    let array = crate::ArrayRef::new_in_domain(&domain, vec![]);
    array.set_property("x".into(), Value::Number(1.0));
    let value = Value::Array(array.clone());
    let cache = NamedPropertyCache::default();
    let key = domain.intern("x");
    assert_eq!(
        cache.read_value(&value, &key, None),
        Some(Value::Number(1.0))
    );
    array.set_property("x".into(), Value::Number(2.0));
    assert_eq!(cache.get_value(&value, None), Some(Value::Number(2.0)));
    assert!(matches!(
        cache.write_value(&value, &Value::Number(3.0)),
        Some(OwnDataPropertyWrite::Written)
    ));
    let f = Function::new_native_in_domain(Some("f"), 1, NativeFunction::MathAbs, false, &domain);
    let fvalue = Value::Function(f.clone());
    let names = NamedPropertyCache::default();
    assert_eq!(
        names.read_value(&fvalue, &domain.intern("name"), None),
        Some(Value::String("f".into()))
    );
    f.define_property(
        "name".into(),
        Property::data(Value::String("g".into()), false, false, true),
    );
    assert_eq!(
        names.read_value(&fvalue, &domain.intern("name"), None),
        Some(Value::String("g".into()))
    );
    assert_eq!(f.own_property_names(), ["length", "name"]);
}
#[test]
fn accessors_are_never_admitted_and_inherited_entries_never_write_holder() {
    let domain = AtomDomain::new();
    let holder = object(&domain, "x", 1.0);
    let receiver = ObjectRef::with_prototype(HashMap::new(), Some(holder.clone()), &domain);
    let cache = NamedPropertyCache::default();
    cache.read_object(&receiver, "x");
    assert!(cache.write(&receiver, "x", &Value::Null).is_none());
    holder.define_property("x".into(), Property::accessor(None, None, true, true));
    assert!(cache.get(&receiver).is_none());
    assert!(cache.read_object(&receiver, "x").is_none());
    cache.clear();
}

#[cfg(feature = "perf-counters")]
#[test]
fn own_hit_has_no_id_probe_text_hash_or_descriptor_path() {
    let domain = AtomDomain::new();
    let o = object(&domain, "x", 1.0);
    let cache = NamedPropertyCache::default();
    cache.update(&o, "x", &Value::Null);
    // Verification intentionally resolves the key and semantic result again.
    #[cfg(debug_assertions)]
    let cache = modified_first(&cache, |e| e.flags &= !VERIFY);
    crate::diagnostics::reset_counters();
    assert_eq!(cache.get(&o), Some(Value::Number(1.0)));
    let c = crate::diagnostics::counters().unwrap();
    assert_eq!(c.ic_object_hits, 1);
    assert_eq!(c.ic_object_probes, 0);
    assert_eq!(c.property_text_probes, 0);
    assert_eq!(c.property_id_probes, 0);
}

#[test]
fn namespace_live_binding_hooks_cannot_borrow_an_ordinary_shape_entry() {
    let domain = AtomDomain::new();
    let ordinary = object(&domain, "x", 1.0);
    let namespace = object(&domain, "x", 2.0);
    namespace.mark_module_namespace_exotic();
    let cache = NamedPropertyCache::default();
    cache.update(&ordinary, "x", &Value::Null);
    assert!(cache.get(&namespace).is_none());
    assert!(cache.read_object(&namespace, "x").is_none());
    assert!(cache.write(&namespace, "x", &Value::Null).is_none());
}

#[test]
fn ic_entry_and_site_sizes_are_explicit() {
    assert_eq!(std::mem::size_of::<Entry>(), 32);
    assert_eq!(std::mem::size_of::<NamedPropertyCache>(), 8);
    eprintln!(
        "S2 sizes: entry={} site-state={} site-handle={}",
        std::mem::size_of::<Entry>(),
        std::mem::size_of::<State>(),
        std::mem::size_of::<NamedPropertyCache>()
    );
}

#[test]
fn absent_text_reads_never_create_key_records() {
    let domain = AtomDomain::new();
    let object = ObjectRef::new(HashMap::new(), &domain);
    let cache = NamedPropertyCache::default();
    for _ in 0..10_000 {
        assert_eq!(
            cache.read_object(&object, "neverInstalled"),
            Some(Value::Undefined)
        );
    }
    assert!(domain.find("neverInstalled").is_none());
}

#[test]
fn ordinary_root_churn_preserves_uncapped_shared_prototype_generation() {
    let domain = AtomDomain::new();
    let holder = object(&domain, "m", 7.0);
    let middle = ObjectRef::with_prototype(HashMap::new(), Some(holder.clone()), &domain);
    let receiver = ObjectRef::with_prototype(HashMap::new(), Some(middle.clone()), &domain);
    let sites: Vec<_> = (0..600).map(|_| NamedPropertyCache::default()).collect();
    for site in &sites {
        assert_eq!(site.read_object(&receiver, "m"), Some(Value::Number(7.0)));
    }
    for i in 0..1000 {
        let ordinary = ObjectRef::with_prototype(HashMap::new(), Some(middle.clone()), &domain);
        ordinary.set("v".into(), Value::Number(i as f64));
    }
    for site in &sites {
        assert_eq!(site.get(&receiver), Some(Value::Number(7.0)));
    }
    holder.set("m".into(), Value::Number(8.0));
    assert!(sites[0].get(&receiver).is_none());
    assert_eq!(
        sites[0].read_object(&receiver, "m"),
        Some(Value::Number(8.0))
    );
    holder.set("new".into(), Value::Null);
    assert!(sites.iter().all(|s| s.get(&receiver).is_none()));
}

#[test]
fn absent_and_accessor_entries_cache_pure_outcomes_and_invalidate_on_shadowing() {
    let domain = AtomDomain::new();
    let holder = ObjectRef::new(HashMap::new(), &domain);
    let receiver = ObjectRef::with_prototype(HashMap::new(), Some(holder.clone()), &domain);
    let cache = NamedPropertyCache::default();
    cache.read_object(&receiver, "absent");
    assert_eq!(cache.get(&receiver), Some(Value::Undefined));
    holder.define_property("absent".into(), Property::accessor(None, None, true, true));
    assert!(cache.get(&receiver).is_none());
    let value = Value::Object(receiver.clone());
    assert!(matches!(
        cache.read_outcome(&value, "absent"),
        Some(ReadOutcome::Accessor(_))
    ));
    assert!(cache.accessor(&value).is_some());
    receiver.set("absent".into(), Value::Number(3.0));
    assert!(cache.accessor(&value).is_none());
    assert_eq!(
        cache.read_object(&receiver, "absent"),
        Some(Value::Number(3.0))
    );
}

#[test]
fn dead_dictionary_guards_do_not_make_a_site_megamorphic() {
    let domain = AtomDomain::new();
    let cache = NamedPropertyCache::default();
    for _ in 0..20 {
        let object = object(&domain, "x", 1.0);
        for i in 0..130 {
            object.set(format!("p{i}"), Value::Null);
        }
        cache.read_object(&object, "x");
        assert!(matches!(
            cache.0.state.borrow().entries,
            Entries::MonoFirst | Entries::Mono(_)
        ));
    }
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "IC complete outcome differs")]
fn verifier_rejects_a_data_hit_when_pure_lookup_is_exotic() {
    let domain = AtomDomain::new();
    let object = object(&domain, "x", 1.0);
    let cache = NamedPropertyCache::default();
    cache.read_object(&object, "x");
    // Simulate an invalid admission: the slot still loads, but the semantic
    // receiver has become a namespace exotic. Verification must fail loudly.
    object.mark_module_namespace_exotic();
    let cache = modified_first(&cache, |entry| {
        entry.version = object.with_named(|s| s.version());
        entry.flags |= VERIFY;
    });
    cache.0.state.borrow_mut().name = Some("x".into());
    cache.get(&object);
}

#[test]
fn shared_poly_promotes_to_versioned_guards_when_a_dictionary_arrives() {
    let domain = AtomDomain::new();
    let _owner = crate::value::layout::PropertyOwner::new(domain.clone());
    let a = object(&domain, "x", 1.0);
    let b = object(&domain, "x", 2.0);
    b.set("y".into(), Value::Null);
    let cache = NamedPropertyCache::default();
    cache.read_object(&a, "x");
    cache.read_object(&b, "x");
    assert!(matches!(
        cache.0.state.borrow().entries,
        Entries::SharedOwnPoly(_)
    ));
    let dict = object(&domain, "x", 3.0);
    for i in 0..130 {
        dict.set(format!("p{i}"), Value::Null);
    }
    cache.read_object(&dict, "x");
    assert!(matches!(cache.0.state.borrow().entries, Entries::Poly(_)));
    dict.define_property("x".into(), Property::accessor(None, None, true, true));
    assert!(cache.get(&dict).is_none());
    assert_eq!(cache.get(&a), Some(Value::Number(1.0)));
    assert_eq!(cache.get(&b), Some(Value::Number(2.0)));
}

#[cfg(feature = "perf-counters")]
#[test]
fn shared_poly_hits_use_no_text_id_or_descriptor_probe() {
    let domain = AtomDomain::new();
    let _owner = crate::value::layout::PropertyOwner::new(domain.clone());
    let a = object(&domain, "x", 1.0);
    let b = object(&domain, "x", 2.0);
    b.set("y".into(), Value::Null);
    let cache = NamedPropertyCache::default();
    cache.read_object(&a, "x");
    cache.read_object(&b, "x");
    #[cfg(debug_assertions)]
    if let Entries::SharedOwnPoly(ways) = &mut cache.0.state.borrow_mut().entries {
        for way in ways.iter_mut() {
            way.flags &= !VERIFY;
        }
    }
    crate::diagnostics::reset_counters();
    assert_eq!(cache.get(&a), Some(Value::Number(1.0)));
    assert_eq!(cache.get(&b), Some(Value::Number(2.0)));
    let c = crate::diagnostics::counters().unwrap();
    assert_eq!(c.ic_object_hits, 2);
    assert_eq!(c.property_text_probes, 0);
    assert_eq!(c.property_id_probes, 0);
}

#[test]
fn numeric_hits_read_live_types_without_invalidating_shared_shapes() {
    let domain = AtomDomain::new();
    let _owner = crate::value::layout::PropertyOwner::new(domain.clone());
    let object = object(&domain, "x", 1.0);
    let cache = NamedPropertyCache::default();
    cache.read_object(&object, "x");
    let version = object.layout_revision();
    assert_eq!(cache.get_number(&object), Some(1.0));
    object.set("x".into(), Value::String("changed".into()));
    assert_eq!(object.layout_revision(), version);
    assert_eq!(cache.get_number(&object), None);
    assert_eq!(cache.get(&object), Some(Value::String("changed".into())));
    object.set("x".into(), Value::Number(7.0));
    assert_eq!(cache.get_number(&object), Some(7.0));
}

#[cfg(feature = "perf-counters")]
#[test]
fn megamorphic_reads_reuse_static_names_without_pinning_dead_keys() {
    let domain = AtomDomain::new();
    let _owner = crate::value::layout::PropertyOwner::new(domain.clone());
    let cache = NamedPropertyCache::default();
    let mut objects = Vec::new();
    for i in 0..5 {
        let o = object(&domain, "x", i as f64);
        for j in 0..i {
            o.set(format!("p{j}"), Value::Null);
        }
        cache.read_object(&o, "x");
        objects.push(o);
    }
    assert!(matches!(cache.0.state.borrow().entries, Entries::Mega));
    crate::diagnostics::reset_counters();
    for i in 0..1000 {
        assert_eq!(
            cache.read_object(&objects[i % 5], "x"),
            Some(Value::Number((i % 5) as f64))
        );
    }
    let c = crate::diagnostics::counters().unwrap();
    assert_eq!(c.property_text_probes, 0);
    assert_eq!(c.property_atoms_created, 0);
    assert_eq!(c.property_id_probes, 1000);
}

#[test]
fn append_recipe_reuses_prefix_and_retires_on_prototype_and_integrity_changes() {
    let domain = AtomDomain::new();
    let _owner = crate::value::layout::PropertyOwner::new(domain.clone());
    let parent = ObjectRef::new(HashMap::new(), &domain);
    let make = || ObjectRef::with_prototype(HashMap::new(), Some(parent.clone()), &domain);
    let cache = NamedPropertyCache::default();
    let first = make();
    let key = domain.intern("x");
    let before = first.with_named(|s| s.version());
    let write = first.write_or_create_data_key(&key, &Value::Number(1.0), false);
    assert!(write.created);
    cache.record_data_slot(&first, &key, write.cache_slot.unwrap(), Some(before), true);
    assert!(cache.0.state.borrow().add.is_some());
    let next = make();
    assert!(
        cache.write_hot(&next, &Value::Null).is_none(),
        "loop writes must not add"
    );
    assert!(matches!(
        cache.write(&next, "x", &Value::Null),
        Some(OwnDataPropertyWrite::Written)
    ));
    assert_eq!(next.own_property("x").unwrap().value, Value::Null);
    assert_eq!(next.layout_revision(), first.layout_revision());
    let sealed = make();
    sealed.prevent_extensions();
    assert!(cache.write(&sealed, "x", &Value::Null).is_none());
    parent.define_property("x".into(), Property::data(Value::Null, true, false, true));
    assert!(cache.write(&make(), "x", &Value::Null).is_none());
    next.define_property("x".into(), Property::data(Value::Null, true, false, true));
    assert!(cache.write(&next, "x", &Value::Null).is_none());
}

#[test]
fn append_recipe_counts_indices_and_keeps_values_live_without_pinning_receiver() {
    let domain = AtomDomain::new();
    let _owner = crate::value::layout::PropertyOwner::new(domain.clone());
    let cache = NamedPropertyCache::default();
    let first = ObjectRef::new(HashMap::new(), &domain);
    let key = domain.intern("2");
    let before = first.with_named(|s| s.version());
    let write = first.write_or_create_data_key(&key, &Value::Null, false);
    cache.record_data_slot(&first, &key, write.cache_slot.unwrap(), Some(before), true);
    let weak = first.downgrade();
    drop(first);
    assert!(weak.upgrade().is_none());
    let next = ObjectRef::new(HashMap::new(), &domain);
    assert!(matches!(
        cache.write(&next, "2", &Value::Number(2.0)),
        Some(OwnDataPropertyWrite::Written)
    ));
    assert!(next.has_own_index_property());
    assert_eq!(next.own_property_names(), vec!["2"]);
    assert!(matches!(
        cache.write(&next, "2", &Value::Null),
        Some(OwnDataPropertyWrite::Written)
    ));
    assert_eq!(next.own_property("2").unwrap().value, Value::Null);
}

#[test]
fn numeric_store_loads_live_types_and_rejects_reconfigured_descriptors() {
    let domain = AtomDomain::new();
    let _owner = crate::value::layout::PropertyOwner::new(domain.clone());
    let o = object(&domain, "x", 1.0);
    let cache = NamedPropertyCache::default();
    cache.update(&o, "x", &Value::Undefined);
    let version = o.layout_revision();
    assert!(cache.write_number(&o, -0.0));
    assert!(o.get("x").unwrap().same_value(&Value::Number(-0.0)));
    o.set("x".into(), Value::String("changed".into()));
    assert!(cache.write_number(&o, f64::NAN));
    assert!(matches!(o.get("x"), Some(Value::Number(n)) if n.is_nan()));
    assert_eq!(version, o.layout_revision());
    o.define_property(
        "x".into(),
        Property::data(Value::Number(4.0), true, false, true),
    );
    assert!(!cache.write_number(&o, 5.0));
    assert_eq!(o.get("x"), Some(Value::Number(4.0)));
}

#[cfg(debug_assertions)]
fn modified_first(cache: &NamedPropertyCache, edit: impl FnOnce(&mut Entry)) -> NamedPropertyCache {
    let fresh = NamedPropertyCache::default();
    let mut entry = cache.0.first.get().expect("warm first").clone();
    edit(&mut entry);
    fresh.0.first.set(entry).unwrap();
    {
        let mut state = fresh.0.state.borrow_mut();
        state.entries = Entries::MonoFirst;
        state.name = cache.0.state.borrow().name.clone();
    }
    fresh.0.first_active.set(true);
    fresh
}

#[test]
fn immutable_mono_hits_read_live_slots_without_borrowing_mutable_metadata() {
    let domain = AtomDomain::new();
    let o = object(&domain, "x", 1.0);
    let cache = NamedPropertyCache::default();
    cache.update(&o, "x", &Value::Undefined);
    #[cfg(debug_assertions)]
    let cache = modified_first(&cache, |e| e.flags &= !VERIFY);
    let clone = cache.clone();
    let _exclusive_metadata = cache.0.state.borrow_mut();
    assert_eq!(clone.get(&o), Some(Value::Number(1.0)));
    assert_eq!(clone.get_number(&o), Some(1.0));
    assert!(clone.write_number(&o, -0.0));
    assert!(clone.get_number(&o).unwrap().is_sign_negative());
    o.set("x".into(), Value::String("live".into()));
    assert_eq!(clone.get(&o), Some(Value::String("live".into())));
    assert_eq!(clone.get_number(&o), None);
}
#[test]
fn first_guard_is_disabled_on_promotion_and_stays_disabled_after_clear() {
    let domain = AtomDomain::new();
    let a = object(&domain, "x", 1.0);
    let b = object(&domain, "before", 0.0);
    b.set("x".into(), Value::Number(2.0));
    let cache = NamedPropertyCache::default();
    cache.read_object(&a, "x");
    assert!(cache.0.first_active.get());
    assert_eq!(cache.read_object(&b, "x"), Some(Value::Number(2.0)));
    assert!(!cache.0.first_active.get());
    assert_eq!(cache.get(&a), Some(Value::Number(1.0)));
    cache.clear();
    cache.read_object(&b, "x");
    assert!(!cache.0.first_active.get());
    assert!(cache.get(&a).is_none());
    assert_eq!(cache.get(&b), Some(Value::Number(2.0)));
    let weak = a.downgrade();
    drop(a);
    assert!(
        weak.upgrade().is_none(),
        "inactive token cannot pin its receiver"
    );
    eprintln!(
        "S2 cache sizes: cache={} state={} entry={}",
        std::mem::size_of::<CacheData>(),
        std::mem::size_of::<State>(),
        std::mem::size_of::<Entry>()
    );
}

#[test]
fn boxed_string_length_is_ordinary_data_while_indices_stay_exotic() {
    let domain = AtomDomain::new();
    let object = ObjectRef::new(HashMap::new(), &domain);
    object.set_string_data("a😀".into());
    object.define_property(
        "length".into(),
        Property::data(Value::Number(3.0), false, false, false),
    );
    object.defer_string_indices();
    let cache = NamedPropertyCache::default();
    assert_eq!(
        cache.read_object(&object, "length"),
        Some(Value::Number(3.0))
    );
    assert_eq!(cache.get_number(&object), Some(3.0));
    assert!(!cache.write_number(&object, 9.0));
    assert_eq!(
        object.own_property("length").unwrap().value,
        Value::Number(3.0)
    );
    let indices = NamedPropertyCache::default();
    assert!(indices.read_object(&object, "0").is_none());
    assert_eq!(object.get("0"), Some(Value::String("a".into())));
}

#[test]
fn owned_and_shared_sites_use_the_same_live_slots_and_clear_protocol() {
    let domain = AtomDomain::new();
    let object = object(&domain, "x", 1.0);
    let shared = NamedPropertyCache::for_local(7);
    let owned = CacheData {
        local_slot: Some(7),
        ..CacheData::local_site()
    };
    for cache in [&*shared, &owned] {
        cache.update(&object, "x", &Value::Undefined);
    }
    assert!(owned.write_number(&object, -0.0));
    assert!(shared.get_number(&object).unwrap().is_sign_negative());
    let cloned = owned.clone();
    assert_eq!(cloned.local_slot(), Some(7));
    assert!(
        cloned.hot_mono().is_none(),
        "a code clone learns fresh local feedback"
    );
    for cache in [&*shared, &owned] {
        cache.clear();
        assert_eq!(cache.local_slot(), Some(7));
        assert!(cache.get(&object).is_none());
    }
    let weak = object.downgrade();
    drop(object);
    assert!(weak.upgrade().is_none());
    assert_eq!(
        std::mem::size_of::<NamedPropertyCache>(),
        std::mem::size_of::<Rc<()>>()
    );
}

#[test]
fn megamorphic_store_sites_do_not_capture_unused_before_write_roots() {
    let domain = AtomDomain::new();
    let _owner = crate::value::layout::PropertyOwner::new(domain.clone());
    let parent = ObjectRef::new(HashMap::new(), &domain);
    let receiver = ObjectRef::with_prototype(HashMap::new(), Some(parent), &domain);
    let cache = NamedPropertyCache::default();
    cache.0.state.borrow_mut().entries = Entries::Mega;
    assert!(cache.before_write_version(&receiver).is_none());
}

#[test]
fn own_slot_admission_publishes_rebased_full_array_and_function_authorities() {
    let domain = AtomDomain::new();
    let _owner = crate::value::layout::PropertyOwner::new(domain.clone());
    let parent = ObjectRef::new(HashMap::new(), &domain);
    let array = crate::ArrayRef::new_in_domain(&domain, vec![]);
    let function =
        Function::new_native_in_domain(Some("f"), 0, NativeFunction::MathAbs, false, &domain);
    let fill = |store: &mut crate::value::layout::NamedStore| {
        store.insert_text("old", Property::enumerable(Value::Null));
        store.remove_text("old");
        store.set_prototype(Some(crate::Prototype::Object(parent.clone())));
        store.insert_text("x", Property::enumerable(Value::Number(9.0)));
    };
    array.with_named_mut(fill);
    function.with_named_mut(fill);
    for value in [Value::Array(array), Value::Function(function)] {
        let cache = NamedPropertyCache::default();
        assert_eq!(
            cache.read_value(&value, &domain.intern("x"), None),
            Some(Value::Number(9.0))
        );
        assert_eq!(cache.get_value(&value, None), Some(Value::Number(9.0)));
    }
}

#[test]
fn shared_own_witness_retires_on_dictionary_conversion_and_private_reconfiguration() {
    let domain = AtomDomain::new();
    let _owner = crate::value::layout::PropertyOwner::new(domain.clone());
    let object = object(&domain, "x", 1.0);
    let shared = NamedPropertyCache::default();
    assert_eq!(shared.read_object(&object, "x"), Some(Value::Number(1.0)));
    assert_ne!(shared.0.first.get().unwrap().flags & SHARED, 0);
    object.set("x".into(), Value::Number(2.0));
    assert_eq!(shared.get(&object), Some(Value::Number(2.0)));
    for i in 0..130 {
        object.set(format!("p{i}"), Value::Null);
    }
    assert!(shared.get(&object).is_none());
    let private = NamedPropertyCache::default();
    assert_eq!(private.read_object(&object, "x"), Some(Value::Number(2.0)));
    assert_eq!(private.0.first.get().unwrap().flags & SHARED, 0);
    object.delete_own_property("x");
    object.set("x".into(), Value::Number(3.0));
    assert!(private.get(&object).is_none());
    assert_eq!(private.read_object(&object, "x"), Some(Value::Number(3.0)));
}
