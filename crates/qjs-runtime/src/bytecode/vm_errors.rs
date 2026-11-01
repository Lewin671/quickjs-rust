use crate::{RuntimeError, Value};

use super::vm::Vm;

impl Vm<'_, '_> {
    pub(super) fn handle_call_result(
        &mut self,
        result: Result<Value, RuntimeError>,
    ) -> Result<Option<Value>, RuntimeError> {
        self.handle_runtime_result(result)
    }

    #[inline]
    pub(super) fn handle_runtime_result<T>(
        &mut self,
        result: Result<T, RuntimeError>,
    ) -> Result<Option<T>, RuntimeError> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(error) => {
                self.handle_runtime_error(error)?;
                Ok(None)
            }
        }
    }

    /// Raises `error` inside the current frame.
    ///
    /// The frame's `try`/`catch`/`finally` claims it when there is a live
    /// handler; otherwise it propagates. The frame-stack driver uses this to
    /// deliver a failed callee to its caller, so a routed call unwinds exactly
    /// as a nested `Vm` returning `Err` does today.
    pub(super) fn raise_in_current_frame(
        &mut self,
        error: RuntimeError,
    ) -> Result<(), RuntimeError> {
        self.handle_runtime_error(error)
    }

    #[cold]
    #[inline(never)]
    fn handle_runtime_error(&mut self, error: RuntimeError) -> Result<(), RuntimeError> {
        let error = crate::error::capture_error_origin(error, &self.context());
        if self.try_stack_is_empty() {
            return Err(error);
        }
        let value = *error
            .thrown
            .expect("origin capture materializes a thrown value");
        self.throw_value(value)?;
        Ok(())
    }
}
