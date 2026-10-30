//! Structural proof tests inspect hits as well as observable JavaScript results.
use super::*;
use crate::property::atom::AtomDomain;
use crate::{Function, NativeFunction, Property};
use std::collections::HashMap;

fn object(domain: &AtomDomain, name: &str, value: f64) -> ObjectRef {
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
    assert!(matches!(cache.0.borrow().entries, Entries::Mega));
    cache.update(&objects[0], "x", &Value::Null);
    assert!(cache.get(&objects[0]).is_none());
}
#[test]
fn inherited_guards_first_edge_and_all_chain_nodes_but_preserves_value_writes() {
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
    assert_eq!(cache.get(&receiver), Some(Value::Number(9.0)));
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
    {
        if let Entries::Mono(e) = &mut cache.0.borrow_mut().entries {
            e.flags &= !VERIFY;
        }
    }
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
    assert_eq!(sites[0].get(&receiver), Some(Value::Number(8.0)));
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
        assert!(matches!(cache.0.borrow().entries, Entries::Mono(_)));
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
    let mut state = cache.0.borrow_mut();
    let Entries::Mono(entry) = &mut state.entries else {
        panic!()
    };
    entry.version = object.with_named(|s| s.version());
    entry.flags |= VERIFY;
    state.name = Some("x".into());
    drop(state);
    cache.get(&object);
}
