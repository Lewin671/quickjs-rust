use super::regexp_match_range;
use crate::string::string_from_code_unit;

#[test]
fn captures_greedy_quantified_group_range() {
    let matched = regexp_match_range(r"([0-9]+)", "31", 0, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 2));
    assert_eq!(matched.captures, vec![Some((0, 2))]);
}

#[test]
fn captures_nested_group_ranges() {
    let matched = regexp_match_range(r"((x))", "foo-x-bar", 0, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (4, 5));
    assert_eq!(matched.captures, vec![Some((4, 5)), Some((4, 5))]);
}

#[test]
fn matches_numbered_backreferences() {
    let matched = regexp_match_range(
        r"^(a+)\1*,\1+$",
        "aaaaaaaaaa,aaaaaaaaaaaaaaa",
        0,
        false,
        false,
    )
    .unwrap();
    assert_eq!((matched.start, matched.end), (0, 26));
    assert_eq!(matched.captures, vec![Some((0, 5))]);
}

#[test]
fn unicode_character_classes_match_surrogate_pairs_as_code_points() {
    let high = string_from_code_unit(0xD834);
    let low = string_from_code_unit(0xDF06);
    let pattern = format!("[{high}{low}]");
    let input = format!("{high}{low}");

    let matched = regexp_match_range(&pattern, &input, 0, false, true).unwrap();
    assert_eq!((matched.start, matched.end), (0, 2));
    assert!(regexp_match_range(&pattern, &high, 0, false, true).is_none());
    assert!(regexp_match_range(&pattern, &low, 0, false, true).is_none());
}

#[test]
fn legacy_decimal_escapes_define_character_class_ranges() {
    let matched = regexp_match_range(r"[\12-\14]+", "\n\n", 0, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 2));
    assert!(regexp_match_range(r"[\12-\14]+", "\t", 0, false, false).is_none());
}
