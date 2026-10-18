use super::*;
use crate::Value;

fn fields(domain: &AtomDomain, key: &str, attributes: Attributes) -> Vec<Field> {
    vec![Field {
        key: domain.intern_key(key),
        attributes,
    }]
}
#[test]
fn equal_shapes_share_across_construction_histories_and_encodings_stay_distinct() {
    let domain = AtomDomain::new();
    let mut a = PropertyLayout::new(domain.clone(), Encoding::Small);
    let mut b = PropertyLayout::new(domain.clone(), Encoding::Small);
    a.publish(
        Encoding::Small,
        fields(&domain, "x", Attributes::DEFAULT),
        vec![0],
    );
    b.publish(
        Encoding::Small,
        fields(&domain, "temporary", Attributes::DEFAULT),
        vec![0],
    );
    b.publish(
        Encoding::Small,
        fields(&domain, "x", Attributes::DEFAULT),
        vec![0],
    );
    assert_eq!(a.token(), b.token());
    b.publish(
        Encoding::Pair,
        fields(&domain, "x", Attributes::DEFAULT),
        vec![0],
    );
    assert_ne!(a.token(), b.token());
}
#[test]
fn atom_lookup_checks_domains_and_exact_code_units() {
    let home = AtomDomain::new();
    let foreign = AtomDomain::new();
    let literal = "😀";
    let pair = crate::string::js_string_from_code_unit(0xd83d).to_string()
        + crate::string::js_string_from_code_unit(0xde00).as_ref();
    let mut layout = PropertyLayout::new(home.clone(), Encoding::Small);
    layout.publish(
        Encoding::Small,
        fields(&home, literal, Attributes::DEFAULT),
        vec![0],
    );
    assert_eq!(layout.lookup(&pair), layout.lookup(literal));
    assert_eq!(
        layout.lookup_atom(&foreign.intern(&pair).unwrap()),
        layout.lookup(literal)
    );
    let lone = crate::string::js_string_from_code_unit(0xd83d);
    assert!(layout.lookup(&lone).is_none());
}
#[test]
fn churn_switches_to_versioned_dictionary_and_retention_is_bounded() {
    let domain = AtomDomain::new();
    let mut layout = PropertyLayout::new(domain.clone(), Encoding::NamedMap);
    for index in 0..10000 {
        let name = format!("churn{index}");
        layout.publish(
            Encoding::NamedMap,
            fields(&domain, &name, Attributes::DEFAULT),
            vec![0],
        );
        assert_eq!(layout.lookup(&name).unwrap().slot, 0);
        layout.publish(Encoding::NamedMap, vec![], vec![]);
        assert!(layout.lookup(&name).is_none());
    }
    assert!(layout.is_dictionary());
    let before = layout.token();
    layout.publish(
        Encoding::NamedMap,
        fields(&domain, "kept", Attributes::DEFAULT),
        vec![0],
    );
    assert_eq!(before.0, layout.token().0);
    assert_ne!(before.1, layout.token().1);
    assert_eq!(domain.live_count(), 1);
    assert!(domain.table_len() <= 65, "{}", domain.table_len());
    assert!(domain.shapes().borrow().count <= 64);
    let before = layout.token();
    layout.publish(
        Encoding::NamedMap,
        fields(&domain, "kept", Attributes::DEFAULT),
        vec![0],
    );
    assert_eq!(before, layout.token());
}
#[test]
fn dictionary_version_and_identity_do_not_wrap_into_an_old_token() {
    let domain = AtomDomain::new();
    let mut layout = PropertyLayout::new(domain.clone(), Encoding::NamedMap);
    layout.state = State::Dictionary(Box::new(Dictionary {
        fields: vec![],
        order: vec![],
        index: HashMap::new(),
        encoding: Encoding::NamedMap,
        version: u64::MAX,
        identity: Rc::new(()),
    }));
    let State::Dictionary(dict) = &layout.state else {
        unreachable!()
    };
    let pinned = dict.identity.clone();
    layout.publish(
        Encoding::NamedMap,
        fields(&domain, "x", Attributes::DEFAULT),
        vec![0],
    );
    assert_ne!(Rc::as_ptr(&pinned) as usize, layout.token().0);
    assert_eq!(layout.token().1, 0);
}
#[test]
fn accessor_kind_and_each_attribute_are_part_of_the_authority() {
    let domain = AtomDomain::new();
    let mut layout = PropertyLayout::new(domain.clone(), Encoding::Small);
    let properties = [
        Property::enumerable(Value::Number(1.0)),
        Property::data(Value::Number(2.0), false, true, true),
        Property::data(Value::Number(2.0), false, false, true),
        Property::data(Value::Number(2.0), false, false, false),
        Property::accessor(None, None, false, false),
    ];
    for property in properties {
        let flags = Attributes::of(&property);
        let before = layout.token();
        layout.publish(Encoding::Small, fields(&domain, "x", flags), vec![0]);
        assert_ne!(before, layout.token());
        assert_eq!(layout.lookup("x").unwrap().attributes, flags);
    }
}
