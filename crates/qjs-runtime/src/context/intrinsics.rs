//! Stable prototype identities used by primitive boxing/property operations.
//! Public constructor bindings are mutable; these semantic identities are not.
use crate::{Function, ObjectRef, Value};

#[derive(Clone, Copy)]
pub(crate) enum PrimitiveIntrinsic {
    Boolean,
    Number,
    String,
    BigInt,
    Symbol,
}
impl PrimitiveIntrinsic {
    pub(crate) fn index(self) -> usize {
        self as usize
    }
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        match name {
            "Boolean" => Some(Self::Boolean),
            "Number" => Some(Self::Number),
            "String" => Some(Self::String),
            "BigInt" => Some(Self::BigInt),
            "Symbol" => Some(Self::Symbol),
            _ => None,
        }
    }
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Boolean => "Boolean",
            Self::Number => "Number",
            Self::String => "String",
            Self::BigInt => "BigInt",
            Self::Symbol => "Symbol",
        }
    }
}

pub(crate) fn constructor_object_prototype(value: Value) -> Option<ObjectRef> {
    let Value::Function(function) = value else {
        return None;
    };
    crate::function_prototype(&function)
}

pub(crate) fn constructor_function_prototype(function: &Function) -> Option<crate::Prototype> {
    match function.own_property("prototype")?.value {
        Value::Object(object) if !crate::symbol::is_symbol_primitive(&object) => {
            Some(crate::Prototype::Object(object))
        }
        Value::Function(function) => Some(crate::Prototype::Function(function)),
        _ => None,
    }
}

/// Error identities used by internal ThrowCompletion/rejection construction.
/// Mutable public constructor bindings never select these intrinsic prototypes.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ErrorKind {
    Error,
    EvalError,
    RangeError,
    ReferenceError,
    SyntaxError,
    TypeError,
    UriError,
    AggregateError,
    SuppressedError,
}
impl ErrorKind {
    pub(crate) const ALL: [Self; 9] = [
        Self::Error,
        Self::EvalError,
        Self::RangeError,
        Self::ReferenceError,
        Self::SyntaxError,
        Self::TypeError,
        Self::UriError,
        Self::AggregateError,
        Self::SuppressedError,
    ];
    pub(crate) fn index(self) -> usize {
        self as usize
    }
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Error => "Error",
            Self::EvalError => "EvalError",
            Self::RangeError => "RangeError",
            Self::ReferenceError => "ReferenceError",
            Self::SyntaxError => "SyntaxError",
            Self::TypeError => "TypeError",
            Self::UriError => "URIError",
            Self::AggregateError => "AggregateError",
            Self::SuppressedError => "SuppressedError",
        }
    }
    pub(crate) fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.name() == name)
    }
}

pub(super) struct ErrorIntrinsic {
    pub(super) constructor: Function,
    pub(super) prototype: ObjectRef,
}
