//! Linear unit search for patterns that can match inside a stored scalar.
//! Prefix links are built once per operation and reused across disjoint matches.

pub(super) struct UnitPattern {
    pub(super) units: Vec<u16>,
    prefix: Vec<usize>,
    #[cfg(test)]
    comparisons: std::cell::Cell<usize>,
}

impl UnitPattern {
    pub(super) fn new(search: &str) -> Self {
        let units = crate::string::string_code_units(search);
        let mut prefix = vec![0; units.len()];
        let mut matched = 0;
        for index in 1..units.len() {
            while matched > 0 && units[index] != units[matched] {
                matched = prefix[matched - 1];
            }
            if units[index] == units[matched] {
                matched += 1;
            }
            prefix[index] = matched;
        }
        Self {
            units,
            prefix,
            #[cfg(test)]
            comparisons: std::cell::Cell::new(0),
        }
    }

    pub(super) fn find_in(&self, input: &[u16], start: usize, last: bool) -> Option<usize> {
        if self.units.is_empty() {
            return (start <= input.len()).then_some(if last { input.len() } else { start });
        }
        let mut matched = 0;
        let mut found = None;
        for (offset, &unit) in input.get(start..)?.iter().enumerate() {
            loop {
                #[cfg(test)]
                self.comparisons.set(self.comparisons.get() + 1);
                if unit == self.units[matched] {
                    matched += 1;
                    break;
                }
                if matched == 0 {
                    break;
                }
                matched = self.prefix[matched - 1];
            }
            if matched == self.units.len() {
                found = Some(start + offset + 1 - matched);
                if !last {
                    return found;
                }
                matched = self.prefix[matched - 1];
            }
        }
        found
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeated_prefix_search_work_is_linear() {
        for size in [100_000, 200_000] {
            let input = vec![0xD800; size];
            for length in [10, 1000] {
                let mut needle = vec![0xD800; length];
                needle.push(0xDC01);
                let needle = crate::string::string_from_code_units(&needle);
                let pattern = UnitPattern::new(&needle);
                for last in [false, true] {
                    pattern.comparisons.set(0);
                    assert_eq!(pattern.find_in(&input, 0, last), None);
                    assert!(pattern.comparisons.get() <= 2 * size);
                }
            }
        }
    }

    #[test]
    fn reverse_search_keeps_overlapping_matches() {
        let pattern = UnitPattern::new("aba");
        assert_eq!(pattern.find_in(&[97, 98, 97, 98, 97], 0, true), Some(2));
    }
}
