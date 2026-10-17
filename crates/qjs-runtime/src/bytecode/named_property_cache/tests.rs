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

#[cfg(feature = "perf-counters")]
#[test]
fn cache_counter_answers_and_rejections_partition_each_kind() {
    use crate::diagnostics::{counters, reset_counters};
    let object = ObjectRef::with_prototype(HashMap::new(), None);
    object.set("x".to_owned(), Value::Number(3.0));
    let cache = NamedPropertyCache::default();
    reset_counters();
    assert_eq!(cache.probe_hot(&object), None);
    cache.update(&object, "x", &Value::Number(3.0));
    assert!(matches!(
        cache.probe(&object),
        CacheProbe::Own(Value::Number(3.0))
    ));
    cache.record_write(&object, "x");
    assert!(matches!(
        cache.write(&object, "x", &Value::Number(5.0)),
        Some(crate::value::OwnDataPropertyWrite::Written)
    ));
    let _ = cache.write_hot(&object, &Value::Number(6.0));
    object.delete_own_property("x");
    assert!(matches!(cache.probe(&object), CacheProbe::Miss));
    let c = counters().unwrap();
    assert_eq!(c.ic_write_probes, 1);
    assert_eq!(c.ic_write_hits, 1);
    assert_eq!(c.ic_hot_write_probes, 1);
    assert_eq!(c.ic_hot_probes, 1);
    assert_eq!(c.ic_hot_rejects, 1);
    assert!(c.ic_shared_slot_probes + c.ic_own_slot_probes + c.ic_exact_probes > 0);
    for kind in [
        "hot",
        "write",
        "hot_write",
        "exact",
        "literal",
        "own_slot",
        "shared_slot",
        "prototype",
        "grandprototype",
    ] {
        let entries: HashMap<_, _> = c.entries().into_iter().collect();
        assert_eq!(
            entries[format!("ic_{kind}_probes").as_str()],
            entries[format!("ic_{kind}_hits").as_str()]
                + entries[format!("ic_{kind}_rejects").as_str()]
        );
    }
}
