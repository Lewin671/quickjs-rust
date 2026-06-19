//! Dynamic `import()` evaluation and module-host threading for the VM.
//!
//! The realm's [`crate::module::ModuleHostRef`] rides on the VM so every
//! `CallEnv` it builds (frame envs, nested-call envs, the job-draining env)
//! keeps the host reachable; an `Op::ImportCall` then builds the import promise
//! through [`crate::promise::dynamic_import`], which schedules a host load job
//! that settles the promise as a microtask.

use crate::{RuntimeError, Value, function::CallEnv};

use super::ir::Bytecode;
use super::vm::Vm;

/// Evaluates compiled script bytecode with a dynamic-import `host` installed on
/// its environment, draining the promise job queue (including any import jobs)
/// before returning the completion value.
pub(super) fn eval_bytecode_with_module_host(
    bytecode: &Bytecode,
    host: crate::module::ModuleHostRef,
) -> Result<Value, RuntimeError> {
    let mut vm = Vm::new(bytecode)?;
    vm.module_host = Some(host);
    let value = vm.run()?;
    vm.drain_promise_jobs()?;
    Ok(value)
}

impl Vm<'_> {
    /// Stamps this VM's dynamic-import host (if any) onto a freshly built
    /// `CallEnv` so the host survives across frame, nested-call, and
    /// job-draining environments rebuilt from the shared realm.
    pub(super) fn attach_host(&self, mut env: CallEnv) -> CallEnv {
        if let Some(host) = &self.module_host {
            env.set_module_host(host.clone());
        }
        env
    }

    /// Evaluates `Op::ImportCall`: the specifier is on top of the stack (and the
    /// options argument below it when `has_options`). Builds the import promise
    /// (coercion failures reject it rather than throwing) and pushes it.
    pub(super) fn import_call(&mut self, has_options: bool) -> Result<(), RuntimeError> {
        let specifier = self.pop()?;
        // The second (options/attributes) argument is validated per spec
        // (EvaluateImportCall): a non-object or a non-string `with` attribute
        // rejects the import promise. Attributes do not affect resolution here.
        let options = if has_options { Some(self.pop()?) } else { None };
        let mut env = self.current_env();
        let promise = crate::promise::dynamic_import(specifier, options, &mut env);
        self.apply_env(env);
        self.stack.push(promise);
        Ok(())
    }
}
