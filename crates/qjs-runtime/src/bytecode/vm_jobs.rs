//! Host microtask ownership. Jobs retain their canonical realm/creation and
//! services, never a borrowed Context or an execution binding projection.
use super::{ir::Bytecode, vm::Vm};
use crate::{RuntimeError, Value, promise};
use std::rc::Rc;

pub(super) struct JobRoot {
    runtime: crate::context::RuntimeServices,
    creation: Rc<crate::context::CreationContext>,
}
impl Clone for JobRoot {
    fn clone(&self) -> Self {
        Self {
            runtime: self.runtime.retain_host_root(),
            creation: self.creation.clone(),
        }
    }
}
impl std::fmt::Debug for JobRoot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JobRoot").finish_non_exhaustive()
    }
}

pub(super) fn eval_bytecode_keep_jobs(
    bytecode: &Bytecode,
) -> Result<(Value, JobRoot), RuntimeError> {
    let (mut runtime, activation) = crate::context::HostRoot::script().into_parts();
    let mut vm = Vm::new(bytecode, &mut runtime, activation)?;
    let value = vm.run()?;
    let creation = vm.env.creation_context();
    drop(vm);
    Ok((value, JobRoot { runtime, creation }))
}

pub(super) fn run_pending_jobs(root: &mut JobRoot) -> Result<(), RuntimeError> {
    let mut activation = crate::context::ActivationContext::new(
        root.creation.clone(),
        crate::context::ExecutionMode::Script,
    );
    let mut context = crate::context::Context {
        runtime: &mut root.runtime,
        activation: &mut activation,
        native: None,
    };
    promise::drain_promise_jobs(&mut context)
}
