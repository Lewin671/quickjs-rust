//! Mechanism coverage: a successful JavaScript read alone cannot prove that
//! the prototype cache installed a slot instead of taking its fallback.

use std::{collections::HashMap, rc::Rc};

use super::{CacheProbe, NamedPropertyCache};
use crate::value::ObjectLiteralShape;
use crate::{ObjectRef, Value};

#[test]
fn prototype_cache_records_and_reads_both_literal_representations() {
    for count in [2, 4, 20] {
        let keys: Vec<Rc<str>> = (0..count).map(|i| Rc::from(format!("field{i}"))).collect();
        let key = keys[count - 1].clone();
        let holder = ObjectRef::with_literal_properties(
            ObjectLiteralShape::new(keys),
            (0..count).map(|i| Value::Number(i as f64)).collect(),
            None,
        );
        let receiver = ObjectRef::with_prototype(HashMap::new(), Some(holder.clone()));
        assert!(holder.own_data_slot(&key).is_none());
        assert!(holder.own_data_slot_value(count - 1).is_none());
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
    let object = ObjectRef::with_prototype(HashMap::new(), None);
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
fn a_function_on_slotless_storage_is_cached_by_value_and_revision() {
    // Enough properties to leave the small slot storage, as `Math` has.
    let object = ObjectRef::with_prototype(HashMap::new(), None);
    for index in 0..20 {
        object.set(format!("p{index}"), Value::Number(index as f64));
    }
    let function = crate::Function::new_native(Some("f"), 0, crate::NativeFunction::MathAbs, false);
    object.set("f".to_owned(), Value::Function(function.clone()));
    assert!(object.own_data_slot("f").is_none());
    let cache = NamedPropertyCache::default();
    cache.update(&object, "f", &Value::Function(function.clone()));
    assert!(
        matches!(cache.probe(&object), CacheProbe::Own(Value::Function(hit)) if hit.ptr_eq(&function))
    );
    // Any write to the object advances its revision and retires the entry.
    object.set("p0".to_owned(), Value::Number(-1.0));
    assert!(matches!(cache.probe(&object), CacheProbe::Miss));
}

#[test]
fn hot_own_slot_reads_live_values_and_rejects_another_receiver() {
    let object = ObjectRef::with_prototype(HashMap::new(), None);
    let other = ObjectRef::with_prototype(HashMap::new(), None);
    for receiver in [&object, &other] {
        receiver.set("items".to_owned(), Value::String("first".into()));
        receiver.set("name".to_owned(), Value::Null);
    }
    assert_eq!(object.layout_revision(), other.layout_revision());
    let cache = NamedPropertyCache::default();
    cache.update(&object, "items", &Value::String("first".into()));
    assert!(matches!(cache.probe(&object), CacheProbe::Own(_)));
    assert_eq!(
        cache.probe_hot(&object),
        Some(Value::String("first".into()))
    );
    assert_eq!(cache.probe_hot(&other), None);
    let revision = object.layout_revision();
    object.write_existing_own_data_property("items", &Value::Object(other.clone()));
    assert_eq!(object.layout_revision(), revision);
    assert!(matches!(cache.probe_hot(&object), Some(Value::Object(hit)) if hit.ptr_eq(&other)));
}

#[test]
fn hot_own_slot_declines_deleted_reinserted_and_reconfigured_properties() {
    let object = ObjectRef::with_prototype(HashMap::new(), None);
    object.set("items".to_owned(), Value::String("first".into()));
    let cache = NamedPropertyCache::default();
    cache.update(&object, "items", &Value::String("first".into()));
    assert!(cache.probe_hot(&object).is_some());
    object.delete_own_property("items");
    assert_eq!(cache.probe_hot(&object), None);
    object.set("replacement".to_owned(), Value::Number(9.0));
    object.set("items".to_owned(), Value::String("second".into()));
    assert_eq!(cache.probe_hot(&object), None);
    cache.update(&object, "items", &Value::String("second".into()));
    assert!(matches!(cache.probe(&object), CacheProbe::Own(_)));
    assert_eq!(
        cache.probe_hot(&object),
        Some(Value::String("second".into()))
    );
    object.define_property(
        "items".to_owned(),
        crate::value::Property::accessor(Some(Value::Undefined), None, true, true),
    );
    assert_eq!(cache.probe_hot(&object), None);
    assert!(matches!(cache.probe(&object), CacheProbe::Miss));
}

#[test]
fn hot_kind_tracks_exact_to_slot_writes_and_declines_hot_writes() {
    let object = ObjectRef::with_prototype(HashMap::new(), None);
    object.set("x".to_owned(), Value::Number(1.0));
    let cache = NamedPropertyCache::default();
    cache.update(&object, "x", &Value::Number(1.0));
    assert_eq!(cache.probe_hot(&object), None);
    assert!(matches!(
        cache.write(&object, "x", &Value::Number(2.0)),
        Some(crate::value::OwnDataPropertyWrite::Written)
    ));
    assert_eq!(cache.probe_hot(&object), Some(Value::Number(2.0)));
    assert!(cache.write_hot(&object, &Value::Number(3.0)).is_none());
    assert_eq!(object.get("x"), Some(Value::Number(2.0)));
}

#[test]
fn hot_kind_follows_polymorphic_selection_overwrite_and_clear() {
    use super::{HOT_INDEX_MASK, HOT_OWN_SLOT, NamedPropertyCacheEntry};
    let cache = NamedPropertyCache::default();
    let objects: Vec<_> = (0..6)
        .map(|i| {
            let object = ObjectRef::with_prototype(HashMap::new(), None);
            object.set("x".to_owned(), Value::String(format!("v{i}").into()));
            object
        })
        .collect();
    let check = || {
        let state = cache.0.borrow();
        let hot = state.hot.get();
        assert_eq!(
            hot & HOT_OWN_SLOT != 0,
            matches!(
                state.entries[usize::from(hot & HOT_INDEX_MASK)],
                Some(NamedPropertyCacheEntry::OwnSlot { .. })
            )
        );
    };
    for object in &objects {
        cache.update(object, "x", &object.get("x").unwrap());
        check();
        assert!(matches!(cache.probe(object), CacheProbe::Own(_)));
        check();
    }
    for object in objects.iter().rev().take(4) {
        assert!(matches!(cache.probe(object), CacheProbe::Own(_)));
        check();
    }
    cache.clear();
    check();
    assert_eq!(cache.probe_hot(&objects[5]), None);
    let literal = ObjectRef::with_literal_properties(
        ObjectLiteralShape::new(vec![Rc::from("x")]),
        vec![Value::Number(7.0)],
        None,
    );
    cache.update(&literal, "x", &Value::Number(7.0));
    check();
    assert!(matches!(
        cache.probe(&literal),
        CacheProbe::Own(Value::Number(7.0))
    ));
    check();
    assert_eq!(cache.probe_hot(&literal), None);
}
