use std::num::NonZeroU32;

use super::{AtomDomain, AtomDomainExhausted, canonical_property_text};
use crate::string::{string_code_units, string_from_code_units};

#[test]
fn equal_keys_share_one_atom_and_distinct_keys_do_not() {
    let domain = AtomDomain::new();
    let first = domain.intern("length").expect("intern");
    let again = domain.intern("length").expect("intern");
    let other = domain.intern("lengths").expect("intern");
    assert_eq!(first, again);
    assert_eq!(first.id(), again.id());
    assert_ne!(first, other);
    assert_ne!(first.id(), other.id());
    assert_eq!(domain.created_count(), 2);
}

#[test]
fn numeric_strings_keep_their_spelling() {
    let domain = AtomDomain::new();
    let one = domain.intern("1").expect("intern");
    let padded = domain.intern("01").expect("intern");
    let max = domain.intern("4294967295").expect("intern");
    let below_max = domain.intern("4294967294").expect("intern");
    let negative = domain.intern("-0").expect("intern");
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
    let lone_high = domain.intern(&high).expect("intern");
    let lone_low = domain.intern(&low).expect("intern");
    let replacement = domain.intern("\u{FFFD}").expect("intern");
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
    let from_halves = domain.intern(&halves).expect("intern");
    let from_scalar = domain.intern("\u{1F600}").expect("intern");
    assert_eq!(from_halves, from_scalar);
    assert_eq!(from_halves.as_str(), "\u{1F600}");
    // Reversed halves are not a pair and stay distinct.
    let reversed = domain
        .intern(&string_from_code_units(&[0xDE00, 0xD83D]))
        .expect("intern");
    assert_ne!(reversed, from_scalar);
}

#[test]
fn sentinel_range_scalars_stay_distinct_from_lone_surrogates() {
    let domain = AtomDomain::new();
    let scalar = crate::string::string_from_utf8_scalars("\u{F0000}");
    let lone = string_from_code_units(&[0xD800]);
    let scalar_atom = domain.intern(&scalar).expect("intern");
    let lone_atom = domain.intern(&lone).expect("intern");
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
    let a = first.intern("x").expect("intern");
    let b = second.intern("x").expect("intern");
    assert_ne!(a, b);
    assert!(a.belongs_to(&first));
    assert!(!a.belongs_to(&second));
    assert!(b.belongs_to(&second));
    let weak = first.downgrade();
    assert!(weak.is(&first));
    assert!(!weak.is(&second));
}

#[test]
fn reclaimed_atoms_get_fresh_ids() {
    let domain = AtomDomain::new();
    let first_id = domain.intern("transient").expect("intern").id();
    assert_eq!(domain.live_count(), 0);
    let second_id = domain.intern("transient").expect("intern").id();
    assert!(second_id > first_id, "ids must never be reused");
}

#[test]
fn key_churn_retention_is_bounded_by_live_atoms() {
    let domain = AtomDomain::new();
    let kept = domain.intern("kept").expect("intern");
    for index in 0..10_000 {
        let atom = domain.intern(&format!("churn{index}")).expect("intern");
        drop(atom);
    }
    assert_eq!(domain.live_count(), 1);
    assert!(
        domain.table_len() <= 2 * super::MIN_SWEEP_LEN,
        "{}",
        domain.table_len()
    );
    assert_eq!(domain.intern("kept").expect("intern"), kept);
}

#[test]
fn exhaustion_fails_instead_of_wrapping() {
    let domain = AtomDomain::starting_at(NonZeroU32::new(u32::MAX - 1).expect("nonzero"));
    let penultimate = domain.intern("a").expect("intern");
    let last = domain.intern("b").expect("intern");
    assert_eq!(penultimate.id().get(), u32::MAX - 1);
    assert_eq!(last.id().get(), u32::MAX);
    assert_eq!(domain.intern("c"), Err(AtomDomainExhausted));
    // Existing atoms are still found after exhaustion.
    assert_eq!(domain.intern("a").expect("intern"), penultimate);
}

#[test]
fn symbols_share_the_namespace_but_only_alias_by_identity() {
    let domain = AtomDomain::new();
    let first = crate::ObjectRef::new(std::collections::HashMap::new(), &domain);
    let second = crate::ObjectRef::new(std::collections::HashMap::new(), &domain);
    let text = domain.intern("same description").unwrap();
    let a = domain.intern_symbol(&first).unwrap();
    let again = domain.intern_symbol(&first).unwrap();
    let b = domain.intern_symbol(&second).unwrap();
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
        let lease = domain.intern_symbol(&symbol).unwrap();
        assert!(domain.lease(lease.id()).is_some());
    }
    assert_eq!(domain.live_count(), 0);
    assert!(domain.table_len() <= super::MIN_SWEEP_LEN);
    let table = domain.0.table.borrow();
    assert!(table.symbols.len() <= super::MIN_SWEEP_LEN);
}

#[test]
fn symbols_and_strings_exhaust_the_same_counter_without_wrapping() {
    let domain = AtomDomain::starting_at(NonZeroU32::new(u32::MAX).unwrap());
    let symbol = crate::ObjectRef::new(std::collections::HashMap::new(), &domain);
    let last = domain.intern_symbol(&symbol).unwrap();
    assert_eq!(last.id().get(), u32::MAX);
    assert_eq!(domain.intern("overflow"), Err(AtomDomainExhausted));
    assert_eq!(domain.intern_symbol(&symbol), Ok(last));
}

#[test]
fn read_only_lookup_remains_available_after_id_exhaustion() {
    let domain = AtomDomain::starting_at(NonZeroU32::new(u32::MAX).unwrap());
    let last = domain.intern("last").unwrap();
    assert_eq!(domain.intern("overflow"), Err(AtomDomainExhausted));
    assert_eq!(domain.find("last"), Some(last));
    assert_eq!(domain.find("absent"), None);
    assert_eq!(domain.created_count(), 1);
}
