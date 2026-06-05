use crate::RuntimeError;

use super::compiler::Compiler;

impl Compiler {
    pub(super) fn validate_strict_binding_name(&self, name: &str) -> Result<(), RuntimeError> {
        if self.strict && matches!(name, "eval" | "arguments") {
            return Err(RuntimeError {
                thrown: None,
                message: format!("SyntaxError: invalid strict binding `{name}`"),
            });
        }
        Ok(())
    }
}
