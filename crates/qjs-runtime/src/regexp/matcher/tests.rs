use super::regexp_match_range as regexp_match_range_inner;
use super::{RegexpMatch, regexp_match_at};
use crate::string::string_from_code_unit;

/// Test wrapper keeping the historical six-argument signature (multiline off).
fn regexp_match_range(
    source: &str,
    input: &str,
    start_index: usize,
    ignore_case: bool,
    unicode: bool,
    dot_all: bool,
) -> Option<RegexpMatch> {
    regexp_match_range_inner(
        source,
        input,
        start_index,
        ignore_case,
        unicode,
        dot_all,
        false,
    )
}

#[test]
fn captures_greedy_quantified_group_range() {
    let matched = regexp_match_range(r"([0-9]+)", "31", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 2));
    assert_eq!(matched.captures, vec![Some((0, 2))]);
}

#[test]
fn greedy_simple_atom_backtracks_against_trailing_atom() {
    // The linear-scan fast path for `\d+` must still backtrack so the trailing
    // `5` can match: the greedy run gives back its last digit.
    let matched = regexp_match_range(r"\d+5", "12345", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 5));

    // Anchored class repetition over the whole input, then `$`.
    let matched = regexp_match_range(r"^[a-z]+$", "abcdef", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 6));
    assert!(regexp_match_range(r"^[a-z]+$", "abc1", 0, false, false, false).is_none());
}

#[test]
fn lazy_simple_atom_takes_shortest_run() {
    let matched = regexp_match_range(r"a.*?c", "axxcxxc", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 4));
}

#[test]
fn counted_simple_atom_respects_bounds() {
    let matched = regexp_match_range(r"a{2,3}", "aaaa", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 3));
    assert!(regexp_match_range(r"a{2,3}", "a", 0, false, false, false).is_none());
}

#[test]
fn property_escape_repetition_matches_full_run() {
    let text = "z".repeat(5000);
    let matched = regexp_match_range(r"^\p{L}+$", &text, 0, false, true, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 5000));
    let mut mixed = text.clone();
    mixed.push('1');
    assert!(regexp_match_range(r"^\p{L}+$", &mixed, 0, false, true, false).is_none());
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
fn legacy_octal_escapes_match_top_level_atoms() {
    let matched = regexp_match_range(r"\00", "\u{0000}", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));
    let matched = regexp_match_range(r"\07", "\u{0007}", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));
    let matched = regexp_match_range(r"\0111", "\u{0009}1", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 2));
    let matched = regexp_match_range(r"\0003", "\u{0000}3", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 2));
}

#[test]
fn hex_escapes_match_code_units() {
    // `\xHH` decodes two hex digits to a single code unit, as a top-level atom
    // and inside a character class (including as a range bound).
    let matched = regexp_match_range(r"\x41", "A", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));
    assert!(regexp_match_range(r"\x41", "x41", 0, false, false, false).is_none());
    let matched = regexp_match_range(r"[\x5D]", "]", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));
    assert!(regexp_match_range(r"[\x5D]", "x", 0, false, false, false).is_none());
    let matched = regexp_match_range(r"[\xC0-\xD6]", "\u{00C7}", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));
    assert!(regexp_match_range(r"[\xC0-\xD6]", "A", 0, false, false, false).is_none());
    // Unicode mode and case-insensitive matching.
    let matched = regexp_match_range(r"\x41", "A", 0, false, true, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));
    let matched = regexp_match_range(r"\x41", "a", 0, true, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));
    // Annex B: `\x` without two hex digits is a literal `x`.
    let matched = regexp_match_range(r"\x4", "x4", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 2));
}

#[test]
fn unicode_ignore_case_uses_simple_case_folding() {
    assert!(regexp_match_range(r"\u212a", "k", 0, true, false, false).is_none());
    assert!(regexp_match_range(r"\u212a", "K", 0, true, false, false).is_none());
    assert!(regexp_match_range(r"\u212a", "k", 0, false, true, false).is_none());
    assert!(regexp_match_range(r"\u212a", "K", 0, false, true, false).is_none());

    let matched = regexp_match_range(r"\u212a", "k", 0, true, true, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));
    let matched = regexp_match_range(r"\u212a", "K", 0, true, true, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));
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
fn annex_b_control_escape_fallbacks_match_literals() {
    let matched = regexp_match_range(r"\cA", "\u{0001}", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));
    assert!(regexp_match_range(r"\cЖ", "cЖ", 0, false, false, false).is_none());
    let matched = regexp_match_range(r"\cЖ", r"\cЖ", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 3));

    let matched = regexp_match_range(r"[\c!]+", r"\c!", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 3));
    assert!(regexp_match_range(r"[\c!]", "\u{0001}", 0, false, false, false).is_none());
    let matched =
        regexp_match_range(r"[0-9A-Za-z_\$(|)\[\]\/\\^]", "$", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));
}

#[test]
fn annex_b_malformed_named_backreferences_are_identity_escapes() {
    let matched = regexp_match_range(r"\k<a>", "k<a>", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 4));
    let matched = regexp_match_range(r"\k<a>\1", "k<a>\u{0001}", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 5));
    let matched = regexp_match_range(r"\1(b)\k<a>", "bk<a>", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 5));
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

#[test]
fn unicode_property_escapes_match_code_points() {
    // General_Category, scripts, and binary properties, positive and negated.
    assert!(regexp_match_range(r"^\p{Nd}$", "5", 0, false, true, false).is_some());
    assert!(regexp_match_range(r"^\p{Lu}$", "A", 0, false, true, false).is_some());
    assert!(regexp_match_range(r"^\p{Lu}$", "a", 0, false, true, false).is_none());
    assert!(regexp_match_range(r"^\P{Lu}$", "a", 0, false, true, false).is_some());
    assert!(regexp_match_range(r"^\p{L}$", "\u{00E9}", 0, false, true, false).is_some());
    assert!(regexp_match_range(r"^\p{Script=Greek}$", "\u{03B1}", 0, false, true, false).is_some());
    // Astral code point matched as a single code point.
    assert!(regexp_match_range(r"^\p{Any}$", "\u{1F600}", 0, false, true, false).is_some());
    let high = string_from_code_unit(0xD83D);
    let low = string_from_code_unit(0xDE00);
    let pair = format!("{high}{low}");
    assert!(regexp_match_range(r"^\p{Surrogate}$", &low, 0, false, true, false).is_some());
    assert!(regexp_match_range(r"^\p{Surrogate}$", &pair, 0, false, true, false).is_none());
    // Inside a character class, optionally combined with other atoms.
    assert!(regexp_match_range(r"^[\p{Nd}\p{Lu}]$", "5", 0, false, true, false).is_some());
    assert!(regexp_match_range(r"^[\p{Nd}A-F]$", "C", 0, false, true, false).is_some());
    assert!(regexp_match_range(r"^[^\p{Nd}]$", "x", 0, false, true, false).is_some());
    assert!(regexp_match_range(r"^[^\p{Nd}]$", "5", 0, false, true, false).is_none());
    // In non-unicode mode `\p` is an identity escape for the literal `p`.
    assert!(regexp_match_range(r"\p{Nd}", "p{Nd}", 0, false, false, false).is_some());
}

#[test]
fn property_escapes_resolve_once_and_match_repeatedly() {
    // Resolving the property table per character was prohibitively slow; the
    // matcher now caches each escape's resolution keyed by pattern position.
    // A long greedy repetition must still match every code point correctly.
    let letters: String = std::iter::repeat_n('a', 50_000).collect();
    let matched = regexp_match_range(r"^\p{L}+$", &letters, 0, false, true, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, letters.len()));

    // A single non-letter at the end must fail the anchored match.
    let mut mixed = letters.clone();
    mixed.push('5');
    assert!(regexp_match_range(r"^\p{L}+$", &mixed, 0, false, true, false).is_none());

    // A property escape inside a class followed by a literal range still uses
    // the correct class-relative bounds after the cached escape.
    let matched = regexp_match_range(r"^[\p{Lu}a-f]+$", "ABCabf", 0, false, true, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 6));
    assert!(regexp_match_range(r"^[\p{Lu}a-f]+$", "ABCg", 0, false, true, false).is_none());

    // A negated character class with a cached property escape.
    let matched = regexp_match_range(r"^[^\p{Nd}]+$", "abc", 0, false, true, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 3));
    assert!(regexp_match_range(r"^[^\p{Nd}]+$", "ab2", 0, false, true, false).is_none());
}

#[test]
fn long_greedy_repetition_does_not_overflow_the_stack() {
    // A greedy quantified atom over a long input must not recurse per
    // repetition; the matcher uses an explicit work stack instead.
    let input: String = std::iter::repeat_n('0', 200_000).collect();
    let matched = regexp_match_range(r"^\d+$", &input, 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, input.len()));

    // A trailing mismatch must still terminate without exploring exponential
    // backtracking states.
    let mut mismatch = input.clone();
    mismatch.push('x');
    assert!(regexp_match_range(r"^\d+$", &mismatch, 0, false, false, false).is_none());
}

#[test]
fn multiline_anchors_match_around_line_terminators() {
    // Without the multiline flag, `$` only matches at end of input.
    let input = "pairs\nmakes\tdouble";
    assert!(regexp_match_range_inner("s$", input, 0, false, false, false, false).is_none());
    // With multiline, `$` matches before a `\n` (the `s` in "pairs").
    let matched = regexp_match_range_inner("s$", input, 0, false, false, false, true).unwrap();
    assert_eq!((matched.start, matched.end), (4, 5));

    // `^` matches after a line terminator in multiline mode.
    let matched =
        regexp_match_range_inner("^makes", "pairs\nmakes", 0, false, false, false, true).unwrap();
    assert_eq!((matched.start, matched.end), (6, 11));
    assert!(
        regexp_match_range_inner("^makes", "pairs\nmakes", 0, false, false, false, false).is_none()
    );

    // All ECMAScript line terminators are recognized.
    for terminator in ['\n', '\r', '\u{2028}', '\u{2029}'] {
        let input = format!("a{terminator}b");
        assert!(
            regexp_match_range_inner("^b", &input, 0, false, false, false, true).is_some(),
            "`^b` should match after U+{:04X}",
            terminator as u32
        );
    }

    // Sticky multiline still honors line boundaries.
    assert!(regexp_match_at("^b", "a\nb", 2, false, false, false, true).is_some());
}

#[test]
fn named_groups_capture_and_backreference() {
    let matched = regexp_match_range(
        r"(?<year>\d{4})-(?<month>\d{2})",
        "2024-06",
        0,
        false,
        false,
        false,
    )
    .unwrap();
    assert_eq!((matched.start, matched.end), (0, 7));
    assert_eq!(matched.captures, vec![Some((0, 4)), Some((5, 7))]);

    // `\k<name>` matches the same text as the named group.
    let matched = regexp_match_range(r"(?<c>.)\k<c>", "abxx", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (2, 4));
    assert!(regexp_match_range(r"^(?<c>.)\k<c>$", "ab", 0, false, false, false).is_none());
}

#[test]
fn lookahead_assertions_are_zero_width() {
    let matched = regexp_match_range(r"a(?=b)", "ab", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));
    assert!(regexp_match_range(r"a(?=b)", "ac", 0, false, false, false).is_none());

    let matched = regexp_match_range(r"a(?!b)", "ac", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (0, 1));
    assert!(regexp_match_range(r"a(?!b)", "ab", 0, false, false, false).is_none());
}

#[test]
fn lookbehind_assertions_match_preceding_text() {
    let matched = regexp_match_range(r"(?<=\$)\d+", "$100", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (1, 4));
    assert!(regexp_match_range(r"(?<=\$)\d+", "100", 0, false, false, false).is_none());

    let matched = regexp_match_range(r"(?<!\$)\d+", "a100", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (1, 4));
    // The first digit after `$` is excluded, so matching starts one later.
    let matched = regexp_match_range(r"(?<!\$)\d+", "$100", 0, false, false, false).unwrap();
    assert_eq!((matched.start, matched.end), (2, 4));
}
