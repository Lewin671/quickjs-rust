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
        // slot. No new lookup is allowed to mistake a dynamic table for it.
        holder.set("extra".to_owned(), Value::Null);
        assert!(matches!(cache.probe(&receiver), CacheProbe::Miss));
        assert!(holder.prototype_data_slot_value(slot).is_none());
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
