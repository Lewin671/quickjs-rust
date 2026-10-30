//! Unit tests for the object representation.
//!
//! Split out of `object.rs`, which holds the representation itself: the
//! property storage kinds, the slot and shape accessors, and the ordinary
//! object protocol. The tests are about the same subject but are not part of
//! reading it.

use std::{collections::HashMap, mem, rc::Rc};

use super::{ObjectData, ObjectLiteralShape, ObjectRef, OwnDataPropertyWrite};
use crate::{Property, Value};

#[test]
fn array_index_keys_accept_only_canonical_decimal_spellings() {
    use super::array_index_property_key as parse;
    assert_eq!(parse("0"), Some(0));
    assert_eq!(parse("7"), Some(7));
    assert_eq!(parse("4294967294"), Some(u32::MAX - 1));
    // `u32::MAX` is the length bound, never an index.
    assert_eq!(parse("4294967295"), None);
    assert_eq!(parse("4294967296"), None);
    // Non-canonical spellings name ordinary string properties instead.
    assert_eq!(parse(""), None);
    assert_eq!(parse("00"), None);
    assert_eq!(parse("01"), None);
    assert_eq!(parse("+1"), None);
    assert_eq!(parse("-1"), None);
    assert_eq!(parse(" 1"), None);
    assert_eq!(parse("1 "), None);
    assert_eq!(parse("1.0"), None);
    assert_eq!(parse("1e2"), None);
    assert_eq!(parse("0x1"), None);
    assert_eq!(parse("99999999999"), None);
}

#[test]
fn cloned_object_is_a_pointer_sized_shared_handle() {
    let domain = crate::property::atom::AtomDomain::new();
    let object = ObjectRef::new(HashMap::new(), &domain);
    let cloned = object.clone();

    assert!(Rc::ptr_eq(&object.0, &cloned.0));
    assert!(object.ptr_eq(&cloned));
    assert_eq!(
        mem::size_of::<ObjectRef>(),
        mem::size_of::<Rc<ObjectData>>()
    );
}

#[test]
fn ordinary_object_keeps_cold_state_unallocated() {
    let domain = crate::property::atom::AtomDomain::new();
    let object = ObjectRef::new_ordered(
        vec![
            ("a".to_owned(), Value::Number(1.0)),
            ("b".to_owned(), Value::Number(2.0)),
        ],
        &domain,
    );

    assert_eq!(object.get("a"), Some(Value::Number(1.0)));
    assert!(object.own_property_symbols().is_empty());
    assert!(object.to_string_tag().is_none());
    assert!(object.0.cold.get().is_none());
    assert_eq!(mem::size_of::<ObjectData>(), 80);
}

#[test]
fn ordinary_object_enters_dictionary_above_shared_shape_width() {
    let domain = crate::property::atom::AtomDomain::new();
    let object = ObjectRef::new(HashMap::new(), &domain);

    for index in 0..128 {
        object.set(format!("field{index}"), Value::Number(index as f64));
    }

    let dynamic_index = 128;
    object.set(
        format!("field{dynamic_index}"),
        Value::Number(dynamic_index as f64),
    );

    assert_eq!(
        object.own_property_names(),
        (0..=128)
            .map(|index| format!("field{index}"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn ordinary_object_retains_shared_static_property_key() {
    let domain = crate::property::atom::AtomDomain::new();
    let object = ObjectRef::new(HashMap::new(), &domain);
    let key: Rc<str> = Rc::from("field");

    object.set_shared_key(Rc::clone(&key), Value::Number(1.0));

    assert_eq!(object.get("field"), Some(Value::Number(1.0)));
}

#[test]
fn small_object_removal_preserves_property_order() {
    let domain = crate::property::atom::AtomDomain::new();
    let object = ObjectRef::new(HashMap::new(), &domain);
    object.set("first".to_owned(), Value::Number(1.0));
    object.set("second".to_owned(), Value::Number(2.0));
    object.set("third".to_owned(), Value::Number(3.0));

    assert!(object.delete_own_property("second"));
    object.set("second".to_owned(), Value::Number(4.0));

    assert_eq!(object.own_property_names(), ["first", "third", "second"]);
}

#[test]
fn small_object_enumerates_indices_before_strings() {
    let domain = crate::property::atom::AtomDomain::new();
    let object = ObjectRef::new(HashMap::new(), &domain);
    object.set("10".to_owned(), Value::Number(10.0));
    object.set("label".to_owned(), Value::Number(0.0));
    object.set("2".to_owned(), Value::Number(2.0));

    assert_eq!(object.own_property_names(), ["2", "10", "label"]);
}

#[test]
fn existing_own_data_write_updates_or_rejects_without_slow_path() {
    let domain = crate::property::atom::AtomDomain::new();
    let object = ObjectRef::new_ordered(vec![("writable".to_owned(), Value::Number(1.0))], &domain);
    object.define_property(
        "readonly".to_owned(),
        Property::data(Value::Number(2.0), true, false, true),
    );

    assert!(matches!(
        object.write_existing_own_data_property("writable", &Value::Number(3.0)),
        OwnDataPropertyWrite::Written
    ));
    assert_eq!(object.get("writable"), Some(Value::Number(3.0)));
    assert!(matches!(
        object.write_existing_own_data_property("readonly", &Value::Number(4.0)),
        OwnDataPropertyWrite::ReadOnly
    ));
    assert_eq!(object.get("readonly"), Some(Value::Number(2.0)));
    assert!(matches!(
        object.write_existing_own_data_property("missing", &Value::Number(5.0)),
        OwnDataPropertyWrite::NeedsSlowPath
    ));
}

#[test]
fn literal_data_writes_preserve_slots_until_descriptor_mutation() {
    let domain = crate::property::atom::AtomDomain::new();
    let shape = ObjectLiteralShape::new(vec![Rc::from("a"), Rc::from("b")]);
    let object = ObjectRef::with_literal_pair(
        shape,
        [Value::Number(1.0), Value::Number(2.0)],
        None,
        &domain,
    );

    assert!(matches!(
        object.write_existing_own_data_property("a", &Value::Number(3.0)),
        OwnDataPropertyWrite::Written
    ));
    assert_eq!(object.get("a"), Some(Value::Number(3.0)));

    object.define_property(
        "a".to_owned(),
        Property::data(Value::Number(4.0), false, false, true),
    );

    let descriptor = object.own_property("a").expect("defined property");
    assert_eq!(descriptor.value, Value::Number(4.0));
    assert!(!descriptor.enumerable);
    assert!(!descriptor.writable);
}

#[test]
fn module_namespace_own_data_write_stays_on_slow_path() {
    let domain = crate::property::atom::AtomDomain::new();
    let object = ObjectRef::new_ordered(vec![("exported".to_owned(), Value::Number(1.0))], &domain);
    object.mark_module_namespace_exotic();

    assert!(matches!(
        object.write_existing_own_data_property("exported", &Value::Number(2.0)),
        OwnDataPropertyWrite::NeedsSlowPath
    ));
    assert_eq!(object.get("exported"), Some(Value::Number(1.0)));
}

/// A literal's shared shape identifies its layout, so a value overwritten in
/// place keeps the cached slot valid; redefining the slot as an accessor is
/// what invalidates it.
#[test]
fn literal_slot_survives_value_writes_and_misses_on_accessors() {
    let domain = crate::property::atom::AtomDomain::new();
    let shape = ObjectLiteralShape::new(vec![Rc::from("a"), Rc::from("b")]);
    let object = ObjectRef::with_literal_pair(
        shape,
        [Value::Number(1.0), Value::Number(2.0)],
        None,
        &domain,
    );
    let (cached_shape, slot) = object.layout_data_slot("b").expect("literal slot");
    assert_eq!(slot, 1);
    assert!(matches!(
        object.write_existing_own_data_property("b", &Value::Number(5.0)),
        OwnDataPropertyWrite::Written
    ));
    assert_eq!(
        object.layout_data_slot_value(&cached_shape, slot),
        Some(Value::Number(5.0))
    );
    assert!(object.layout_data_slot("b").is_some());

    object.define_property(
        "b".to_owned(),
        Property::accessor(Some(Value::Undefined), None, true, true),
    );
    assert_eq!(object.layout_data_slot_value(&cached_shape, slot), None);
    assert!(object.layout_data_slot("b").is_none());
}

#[test]
fn dictionary_keeps_order_values_and_slots_across_a_removal() {
    let domain = crate::property::atom::AtomDomain::new();
    let object = ObjectRef::with_prototype(HashMap::new(), None, &domain);
    let count = 132;
    for index in 0..count {
        object.set(format!("k{index}"), Value::Number(index as f64));
    }
    let last = format!("k{}", count - 1);
    assert_eq!(object.any_storage_data_slot(&last), Some(count - 1));
    let layout = object.layout_revision();
    assert!(object.delete_own_property("k2"));
    assert_ne!(object.layout_revision(), layout);
    // The last property moved into the removed slot; the order is unchanged.
    assert_eq!(object.any_storage_data_slot(&last), Some(2));
    assert_eq!(object.get(&last), Some(Value::Number((count - 1) as f64)));
    let expected: Vec<String> = (0..count)
        .filter(|&index| index != 2)
        .map(|index| format!("k{index}"))
        .collect();
    assert_eq!(object.own_property_names(), expected);
    assert!(matches!(
        object.any_storage_data_slot_write(2, &Value::Number(-1.0)),
        Some(OwnDataPropertyWrite::Written)
    ));
    assert_eq!(object.get(&last), Some(Value::Number(-1.0)));
}

#[test]
fn ordinary_creation_paths_share_surrogate_pair_key_identity() {
    let domain = crate::property::atom::AtomDomain::new();
    use crate::string::{push_js_str, string_from_code_unit};
    let literal = "😀";
    let mut built = string_from_code_unit(0xD83D);
    push_js_str(&mut built, &string_from_code_unit(0xDE00));
    let small = ObjectRef::new(HashMap::new(), &domain);
    let dynamic = ObjectRef::new(HashMap::new(), &domain);
    for index in 0..13 {
        dynamic.set(format!("p{index}"), Value::Undefined);
    }
    let shaped = ObjectRef::with_literal_properties(
        ObjectLiteralShape::new(vec![Rc::from(literal)]),
        vec![Value::Number(0.0)],
        None,
        &domain,
    );
    let pair = ObjectRef::with_literal_pair(
        ObjectLiteralShape::new(vec![Rc::from(literal), Rc::from("other")]),
        [Value::Number(0.0), Value::Undefined],
        None,
        &domain,
    );

    for object in [small, dynamic, shaped, pair] {
        for (stored, queried) in [(literal, built.as_str()), (built.as_str(), literal)] {
            object.set(stored.to_owned(), Value::Number(1.0));
            assert_eq!(object.get(queried), Some(Value::Number(1.0)));
            object.set(queried.to_owned(), Value::Number(2.0));
            assert_eq!(object.get(stored), Some(Value::Number(2.0)));
            assert_eq!(
                object
                    .own_property_names()
                    .iter()
                    .filter(|key| key.as_str() == literal)
                    .count(),
                1
            );
        }
    }
}

#[test]
fn explicit_host_order_is_validated_before_key_creation() {
    let domain = crate::property::atom::AtomDomain::new();
    let make = || {
        HashMap::from([
            ("a".to_owned(), Value::Number(1.0)),
            ("b".to_owned(), Value::Number(2.0)),
        ])
    };
    assert!(
        ObjectRef::with_property_order(make(), vec!["a".into(), "a".into()], None, &domain)
            .is_err()
    );
    assert_eq!(domain.created_count(), 0);
    let object =
        ObjectRef::with_property_order(make(), vec!["b".into(), "a".into()], None, &domain)
            .unwrap();
    assert_eq!(object.own_property_names(), ["b", "a"]);
}

#[test]
fn string_virtual_indices_do_not_intern_or_materialize_on_reads() {
    let domain = crate::property::atom::AtomDomain::new();
    let object = ObjectRef::new(HashMap::new(), &domain);
    object.set_string_data("a😀".into());
    object.defer_string_indices();
    let count = domain.created_count();
    assert_eq!(object.get("0"), Some(Value::String("a".into())));
    assert!(object.own_property("1").is_some());
    assert_eq!(object.own_property_keys(), ["0", "1", "2"]);
    assert_eq!(object.0.properties.borrow().len(), 0);
    assert_eq!(domain.created_count(), count);
    assert!(!object.delete_own_property("0"));
}

#[cfg(feature = "perf-counters")]
#[test]
fn ordinary_constructor_append_hit_uses_one_probe_and_no_interns_or_copies() {
    let owner = crate::value::layout::PropertyOwner::new(crate::property::atom::AtomDomain::new());
    let first = ObjectRef::new(HashMap::new(), owner.domain());
    first.set("x".into(), Value::Number(1.0));
    let second = ObjectRef::new(HashMap::new(), owner.domain());
    let before = crate::diagnostics::counters().unwrap();
    second.set("x".into(), Value::Number(2.0));
    let after = crate::diagnostics::counters().unwrap();
    assert_eq!(after.property_id_probes - before.property_id_probes, 1);
    assert_eq!(after.property_key_interns - before.property_key_interns, 0);
    assert_eq!(
        after.property_shape_field_copies - before.property_shape_field_copies,
        0
    );
    assert_eq!(
        after.property_transition_hits - before.property_transition_hits,
        1
    );
}

#[test]
fn nonextensible_receiver_still_calls_an_inherited_setter() {
    assert_eq!(
        crate::eval(
            "let hit = 0; let parent = { set x(v) { hit = v; } }; let child = Object.create(parent); Object.preventExtensions(child); child.x = 7; hit;"
        ),
        Ok(Value::Number(7.0))
    );
}

#[test]
fn construction_routes_share_a_layout_only_after_prototypes_match() {
    let Value::Array(values) = crate::eval("function C() { this.a = 1; this.b = 2; } class K { a = 3; b = 4; } const n = Object.create(null); n.a = 5; n.b = 6; [new C(), new K(), JSON.parse('{\"a\":7,\"b\":8}'), {a:9,b:10}, n];").unwrap() else { panic!("expected objects"); };
    let mut layouts = Vec::new();
    for index in 0..values.len() {
        let Value::Object(object) = values.get(index).unwrap() else {
            panic!("expected ordinary object");
        };
        assert_eq!(object.own_property_names(), ["a", "b"]);
        layouts.push(object.layout_data_slot("a").unwrap().0);
    }
    assert!(Rc::ptr_eq(&layouts[2], &layouts[3]));
    assert!(!Rc::ptr_eq(&layouts[0], &layouts[2]));
    assert!(!Rc::ptr_eq(&layouts[1], &layouts[2]));
    assert!(!Rc::ptr_eq(&layouts[4], &layouts[2]));
    let Value::Object(reference) = values.get(2).unwrap() else {
        unreachable!()
    };
    for index in 0..values.len() {
        let Value::Object(object) = values.get(index).unwrap() else {
            unreachable!()
        };
        object
            .set_prototype_slot(reference.prototype_slot())
            .unwrap();
        assert!(Rc::ptr_eq(
            &reference.layout_data_slot("a").unwrap().0,
            &object.layout_data_slot("a").unwrap().0
        ));
    }
}

#[test]
fn ordered_host_input_preserves_duplicates_and_observable_insertion_order() {
    let domain = crate::property::atom::AtomDomain::new();
    let object = ObjectRef::new_ordered(
        vec![
            ("value".into(), Value::Number(1.0)),
            ("done".into(), Value::Boolean(false)),
            ("value".into(), Value::Number(2.0)),
        ],
        &domain,
    );
    assert_eq!(object.own_property_names(), ["value", "done"]);
    assert_eq!(object.get("value"), Some(Value::Number(2.0)));
}

#[test]
fn string_data_cannot_be_forged_or_overwritten_by_an_ordinary_key() {
    let object = ObjectRef::new(HashMap::new(), &crate::property::atom::AtomDomain::new());
    object.set("\0StringData".into(), Value::String("forged".into()));
    assert!(object.string_data().is_none());
    object.set_string_data("actual".into());
    object.set("\0StringData".into(), Value::String("changed".into()));
    assert_eq!(object.string_data().unwrap(), "actual");
    assert_eq!(object.own_property_names(), ["\0StringData"]);
}
