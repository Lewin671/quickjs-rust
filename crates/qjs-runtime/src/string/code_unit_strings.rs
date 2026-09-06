//! Bounded, realm-owned immutable strings for boxed String index values.

use std::cell::OnceCell;

use crate::JsString;

const LATIN1_CODE_UNITS: usize = 256;

/// String boxing still installs ordinary indexed descriptors. Only their
/// immutable primitive values are shared, so no ordinary-object lookup needs
/// an exotic-property check and no object or property layout changes.
#[derive(Default)]
pub(crate) struct CodeUnitStrings {
    latin1: OnceCell<Box<[OnceCell<JsString>; LATIN1_CODE_UNITS]>>,
}

impl CodeUnitStrings {
    #[inline]
    pub(crate) fn get(&self, code_unit: u16) -> JsString {
        let index = usize::from(code_unit);
        if index >= LATIN1_CODE_UNITS {
            return super::string_from_code_unit(code_unit).into();
        }
        let entries = self
            .latin1
            .get_or_init(|| Box::new(std::array::from_fn(|_| OnceCell::new())));
        entries[index]
            .get_or_init(|| super::string_from_code_unit(code_unit).into())
            .clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caches_only_requested_latin1_values_and_bounds_retention() {
        let cache = CodeUnitStrings::default();
        assert!(cache.latin1.get().is_none());
        for unit in [0x100, 0xd800, 0xdc00, 0xffff] {
            let value = cache.get(unit);
            assert_eq!(crate::string::string_code_units(&value), [unit]);
            assert!(!JsString::ptr_eq(&value, &cache.get(unit)));
        }
        assert!(cache.latin1.get().is_none());
        for unit in 0..LATIN1_CODE_UNITS as u16 {
            let value = cache.get(unit);
            assert_eq!(crate::string::string_code_units(&value), [unit]);
            assert!(JsString::ptr_eq(&value, &cache.get(unit)));
        }
        assert_eq!(
            cache
                .latin1
                .get()
                .unwrap()
                .iter()
                .filter(|entry| entry.get().is_some())
                .count(),
            LATIN1_CODE_UNITS
        );
    }

    #[test]
    fn caches_are_independent_and_shared_characters_remain_copy_on_write() {
        let first = CodeUnitStrings::default();
        let second = CodeUnitStrings::default();
        let mut value = first.get(u16::from(b'A'));
        assert!(!JsString::ptr_eq(&value, &second.get(u16::from(b'A'))));
        assert!(value.is_ascii());
        value.make_mut().push('\u{100}');
        assert_eq!(value.as_str(), "A\u{100}");
        assert!(!value.is_ascii());
        assert_eq!(first.get(u16::from(b'A')).as_str(), "A");
        assert_eq!(second.get(u16::from(b'A')).as_str(), "A");
    }
}
