//! Counter wiring and lifetime invariants, not performance assertions.
use crate::diagnostics::{
    counters, mechanism::PropertyScope, property_index::PropertyIndex, reset_counters,
};
use crate::{JsString, ObjectRef, Value};
use std::{collections::HashMap, rc::Rc};

#[test]
fn payload_lifetime_is_distinct_from_value_clones_and_handle_releases() {
    reset_counters();
    let value = Value::Object(ObjectRef::with_prototype(HashMap::new(), None));
    let copy = value.clone();
    let scalar = Value::Number(1.0).clone();
    assert_eq!(scalar, Value::Number(1.0));
    let c = counters().unwrap();
    assert_eq!(c.value_clones, 2);
    assert_eq!(c.value_heap_clones, 1);
    assert_eq!(c.heap_object_allocations, 1);
    drop(value);
    assert_eq!(counters().unwrap().heap_object_frees, 0);
    drop(copy);
    let c = counters().unwrap();
    assert_eq!(c.heap_handle_object_drops, 2);
    assert_eq!(c.heap_object_frees, 1);
}

#[test]
fn string_cow_allocates_and_releases_both_payloads() {
    reset_counters();
    let mut first = JsString::new("first".to_owned());
    let second = first.clone();
    first.make_mut().push('!');
    assert_eq!(second.as_str(), "first");
    assert_eq!(first.as_str(), "first!");
    assert_eq!(counters().unwrap().heap_string_allocations, 2);
    drop((first, second));
    let c = counters().unwrap();
    assert_eq!(c.heap_string_frees, 2);
    assert_eq!(c.heap_handle_string_drops, 2);
}

#[test]
fn property_index_counts_actual_hash_and_collision_equality() {
    let mut index = PropertyIndex::default();
    index.insert(Rc::from("field"), 7);
    reset_counters();
    assert_eq!(index.get("field"), Some(&7));
    let c = counters().unwrap();
    assert_eq!(c.property_name_hashes, 1);
    assert_eq!(c.property_name_hash_bytes, 5);
    assert_eq!(c.property_name_comparisons, 1);
    assert_eq!(c.property_name_compared_bytes, 5);
    assert_eq!(index.remove("field"), Some(7));
    assert_eq!(index.get("field"), None);
}

#[test]
fn name_scope_restores_after_nested_queries_and_unwind() {
    use crate::value::name_hash::NameHasher;
    use std::hash::{Hash, Hasher};
    reset_counters();
    let hash = || {
        let mut h = NameHasher::default();
        "name".hash(&mut h);
        h.finish()
    };
    hash();
    assert_eq!(counters().unwrap().property_name_hashes, 0);
    {
        let _first = PropertyScope::enter();
        {
            let _second = PropertyScope::enter();
            hash();
        }
        hash();
    }
    let _ = std::panic::catch_unwind(|| {
        let _scope = PropertyScope::enter();
        panic!("test unwind");
    });
    hash();
    assert_eq!(counters().unwrap().property_name_hashes, 2);
}

#[test]
fn vm_and_virtual_dispatch_counters_follow_executed_work() {
    reset_counters();
    crate::eval("var o = { x: 1 }; var s = 0; for(var i=0;i<10;i++) { s += o.x; } s;").unwrap();
    let c = counters().unwrap();
    assert!(c.frame_constructions > 0);
    assert_eq!(c.frame_constructions, c.nested_vm_constructions);
    assert!(c.executed_ops > 0);
    assert!(c.virtual_field_dispatches > 0);
    assert_eq!(c.frame_stack_transfers, 0);
}

#[test]
fn bigint_payload_destruction_is_swept_once() {
    reset_counters();
    let value = Value::bigint(123.into());
    let copy = value.clone();
    assert_eq!(counters().unwrap().heap_bigint_allocations, 1);
    drop(value);
    assert_eq!(counters().unwrap().heap_bigint_frees, 0);
    drop(copy);
    assert_eq!(counters().unwrap().heap_bigint_frees, 1);
    assert_eq!(counters().unwrap().heap_bigint_frees, 1);
}

#[test]
fn function_and_array_maps_count_hashes_and_equalities() {
    use crate::diagnostics::property_index::{ArrayPropertyMap, FunctionPropertyMap, PropertyMap};
    fn check<S: std::hash::BuildHasher>(mut map: PropertyMap<String, crate::Property, S>) {
        map.insert(
            "field".to_owned(),
            crate::Property::enumerable(Value::Number(7.0)),
        );
        reset_counters();
        assert!(map.get("field").is_some());
        let c = counters().unwrap();
        assert_eq!(c.property_name_hashes, 1);
        assert_eq!(c.property_name_hash_bytes, 5);
        assert_eq!(c.property_name_comparisons, 1);
    }
    check(FunctionPropertyMap::default());
    check(ArrayPropertyMap::default());
}

#[test]
fn generator_snapshot_counts_source_aggregate_transfer() {
    reset_counters();
    crate::eval("function* g() { yield 3; } var it = g(); it.next();").unwrap();
    let c = counters().unwrap();
    assert!(c.frame_aggregate_transfers > 0);
    assert!(c.frame_aggregate_transfer_bytes > 0);
    assert_eq!(c.frame_stack_transfers, 0);
}
