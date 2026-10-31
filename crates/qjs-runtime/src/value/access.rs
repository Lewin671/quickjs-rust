//! Typed access to a [`Value`] that does not depend on its representation.
//!
//! `Value` is still the 16-byte enum, but code outside `value/` should not
//! need to know that: the interpreter-core migration (T056 M3) replaces the
//! enum with an opaque 8-byte word behind these same methods. Callers that
//! classify a value or read a primitive use this facade instead of matching
//! variants, so the backend can change without touching them.
//!
//! The invariant: every accessor here is a view. It borrows the value or
//! copies an immediate (`f64`, `bool`, [`ValueKind`]); none clones a handle,
//! updates a reference count, allocates or builds a temporary owner. The
//! constructors move their payload into the value without copying it.

use super::{JsString, ObjectRef, Value};

/// The ECMAScript language type of a value (ECMA-262 §6.1).
///
/// Every object kind (ordinary, array, function, Proxy, Map, Set) is
/// `Object`. Symbols are their own type even though the runtime stores them
/// as branded objects.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ValueKind {
    /// `undefined`.
    Undefined,
    /// `null`.
    Null,
    /// `true` or `false`.
    Boolean,
    /// A String.
    String,
    /// A Symbol.
    Symbol,
    /// A Number.
    Number,
    /// A BigInt.
    BigInt,
    /// Any object.
    Object,
}

impl Value {
    /// `undefined`.
    #[inline]
    #[must_use]
    pub const fn undefined() -> Self {
        Self::Undefined
    }

    /// `null`.
    #[inline]
    #[must_use]
    pub const fn null() -> Self {
        Self::Null
    }

    /// A Boolean.
    #[inline]
    #[must_use]
    pub const fn boolean(value: bool) -> Self {
        Self::Boolean(value)
    }

    /// A Number.
    #[inline]
    #[must_use]
    pub const fn number(value: f64) -> Self {
        Self::Number(value)
    }

    /// A String from an already canonical runtime string.
    #[inline]
    #[must_use]
    pub fn string(value: JsString) -> Self {
        Self::String(value)
    }

    /// The value's language type.
    #[inline]
    #[must_use]
    pub fn kind(&self) -> ValueKind {
        match self {
            Self::Undefined => ValueKind::Undefined,
            Self::Null => ValueKind::Null,
            Self::Boolean(_) => ValueKind::Boolean,
            Self::String(_) => ValueKind::String,
            Self::Number(_) => ValueKind::Number,
            Self::BigInt(_) => ValueKind::BigInt,
            Self::Object(object) if object.is_symbol_primitive() => ValueKind::Symbol,
            Self::Function(_)
            | Self::Array(_)
            | Self::Object(_)
            | Self::Proxy(_)
            | Self::Map(_)
            | Self::Set(_) => ValueKind::Object,
        }
    }

    /// Whether this is `undefined`.
    #[inline]
    #[must_use]
    pub fn is_undefined(&self) -> bool {
        matches!(self, Self::Undefined)
    }

    /// Whether this is `null`.
    #[inline]
    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Whether this is `undefined` or `null`.
    #[inline]
    #[must_use]
    pub fn is_nullish(&self) -> bool {
        matches!(self, Self::Undefined | Self::Null)
    }

    /// Whether this is an object of any kind (not a Symbol).
    #[inline]
    #[must_use]
    pub fn is_object(&self) -> bool {
        self.kind() == ValueKind::Object
    }

    /// The Number, if this is one.
    #[inline]
    #[must_use]
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }

    /// The Boolean, if this is one.
    #[inline]
    #[must_use]
    pub fn as_boolean(&self) -> Option<bool> {
        match self {
            Self::Boolean(value) => Some(*value),
            _ => None,
        }
    }

    /// A borrowed view of the String, if this is one.
    #[inline]
    #[must_use]
    pub fn as_js_string(&self) -> Option<&JsString> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    /// A borrowed view of the ordinary-object handle, if this is one.
    /// Symbols are excluded: they are not objects to the language.
    #[inline]
    pub(crate) fn as_ordinary_object(&self) -> Option<&ObjectRef> {
        match self {
            Self::Object(object) if !object.is_symbol_primitive() => Some(object),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{Value, ValueKind};
    use crate::{JsString, ObjectRef};

    #[test]
    fn views_add_no_size_or_owner() {
        let domain = crate::property::atom::AtomDomain::new();
        assert_eq!(std::mem::size_of::<Value>(), 16);
        assert_eq!(std::mem::size_of::<ValueKind>(), 1);
        assert_eq!(std::mem::size_of::<Option<&JsString>>(), 8);
        let string = Value::string(JsString::from("unique"));
        let view = string.as_js_string().expect("a string");
        // Borrowing the payload took no second owner.
        assert!(view.is_unique());
        assert_eq!(string.kind(), ValueKind::String);
        let object = ObjectRef::new(HashMap::new(), &domain);
        let value = Value::Object(object.clone());
        let view = value.as_ordinary_object().expect("an object");
        // The view is the value's own handle, not a clone of it.
        assert!(std::ptr::eq(
            view,
            match &value {
                Value::Object(handle) => handle,
                _ => unreachable!(),
            }
        ));
        assert_eq!(value.kind(), ValueKind::Object);
    }

    #[test]
    fn kinds_follow_language_types() {
        assert_eq!(Value::undefined().kind(), ValueKind::Undefined);
        assert_eq!(Value::null().kind(), ValueKind::Null);
        assert_eq!(Value::boolean(true).kind(), ValueKind::Boolean);
        assert_eq!(Value::number(-0.0).kind(), ValueKind::Number);
        assert_eq!(Value::bigint(7.into()).kind(), ValueKind::BigInt);
        assert!(Value::null().is_nullish() && Value::undefined().is_nullish());
        assert!(!Value::boolean(false).is_nullish());
        assert_eq!(
            Value::number(-0.0).as_number().map(f64::to_bits),
            Some((-0.0f64).to_bits())
        );
        assert!(Value::number(f64::NAN).as_number().is_some_and(f64::is_nan));
        assert_eq!(Value::boolean(false).as_boolean(), Some(false));
        assert_eq!(Value::number(1.0).as_boolean(), None);
        let array = crate::eval("[]").expect("evaluates");
        let function = crate::eval("(function () {})").expect("evaluates");
        let map = crate::eval("new Map()").expect("evaluates");
        let proxy = crate::eval("new Proxy({}, {})").expect("evaluates");
        for object in [array, function, map, proxy] {
            assert_eq!(object.kind(), ValueKind::Object);
            assert!(object.is_object());
        }
    }

    #[test]
    fn symbols_are_not_objects() {
        let symbol = crate::eval("Symbol('s')").expect("evaluates");
        assert_eq!(symbol.kind(), ValueKind::Symbol);
        assert!(!symbol.is_object());
        assert!(symbol.as_ordinary_object().is_none());
        let object = crate::eval("({})").expect("evaluates");
        assert!(object.as_ordinary_object().is_some());
    }
}
