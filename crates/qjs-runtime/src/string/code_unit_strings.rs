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

/// Indices whose property keys a String object shares instead of allocating.
const SHARED_INDEX_KEYS: usize = 64;

/// Property keys every boxed String installs, allocated once per realm: the
/// internal data key, `length`, and the first indices. Installing them by
/// shared handle skips the two allocations each owned key cost (the
/// `String` and its `Rc<str>` copy). Keys are immutable and hold nothing.
#[derive(Default)]
pub(crate) struct StringObjectKeys {
    data: OnceCell<std::rc::Rc<str>>,
    length: OnceCell<std::rc::Rc<str>>,
    indices: OnceCell<Box<[OnceCell<std::rc::Rc<str>>; SHARED_INDEX_KEYS]>>,
}

impl StringObjectKeys {
    pub(crate) fn data(&self) -> std::rc::Rc<str> {
        std::rc::Rc::clone(self.data.get_or_init(|| super::STRING_DATA_PROPERTY.into()))
    }

    pub(crate) fn length(&self) -> std::rc::Rc<str> {
        std::rc::Rc::clone(self.length.get_or_init(|| "length".into()))
    }

    pub(crate) fn index(&self, index: usize) -> std::rc::Rc<str> {
        if index >= SHARED_INDEX_KEYS {
            return index.to_string().into();
        }
        let entries = self
            .indices
            .get_or_init(|| Box::new(std::array::from_fn(|_| OnceCell::new())));
        std::rc::Rc::clone(entries[index].get_or_init(|| index.to_string().into()))
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

#[cfg(test)]
mod key_tests {
    use super::*;

    #[test]
    fn shares_the_first_index_keys_and_builds_the_rest() {
        let keys = StringObjectKeys::default();
        assert!(std::rc::Rc::ptr_eq(&keys.index(3), &keys.index(3)));
        assert_eq!(&*keys.index(3), "3");
        assert_eq!(
            &*keys.index(SHARED_INDEX_KEYS),
            SHARED_INDEX_KEYS.to_string()
        );
        assert!(!std::rc::Rc::ptr_eq(&keys.index(100), &keys.index(100)));
        assert_eq!(&*keys.length(), "length");
        assert_eq!(&*keys.data(), crate::string::STRING_DATA_PROPERTY);
    }
}
