use super::regexp_match_range;
use crate::string::string_from_code_unit;

#[test]
fn captures_greedy_quantified_group_range() {
    let matched = regexp_match_range(r"([0-9]+)", "31", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 2));
    assert_eq!(matched.captures, vec![Some((0, 2))]);
}

#[test]
fn captures_nested_group_ranges() {
    let matched = regexp_match_range(r"((x))", "foo-x-bar", 0, false, false, false).unwrap();
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

    let matched = regexp_match_range(&pattern, &input, 0, false, true, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 2));
    assert!(regexp_match_range(&pattern, &high, 0, false, true, false).is_none());
    assert!(regexp_match_range(&pattern, &low, 0, false, true, false).is_none());
}

#[test]
fn legacy_decimal_escapes_define_character_class_ranges() {
    let matched = regexp_match_range(r"[\12-\14]+", "\n\n", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 2));
    assert!(regexp_match_range(r"[\12-\14]+", "\t", 0, false, false, false).is_none());
}

#[test]
fn legacy_class_control_escapes_match_digits_and_underscore() {
    let matched = regexp_match_range(r"[\c0]", "\u{0010}", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));
    let matched = regexp_match_range(r"[\c_]", "\u{001f}", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));
    let matched = regexp_match_range(r"[\c00]+", "0\u{0010}", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 2));
    assert!(regexp_match_range(r"\c0", "\u{0010}", 0, false, false, false).is_none());
}

#[test]
fn top_level_alternatives_match_leftmost_first() {
    let matched = regexp_match_range("1|12", "123", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));

    let matched = regexp_match_range("2|12", "1.012", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (3, 5));
}

#[test]
fn top_level_alternatives_ignore_character_class_pipes() {
    let matched = regexp_match_range("[a|b]c|de", "bc", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 2));

    let matched = regexp_match_range("[a|b]c|de", "de", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 2));
}

#[test]
fn lazy_quantifiers_try_shorter_matches_first() {
    let matched = regexp_match_range("a+?", "aaa", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));

    let matched = regexp_match_range("a??a", "a", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));

    let matched = regexp_match_range("a[a-z]{2,4}?", "abcdefghi", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 3));
}

#[test]
fn quantified_groups_preserve_atom_order_and_clear_skipped_captures() {
    let matched =
        regexp_match_range("(aa|aabaac|ba|b|c)*", "aabaac", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 4));
    assert_eq!(matched.captures, vec![Some((2, 4))]);

    let matched =
        regexp_match_range("(z)((a+)?(b+)?(c))*", "zaacbbbcac", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 10));
    assert_eq!(
        matched.captures,
        vec![
            Some((0, 1)),
            Some((8, 10)),
            Some((8, 9)),
            None,
            Some((9, 10)),
        ]
    );
}

#[test]
fn dot_excludes_line_terminators_unless_dot_all() {
    for line_terminator in ['\n', '\r', '\u{2028}', '\u{2029}'] {
        assert!(
            regexp_match_range(".", &line_terminator.to_string(), 0, false, false, false).is_none()
        );
        assert!(
            regexp_match_range(".", &line_terminator.to_string(), 0, false, false, true).is_some()
        );
    }

    assert!(regexp_match_range(".", "\u{0085}", 0, false, false, false).is_some());
    assert!(regexp_match_range(".", "\u{000b}", 0, false, false, false).is_some());
    assert!(regexp_match_range(".", "\u{000c}", 0, false, false, false).is_some());
    assert!(regexp_match_range("^.$", "\u{10300}", 0, false, false, false).is_none());
    assert!(regexp_match_range("^.$", "\u{10300}", 0, false, true, false).is_some());
}

#[test]
fn whitespace_escapes_use_ecmascript_character_set() {
    for whitespace in [
        '\t', '\n', '\u{000b}', '\u{000c}', '\r', ' ', '\u{00a0}', '\u{1680}', '\u{2000}',
        '\u{200a}', '\u{2028}', '\u{2029}', '\u{202f}', '\u{205f}', '\u{3000}', '\u{feff}',
    ] {
        let input = whitespace.to_string();
        assert!(regexp_match_range(r"^\s$", &input, 0, false, false, false).is_some());
        assert!(regexp_match_range(r"^[\s]$", &input, 0, false, false, false).is_some());
        assert!(regexp_match_range(r"^\S$", &input, 0, false, false, false).is_none());
        assert!(regexp_match_range(r"^[\S]$", &input, 0, false, false, false).is_none());
    }

    for non_whitespace in ['\u{0085}', '\u{180e}', 'A', '_', '0'] {
        let input = non_whitespace.to_string();
        assert!(regexp_match_range(r"^\s$", &input, 0, false, false, false).is_none());
        assert!(regexp_match_range(r"^[\s]$", &input, 0, false, false, false).is_none());
        assert!(regexp_match_range(r"^\S$", &input, 0, false, false, false).is_some());
        assert!(regexp_match_range(r"^[\S]$", &input, 0, false, false, false).is_some());
    }
}
