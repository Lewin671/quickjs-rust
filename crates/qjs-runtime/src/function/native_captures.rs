//! Immutable native closure payloads. Each operation family selects a product
//! type; ordinary native functions retain no payload allocation or name table.
use crate::{ArrayRef, ObjectRef, Value};

#[derive(Clone, Default)]
pub(crate) enum NativeCaptures {
    #[default]
    None,
    RegExpPrototype(ObjectRef),
    PromisePrototypes(PromisePrototypeCaptures),
    PromiseElement(PromiseElementCaptures),
    PromiseCapability(ObjectRef),
    PromiseResolving {
        promise: Value,
        already_resolved: ObjectRef,
    },
    PromiseFinally {
        handler: Value,
        constructor: Value,
    },
    PromiseFinallyValue(Value),
    AsyncFunction {
        context: ObjectRef,
        result_promise: ObjectRef,
    },
    AsyncGenerator(ObjectRef),
    AsyncFromSync {
        iterator: Value,
        next: Option<Value>,
    },
    AsyncFromSyncValue {
        capability: ObjectRef,
        done: bool,
        close_iterator: Option<Value>,
    },
    ArrayFromAsync(ObjectRef),
}

#[derive(Clone)]
pub(crate) struct PromisePrototypeCaptures {
    pub(crate) promise: ObjectRef,
    pub(crate) object: Option<ObjectRef>,
    pub(crate) aggregate_error: Option<Value>,
}

#[derive(Clone)]
pub(crate) struct PromiseElementCaptures {
    pub(crate) index: usize,
    pub(crate) values: ArrayRef,
    pub(crate) remaining: ObjectRef,
    pub(crate) already_called: ObjectRef,
    pub(crate) settle: Value,
    pub(crate) aggregate_error: Option<Value>,
}

impl NativeCaptures {
    pub(crate) fn promise_prototypes(&self) -> Option<&PromisePrototypeCaptures> {
        match self {
            Self::PromisePrototypes(captures) => Some(captures),
            _ => None,
        }
    }
    pub(crate) fn promise_element(&self) -> Option<&PromiseElementCaptures> {
        match self {
            Self::PromiseElement(captures) => Some(captures),
            _ => None,
        }
    }
}
