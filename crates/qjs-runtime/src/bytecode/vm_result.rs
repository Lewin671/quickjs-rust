use crate::{
    RuntimeError, Value,
    function::{CallEnv, Upvalue},
};

use super::ir::Bytecode;

pub(crate) struct FunctionBytecodeResult<'a> {
    pub(crate) value: Option<Result<Value, RuntimeError>>,
    pub(super) bytecode: &'a Bytecode,
    pub(super) env: CallEnv,
    pub(super) locals: Vec<Option<Value>>,
    pub(super) local_upvalues: Vec<Option<Upvalue>>,
    pub(crate) sloppy_global_names: Vec<String>,
}

/// How the bytecode loop exited: an ordinary/abrupt return value, or a
/// generator `yield` carrying the yielded value.
pub(super) enum Completion {
    Return(Value),
    Yield(Value),
    /// An `await` (`Op::Await`) suspended the body. The carried value is the
    /// operand being awaited. On resume the fulfillment value (or an injected
    /// throw for a rejection) is delivered at the `await` site. Distinct from
    /// `Yield` so an async generator's driver can route the suspension to a
    /// promise reaction rather than to its consumer's next/return/throw.
    Await(Value),
    /// A `yield*` suspended the generator while delegating to an inner
    /// iterator. The yielded value is the inner iterator's result object
    /// (returned to the outer caller unwrapped). On resume the next/return/
    /// throw is forwarded to the inner iterator rather than delivered at the
    /// `yield*` site.
    YieldDelegate(Value),
    /// Async `yield*` yielded a not-done iterator result to the consumer. A
    /// later `return(v)` must first await `v` before consulting the inner
    /// iterator's `return` method.
    YieldDelegateAsync(Value),
    /// Async `yield*` suspended while awaiting an inner iterator method result
    /// before it can inspect the iterator-result object. The suspension is not
    /// consumer-facing; the async-generator driver resumes it through the
    /// promise job queue, then the same `Op::YieldDelegate` classifies the
    /// awaited result.
    YieldDelegateAwait(Value),
    YieldDelegateAwaitReturn(Value),
    YieldDelegateAwaitReturnValue(Value),
    /// The body reached `Op::FunctionPrologueEnd`, the boundary after parameter
    /// instantiation. Used only when starting a generator/async-generator: the
    /// spec performs `FunctionDeclarationInstantiation` synchronously at the
    /// call (so a parameter-binding error throws before the generator object
    /// exists), then suspends at the start of the body. The carried snapshot
    /// state is captured by the generator driver.
    PrologueEnd,
}

/// How a delegating `yield*` is resumed: this mirrors `Resume` but is staged on
/// the VM so the re-entered `Op::YieldDelegate` can forward it to the inner
/// iterator instead of having it delivered at the bytecode level.
pub(super) enum ResumeMode {
    Next(Value),
    Return(Value),
    Throw(Value),
    Awaited(Value),
    AwaitRejected(Value),
    AwaitedReturn(Value),
    AwaitReturnRejected(Value),
    AwaitedReturnValue(Value),
    AwaitReturnValueRejected(Value),
}

impl FunctionBytecodeResult<'_> {
    pub(crate) fn into_value(mut self) -> Result<Value, RuntimeError> {
        self.value
            .take()
            .expect("function bytecode result value is consumed once")
    }

    pub(crate) fn frame_binding(&self, name: &str) -> Option<Value> {
        self.bytecode
            .local_slot(name)
            .and_then(|index| {
                self.locals.get(index).and_then(Option::as_ref)?;
                self.local_upvalues
                    .get(index)
                    .and_then(Option::as_ref)
                    .map(Upvalue::get)
                    .or_else(|| self.locals.get(index).and_then(Option::as_ref).cloned())
            })
            .or_else(|| self.env.get_local(name))
    }

    pub(crate) fn frame_cell_binding(&self, name: &str) -> Option<Value> {
        self.bytecode
            .local_slot(name)
            .and_then(|index| {
                self.local_upvalues
                    .get(index)
                    .and_then(Option::as_ref)
                    .map(Upvalue::get)
                    .or_else(|| self.locals.get(index).and_then(Option::as_ref).cloned())
            })
            .filter(|value| !value.is_uninitialized_lexical_marker())
            .or_else(|| self.env.get_local(name))
    }

    pub(crate) fn binding(&self, name: &str) -> Option<Value> {
        self.bytecode
            .local_slot(name)
            .and_then(|index| {
                self.locals.get(index).and_then(Option::as_ref)?;
                self.local_upvalues
                    .get(index)
                    .and_then(Option::as_ref)
                    .map(Upvalue::get)
                    .or_else(|| self.locals.get(index).and_then(Option::as_ref).cloned())
            })
            .or_else(|| self.env.get(name))
    }
}

impl Drop for FunctionBytecodeResult<'_> {
    fn drop(&mut self) {
        self.bytecode
            .recycle_frame_locals(std::mem::take(&mut self.locals));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::function::new_realm;
    use std::collections::HashMap;

    #[test]
    fn dropping_result_returns_cleared_locals_to_bytecode_pool() {
        let bytecode = Bytecode::new(Vec::new(), Vec::new(), Vec::new());
        let locals = vec![Some(Value::Number(1.0))];
        let allocation = locals.as_ptr();
        let result = FunctionBytecodeResult {
            value: Some(Ok(Value::Undefined)),
            bytecode: &bytecode,
            env: CallEnv::new(new_realm(HashMap::new())),
            locals,
            local_upvalues: Vec::new(),
            sloppy_global_names: Vec::new(),
        };

        drop(result);
        let reused = bytecode.take_frame_locals();

        assert!(reused.is_empty());
        assert_eq!(reused.as_ptr(), allocation);
    }
}
