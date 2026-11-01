//! Stack borrows for effectful VM segments. No activation/realm clone or
//! binding container is created around a native/coercion/property operation.
use super::vm::Vm;
impl Vm<'_, '_> {
    pub(super) fn context(&mut self) -> crate::context::Context<'_> {
        crate::context::Context {
            runtime: self.runtime,
            activation: &mut self.current.env,
            native: None,
        }
    }
}
