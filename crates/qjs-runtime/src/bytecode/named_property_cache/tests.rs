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

fn selected_own_slot_ready_for_overwrite() -> (NamedPropertyCache, ObjectRef) {
    let object = ObjectRef::with_prototype(HashMap::new(), None);
    object.set("items".to_owned(), Value::String("own".into()));
    let cache = NamedPropertyCache::default();
    for _ in 0..super::POLYMORPHIC_CACHE_SLOTS {
        cache.update(&object, "items", &Value::String("own".into()));
    }
    assert!(matches!(cache.probe(&object), CacheProbe::Own(_)));
    let state = cache.0.borrow();
    assert_eq!(state.next_slot, 0);
    assert_eq!(state.hot.get(), super::HOT_OWN_SLOT);
    drop(state);
    (cache, object)
}

fn assert_selected_hint_is_plain(cache: &NamedPropertyCache) {
    let state = cache.0.borrow();
    assert_eq!(state.hot.get(), 0);
    assert_eq!(state.hot_codes[0], 0);
}

#[test]
fn selected_own_slot_replacement_tracks_exact_shared_and_literal_entries() {
    use super::NamedPropertyCacheEntry;

    let (cache, object) = selected_own_slot_ready_for_overwrite();
    object.write_existing_own_data_property("items", &Value::Number(7.0));
    cache.update(&object, "items", &Value::Number(7.0));
    assert_selected_hint_is_plain(&cache);
    assert!(matches!(
        cache.0.borrow().entries[0],
        Some(NamedPropertyCacheEntry::Exact { .. })
    ));
    assert_eq!(cache.probe_hot(&object), None);
    assert!(matches!(
        cache.probe(&object),
        CacheProbe::Own(Value::Number(7.0))
    ));

    let (cache, _) = selected_own_slot_ready_for_overwrite();
    let shared_receiver = ObjectRef::with_prototype(HashMap::new(), None);
    shared_receiver.set("items".to_owned(), Value::String("shared".into()));
    cache.update(&shared_receiver, "items", &Value::String("shared".into()));
    assert_selected_hint_is_plain(&cache);
    assert!(matches!(
        cache.0.borrow().entries[0],
        Some(NamedPropertyCacheEntry::SharedSlot { .. })
    ));
    assert_eq!(
        cache.probe_hot(&shared_receiver),
        Some(Value::String("shared".into()))
    );

    let (cache, _) = selected_own_slot_ready_for_overwrite();
    let literal = ObjectRef::with_literal_properties(
        ObjectLiteralShape::new(vec![Rc::from("items")]),
        vec![Value::Number(9.0)],
        None,
    );
    cache.update(&literal, "items", &Value::Number(9.0));
    assert_selected_hint_is_plain(&cache);
    assert!(matches!(
        cache.0.borrow().entries[0],
        Some(NamedPropertyCacheEntry::LiteralShape { .. })
    ));
    assert_eq!(cache.probe_hot(&literal), None);
    assert!(matches!(
        cache.probe(&literal),
        CacheProbe::Own(Value::Number(9.0))
    ));
}

#[test]
fn selected_own_slot_replacement_tracks_both_prototype_entry_kinds() {
    use super::NamedPropertyCacheEntry;

    for grandprototype in [false, true] {
        let (cache, _) = selected_own_slot_ready_for_overwrite();
        let holder = ObjectRef::with_prototype(HashMap::new(), None);
        holder.set("items".to_owned(), Value::String("inherited".into()));
        let parent = if grandprototype {
            ObjectRef::with_prototype(HashMap::new(), Some(holder))
        } else {
            holder
        };
        let receiver = ObjectRef::with_prototype(HashMap::new(), Some(parent));
        cache.update_from_prototype(&receiver, "items");
        assert_selected_hint_is_plain(&cache);
        assert_eq!(
            matches!(
                cache.0.borrow().entries[0],
                Some(NamedPropertyCacheEntry::GrandprototypeSlot { .. })
            ),
            grandprototype
        );
        assert_eq!(cache.probe_hot(&receiver), None);
        cache.remember_receiver_miss(&receiver);
        assert_eq!(
            cache.probe_hot(&receiver),
            Some(Value::String("inherited".into()))
        );
    }
}

#[test]
fn nonselected_exact_write_updates_the_hint_before_a_later_probe_selects_it() {
    use super::{HOT_OWN_SLOT, NamedPropertyCacheEntry};

    let object = ObjectRef::with_prototype(HashMap::new(), None);
    object.set("items".to_owned(), Value::Number(1.0));
    let other = ObjectRef::with_prototype(HashMap::new(), None);
    other.set("items".to_owned(), Value::String("other".into()));
    let cache = NamedPropertyCache::default();
    cache.update(&object, "items", &Value::Number(1.0));
    cache.update(&other, "items", &Value::String("other".into()));
    assert!(matches!(cache.probe(&other), CacheProbe::Own(_)));
    assert_eq!(cache.0.borrow().hot.get(), 1);
    assert!(matches!(
        cache.0.borrow().entries[0],
        Some(NamedPropertyCacheEntry::Exact { .. })
    ));
    assert!(matches!(
        cache.write(&object, "items", &Value::Number(2.0)),
        Some(crate::value::OwnDataPropertyWrite::Written)
    ));
    let state = cache.0.borrow();
    assert_eq!(state.hot.get(), 1);
    assert_eq!(state.hot_codes[0], HOT_OWN_SLOT);
    assert!(matches!(
        state.entries[0],
        Some(NamedPropertyCacheEntry::OwnSlot { .. })
    ));
    drop(state);
    assert_eq!(cache.probe_hot(&other), Some(Value::String("other".into())));
    assert!(matches!(
        cache.probe(&object),
        CacheProbe::Own(Value::Number(2.0))
    ));
    assert_eq!(cache.0.borrow().hot.get(), HOT_OWN_SLOT);
    assert_eq!(cache.probe_hot(&object), Some(Value::Number(2.0)));
}

#[test]
fn hot_own_slot_entries_keep_neither_receivers_nor_replaced_values_alive() {
    let receiver = ObjectRef::with_prototype(HashMap::new(), None);
    receiver.set("items".to_owned(), Value::String("first".into()));
    let weak_receiver = receiver.downgrade();
    let value = ObjectRef::with_prototype(HashMap::new(), None);
    let weak_value = value.downgrade();
    let cache = NamedPropertyCache::default();
    cache.update(&receiver, "items", &Value::String("first".into()));
    receiver.write_existing_own_data_property("items", &Value::Object(value));
    assert!(matches!(cache.probe_hot(&receiver), Some(Value::Object(_))));
    receiver.write_existing_own_data_property("items", &Value::Undefined);
    assert!(weak_value.upgrade().is_none());
    assert_eq!(cache.probe_hot(&receiver), Some(Value::Undefined));
    drop(receiver);
    assert!(weak_receiver.upgrade().is_none());
}

#[test]
fn named_reads_follow_getters_and_proxy_fallbacks_after_own_data_warmup() {
    let source = r#"
        var log = [], answers = [], guard = 0;
        var receiver = Object.create(null), other = Object.create(null);
        receiver.items = 'own'; other.items = 'other';
        function read(o) {
            var value = o.items;
            return typeof value + ':' + value;
        }
        for (var i = 0; i < 8; i++) {
            if (++guard > 16) throw new Error('warmup guard');
            if (read(receiver) !== 'string:own') throw new Error('own data');
        }
        receiver.items = 'changed';
        answers.push(read(receiver));
        Object.defineProperty(receiver, 'items', {
            configurable: true,
            get: function () {
                log.push('own:' + (this === receiver));
                return read(other);
            }
        });
        answers.push(read(receiver));
        delete receiver.items;
        var prototype = Object.create(null);
        Object.defineProperty(prototype, 'items', {
            configurable: true,
            get: function () {
                log.push('inherited:' + (this === receiver ? 'receiver' : this === proxy ? 'proxy' : 'wrong'));
                return read(other);
            }
        });
        Object.setPrototypeOf(receiver, prototype);
        answers.push(read(receiver));
        var proxy = new Proxy(receiver, {
            get: function (target, key, self) {
                log.push('trap:' + (key === 'items' && self === proxy));
                return Reflect.get(target, key, self);
            }
        });
        answers.push(read(proxy));
        answers.join('|') + ';' + log.join('|');
    "#;
    assert_eq!(
        crate::eval(source).expect("reentrant getter and Proxy reads should complete"),
        Value::String(
            concat!(
                "string:changed|string:string:other|string:string:other|string:string:other;",
                "own:true|inherited:receiver|trap:true|inherited:proxy"
            )
            .into()
        )
    );
}
