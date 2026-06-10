use std::collections::HashMap;

use crate::{RuntimeError, Value};

use super::ir::Bytecode;

pub(crate) struct FunctionBytecodeResult<'a> {
    pub(crate) value: Result<Value, RuntimeError>,
    pub(super) bytecode: &'a Bytecode,
    pub(super) globals: HashMap<String, Value>,
    pub(super) locals: Vec<Option<Value>>,
    pub(crate) sloppy_global_names: Vec<String>,
}

/// How the bytecode loop exited: an ordinary/abrupt return value, or a
/// generator `yield` carrying the yielded value.
pub(super) enum Completion {
    Return(Value),
    Yield(Value),
    /// A `yield*` suspended the generator while delegating to an inner
    /// iterator. The yielded value is the inner iterator's result object
    /// (returned to the outer caller unwrapped). On resume the next/return/
    /// throw is forwarded to the inner iterator rather than delivered at the
    /// `yield*` site.
    YieldDelegate(Value),
}

/// How a delegating `yield*` is resumed: this mirrors `Resume` but is staged on
/// the VM so the re-entered `Op::YieldDelegate` can forward it to the inner
/// iterator instead of having it delivered at the bytecode level.
pub(super) enum ResumeMode {
    Next(Value),
    Return(Value),
    Throw(Value),
}

impl FunctionBytecodeResult<'_> {
    pub(crate) fn binding(&self, name: &str) -> Option<&Value> {
        self.bytecode
            .local_slot(name)
            .and_then(|index| self.locals.get(index))
            .and_then(Option::as_ref)
            .or_else(|| self.globals.get(name))
    }
}
