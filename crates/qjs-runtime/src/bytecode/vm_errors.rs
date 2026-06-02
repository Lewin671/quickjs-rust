use crate::{GLOBAL_THIS_BINDING, RuntimeError, Value, error};

use super::vm::Vm;
use super::vm_call::native_error_message;

impl Vm<'_> {
    pub(super) fn handle_call_result(
        &mut self,
        result: Result<Value, RuntimeError>,
    ) -> Result<Option<Value>, RuntimeError> {
        self.handle_runtime_result(result)
    }

    pub(super) fn handle_runtime_result<T>(
        &mut self,
        result: Result<T, RuntimeError>,
    ) -> Result<Option<T>, RuntimeError> {
        match result {
            Ok(value) => Ok(Some(value)),
            Err(error) if self.should_rethrow_js_error(&error) => {
                let value = error.thrown.as_deref().cloned().unwrap_or_else(|| {
                    Value::String(
                        error
                            .message
                            .trim_start_matches("throw statement executed: ")
                            .to_owned(),
                    )
                });
                self.throw_value(value)?;
                Ok(None)
            }
            Err(error) if self.should_throw_native_error(&error) => {
                let value = self.native_error_value(&error.message)?;
                self.throw_value(value)?;
                Ok(None)
            }
            Err(error) => Err(error),
        }
    }

    fn should_rethrow_js_error(&self, error: &RuntimeError) -> bool {
        !self.try_stack.is_empty()
            && (error.thrown.is_some() || error.message.starts_with("throw statement executed: "))
    }

    fn should_throw_native_error(&self, error: &RuntimeError) -> bool {
        !self.try_stack.is_empty() && !error.message.starts_with("throw statement executed:")
    }

    fn native_error_value(&self, message: &str) -> Result<Value, RuntimeError> {
        let (constructor_name, message) = native_error_message(message);
        let Value::Function(function) = self
            .native_error_constructor(constructor_name)
            .ok_or_else(|| RuntimeError {
                thrown: None,
                message: format!("{constructor_name} constructor is not available"),
            })?
        else {
            return Err(RuntimeError {
                thrown: None,
                message: format!("{constructor_name} constructor is not callable"),
            });
        };
        error::native_error(
            &function,
            Value::Undefined,
            &[Value::String(message)],
            false,
        )
    }

    fn native_error_constructor(&self, name: &str) -> Option<Value> {
        self.globals.get(name).cloned().or_else(|| {
            let Some(Value::Object(global_this)) = self.globals.get(GLOBAL_THIS_BINDING) else {
                return None;
            };
            global_this.get(name)
        })
    }
}
