//! Realm-owned property keys for boxed String objects.

use std::cell::OnceCell;

/// The boxed String length key is shared per realm. StringData is an internal
/// slot; virtual indices never need keys installed in named storage.
#[derive(Default)]
pub(crate) struct StringObjectKeys {
    length: OnceCell<std::rc::Rc<str>>,
}

impl StringObjectKeys {
    pub(crate) fn length(&self) -> std::rc::Rc<str> {
        std::rc::Rc::clone(self.length.get_or_init(|| "length".into()))
    }
}

#[cfg(test)]
mod key_tests {
    use super::*;

    #[test]
    fn shares_the_length_key() {
        let keys = StringObjectKeys::default();
        assert!(std::rc::Rc::ptr_eq(&keys.length(), &keys.length()));
        assert_eq!(&*keys.length(), "length");
    }
}
