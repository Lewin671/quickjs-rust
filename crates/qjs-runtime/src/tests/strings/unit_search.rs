use crate::{Value, eval};

#[test]
fn search_uses_unit_positions_and_matches_half_pairs() {
    assert_eq!(
        eval(include_str!(
            "../../../../../tests/fixtures/compare-qjs/string-search-code-units.js"
        )),
        Ok(Value::String("ok".into()))
    );
}

#[test]
fn literal_split_preserves_units_limits_and_coercion_order() {
    assert_eq!(
        eval(include_str!(
            "../../../../../tests/fixtures/compare-qjs/string-split-code-units.js"
        )),
        Ok(Value::String("ok".into()))
    );
}

#[test]
fn literal_replacement_uses_unit_ranges_and_callback_positions() {
    assert_eq!(
        eval(include_str!(
            "../../../../../tests/fixtures/compare-qjs/string-replace-code-units.js"
        )),
        Ok(Value::String("ok".into()))
    );
}

#[test]
fn literal_search_handles_repeated_prefixes_and_canonical_boundaries() {
    let source = include_str!("../../../../../tests/fixtures/compare-qjs/string-search-linear.js");
    assert_eq!(eval(source).unwrap(), Value::String("ok".into()));
}
