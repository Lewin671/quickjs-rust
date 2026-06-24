//! Dynamic `import()` evaluation and module-host threading for the VM.
//!
//! The realm's [`crate::module::ModuleHostRef`] rides on the VM so every
//! `CallEnv` it builds (frame envs, nested-call envs, the job-draining env)
//! keeps the host reachable; an `Op::ImportCall` then builds the import promise
//! through [`crate::promise::dynamic_import`], which schedules a host load job
//! that settles the promise as a microtask.

use crate::{ModuleResolver, RuntimeError, Value, function::CallEnv};

use super::ir::Bytecode;
use super::vm::Vm;

/// Evaluates compiled script bytecode with a dynamic-import host installed on
/// the script realm, draining the promise job queue (including any import jobs)
/// before returning the completion value.
pub(super) fn eval_bytecode_with_module_resolver(
    bytecode: &Bytecode,
    referrer: &str,
    resolver: Box<dyn ModuleResolver>,
) -> Result<Value, RuntimeError> {
    let mut vm = Vm::new(bytecode)?;
    let host = crate::module::new_script_module_host(referrer, resolver, vm.realm.clone());
    vm.module_host = Some(host);
    let value = vm.run()?;
    vm.drain_promise_jobs()?;
    Ok(value)
}

/// Like [`eval_bytecode_with_module_resolver`], but installs a Test262
/// `$262.agent` context whose `AgentCanSuspend()` is `can_block`. Used by the
/// CLI's `--agent-cannot-block` mode so a `CanBlockIsFalse`-flagged case makes
/// `Atomics.wait` throw a `TypeError`.
#[cfg(feature = "agents")]
pub(super) fn eval_bytecode_with_module_resolver_in_agent(
    bytecode: &Bytecode,
    referrer: &str,
    resolver: Box<dyn ModuleResolver>,
    can_block: bool,
) -> Result<Value, RuntimeError> {
    let mut vm = Vm::new(bytecode)?;
    let host = crate::module::new_script_module_host(referrer, resolver, vm.realm.clone());
    vm.module_host = Some(host);
    vm.agent_context = Some(crate::agent::AgentContext::main(can_block));
    let value = vm.run()?;
    vm.drain_promise_jobs()?;
    Ok(value)
}

/// Evaluates a worker agent's script bytecode with `context` installed (its
/// shared cluster and broadcast inbox), draining the promise job queue. Runs on
/// the worker's own OS thread; it has no module host (worker sources do not use
/// dynamic `import()`).
#[cfg(feature = "agents")]
pub(super) fn eval_bytecode_in_agent_context(
    bytecode: &Bytecode,
    context: crate::agent::AgentContextRef,
) -> Result<Value, RuntimeError> {
    let mut vm = Vm::new(bytecode)?;
    vm.agent_context = Some(context);
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
        #[cfg(feature = "agents")]
        if let Some(context) = &self.agent_context {
            env.set_agent_context(context.clone());
        }
        env
    }

    /// Evaluates `Op::ImportCall`: the options argument is on top of the stack
    /// (when `has_options`) with the specifier below it. Builds the import
    /// promise (coercion failures reject it rather than throwing) and pushes it.
    pub(super) fn import_call(&mut self, has_options: bool) -> Result<(), RuntimeError> {
        // The second (options/attributes) argument is validated per spec
        // (EvaluateImportCall): a non-object or a non-string `with` attribute
        // rejects the import promise. Attributes do not affect resolution here.
        let options = if has_options { Some(self.pop()?) } else { None };
        let specifier = self.pop()?;
        let mut env = self.current_env();
        let promise = crate::promise::dynamic_import(specifier, options, &mut env);
        self.apply_env(env);
        self.stack.push(promise);
        Ok(())
    }
}
