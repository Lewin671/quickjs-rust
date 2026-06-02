use std::collections::HashMap;

use crate::{RuntimeError, Value};

use super::ir::Bytecode;

pub(crate) struct FunctionBytecodeResult<'a> {
    pub(crate) value: Result<Value, RuntimeError>,
    pub(super) bytecode: &'a Bytecode,
    pub(super) globals: HashMap<String, Value>,
    pub(super) locals: Vec<Option<Value>>,
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
