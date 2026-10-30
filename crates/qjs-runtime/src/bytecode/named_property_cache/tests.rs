//! Mechanism coverage: a successful JavaScript read alone cannot prove that
//! the prototype cache installed a slot instead of taking its fallback.

use std::{collections::HashMap, rc::Rc};

use super::{CacheProbe, NamedPropertyCache};
use crate::value::ObjectLiteralShape;
use crate::{ObjectRef, Value};

#[test]
fn prototype_cache_records_and_reads_slots_across_literal_widths() {
    let domain = crate::property::atom::AtomDomain::new();
    for count in [2, 4, 20] {
        let keys: Vec<Rc<str>> = (0..count).map(|i| Rc::from(format!("field{i}"))).collect();
        let key = keys[count - 1].clone();
        let holder = ObjectRef::with_literal_properties(
            ObjectLiteralShape::new(keys),
            (0..count).map(|i| Value::Number(i as f64)).collect(),
            None,
            &domain,
        );
        let receiver = ObjectRef::with_prototype(HashMap::new(), Some(holder.clone()), &domain);
        assert_eq!(holder.own_data_slot(&key), Some(count - 1));
        assert_eq!(
            holder.own_data_slot_value(count - 1),
            Some(Value::Number((count - 1) as f64))
        );
        let cache = NamedPropertyCache::default();
        cache.update_from_prototype(&receiver, &key);
        let CacheProbe::PrototypeCandidate { holder: hit, slot } = cache.probe(&receiver) else {
            panic!("literal with {count} properties must install a prototype slot");
        };
        assert!(hit.ptr_eq(&holder));
        assert_eq!(slot, count - 1);
        assert_eq!(
            hit.prototype_data_slot_value(slot),
            Some(Value::Number((count - 1) as f64))
        );

        let revision = holder.layout_revision();
        holder.write_existing_own_data_property(&key, &Value::Number(99.0));
        assert_eq!(holder.layout_revision(), revision);
        assert_eq!(
            hit.prototype_data_slot_value(slot),
            Some(Value::Number(99.0))
        );
        assert!(matches!(
            cache.probe(&receiver),
            CacheProbe::PrototypeCandidate { .. }
        ));

        // An insertion materializes the literal table and invalidates the old
        // slot; the dynamic table it becomes is cached afresh by its own slot.
        holder.set("extra".to_owned(), Value::Null);
        assert!(matches!(cache.probe(&receiver), CacheProbe::Miss));
        cache.update_from_prototype(&receiver, &key);
        let CacheProbe::PrototypeCandidate { holder: hit, slot } = cache.probe(&receiver) else {
            panic!("a dynamic prototype table must install a prototype slot");
        };
        assert_eq!(
            hit.prototype_data_slot_value(slot),
            Some(Value::Number(99.0))
        );
    }
}

#[test]
fn a_value_with_no_by_value_entry_is_cached_by_slot() {
    let domain = crate::property::atom::AtomDomain::new();
    let object = ObjectRef::with_prototype(HashMap::new(), None, &domain);
    object.set("items".to_owned(), Value::String("first".into()));
    object.set("name".to_owned(), Value::String("n".into()));
    let cache = NamedPropertyCache::default();
    let value = Value::String("first".into());
    cache.update(&object, "items", &value);
    let CacheProbe::Own(hit) = cache.probe(&object) else {
        panic!("a string-valued property must install a slot entry");
    };
    assert_eq!(hit, value);
    // The slot follows a later write, and a layout change invalidates it.
    object.write_existing_own_data_property("items", &Value::String("second".into()));
    assert!(
        matches!(cache.probe(&object), CacheProbe::Own(Value::String(s)) if s.as_str() == "second")
    );
    object.delete_own_property("items");
    assert!(matches!(cache.probe(&object), CacheProbe::Miss));
}

#[test]
fn a_function_in_shared_layout_is_read_live_across_unrelated_writes() {
    let domain = crate::property::atom::AtomDomain::new();
    // Enough properties to leave the small slot storage, as `Math` has.
    let object = ObjectRef::with_prototype(HashMap::new(), None, &domain);
    for index in 0..20 {
        object.set(format!("p{index}"), Value::Number(index as f64));
    }
    let function =
        crate::Function::new_native(Some("f"), 0, crate::NativeFunction::MathAbs, false, &domain);
    object.set("f".to_owned(), Value::Function(function.clone()));
    assert!(object.own_data_slot("f").is_some());
    let cache = NamedPropertyCache::default();
    cache.update(&object, "f", &Value::Function(function.clone()));
    assert!(
        matches!(cache.probe(&object), CacheProbe::Own(Value::Function(hit)) if hit.ptr_eq(&function))
    );
    // Any write to the object advances its revision and retires the entry.
    object.set("p0".to_owned(), Value::Number(-1.0));
    assert!(
        matches!(cache.probe(&object), CacheProbe::Own(Value::Function(hit)) if hit.ptr_eq(&function))
    );
}

#[test]
fn shared_layout_cache_owns_key_leases_until_its_last_entry_drops() {
    use crate::property::atom::AtomDomain;
    let domain = AtomDomain::limited_for_test(1);
    let owner = crate::value::layout::PropertyOwner::new(domain.clone());
    let object = ObjectRef::new(HashMap::new(), &domain);
    object.set("old".into(), Value::Number(1.0));
    let id = domain.find("old").unwrap().id();
    let cache = NamedPropertyCache::default();
    cache.update(&object, "old", &Value::Number(1.0));
    let guard = Rc::downgrade(&object.layout_data_slot("old").unwrap().0);
    drop(object);
    drop(owner);
    assert!(
        domain.try_intern("new").is_err(),
        "a live cached layout must retain its key lease"
    );
    cache.clear();
    assert!(guard.upgrade().is_none());
    assert_eq!(domain.try_intern("new").unwrap().id(), id);
}

#[test]
fn weak_slot_cache_rejects_recycled_ids_even_when_epoch_restarts() {
    use crate::property::atom::AtomDomain;
    let domain = AtomDomain::limited_for_test(1);
    let object = ObjectRef::with_literal_properties(
        ObjectLiteralShape::new(vec![Rc::from("old")]),
        vec![Value::Number(1.0)],
        None,
        &domain,
    );
    let id = domain.find("old").unwrap().id();
    let cache = NamedPropertyCache::default();
    cache.record_write(&object, "old");
    object.test_set_layout_epoch(u64::MAX - 1);
    assert!(object.delete_own_property("old"));
    let new = domain
        .try_intern("new")
        .expect("weak guards do not keep dead keys alive");
    assert_eq!(new.id(), id);
    object.set("new".into(), Value::Number(2.0));
    assert!(matches!(cache.probe(&object), CacheProbe::Miss));
    assert!(cache.write(&object, "old", &Value::Number(9.0)).is_none());
    assert_eq!(object.get("new"), Some(Value::Number(2.0)));
}
