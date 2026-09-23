//! Realm-owned property keys for boxed String objects.

use std::cell::OnceCell;

/// Property keys every boxed String installs, allocated once per realm: the
/// internal data key and `length`. Its index properties are defined only when
/// the wrapper's property table is read (`ObjectRef::properties`). Installing them by
/// shared handle skips the two allocations each owned key cost (the
/// `String` and its `Rc<str>` copy). Keys are immutable and hold nothing.
#[derive(Default)]
pub(crate) struct StringObjectKeys {
    data: OnceCell<std::rc::Rc<str>>,
    length: OnceCell<std::rc::Rc<str>>,
}

impl StringObjectKeys {
    pub(crate) fn data(&self) -> std::rc::Rc<str> {
        std::rc::Rc::clone(self.data.get_or_init(|| super::STRING_DATA_PROPERTY.into()))
    }

    pub(crate) fn length(&self) -> std::rc::Rc<str> {
        std::rc::Rc::clone(self.length.get_or_init(|| "length".into()))
    }
}

#[cfg(test)]
mod key_tests {
    use super::*;

    #[test]
    fn shares_the_data_and_length_keys() {
        let keys = StringObjectKeys::default();
        assert!(std::rc::Rc::ptr_eq(&keys.length(), &keys.length()));
        assert_eq!(&*keys.length(), "length");
        assert_eq!(&*keys.data(), crate::string::STRING_DATA_PROPERTY);
    }
}
