//! Shared JavaScript string storage with cached code-unit metadata.
//!
//! JavaScript strings are indexed by UTF-16 code unit while the runtime stores
//! them as UTF-8 text. Answering `charCodeAt`, `length`, or an indexed read
//! therefore needs to know whether the buffer is ASCII (where a byte index is a
//! code-unit index) and, when it is not, how long the UTF-16 view is. Computing
//! either answer is linear in the buffer, so a loop that walks a string one code
//! unit at a time used to be quadratic.
//!
//! [`JsString`] keeps the buffer behind a shared allocation and memoizes both
//! answers next to it. The cache is filled on first use and invalidated whenever
//! the uniquely held buffer is mutated in place.

use std::cell::{Cell, RefCell};
use std::fmt;
use std::hash::{Hash, Hasher};
use std::ops::Deref;
use std::rc::Rc;

const ASCII_UNKNOWN: u8 = 0;
const ASCII_YES: u8 = 1;
const ASCII_NO: u8 = 2;

const UTF16_LEN_UNKNOWN: usize = usize::MAX;

#[derive(Debug)]
struct StringData {
    text: String,
    ascii: Cell<u8>,
    utf16_len: Cell<usize>,
    /// The matcher-indexed character view and the Unicode mode it was built
    /// for. A RegExp scan re-executes against the same subject once per match,
    /// and rebuilding this view every time made scanning quadratic.
    matcher_view: RefCell<Option<(bool, Rc<[char]>)>>,
}

impl StringData {
    fn new(text: String) -> Self {
        Self {
            text,
            ascii: Cell::new(ASCII_UNKNOWN),
            utf16_len: Cell::new(UTF16_LEN_UNKNOWN),
            matcher_view: RefCell::new(None),
        }
    }

    fn invalidate(&mut self) {
        self.ascii.set(ASCII_UNKNOWN);
        self.utf16_len.set(UTF16_LEN_UNKNOWN);
        *self.matcher_view.borrow_mut() = None;
    }
}

impl Clone for StringData {
    fn clone(&self) -> Self {
        Self {
            text: self.text.clone(),
            ascii: Cell::new(self.ascii.get()),
            utf16_len: Cell::new(self.utf16_len.get()),
            matcher_view: RefCell::new(self.matcher_view.borrow().clone()),
        }
    }
}

/// An immutable JavaScript string value.
///
/// Cloning shares the buffer, so passing a string through a call, a property
/// read, or an environment lookup is a refcount bump. The in-place append fast
/// path mutates through [`JsString::make_mut`], which only succeeds while the
/// buffer is uniquely held.
#[derive(Clone, Debug)]
pub struct JsString(Rc<StringData>);

impl JsString {
    /// Wraps an owned buffer that already uses the runtime's canonical WTF-16
    /// sentinel representation.
    pub fn new(text: String) -> Self {
        Self(Rc::new(StringData::new(text)))
    }

    /// Borrows the underlying UTF-8 buffer.
    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0.text
    }

    /// Reports whether the buffer is ASCII, answering from the memoized result
    /// after the first call.
    #[inline]
    pub fn is_ascii(&self) -> bool {
        match self.0.ascii.get() {
            ASCII_YES => true,
            ASCII_NO => false,
            _ => {
                let ascii = self.0.text.is_ascii();
                self.0.ascii.set(if ascii { ASCII_YES } else { ASCII_NO });
                if ascii {
                    self.0.utf16_len.set(self.0.text.len());
                }
                ascii
            }
        }
    }

    /// Returns the memoized UTF-16 code-unit length, computing it with
    /// `measure` on the first call.
    #[inline]
    pub(crate) fn utf16_len_with(&self, measure: impl FnOnce(&str) -> usize) -> usize {
        let cached = self.0.utf16_len.get();
        if cached != UTF16_LEN_UNKNOWN {
            return cached;
        }
        if self.is_ascii() {
            return self.0.text.len();
        }
        let length = measure(&self.0.text);
        self.0.utf16_len.set(length);
        length
    }

    /// Returns the memoized character view a RegExp matcher indexes, building
    /// it with `build` on the first request for this Unicode mode.
    pub(crate) fn matcher_view_with(
        &self,
        unicode: bool,
        build: impl FnOnce(&str) -> Vec<char>,
    ) -> Rc<[char]> {
        // In ASCII text a code unit, a scalar value, and a byte all coincide,
        // so one view serves both modes.
        let ascii = self.is_ascii();
        if let Some((cached_unicode, view)) = self.0.matcher_view.borrow().as_ref()
            && (ascii || *cached_unicode == unicode)
        {
            return Rc::clone(view);
        }
        let view: Rc<[char]> = build(&self.0.text).into();
        *self.0.matcher_view.borrow_mut() = Some((unicode, Rc::clone(&view)));
        view
    }

    /// Borrows the buffer mutably, cloning it when it is shared. Any memoized
    /// metadata is dropped because the buffer is about to change.
    pub fn make_mut(&mut self) -> &mut String {
        let data = Rc::make_mut(&mut self.0);
        data.invalidate();
        &mut data.text
    }

    /// Reports whether two values share one buffer.
    pub fn ptr_eq(left: &Self, right: &Self) -> bool {
        Rc::ptr_eq(&left.0, &right.0)
    }

    /// Reports whether this value is the only owner of its buffer.
    pub fn is_unique(&self) -> bool {
        Rc::strong_count(&self.0) == 1 && Rc::weak_count(&self.0) == 0
    }

    /// Consumes the value, reusing the buffer when it is uniquely held.
    pub fn into_string(self) -> String {
        match Rc::try_unwrap(self.0) {
            Ok(data) => data.text,
            Err(shared) => shared.text.clone(),
        }
    }
}

impl Default for JsString {
    fn default() -> Self {
        Self::new(String::new())
    }
}

impl Deref for JsString {
    type Target = str;

    #[inline]
    fn deref(&self) -> &str {
        &self.0.text
    }
}

impl AsRef<str> for JsString {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0.text
    }
}

impl From<String> for JsString {
    #[inline]
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

impl From<&str> for JsString {
    #[inline]
    fn from(text: &str) -> Self {
        Self::new(text.to_owned())
    }
}

impl From<&String> for JsString {
    #[inline]
    fn from(text: &String) -> Self {
        Self::new(text.clone())
    }
}

impl From<std::borrow::Cow<'_, str>> for JsString {
    #[inline]
    fn from(text: std::borrow::Cow<'_, str>) -> Self {
        Self::new(text.into_owned())
    }
}

impl From<char> for JsString {
    #[inline]
    fn from(value: char) -> Self {
        Self::new(value.to_string())
    }
}

impl From<JsString> for String {
    #[inline]
    fn from(value: JsString) -> Self {
        value.into_string()
    }
}

impl PartialEq for JsString {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0) || self.0.text == other.0.text
    }
}

impl Eq for JsString {}

impl PartialEq<str> for JsString {
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.0.text == other
    }
}

impl PartialEq<&str> for JsString {
    #[inline]
    fn eq(&self, other: &&str) -> bool {
        self.0.text == *other
    }
}

impl PartialEq<String> for JsString {
    #[inline]
    fn eq(&self, other: &String) -> bool {
        &self.0.text == other
    }
}

impl PartialOrd for JsString {
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for JsString {
    #[inline]
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.text.cmp(&other.0.text)
    }
}

impl Hash for JsString {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.text.hash(state);
    }
}

impl fmt::Display for JsString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0.text)
    }
}

impl std::borrow::Borrow<str> for JsString {
    #[inline]
    fn borrow(&self) -> &str {
        &self.0.text
    }
}

#[cfg(test)]
mod tests {
    use super::JsString;

    #[test]
    fn ascii_state_is_memoized_and_matches_the_buffer() {
        let ascii = JsString::from("abc");
        assert!(ascii.is_ascii());
        assert!(ascii.is_ascii());
        let wide = JsString::from("aé");
        assert!(!wide.is_ascii());
        assert!(!wide.is_ascii());
    }

    #[test]
    fn mutation_invalidates_memoized_metadata() {
        let mut value = JsString::from("abc");
        assert!(value.is_ascii());
        assert_eq!(value.utf16_len_with(|text| text.chars().count()), 3);
        value.make_mut().push('é');
        assert!(!value.is_ascii());
        assert_eq!(value.utf16_len_with(|text| text.chars().count()), 4);
    }

    #[test]
    fn cloning_shares_the_buffer_until_it_is_mutated() {
        let original = JsString::from("abc");
        let mut copy = original.clone();
        assert!(JsString::ptr_eq(&original, &copy));
        copy.make_mut().push('d');
        assert!(!JsString::ptr_eq(&original, &copy));
        assert_eq!(original.as_str(), "abc");
        assert_eq!(copy.as_str(), "abcd");
    }

    #[test]
    fn utf16_length_is_memoized_for_wide_text() {
        let value = JsString::from("aé");
        let mut calls = 0;
        let mut measure = |_text: &str| {
            calls += 1;
            2
        };
        assert_eq!(value.utf16_len_with(&mut measure), 2);
        assert_eq!(value.utf16_len_with(&mut measure), 2);
        assert_eq!(calls, 1);
    }
}
