//! Module-body evaluation entry points for the bytecode VM.
//!
//! A module graph shares one realm (see [`new_module_realm`]) so that a
//! function defined in one module retains that module's canonical cells. The realm is
//! distinct from any script realm, so module code never leaks names to the
//! process-wide `globalThis` used by [`crate::eval`].

use std::{collections::HashMap, rc::Rc};

use crate::{
    GLOBAL_THIS_BINDING, ObjectRef, RuntimeError, Value,
    function::{CallEnv, Realm},
    initialize_builtins,
};

use super::ir::Bytecode;
use super::vm::Vm;
use super::vm_generator::{GeneratorStart, GeneratorState};
use super::{ModuleBindings, ModuleEvaluation};

/// Evaluates a prelude *script* against the shared graph `realm` before any
/// module body runs. The prelude is ordinary (non-module) script code — for the
/// Test262 channel, the harness includes (`assert.js`, `sta.js`, the host shim,
/// and the async `$DONE` handler) — whose top-level `var`/`function` bindings
/// must be visible to every module in the graph. Pending promise jobs are
/// drained so a prelude that schedules microtasks settles before module
/// evaluation.
pub(super) fn eval_prelude_script(bytecode: &Bytecode, realm: &Realm) -> Result<(), RuntimeError> {
    Vm::initialize_script_global_bindings(bytecode, realm)?;
    realm.refresh_dynamic_function_realm_global();
    let env = CallEnv::new(Rc::clone(realm));
    let mut vm = Vm::new_with_globals(bytecode, env);
    vm.run()?;
    vm.drain_promise_jobs()?;
    Ok(())
}

/// Builds a fresh realm for a module graph: built-ins, a graph-private
/// `globalThis`. Module entry separately initializes its undefined receiver.
pub(super) fn new_module_realm() -> Realm {
    let mut globals = HashMap::new();
    let atoms = crate::property::atom::AtomDomain::new();
    let global_this = Value::Object(ObjectRef::new(HashMap::new(), &atoms));
    globals.insert(GLOBAL_THIS_BINDING.to_owned(), global_this.clone());
    globals.insert("undefined".to_owned(), Value::Undefined);
    let realm: Realm = crate::function::new_realm_sharing_atoms(globals, atoms);
    let mut env = CallEnv::new(Rc::clone(&realm));
    initialize_builtins(&mut env, &global_this);
    realm
}

/// Evaluates a module body against its canonical declaration and import cells.
///
/// Synchronous and top-level-await module bodies keep top-level declarations
/// in the module frame and shared module cells, so they do not become
/// `globalThis` properties.
pub(super) fn eval_module_body(
    bytecode: &Bytecode,
    realm: &Realm,
    host: Option<crate::module::ModuleHostRef>,
    module_bindings: ModuleBindings,
    drain: bool,
) -> Result<ModuleEvaluation, RuntimeError> {
    if bytecode.is_global_scope() {
        Vm::initialize_script_global_bindings(bytecode, realm)?;
    }
    realm.refresh_dynamic_function_realm_global();
    let mut env = CallEnv::new(Rc::clone(realm));
    // Module this is initialized undefined, including when an arrow captures
    // it into a positional cell before the first computed key runs.
    env.set_call_owner(crate::context::CallOwner::root(
        Value::Undefined,
        crate::context::OwnerKind::Module,
    ));
    // Wire the realm's dynamic-import host so an `import()` in this module body
    // (or in a closure it creates) can load further modules.
    if let Some(host) = host {
        env.set_module_host(host);
    }
    // A module body with top-level `await` evaluates like an async function body
    // (16.2.1.5.3 AsyncModuleExecution): it suspends at each `await` and resumes
    // through the realm job queue, settling a result promise on completion.
    if bytecode.contains_top_level_await() {
        return eval_async_module_body(bytecode, env, module_bindings, drain);
    }
    for import in module_bindings.imports {
        env.set_module_import(import.local_name, import.bindings, import.binding_name);
    }
    env.set_module_live_bindings(module_bindings.bindings.clone());
    let mut vm = Vm::new_with_globals(bytecode, env);
    vm.run()?;
    // The dynamic-import path defers job draining to the outer queue loop so the
    // module graph (borrowed while this body runs) is not re-borrowed by a
    // nested dynamic-import job mid-evaluation.
    if drain {
        vm.drain_promise_jobs()?;
    }
    Ok(ModuleEvaluation {
        async_result_promise: None,
    })
}

pub(super) fn eval_module_function_hoists(
    bytecode: &Bytecode,
    realm: &Realm,
    host: Option<crate::module::ModuleHostRef>,
    module_bindings: ModuleBindings,
) -> Result<(), RuntimeError> {
    let mut env = CallEnv::new(Rc::clone(realm));
    // Module this is initialized undefined, including when an arrow captures
    // it into a positional cell before the first computed key runs.
    env.set_call_owner(crate::context::CallOwner::root(
        Value::Undefined,
        crate::context::OwnerKind::Module,
    ));
    if let Some(host) = host {
        env.set_module_host(host);
    }
    for import in module_bindings.imports {
        env.set_module_import(import.local_name, import.bindings, import.binding_name);
    }
    env.set_module_live_bindings(module_bindings.bindings.clone());
    let mut vm = Vm::new_with_globals(bytecode, env);
    vm.run()?;
    Ok(())
}

/// Evaluates a module body that contains top-level `await`. The body is staged
/// as a `SuspendedStart` async context and driven through the realm job queue.
/// Plain promise awaits settle here before dependents evaluate. Dynamic-import
/// jobs are deferred to the outer module driver because they need to re-borrow
/// the module graph after static evaluation releases it. A rejected completion
/// that settles during this pass is surfaced as a `RuntimeError` (carrying the
/// rejection reason) so the linker propagates it to the caller / import promise.
///
/// Suspended frames and module namespaces retain the same canonical cells.
///
fn eval_async_module_body(
    bytecode: &Bytecode,
    mut env: CallEnv,
    module_bindings: ModuleBindings,
    drain: bool,
) -> Result<ModuleEvaluation, RuntimeError> {
    for import in module_bindings.imports {
        env.set_module_import(import.local_name, import.bindings, import.binding_name);
    }
    // Suspension retains the same module cells that declaration instantiation used.
    env.set_module_live_bindings(module_bindings.bindings.clone());
    let function_env = env.clone();
    let context = ObjectRef::new(HashMap::new(), env.realm().atoms());
    *context.generator_state().borrow_mut() =
        Some(GeneratorState::SuspendedStart(Box::new(GeneratorStart {
            bytecode: Rc::new(bytecode.clone()),
            env: function_env,
            inputs: None,
            upvalues: Vec::new(),
            with_stack: Vec::new(),
        })));

    let result_promise = crate::async_function::drive_async_module(&context, &mut env);
    // Resume ordinary `await`s and reactions when this module body is allowed
    // to settle immediately. Static graph evaluation starts async dependencies
    // without draining them so sibling modules in the same dependency list can
    // run before a top-level await resumes.
    if drain {
        crate::promise::drain_promise_jobs_until_dynamic_import(&mut env)?;
    }
    if let Some(Err(reason)) = crate::promise::settled_outcome(&result_promise) {
        return Err(RuntimeError {
            thrown: Some(Box::new(reason)),
            message: "module top-level await rejected".to_owned(),
        });
    }
    Ok(ModuleEvaluation {
        async_result_promise: Some(result_promise),
    })
}
