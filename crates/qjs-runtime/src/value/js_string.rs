//! Shared JavaScript string storage with cached code-unit metadata.
//!
//! JavaScript strings are indexed by UTF-16 code unit while the runtime stores
//! them as canonical UTF-8 sentinel text (see `qjs_unicode::wtf16`). A high
//! sentinel followed by a low sentinel is merged unless it encodes a real
//! U+F0000..F07FF scalar. Constructors debug-assert this invariant; mutable
//! builders must preserve it with the shared seam helper. Answering `charCodeAt`, `length`, or an indexed read
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

#[derive(Debug, Default)]
struct StringCache {
    matcher_view: Option<Rc<[char]>>,
    key: Option<crate::property::atom::KeyLease>,
}

#[derive(Debug)]
struct StringData {
    text: String,
    /// Set only on the one shared instance an atom table hands out for its
    /// text, so two atoms are equal exactly when they are the same instance.
    /// A copy, or a buffer mutated in place, is never an atom.
    atom: Cell<bool>,
    ascii: Cell<u8>,
    /// Mode for the cached matcher slice; fits existing scalar-header padding.
    matcher_unicode: Cell<bool>,
    utf16_len: Cell<usize>,
    /// The matcher-indexed character view and the Unicode mode it was built
    /// for. A RegExp scan re-executes against the same subject once per match,
    /// and rebuilding this view every time made scanning quadratic.
    cache: RefCell<StringCache>,
    /// Test-only work count: how many times this buffer was scanned to
    /// classify it as ASCII.
    #[cfg(test)]
    ascii_scans: Cell<usize>,
}

impl StringData {
    fn new(text: String) -> Self {
        Self {
            text,
            atom: Cell::new(false),
            ascii: Cell::new(ASCII_UNKNOWN),
            matcher_unicode: Cell::new(false),
            utf16_len: Cell::new(UTF16_LEN_UNKNOWN),
            cache: RefCell::default(),
            #[cfg(test)]
            ascii_scans: Cell::new(0),
        }
    }

    fn invalidate(&mut self) {
        self.atom.set(false);
        self.ascii.set(ASCII_UNKNOWN);
        self.matcher_unicode.set(false);
        self.utf16_len.set(UTF16_LEN_UNKNOWN);
        *self.cache.get_mut() = StringCache::default();
    }
}

impl Clone for StringData {
    fn clone(&self) -> Self {
        Self {
            text: self.text.clone(),
            atom: Cell::new(false),
            ascii: Cell::new(self.ascii.get()),
            matcher_unicode: Cell::new(self.matcher_unicode.get()),
            utf16_len: Cell::new(self.utf16_len.get()),
            cache: RefCell::new(StringCache {
                matcher_view: self.cache.borrow().matcher_view.clone(),
                key: None,
            }),
            #[cfg(test)]
            ascii_scans: Cell::new(0),
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
        debug_assert!(
            crate::string::is_canonical(&text),
            "non-canonical JsString: {text:?}"
        );
        Self(Rc::new(StringData::new(text)))
    }

    /// An atom: the one instance of `text` an atom table shares. The caller
    /// guarantees it builds at most one atom per text per table.
    pub(crate) fn atom(text: &str) -> Self {
        let string = Self::new(text.to_owned());
        string.0.atom.set(true);
        string
    }

    /// Whether this is an atom (see [`JsString::atom`]).
    #[inline]
    pub(crate) fn is_atom(&self) -> bool {
        self.0.atom.get()
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
                #[cfg(test)]
                self.0.ascii_scans.set(self.0.ascii_scans.get() + 1);
                let ascii = self.0.text.is_ascii();
                self.0.ascii.set(if ascii { ASCII_YES } else { ASCII_NO });
                if ascii {
                    self.0.utf16_len.set(self.0.text.len());
                }
                ascii
            }
        }
    }

    /// How many times this buffer was scanned for its ASCII classification.
    #[cfg(test)]
    pub(crate) fn ascii_scan_count(&self) -> usize {
        self.0.ascii_scans.get()
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
        if let Some(view) = self.0.cache.borrow().matcher_view.as_ref()
            && (ascii || self.0.matcher_unicode.get() == unicode)
        {
            return Rc::clone(view);
        }
        let view: Rc<[char]> = build(&self.0.text).into();
        let mut cache = self.0.cache.borrow_mut();
        self.0.matcher_unicode.set(unicode);
        cache.matcher_view = Some(Rc::clone(&view));
        view
    }

    /// Borrows the buffer mutably, cloning it when it is shared. Any memoized
    /// metadata is dropped because the buffer is about to change. The caller
    /// must preserve canonical WTF-16 form with `qjs_unicode::wtf16` builders.
    pub fn make_mut(&mut self) -> &mut String {
        let data = Rc::make_mut(&mut self.0);
        data.invalidate();
        &mut data.text
    }

    /// A memoized live lease for a computed string key. Reads never intern;
    /// COW/string mutation clears this metadata and foreign domains cannot hit.
    #[inline]
    pub(crate) fn property_key_lease(
        &self,
        domain: &crate::property::atom::AtomDomain,
    ) -> Option<crate::property::atom::KeyLease> {
        if let Some(key) = &self.0.cache.borrow().key
            && key.belongs_to(domain)
        {
            return Some(key.clone());
        }
        let key = domain.find(self.as_str())?;
        self.0.cache.borrow_mut().key = Some(key.clone());
        Some(key)
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

#[cfg(test)]
mod property_key_tests {
    use super::*;
    use crate::property::atom::AtomDomain;

    #[test]
    fn property_key_memo_is_find_only_domain_checked_and_cleared_on_mutation() {
        let a = AtomDomain::new();
        let b = AtomDomain::new();
        let mut text = JsString::from("key");
        assert!(text.property_key_lease(&a).is_none());
        assert!(a.find("key").is_none());
        let ak = a.intern("key");
        let bk = b.intern("key");
        assert_eq!(text.property_key_lease(&a), Some(ak.clone()));
        assert_eq!(text.property_key_lease(&b), Some(bk));
        let alias = text.clone();
        text.make_mut().push('2');
        assert!(text.property_key_lease(&a).is_none());
        assert_eq!(alias.property_key_lease(&a), Some(ak));
        drop(alias);
        text.make_mut().clear();
        assert!(text.0.cache.borrow().key.is_none());
    }
}

#[cfg(all(target_pointer_width = "64", not(test)))]
const _: () = {
    assert!(std::mem::size_of::<StringCache>() == 24);
    assert!(std::mem::size_of::<StringData>() == 72);
};

#[cfg(test)]
mod matcher_mode_tests {
    use super::*;
    #[test]
    fn matcher_modes_and_key_memo_coexist_across_cow_and_invalidation() {
        let domain = crate::property::atom::AtomDomain::new();
        let key = domain.intern("😀");
        let original = JsString::from("😀");
        assert_eq!(original.property_key_lease(&domain), Some(key.clone()));
        let unicode = original.matcher_view_with(true, |text| text.chars().collect());
        assert_eq!(&*unicode, &['😀']);
        assert!(Rc::ptr_eq(
            &unicode,
            &original.matcher_view_with(true, |_| panic!("cached"))
        ));
        let units = original.matcher_view_with(false, |text| {
            crate::string::string_code_units(text)
                .into_iter()
                .map(crate::string::char_from_code_unit)
                .collect()
        });
        assert_eq!(units.len(), 2);
        assert!(Rc::ptr_eq(
            &units,
            &original.matcher_view_with(false, |_| panic!("cached"))
        ));
        assert_eq!(original.property_key_lease(&domain), Some(key));
        let mut copy = original.clone();
        copy.make_mut().push('x');
        assert!(copy.property_key_lease(&domain).is_none());
        assert_eq!(
            copy.matcher_view_with(true, |text| text.chars().collect())
                .len(),
            2
        );
        assert!(Rc::ptr_eq(
            &units,
            &original.matcher_view_with(false, |_| panic!("COW alias"))
        ));
        assert_eq!(
            original
                .matcher_view_with(true, |text| text.chars().collect())
                .len(),
            1
        );
    }
}
