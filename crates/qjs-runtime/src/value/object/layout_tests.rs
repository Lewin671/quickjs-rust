//! The same authority must follow every payload, not only literal receivers.
use super::*;
use crate::property::layout::Attributes;

fn receivers() -> Vec<ObjectRef> {
    let small = ObjectRef::new(HashMap::new());
    small.set("x".into(), Value::Number(1.0));
    let dynamic = ObjectRef::new(HashMap::new());
    for index in 0..14 {
        dynamic.set(format!("p{index}"), Value::Undefined);
    }
    dynamic.set("x".into(), Value::Number(1.0));
    let shaped = ObjectRef::with_literal_properties(
        ObjectLiteralShape::new(vec!["x".into(), "y".into(), "z".into()]),
        vec![Value::Number(1.0), Value::Null, Value::Null],
        None,
    );
    let pair = ObjectRef::with_literal_pair(
        ObjectLiteralShape::new(vec!["x".into(), "y".into()]),
        [Value::Number(1.0), Value::Null],
        None,
    );
    vec![small, dynamic, shaped, pair]
}
fn check(object: &ObjectRef) {
    let storage = object.0.properties.borrow();
    for slot in 0..storage.len() {
        let field = storage.layout.field(slot).unwrap();
        let descriptor = storage.get(field.key.text()).unwrap();
        let entry = storage.layout.lookup(field.key.text()).unwrap();
        assert_eq!(entry.slot, slot);
        assert_eq!(entry.attributes, Attributes::of(&descriptor));
    }
}
#[test]
fn every_payload_publishes_define_delete_reinsert_and_value_writes() {
    for object in receivers() {
        check(&object);
        let token = object.0.properties.borrow().layout.token();
        let revision = object.layout_revision();
        assert!(matches!(
            object.write_existing_own_data_property("x", &Value::Number(2.0)),
            OwnDataPropertyWrite::Written
        ));
        assert_eq!(token, object.0.properties.borrow().layout.token());
        assert_eq!(revision, object.layout_revision());
        let atom = object.atom_domain().intern("x").unwrap();
        assert!(matches!(
            object.own_data_property_atom_read(&atom),
            OwnDataPropertyRead::Data(Value::Number(2.0))
        ));
        object.define_property("x".into(), Property::accessor(None, None, false, true));
        check(&object);
        assert!(matches!(
            object.own_data_property_atom_read(&atom),
            OwnDataPropertyRead::NeedsSlowPath
        ));
        object.define_shared_property(
            "x".into(),
            Property::data(Value::Number(3.0), true, false, true),
        );
        check(&object);
        assert!(matches!(
            object.write_existing_own_data_property("x", &Value::Null),
            OwnDataPropertyWrite::ReadOnly
        ));
        assert!(object.delete_own_property("x"));
        assert!(matches!(
            object.own_data_property_atom_read(&atom),
            OwnDataPropertyRead::Missing
        ));
        object.set("x".into(), Value::Number(4.0));
        check(&object);
        assert_eq!(object.own_property_names().last().unwrap(), "x");
        object.set_internal_non_enumerable("hidden", Value::Boolean(true));
        check(&object);
        assert!(!object.own_property_names().contains(&"hidden".into()));
        assert!(object.has_own_property("hidden"));
    }
}
#[test]
fn every_payload_publishes_seal_and_freeze_to_old_and_new_consumers() {
    for object in receivers() {
        let revision = object.layout_revision();
        object.seal();
        check(&object);
        assert!(object.is_sealed());
        assert!(!object.delete_own_property("x"));
        assert_ne!(revision, object.layout_revision());
        assert!(matches!(
            object.write_existing_own_data_property("x", &Value::Number(4.0)),
            OwnDataPropertyWrite::Written
        ));
        let revision = object.layout_revision();
        object.freeze();
        check(&object);
        assert!(object.is_frozen());
        assert_ne!(revision, object.layout_revision());
        assert!(matches!(
            object.write_existing_own_data_property("x", &Value::Number(5.0)),
            OwnDataPropertyWrite::ReadOnly
        ));
    }
}
#[test]
fn lazy_string_indices_and_prototype_changes_keep_live_authority() {
    let object = ObjectRef::new(HashMap::new());
    object.set_internal_non_enumerable(
        crate::string::STRING_DATA_PROPERTY,
        Value::string_from_utf8("😀a"),
    );
    object.defer_string_indices();
    assert!(object.0.string_indices_pending.get());
    object.has_own_property("label");
    assert!(object.0.string_indices_pending.get());
    assert!(object.has_own_property("1"));
    assert!(!object.0.string_indices_pending.get());
    check(&object);
    let p = ObjectRef::new(HashMap::from([("inherited".into(), Value::Number(1.0))]));
    object.set_prototype(Some(p.clone())).unwrap();
    assert_eq!(object.get("inherited"), Some(Value::Number(1.0)));
    p.set("inherited".into(), Value::Number(2.0));
    assert_eq!(object.get("inherited"), Some(Value::Number(2.0)));
    object.set_prototype(None).unwrap();
    assert_eq!(object.get("inherited"), None);
}
#[test]
fn ordinary_construction_shares_shapes_without_literal_admission() {
    let prototype = ObjectRef::new(HashMap::new());
    let a = ObjectRef::with_prototype(HashMap::new(), Some(prototype.clone()));
    let b = ObjectRef::with_prototype(HashMap::new(), Some(prototype));
    for object in [&a, &b] {
        object.set("x".into(), Value::Number(1.0));
        object.set("y".into(), Value::Number(2.0));
    }
    assert_eq!(
        a.0.properties.borrow().layout.token(),
        b.0.properties.borrow().layout.token()
    );
    for i in 0..10000 {
        a.set(format!("temp{i}"), Value::Null);
        assert!(a.delete_own_property(&format!("temp{i}")));
    }
    check(&a);
    assert!(a.0.properties.borrow().layout.is_dictionary());
    assert_eq!(a.own_property_names(), ["x", "y"]);
    assert!(a.atom_domain().table_len() <= 68);
}

#[test]
fn host_map_encodings_collapse_before_publication_and_keep_domains_alive() {
    let halves = crate::string::js_string_from_code_unit(0xd83d).to_string()
        + crate::string::js_string_from_code_unit(0xde00).as_ref();
    let object = ObjectRef::new(HashMap::from([
        ("😀".into(), Value::Number(1.0)),
        (halves.clone(), Value::Number(2.0)),
    ]));
    assert_eq!(object.own_property_names().len(), 1);
    assert_eq!(object.get("😀"), object.get(&halves));
    check(&object);
    let weak_domain = object.atom_domain().downgrade();
    assert!(weak_domain.is_live());
    drop(object);
    assert!(!weak_domain.is_live());
}
