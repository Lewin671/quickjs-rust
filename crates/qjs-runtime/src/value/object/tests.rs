//! Unit tests for the object representation.
//!
//! Split out of `object.rs`, which holds the representation itself: the
//! property storage kinds, the slot and shape accessors, and the ordinary
//! object protocol. The tests are about the same subject but are not part of
//! reading it.

use std::{collections::HashMap, mem, rc::Rc};

use super::{ObjectData, ObjectLiteralShape, ObjectRef, OwnDataPropertyWrite};
use crate::property::layout::{Encoding, LINEAR_LOOKUP_LIMIT};
use crate::{Property, Value};

fn encoding(object: &ObjectRef) -> Encoding {
    object.0.properties.borrow().layout.encoding()
}

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
    let object = ObjectRef::new(HashMap::new());
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
    let object = ObjectRef::new(HashMap::from([
        ("a".to_owned(), Value::Number(1.0)),
        ("b".to_owned(), Value::Number(2.0)),
    ]));

    assert_eq!(object.get("a"), Some(Value::Number(1.0)));
    assert!(object.own_property_symbols().is_empty());
    assert!(object.to_string_tag().is_none());
    assert!(object.0.cold.get().is_none());
    // One layout pointer beside a keyless payload: the layout owns every
    // name, so an object stays within fifteen machine words. The one-byte
    // flags stay unpacked: packing them into a `Cell<u16>` frees six bytes
    // that alignment pads back, so the object would still be 120 bytes.
    assert!(mem::size_of::<ObjectData>() <= 120);
    assert!(mem::size_of::<std::cell::RefCell<super::PropertyStorage>>() <= 56);
}

#[test]
fn ordinary_small_object_promotes_only_after_the_compact_limit() {
    let object = ObjectRef::new(HashMap::new());

    for index in 0..LINEAR_LOOKUP_LIMIT {
        object.set(format!("field{index}"), Value::Number(index as f64));
    }
    assert_eq!(encoding(&object), Encoding::Small);
    assert_eq!(object.0.properties.borrow().len(), LINEAR_LOOKUP_LIMIT);

    let dynamic_index = LINEAR_LOOKUP_LIMIT;
    object.set(
        format!("field{dynamic_index}"),
        Value::Number(dynamic_index as f64),
    );
    assert_eq!(encoding(&object), Encoding::Dynamic);
    assert_eq!(object.0.properties.borrow().len(), LINEAR_LOOKUP_LIMIT + 1);
    assert_eq!(
        object.own_property_names(),
        (0..=LINEAR_LOOKUP_LIMIT)
            .map(|index| format!("field{index}"))
            .collect::<Vec<_>>()
    );
}

#[test]
fn objects_of_one_domain_share_each_interned_property_name() {
    let first = ObjectRef::new(HashMap::new());
    first.set_shared_key(Rc::from("field"), Value::Number(1.0));
    let second = ObjectRef::empty_with_prototype(first.clone());
    second.set_shared_key(Rc::from("field"), Value::Number(2.0));

    let way = first.shared_data_slot("field").expect("small slot");
    let other = second.shared_data_slot("field").expect("small slot");
    assert!(Rc::ptr_eq(&way.key, &other.key));
    assert_eq!(way.slot, other.slot);
    assert_eq!(
        second.shared_data_slot_value(&way),
        Some(Value::Number(2.0))
    );
    assert_eq!(first.get("field"), Some(Value::Number(1.0)));
}

#[test]
fn small_object_removal_preserves_property_order() {
    let object = ObjectRef::new(HashMap::new());
    object.set("first".to_owned(), Value::Number(1.0));
    object.set("second".to_owned(), Value::Number(2.0));
    object.set("third".to_owned(), Value::Number(3.0));

    assert!(object.delete_own_property("second"));
    object.set("second".to_owned(), Value::Number(4.0));

    assert_eq!(object.own_property_names(), ["first", "third", "second"]);
    assert_eq!(encoding(&object), Encoding::Small);
}

#[test]
fn small_object_enumerates_indices_before_strings() {
    let object = ObjectRef::new(HashMap::new());
    object.set("10".to_owned(), Value::Number(10.0));
    object.set("label".to_owned(), Value::Number(0.0));
    object.set("2".to_owned(), Value::Number(2.0));

    assert_eq!(object.own_property_names(), ["2", "10", "label"]);
    assert_eq!(encoding(&object), Encoding::Small);
}

#[test]
fn existing_own_data_write_updates_or_rejects_without_slow_path() {
    let object = ObjectRef::new(HashMap::from([("writable".to_owned(), Value::Number(1.0))]));
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
fn literal_pair_keeps_inline_values_until_descriptor_mutation() {
    let shape = ObjectLiteralShape::new(vec![Rc::from("a"), Rc::from("b")]);
    let object =
        ObjectRef::with_literal_pair(shape, [Value::Number(1.0), Value::Number(2.0)], None);

    assert_eq!(encoding(&object), Encoding::Pair);
    assert!(matches!(
        object.write_existing_own_data_property("a", &Value::Number(3.0)),
        OwnDataPropertyWrite::Written
    ));
    assert_eq!(object.get("a"), Some(Value::Number(3.0)));
    assert_eq!(encoding(&object), Encoding::Pair);

    object.define_property(
        "a".to_owned(),
        Property::data(Value::Number(4.0), false, false, true),
    );
    assert_eq!(encoding(&object), Encoding::Dynamic);
    let descriptor = object.own_property("a").expect("defined property");
    assert_eq!(descriptor.value, Value::Number(4.0));
    assert!(!descriptor.enumerable);
    assert!(!descriptor.writable);
}

#[test]
fn module_namespace_own_data_write_stays_on_slow_path() {
    let object = ObjectRef::new(HashMap::from([("exported".to_owned(), Value::Number(1.0))]));
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
    let shape = ObjectLiteralShape::new(vec![Rc::from("a"), Rc::from("b")]);
    let object =
        ObjectRef::with_literal_pair(shape, [Value::Number(1.0), Value::Number(2.0)], None);
    let (cached_shape, slot) = object.literal_data_slot("b").expect("literal slot");
    assert_eq!(slot, 1);
    assert!(matches!(
        object.write_existing_own_data_property("b", &Value::Number(5.0)),
        OwnDataPropertyWrite::Written
    ));
    assert_eq!(
        object.literal_data_slot_value(&cached_shape, slot),
        Some(Value::Number(5.0))
    );
    assert!(object.literal_data_slot("b").is_some());

    object.define_property(
        "b".to_owned(),
        Property::accessor(Some(Value::Undefined), None, true, true),
    );
    assert_eq!(object.literal_data_slot_value(&cached_shape, slot), None);
    assert!(object.literal_data_slot("b").is_none());
}

#[test]
fn dynamic_storage_keeps_order_values_and_slots_across_a_removal() {
    let object = ObjectRef::with_prototype(HashMap::new(), None);
    let count = LINEAR_LOOKUP_LIMIT + 4;
    for index in 0..count {
        object.set(format!("k{index}"), Value::Number(index as f64));
    }
    let last = format!("k{}", count - 1);
    assert_eq!(object.any_storage_data_slot(&last), Some(count - 1));
    let layout = object.layout_revision();
    assert!(object.delete_own_property("k2"));
    assert_ne!(object.layout_revision(), layout);
    // A shape keeps slots in insertion order, so later slots moved down.
    assert_eq!(object.any_storage_data_slot(&last), Some(count - 2));
    assert_eq!(object.get(&last), Some(Value::Number((count - 1) as f64)));
    let expected: Vec<String> = (0..count)
        .filter(|&index| index != 2)
        .map(|index| format!("k{index}"))
        .collect();
    assert_eq!(object.own_property_names(), expected);
    assert!(matches!(
        object.any_storage_data_slot_write(count - 2, &Value::Number(-1.0)),
        Some(OwnDataPropertyWrite::Written)
    ));
    assert_eq!(object.get(&last), Some(Value::Number(-1.0)));
}

#[test]
fn all_property_storage_forms_share_surrogate_pair_key_bytes() {
    use crate::string::{push_js_str, string_from_code_unit};
    let literal = "😀";
    let mut built = string_from_code_unit(0xD83D);
    push_js_str(&mut built, &string_from_code_unit(0xDE00));
    let small = ObjectRef::new(HashMap::new());
    let dynamic = ObjectRef::new(HashMap::new());
    for index in 0..13 {
        dynamic.set(format!("p{index}"), Value::Undefined);
    }
    let shaped = ObjectRef::with_literal_properties(
        ObjectLiteralShape::new(vec![Rc::from(literal)]),
        vec![Value::Number(0.0)],
        None,
    );
    let pair = ObjectRef::with_literal_pair(
        ObjectLiteralShape::new(vec![Rc::from(literal), Rc::from("other")]),
        [Value::Number(0.0), Value::Undefined],
        None,
    );
    assert_eq!(encoding(&small), Encoding::Small);
    assert_eq!(encoding(&dynamic), Encoding::Dynamic);
    assert_eq!(encoding(&shaped), Encoding::Shaped);
    assert_eq!(encoding(&pair), Encoding::Pair);
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

/// One literal site evaluated against prototypes of two realms builds each
/// object in its prototype's domain; switching realms back reuses that
/// realm's layout, and mutating one realm's instance leaves the other's
/// layout untouched.
#[test]
fn a_literal_site_shared_by_two_realms_builds_in_each_prototypes_domain() {
    let first = crate::property::atom::AtomDomain::new();
    let second = crate::property::atom::AtomDomain::new();
    let (_first_roots, _second_roots) = (first.roots(), second.roots());
    let first_prototype = ObjectRef::new_in(&first, HashMap::new());
    let second_prototype = ObjectRef::new_in(&second, HashMap::new());
    let shape = ObjectLiteralShape::new(vec![Rc::from("a"), Rc::from("b"), Rc::from("c")]);
    let build = |prototype: &ObjectRef, base: f64| {
        ObjectRef::with_literal_properties(
            Rc::clone(&shape),
            vec![
                Value::Number(base),
                Value::Number(base + 1.0),
                Value::Number(base + 2.0),
            ],
            Some(prototype.clone()),
        )
    };
    let a = build(&first_prototype, 1.0);
    let b = build(&second_prototype, 10.0);
    let c = build(&first_prototype, 20.0);
    let domain_is = |object: &ObjectRef, domain: &crate::property::atom::AtomDomain| {
        object.atom_domain().is_some_and(|own| own.ptr_eq(domain))
    };
    assert!(domain_is(&a, &first) && domain_is(&c, &first));
    assert!(domain_is(&b, &second));
    let layout = |object: &ObjectRef| Rc::clone(&object.0.properties.borrow().layout);
    assert!(Rc::ptr_eq(&layout(&a), &layout(&c)));
    assert!(!Rc::ptr_eq(&layout(&a), &layout(&b)));
    b.set("d".to_owned(), Value::Number(13.0));
    assert!(Rc::ptr_eq(&layout(&a), &layout(&c)));
    assert_eq!(c.get("c"), Some(Value::Number(22.0)));
    assert_eq!(b.own_property_keys(), ["a", "b", "c", "d"]);
    assert_eq!(a.own_property_keys(), ["a", "b", "c"]);
}

/// An object built from a host map orders its keys deterministically --
/// indices ascending, then the other keys sorted -- builds its layout through
/// the domain's append edges, so equal maps share one layout, and stays an
/// ordinary store afterwards.
#[test]
fn a_host_map_object_orders_keys_and_shares_its_edge_built_layout() {
    let domain = crate::property::atom::AtomDomain::new();
    let _roots = domain.roots();
    let map = |extra: usize| {
        let mut map: HashMap<String, Value> = ["b", "a", "10", "2"]
            .iter()
            .map(|key| ((*key).to_owned(), Value::String((*key).to_owned().into())))
            .collect();
        for index in 0..extra {
            map.insert(format!("k{index:02}"), Value::Number(index as f64));
        }
        map
    };
    let first = ObjectRef::new_in(&domain, map(0));
    let second = ObjectRef::new_in(&domain, map(0));
    assert_eq!(first.own_property_keys(), ["2", "10", "a", "b"]);
    let layout = |object: &ObjectRef| Rc::clone(&object.0.properties.borrow().layout);
    assert!(Rc::ptr_eq(&layout(&first), &layout(&second)));
    assert!(first.atom_domain().is_some_and(|own| own.ptr_eq(&domain)));
    assert!(first.delete_own_property("a"));
    first.set("a".to_owned(), Value::Number(1.0));
    assert_eq!(first.own_property_keys(), ["2", "10", "b", "a"]);
    assert_eq!(second.own_property_keys(), ["2", "10", "a", "b"]);
    let large = ObjectRef::new_in(&domain, map(20));
    assert_eq!(encoding(&large), Encoding::Dynamic);
    assert_eq!(large.get("k17"), Some(Value::Number(17.0)));
    assert_eq!(large.own_property_keys().len(), 24);
    assert_eq!(large.own_property_keys()[..3], ["2", "10", "a"]);
}

/// The live-shape budget, exercised by objects: 5000 live objects, each
/// holding a one-key shape of its own, push the domain past the budget;
/// past it an add still works but records no edge, and once the objects die
/// the shapes are released and edges are recorded again.
#[test]
fn the_live_shape_budget_stops_recording_edges_until_objects_die() {
    use crate::property::layout::{APPEND_EDGE_LIMIT, LIVE_SHAPE_BUDGET};
    let domain = crate::property::atom::AtomDomain::new();
    let roots = domain.roots();
    let root_edges = || {
        roots
            .root(Encoding::Small)
            .as_shape()
            .expect("root shape")
            .append_edge_keys()
    };
    let object_with = |key: &str| {
        let object = ObjectRef::new_in(&domain, HashMap::new());
        object.set(key.to_owned(), Value::Number(1.0));
        object
    };
    let base = domain.live_shapes();
    let objects: Vec<ObjectRef> = (0..5000)
        .map(|index| object_with(&format!("k{index}")))
        .collect();
    assert!(domain.live_shapes() >= LIVE_SHAPE_BUDGET);
    let edges_at_budget = root_edges();
    let extra = object_with("over-budget");
    assert_eq!(
        root_edges(),
        edges_at_budget,
        "no edge recorded at the budget"
    );
    assert_eq!(extra.get("over-budget"), Some(Value::Number(1.0)));
    assert_eq!(objects[0].get("k0"), Some(Value::Number(1.0)));
    drop(objects);
    drop(extra);
    assert!(domain.live_shapes() <= base + APPEND_EDGE_LIMIT);
    let _after = object_with("after");
    assert!(root_edges().iter().any(|key| key == "after"));
}
