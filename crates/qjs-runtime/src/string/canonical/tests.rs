use super::*;
use crate::JsString;
use crate::string::{
    char_from_code_unit, push_code_unit, string_code_units, string_from_code_units,
};

#[test]
fn append_repairs_only_high_low_seams() {
    for units in [
        [0xD83D, 0xDE00],
        [0xD83D, 0xD800],
        [0xDE00, 0xD800],
        [0xDE00, 0xDC00],
        [0xD800, 0x61],
    ] {
        let mut text = string_from_code_units(&units[..1]);
        push_js_str(&mut text, &string_from_code_units(&units[1..]));
        assert_eq!(string_code_units(&text), units);
        assert!(is_canonical(&text));
        assert_eq!(
            text.chars().count(),
            if units == [0xD83D, 0xDE00] { 1 } else { 2 }
        );
    }
    let mut text = String::new();
    push_js_str(&mut text, "");
    push_code_unit(&mut text, 0xD83D);
    push_js_str(&mut text, "");
    push_code_unit(&mut text, 0xDE00);
    assert_eq!(text, "😀");
}

#[test]
fn every_sentinel_range_scalar_stays_escaped_at_the_seam() {
    for scalar in 0xF0000..=0xF07FF {
        let mut buffer = [0; 2];
        let units = char::from_u32(scalar).unwrap().encode_utf16(&mut buffer);
        let mut text = string_from_code_units(&units[..1]);
        push_js_str(&mut text, &string_from_code_units(&units[1..]));
        assert_eq!(text.chars().count(), 2);
        assert!(is_canonical(&text));
        assert_eq!(string_code_units(&text), units);
    }
}

#[test]
fn repeat_repairs_each_seam_and_preserves_outer_lone_units() {
    let text = string_from_code_units(&[0xDE00, 0xD83D]);
    for count in 0..5 {
        let repeated = repeat_js_str(&text, count);
        assert!(is_canonical(&repeated));
        assert_eq!(string_code_units(&repeated), [0xDE00, 0xD83D].repeat(count));
        assert_eq!(
            repeated.chars().count(),
            if count == 0 { 0 } else { count + 1 }
        );
    }
    assert_eq!(repeat_js_str("ascii", 3), "asciiasciiascii");
}

#[test]
fn pad_and_repeat_results_use_canonical_bytes_and_keep_aliases() {
    for source in [
        r#"'\uDE00'.padStart(2, '\uD83D')"#,
        r#"'\uD83D'.padEnd(2, '\uDE00')"#,
        r#"'\uDE00\uD83D'.repeat(3).slice(1, 5)"#,
        r#"(function () { let text = '\uD83D'; let alias = text; text += '\uDE00'; return alias + ':' + text; })()"#,
    ] {
        let value = crate::eval(source).unwrap();
        let crate::Value::String(text) = value else {
            panic!("string expected")
        };
        assert!(is_canonical(&text));
        if source.contains("pad") {
            assert_eq!(text.as_str(), "😀");
        }
    }
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "non-canonical JsString")]
fn constructor_rejects_unmerged_sentinels_in_debug_builds() {
    JsString::new(
        [char_from_code_unit(0xD83D), char_from_code_unit(0xDE00)]
            .iter()
            .collect(),
    );
}

#[test]
fn canonical_handles_compare_and_hash_identically() {
    use std::collections::{BTreeSet, HashSet};
    let literal = JsString::from("😀");
    let built = JsString::new(string_from_code_units(&[0xD83D, 0xDE00]));
    assert_eq!(literal, built);
    assert_eq!(HashSet::from([literal.clone(), built.clone()]).len(), 1);
    assert_eq!(BTreeSet::from([literal, built]).len(), 1);
}

#[test]
fn producers_and_consumers_observe_one_code_unit_spelling() {
    let source = include_str!("../../../../../tests/fixtures/compare-qjs/string-canonical-keys.js");
    assert_eq!(
        crate::eval(source),
        Ok(crate::Value::String(
            "canonical strings: 28 producers".into()
        ))
    );
}
