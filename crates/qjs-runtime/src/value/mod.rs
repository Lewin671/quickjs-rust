use std::fmt;
use std::rc::Rc;

use num_bigint::BigInt;

mod array;
mod map;
mod object;
mod property;
mod set;

pub use array::ArrayRef;
pub use map::MapRef;
pub(crate) use object::ObjectLiteralShape;
pub use object::ObjectRef;
pub(crate) use object::Prototype;
pub(crate) use object::{
    ModuleNamespaceBindings, ObjectWeakRef, OwnDataPropertyRead, OwnDataPropertyWrite,
};
pub(crate) use property::Property;
pub use set::SetRef;

use crate::{Function, proxy::ProxyRef, string};

/// A JavaScript value supported by the current runtime subset.
#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum Value {
    /// Number value.
    Number(f64),
    /// BigInt value. JavaScript BigInts are immutable, so sharing keeps the
    /// common `Value` representation pointer-sized without observable aliasing.
    BigInt(Rc<BigInt>),
    /// String value. Held behind `Rc<String>` so cloning a string `Value`
    /// (which happens on every property read, parameter pass, and environment
    /// lookup) is a refcount bump rather than a full heap copy. JavaScript
    /// strings are immutable, so sharing is safe; the in-place `+=` append fast
    /// path uses `Rc::make_mut` to mutate the buffer when it is uniquely held.
    /// The buffer must already use the runtime's canonical WTF-16 sentinel
    /// representation; host UTF-8 text should enter through
    /// [`Value::string_from_utf8`].
    String(Rc<String>),
    /// Boolean value.
    Boolean(bool),
    /// Null value.
    Null,
    /// Undefined value.
    Undefined,
    /// User-defined function.
    Function(Function),
    /// Array object value.
    Array(ArrayRef),
    /// Object value.
    Object(ObjectRef),
    /// Proxy exotic object value.
    Proxy(ProxyRef),
    /// Map object value.
    Map(MapRef),
    /// Set object value.
    Set(SetRef),
}

impl fmt::Debug for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(value) => formatter.debug_tuple("Number").field(value).finish(),
            Self::BigInt(value) => formatter.debug_tuple("BigInt").field(value).finish(),
            Self::String(value) => formatter.debug_tuple("String").field(value).finish(),
            Self::Boolean(value) => formatter.debug_tuple("Boolean").field(value).finish(),
            Self::Null => formatter.write_str("Null"),
            Self::Undefined => formatter.write_str("Undefined"),
            Self::Function(function) => formatter.debug_tuple("Function").field(function).finish(),
            Self::Array(elements) => formatter.debug_tuple("Array").field(elements).finish(),
            Self::Object(object) => formatter.debug_tuple("Object").field(object).finish(),
            Self::Proxy(proxy) => formatter.debug_tuple("Proxy").field(proxy).finish(),
            Self::Map(map) => formatter.debug_tuple("Map").field(map).finish(),
            Self::Set(set) => formatter.debug_tuple("Set").field(set).finish(),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left == right,
            (Self::BigInt(left), Self::BigInt(right)) => left == right,
            (Self::String(left), Self::String(right)) => string::string_utf16_eq(left, right),
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            (Self::Null, Self::Null) | (Self::Undefined, Self::Undefined) => true,
            (Self::Function(left), Self::Function(right)) => left == right,
            (Self::Array(left), Self::Array(right)) => left.ptr_eq(right),
            (Self::Object(left), Self::Object(right)) => left.ptr_eq(right),
            (Self::Proxy(left), Self::Proxy(right)) => left.ptr_eq(right),
            (Self::Map(left), Self::Map(right)) => left.ptr_eq(right),
            (Self::Set(left), Self::Set(right)) => left.ptr_eq(right),
            _ => false,
        }
    }
}

impl Value {
    /// Constructs a JavaScript String from host UTF-8 scalar text while
    /// preserving real scalars that overlap the runtime's surrogate sentinels.
    pub fn string_from_utf8(value: &str) -> Self {
        Self::String(Rc::new(string::string_from_utf8_scalars(value)))
    }

    pub(crate) fn bigint(value: BigInt) -> Self {
        Self::BigInt(Rc::new(value))
    }

    pub(crate) fn is_uninitialized_lexical_marker(&self) -> bool {
        matches!(self, Self::Function(function) if function.is_uninitialized_lexical_marker())
    }

    pub(crate) fn same_value(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => {
                (left.is_nan() && right.is_nan()) || left.to_bits() == right.to_bits()
            }
            (Self::BigInt(left), Self::BigInt(right)) => left == right,
            (Self::String(left), Self::String(right)) => string::string_utf16_eq(left, right),
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            (Self::Null, Self::Null) | (Self::Undefined, Self::Undefined) => true,
            (Self::Function(left), Self::Function(right)) => left == right,
            (Self::Array(left), Self::Array(right)) => left.ptr_eq(right),
            (Self::Object(left), Self::Object(right)) => left.ptr_eq(right),
            (Self::Proxy(left), Self::Proxy(right)) => left.ptr_eq(right),
            (Self::Map(left), Self::Map(right)) => left.ptr_eq(right),
            (Self::Set(left), Self::Set(right)) => left.ptr_eq(right),
            _ => false,
        }
    }

    pub(crate) fn same_value_zero(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => {
                (left.is_nan() && right.is_nan()) || left == right
            }
            _ => self.same_value(other),
        }
    }
}

#[cfg(test)]
mod layout_tests {
    use super::Value;
    use crate::string::string_code_units;

    #[test]
    fn value_stays_two_machine_words() {
        assert_eq!(std::mem::size_of::<Value>(), 16);
    }

    #[test]
    fn host_utf8_string_constructor_preserves_sentinel_range_scalars() {
        let Value::String(value) = Value::string_from_utf8("\u{F0000}") else {
            unreachable!("constructor must return a string");
        };
        assert_eq!(string_code_units(&value), vec![0xDB80, 0xDC00]);
    }
}
