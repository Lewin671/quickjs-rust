use super::regexp_match_range;

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
