use std::fmt;

mod array;
mod map;
mod object;
mod property;
mod set;

pub use array::ArrayRef;
pub use map::MapRef;
pub use object::ObjectRef;
pub(crate) use property::Property;
pub use set::SetRef;

use crate::Function;

/// A JavaScript value supported by the current runtime subset.
#[derive(Clone)]
pub enum Value {
    /// Number value.
    Number(f64),
    /// String value.
    String(String),
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
    /// Map object value.
    Map(MapRef),
    /// Set object value.
    Set(SetRef),
}

impl fmt::Debug for Value {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Number(value) => formatter.debug_tuple("Number").field(value).finish(),
            Self::String(value) => formatter.debug_tuple("String").field(value).finish(),
            Self::Boolean(value) => formatter.debug_tuple("Boolean").field(value).finish(),
            Self::Null => formatter.write_str("Null"),
            Self::Undefined => formatter.write_str("Undefined"),
            Self::Function(function) => formatter.debug_tuple("Function").field(function).finish(),
            Self::Array(elements) => formatter.debug_tuple("Array").field(elements).finish(),
            Self::Object(object) => formatter.debug_tuple("Object").field(object).finish(),
            Self::Map(map) => formatter.debug_tuple("Map").field(map).finish(),
            Self::Set(set) => formatter.debug_tuple("Set").field(set).finish(),
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left == right,
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            (Self::Null, Self::Null) | (Self::Undefined, Self::Undefined) => true,
            (Self::Function(left), Self::Function(right)) => left == right,
            (Self::Array(left), Self::Array(right)) => left.ptr_eq(right),
            (Self::Object(left), Self::Object(right)) => left.ptr_eq(right),
            (Self::Map(left), Self::Map(right)) => left.ptr_eq(right),
            (Self::Set(left), Self::Set(right)) => left.ptr_eq(right),
            _ => false,
        }
    }
}

impl Value {
    pub(crate) fn same_value(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => {
                (left.is_nan() && right.is_nan()) || left.to_bits() == right.to_bits()
            }
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            (Self::Null, Self::Null) | (Self::Undefined, Self::Undefined) => true,
            (Self::Function(left), Self::Function(right)) => left == right,
            (Self::Array(left), Self::Array(right)) => left.ptr_eq(right),
            (Self::Object(left), Self::Object(right)) => left.ptr_eq(right),
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
