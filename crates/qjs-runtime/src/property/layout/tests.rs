//! Layout transitions, ownership and retention bounds.

use std::rc::Rc;

use super::{
    APPEND_EDGE_LIMIT, Attributes, BulkEdit, DICTIONARY_AFTER_EDITS, Encoding, Layout, LayoutEdit,
    SHAPE_FIELD_LIMIT, SlotRemoval, layout_for_keys,
};
use crate::property::atom::AtomDomain;

fn keys(layout: &Layout) -> Vec<String> {
    layout
        .ordered_slots()
        .map(|slot| layout.fields()[slot].text().to_string())
        .collect()
}

fn appended(domain: &AtomDomain, names: &[&str]) -> Layout {
    let mut layout = domain.root(Encoding::Small);
    for name in names {
        LayoutEdit::append(&mut layout, name, Attributes::DEFAULT);
    }
    layout
}

#[test]
fn repeated_missing_writes_warm_the_same_index_as_reads() {
    let domain = AtomDomain::new();
    let names: Vec<_> = (0..20).map(|i| format!("field{i}")).collect();
    let refs: Vec<_> = names.iter().map(String::as_str).collect();
    let layout = appended(&domain, &refs);
    let shape = layout.as_shape().expect("shape");
    for _ in 0..super::INDEX_AFTER_PROBES {
        assert_eq!(layout.lookup_for_write("absent"), None);
        assert!(shape.index.get().is_none());
    }
    assert_eq!(layout.lookup_for_write("absent"), None);
    assert!(shape.index.get().is_some());
    assert_eq!(layout.lookup_for_write("field19"), Some(19));
    assert_eq!(layout.lookup("field0"), Some(0));
}

#[test]
fn one_pass_growth_does_not_index_transient_shapes() {
    let domain = AtomDomain::new();
    let mut layout = domain.root(Encoding::Small);
    for i in 0..64 {
        let key = format!("field{i}");
        assert_eq!(layout.lookup_for_write(&key), None);
        assert!(layout.as_shape().unwrap().index.get().is_none());
        LayoutEdit::append(&mut layout, &key, Attributes::DEFAULT);
    }
}

#[test]
fn equal_appends_share_one_shape_and_a_shape_is_a_stable_token() {
    let domain = AtomDomain::new();
    let _roots = domain.roots();
    let first = appended(&domain, &["x", "y"]);
    let second = appended(&domain, &["x", "y"]);
    assert!(Rc::ptr_eq(&first, &second));
    let other_order = appended(&domain, &["y", "x"]);
    assert!(!Rc::ptr_eq(&first, &other_order));
    assert_eq!(keys(&first), ["x", "y"]);
    assert_eq!(first.lookup("y"), Some(1));
    assert_eq!(first.lookup("z"), None);
}

#[test]
fn equal_keys_with_different_attributes_take_different_edges() {
    let domain = AtomDomain::new();
    let _roots = domain.roots();
    let mut hidden = domain.root(Encoding::Small);
    let attributes = Attributes::of(&crate::Property::non_enumerable(crate::Value::Undefined));
    LayoutEdit::append(&mut hidden, "k", attributes);
    let plain = appended(&domain, &["k"]);
    assert!(!Rc::ptr_eq(&hidden, &plain));
    assert!(!hidden.fields()[0].attributes.is_enumerable());
    assert!(plain.fields()[0].attributes.is_enumerable());
}

#[test]
fn a_small_shape_moves_to_dynamic_past_its_limit_through_one_edge() {
    let domain = AtomDomain::new();
    let _roots = domain.roots();
    let names: Vec<String> = (0..20).map(|index| format!("p{index}")).collect();
    let names: Vec<&str> = names.iter().map(String::as_str).collect();
    let first = appended(&domain, &names);
    assert_eq!(first.encoding(), Encoding::Dynamic);
    let shapes = domain.live_shapes();
    let second = appended(&domain, &names);
    assert!(Rc::ptr_eq(&first, &second));
    assert_eq!(
        domain.live_shapes(),
        shapes,
        "the second growth follows edges"
    );
    assert_eq!(second.lookup("p17"), Some(17));
}

#[test]
fn an_append_past_the_shape_limit_switches_to_a_dictionary() {
    let domain = AtomDomain::new();
    let _roots = domain.roots();
    let mut layout = domain.root(Encoding::Small);
    for index in 0..=SHAPE_FIELD_LIMIT {
        LayoutEdit::append(&mut layout, &format!("f{index}"), Attributes::DEFAULT);
    }
    assert!(layout.is_dictionary());
    assert_eq!(layout.len(), SHAPE_FIELD_LIMIT + 1);
    assert_eq!(layout.lookup("f128"), Some(SHAPE_FIELD_LIMIT));
    assert_eq!(keys(&layout)[0], "f0");
}

#[test]
fn removal_shifts_shape_slots_and_swaps_dictionary_slots_keeping_order() {
    let domain = AtomDomain::new();
    let _roots = domain.roots();
    let mut layout = appended(&domain, &["a", "b", "c", "d"]);
    let mut edits = 0;
    assert_eq!(
        LayoutEdit::remove(&mut layout, &mut edits, 1),
        SlotRemoval::Shift
    );
    assert_eq!(keys(&layout), ["a", "c", "d"]);
    assert_eq!(layout.lookup("d"), Some(2));

    LayoutEdit::to_dictionary(&mut layout);
    assert_eq!(
        LayoutEdit::remove(&mut layout, &mut edits, 0),
        SlotRemoval::SwapLast
    );
    assert_eq!(keys(&layout), ["c", "d"]);
    assert_eq!(layout.lookup("d"), Some(0));
    assert_eq!(layout.lookup("c"), Some(1));
    LayoutEdit::append(&mut layout, "a", Attributes::DEFAULT);
    assert_eq!(keys(&layout), ["c", "d", "a"]);
}

#[test]
fn repeated_edits_switch_to_a_dictionary_and_appends_never_count() {
    let domain = AtomDomain::new();
    let _roots = domain.roots();
    let mut layout = appended(&domain, &["a"]);
    let mut edits = 0;
    let hidden = Attributes::of(&crate::Property::non_enumerable(crate::Value::Undefined));
    for index in 0..u32::from(DICTIONARY_AFTER_EDITS) - 1 {
        LayoutEdit::append(&mut layout, &format!("x{index}"), Attributes::DEFAULT);
        let attributes = if index % 2 == 0 {
            hidden
        } else {
            Attributes::DEFAULT
        };
        LayoutEdit::reconfigure(&mut layout, &mut edits, 0, attributes);
        assert!(!layout.is_dictionary(), "edit {index}");
    }
    LayoutEdit::reconfigure(&mut layout, &mut edits, 0, hidden);
    LayoutEdit::reconfigure(&mut layout, &mut edits, 0, Attributes::DEFAULT);
    assert!(layout.is_dictionary());
    assert_eq!(layout.len(), DICTIONARY_AFTER_EDITS as usize);
}

#[test]
fn bulk_freeze_reconfigures_once_and_interns_nothing() {
    let domain = AtomDomain::new();
    let _roots = domain.roots();
    let mut layout = appended(&domain, &["a", "b", "c"]);
    let atoms = domain.created_count();
    let mut edits = 0;
    LayoutEdit::bulk(&mut layout, &mut edits, BulkEdit::Freeze);
    assert_eq!(domain.created_count(), atoms);
    assert!(
        layout.fields().iter().all(|field| {
            !field.attributes.is_writable() && !field.attributes.is_configurable()
        })
    );
    let mut again = appended(&domain, &["a", "b", "c"]);
    LayoutEdit::bulk(&mut again, &mut edits, BulkEdit::Freeze);
    assert!(Rc::ptr_eq(&layout, &again), "the edit edge is shared");
    let frozen = Rc::clone(&layout);
    LayoutEdit::bulk(&mut layout, &mut edits, BulkEdit::Freeze);
    assert!(
        Rc::ptr_eq(&layout, &frozen),
        "an unchanged edit transitions nowhere"
    );
}

#[test]
fn literal_keys_publish_one_layout_per_domain() {
    let domain = AtomDomain::new();
    let _roots = domain.roots();
    let keys = [("a", Attributes::DEFAULT), ("b", Attributes::DEFAULT)];
    let first = layout_for_keys(&domain, Encoding::Pair, keys);
    let second = layout_for_keys(&domain, Encoding::Pair, keys);
    assert!(Rc::ptr_eq(&first, &second));
    let other = AtomDomain::new();
    let _other_roots = other.roots();
    let foreign = layout_for_keys(&other, Encoding::Pair, keys);
    assert!(!Rc::ptr_eq(&first, &foreign));
    assert!(foreign.domain().ptr_eq(&other));
}

#[test]
fn append_edges_are_bounded_and_an_evicted_child_lives_with_its_users() {
    let domain = AtomDomain::new();
    let roots = domain.roots();
    let mut held = Vec::new();
    for index in 0..APPEND_EDGE_LIMIT + 4 {
        held.push(appended(&domain, &[&format!("k{index}")]));
    }
    let root = roots.root(Encoding::Small);
    assert_eq!(
        root.as_shape().expect("root shape").edge_count(),
        APPEND_EDGE_LIMIT
    );
    // The first child's edge was evicted; the object holding it still works.
    assert_eq!(held[0].lookup("k0"), Some(0));
    let shapes = domain.live_shapes();
    drop(held);
    assert_eq!(domain.live_shapes(), shapes - 4);
}

#[test]
fn dictionary_order_tombstones_stay_bounded_under_fixed_live_churn() {
    let domain = AtomDomain::new();
    let roots = domain.roots();
    let mut layout = appended(&domain, &["live"]);
    LayoutEdit::to_dictionary(&mut layout);
    let mut edits = 0;
    for index in 0..2000 {
        let slot = LayoutEdit::append(&mut layout, &format!("churn{index}"), Attributes::DEFAULT);
        assert_eq!(
            LayoutEdit::remove(&mut layout, &mut edits, slot),
            SlotRemoval::SwapLast
        );
        if index % 97 == 0 {
            layout
                .as_dictionary()
                .expect("dictionary")
                .assert_consistent();
        }
    }
    let dictionary = layout.as_dictionary().expect("dictionary");
    let tombstones = dictionary.assert_consistent();
    assert!(
        tombstones <= layout.len() + 64 + 1,
        "{tombstones} tombstones"
    );
    assert!(dictionary.order_len() <= 2 * layout.len() + 64 + 1);
    assert!(dictionary.version() >= 4000);
    assert_eq!(keys(&layout), ["live"]);
    assert_eq!(layout.lookup("live"), Some(0));
    assert_eq!(layout.lookup("churn1999"), None);
    let live = domain.live_count();
    assert!(live <= 1 + APPEND_EDGE_LIMIT, "{live} live atoms");
    assert!(domain.table_len() <= (2 * live).max(64) + 1);
    drop(layout);
    drop(roots);
    assert_eq!(domain.live_count(), 0);
}

#[test]
fn key_probes_distinguish_every_byte_at_inline_comparison_boundaries() {
    for len in 0..=40 {
        let key = "a".repeat(len);
        let equal = key.clone();
        assert!(super::text_matches(&key, &equal), "equal length {len}");
        assert!(!super::text_matches(&key, &format!("{key}a")));
        for changed in 0..len {
            let mut other = key.as_bytes().to_vec();
            other[changed] = b'b';
            let other = String::from_utf8(other).expect("ASCII test key");
            assert!(
                !super::text_matches(&key, &other),
                "length {len}, byte {changed}"
            );
        }
    }
    for key in ["é", "aé", "€é", "💡a", "a💡💡", "a💡💡💡", "a💡💡💡💡"] {
        let equal = key.to_owned();
        assert!(super::text_matches(key, &equal));
        let different = key.replace('é', "ê").replace('💡', "💢");
        assert!(!super::text_matches(key, &different));
    }
}
