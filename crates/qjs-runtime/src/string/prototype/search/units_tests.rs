//! Differential unit oracles for canonical byte search and half-pair fallback.

use super::*;

#[test]
fn search_bounds_include_interior_pair_positions() {
    let low = string_from_code_unit(0xDE00);
    let high = string_from_code_unit(0xD83D);
    assert_eq!(find("😀x😀", &low, 0), Some(1));
    assert_eq!(find("😀x😀", &low, 2), Some(4));
    assert_eq!(rfind("😀x😀", &high, 2), Some(0));
    assert_eq!(rfind("😀x😀", &low, 4), Some(4));
    assert!(starts_with("😀x", &low, 1));
    assert!(ends_with("😀x", &high, 1));
    assert_eq!(find("😀", "", 2), Some(2));
    assert_eq!(find("😀", "", 3), None);
    assert_eq!(find("😀x😀", "😀", 1), Some(3));
    assert_eq!(find("😀x😀", "x", 1), Some(2));
    assert_eq!(rfind("😀x😀", "😀", 1), Some(0));
    assert_eq!(rfind("😀x😀", "x", 1), None);
    assert_eq!(rfind("😀x😀", "x😀", 2), Some(2));
}

#[test]
fn canonical_search_agrees_with_unit_oracle() {
    let fragments = ["a", "é", "😀", "\u{F0000}", "\u{F07FF}"];
    let mut needles: Vec<String> = fragments.iter().map(|s| (*s).to_owned()).collect();
    needles.extend([
        String::new(),
        string_from_code_unit(0xD83D),
        string_from_code_unit(0xDE00),
    ]);
    for left in &needles {
        for right in &needles {
            let mut input = left.clone();
            crate::string::push_js_str(&mut input, right);
            let units = string_code_units(&input);
            for needle in &needles {
                let search = string_code_units(needle);
                for bound in 0..=units.len() + 1 {
                    let same_at = |p: usize| units.get(p..p + search.len()) == Some(&search);
                    let forward = (bound..=units.len()).find(|&p| same_at(p));
                    let reverse = (0..=bound.min(units.len())).rev().find(|&p| same_at(p));
                    assert_eq!(
                        find(&input, needle, bound),
                        forward,
                        "{input:?} {needle:?} {bound}"
                    );
                    assert_eq!(
                        rfind(&input, needle, bound),
                        reverse,
                        "{input:?} {needle:?} {bound}"
                    );
                }
            }
        }
    }
}

#[test]
fn real_sentinel_range_scalars_and_lone_halves_stay_distinct() {
    for scalar in 0xF0000..=0xF07FF {
        let scalar = char::from_u32(scalar).unwrap();
        let mut buffer = [0; 2];
        let units = scalar.encode_utf16(&mut buffer);
        let pattern = string_from_code_units(units);
        let input = format!("😀{pattern}é{pattern}😀");
        assert!(!byte_searchable(&pattern));
        assert_eq!(find(&input, &pattern, 0), Some(2));
        assert_eq!(rfind(&input, &pattern, usize::MAX), Some(5));
        for (offset, unit) in units.iter().enumerate() {
            assert_eq!(
                find(&input, &string_from_code_unit(*unit), 2),
                Some(2 + offset)
            );
        }
        assert_eq!(find(&pattern, "😀", 0), None);
    }
}

#[test]
fn repeated_matches_advance_unit_offsets_and_split_boundaries() {
    for needle in [
        "é".to_owned(),
        string_from_code_unit(0xDE00),
        string_from_code_units(&[0xDB80, 0xDC00]),
    ] {
        let mut input = "😀".to_owned();
        crate::string::push_js_str(&mut input, &needle);
        crate::string::push_js_str(&mut input, "😀");
        crate::string::push_js_str(&mut input, &needle);
        let units = string_code_units(&input);
        let pattern = string_code_units(&needle);
        let mut expected = Vec::new();
        let mut start = 0;
        while start + pattern.len() <= units.len() {
            if units[start..start + pattern.len()] == pattern {
                expected.push(start);
                start += pattern.len();
            } else {
                start += 1;
            }
        }
        let matches = match_positions(&input, &needle, false);
        assert_eq!(
            matches.iter().map(|m| m.start).collect::<Vec<_>>(),
            expected
        );
        assert_eq!(match_positions(&input, &needle, true).len(), 1);
        let mut parts = Vec::new();
        let mut previous = 0;
        for position in expected {
            parts.push(string_from_code_units(&units[previous..position]));
            previous = position + pattern.len();
        }
        parts.push(string_from_code_units(&units[previous..]));
        for limit in 0..=parts.len() + 1 {
            let split = split_code_units(&input, &needle, limit);
            let actual: Vec<_> = split
                .iter()
                .map(|v| match v {
                    Value::String(s) => s.as_str(),
                    _ => unreachable!(),
                })
                .collect();
            assert_eq!(
                actual,
                parts
                    .iter()
                    .take(limit)
                    .map(String::as_str)
                    .collect::<Vec<_>>()
            );
        }
    }
}
