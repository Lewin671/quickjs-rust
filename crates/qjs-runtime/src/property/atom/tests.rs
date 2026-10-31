use super::{AtomDomain, AtomDomainExhausted, canonical_property_text};
use crate::string::{string_code_units, string_from_code_units};

#[test]
fn equal_keys_share_one_atom_and_distinct_keys_do_not() {
    let domain = AtomDomain::new();
    let first = domain.intern("length");
    let again = domain.intern("length");
    let other = domain.intern("lengths");
    assert_eq!(first, again);
    assert_eq!(first.id(), again.id());
    assert_ne!(first, other);
    assert_ne!(first.id(), other.id());
    assert_eq!(domain.created_count(), 2);
}

#[test]
fn numeric_strings_keep_their_spelling() {
    let domain = AtomDomain::new();
    let one = domain.intern("1");
    let padded = domain.intern("01");
    let max = domain.intern("4294967295");
    let below_max = domain.intern("4294967294");
    let negative = domain.intern("-0");
    assert_ne!(one, padded);
    assert_eq!(one.array_index(), Some(1));
    assert_eq!(padded.array_index(), None);
    // 2^32 - 1 is not an array index; the largest one is 2^32 - 2.
    assert_eq!(max.array_index(), None);
    assert_eq!(below_max.array_index(), Some(4_294_967_294));
    assert_eq!(negative.array_index(), None);
    assert_eq!(padded.as_str(), "01");
}

#[test]
fn lone_surrogates_are_exact_keys() {
    let domain = AtomDomain::new();
    let high = string_from_code_units(&[0xD83D]);
    let low = string_from_code_units(&[0xDE00]);
    let lone_high = domain.intern(&high);
    let lone_low = domain.intern(&low);
    let replacement = domain.intern("\u{FFFD}");
    assert_ne!(lone_high, lone_low);
    assert_ne!(lone_high, replacement);
    assert_eq!(string_code_units(lone_high.as_str()), vec![0xD83D]);
}

#[test]
fn a_pair_stored_as_escaped_halves_is_the_whole_pair() {
    let domain = AtomDomain::new();
    // Exercise the defensive untyped intern boundary with raw escaped halves.
    // Runtime builders now merge these before they become JsString values.
    let halves = format!(
        "{}{}",
        string_from_code_units(&[0xD83D]),
        string_from_code_units(&[0xDE00])
    );
    assert_ne!(halves, "\u{1F600}");
    let from_halves = domain.intern(&halves);
    let from_scalar = domain.intern("\u{1F600}");
    assert_eq!(from_halves, from_scalar);
    assert_eq!(from_halves.as_str(), "\u{1F600}");
    // Reversed halves are not a pair and stay distinct.
    let reversed = domain.intern(&string_from_code_units(&[0xDE00, 0xD83D]));
    assert_ne!(reversed, from_scalar);
}

#[test]
fn sentinel_range_scalars_stay_distinct_from_lone_surrogates() {
    let domain = AtomDomain::new();
    let scalar = crate::string::string_from_utf8_scalars("\u{F0000}");
    let lone = string_from_code_units(&[0xD800]);
    let scalar_atom = domain.intern(&scalar);
    let lone_atom = domain.intern(&lone);
    assert_ne!(scalar_atom, lone_atom);
    assert_eq!(
        string_code_units(scalar_atom.as_str()),
        vec![0xDB80, 0xDC00]
    );
    assert!(matches!(
        canonical_property_text(&scalar),
        std::borrow::Cow::Borrowed(_)
    ));
}

#[test]
fn domains_do_not_share_identity() {
    let first = AtomDomain::new();
    let second = AtomDomain::new();
    let a = first.intern("x");
    let b = second.intern("x");
    assert_ne!(a, b);
    assert!(a.belongs_to(&first));
    assert!(!a.belongs_to(&second));
    assert!(b.belongs_to(&second));
    let weak = first.downgrade();
    assert!(weak.is(&first));
    assert!(!weak.is(&second));
}

#[test]
fn reclaimed_atoms_reuse_dead_ids() {
    let domain = AtomDomain::new();
    let first_id = domain.intern("transient").id();
    assert_eq!(domain.live_count(), 0);
    let second_id = domain.intern("transient").id();
    assert_eq!(second_id, first_id);
}

#[test]
fn key_churn_retention_is_bounded_by_live_atoms() {
    let domain = AtomDomain::new();
    let kept = domain.intern("kept");
    for index in 0..10_000 {
        let atom = domain.intern(&format!("churn{index}"));
        drop(atom);
    }
    assert_eq!(domain.live_count(), 1);
    assert!(
        domain.table_len() <= 2 * super::MIN_SWEEP_LEN,
        "{}",
        domain.table_len()
    );
    assert_eq!(domain.intern("kept"), kept);
}

#[test]
fn symbols_share_the_namespace_but_only_alias_by_identity() {
    let domain = AtomDomain::new();
    let first = crate::ObjectRef::new(std::collections::HashMap::new(), &domain);
    let second = crate::ObjectRef::new(std::collections::HashMap::new(), &domain);
    let text = domain.intern("same description");
    let a = domain.intern_symbol(&first);
    let again = domain.intern_symbol(&first);
    let b = domain.intern_symbol(&second);
    assert_eq!(a, again);
    assert_ne!(a, b);
    assert_ne!(a.id(), text.id());
    assert_ne!(b.id(), text.id());
    assert!(a.symbol().unwrap().ptr_eq(&first));
    assert_eq!(a.string_text(), None);
    assert_eq!(a.array_index(), None);
    assert_eq!(domain.lease(a.id()), Some(a.clone()));
    let id = a.id();
    drop(a);
    drop(again);
    assert!(domain.lease(id).is_none());
}

#[test]
fn symbol_and_directory_churn_is_swept_with_text_indices() {
    let domain = AtomDomain::new();
    for _ in 0..1000 {
        let symbol = crate::ObjectRef::new(std::collections::HashMap::new(), &domain);
        let lease = domain.intern_symbol(&symbol);
        assert!(domain.lease(lease.id()).is_some());
    }
    assert_eq!(domain.live_count(), 0);
    assert!(domain.table_len() <= super::MIN_SWEEP_LEN);
    let table = domain.0.table.borrow();
    assert!(table.symbols.len() <= super::MIN_SWEEP_LEN);
}

#[test]
fn test_cap_counts_simultaneously_live_records_and_reuses_both_kinds() {
    let domain = AtomDomain::with_limit(2);
    let symbol = crate::ObjectRef::new(std::collections::HashMap::new(), &domain);
    let a = domain.try_intern("a").unwrap();
    let b = domain.try_intern_symbol(&symbol).unwrap();
    assert_eq!(domain.try_intern("full"), Err(AtomDomainExhausted));
    assert_eq!(domain.find("a"), Some(a.clone()));
    let id = a.id();
    drop(a);
    let new = domain.try_intern("reused").unwrap();
    assert_eq!(new.id(), id);
    drop(b);
    assert!(domain.try_intern_symbol(&symbol).is_ok());
}
