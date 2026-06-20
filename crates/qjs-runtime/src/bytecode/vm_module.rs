//! Module-body evaluation entry points for the bytecode VM.
//!
//! A module graph shares one realm (see [`new_module_realm`]) so that a
//! function defined in one module resolves its free names — and a sibling
//! cyclic module's hoisted functions — against the same bindings. The realm is
//! distinct from any script realm, so module code never leaks names to the
//! process-wide `globalThis` used by [`crate::eval`].

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    Function, GLOBAL_THIS_BINDING, ObjectRef, RuntimeError, Value, function::CallEnv,
    function::Realm, initialize_builtins,
};

use super::ir::Bytecode;
use super::vm::Vm;
use super::vm_generator::{GeneratorStart, GeneratorState};
use super::{ModuleEvaluation, ModuleLiveExports};

/// Evaluates a prelude *script* against the shared graph `realm` before any
/// module body runs. The prelude is ordinary (non-module) script code — for the
/// Test262 channel, the harness includes (`assert.js`, `sta.js`, the host shim,
/// and the async `$DONE` handler) — whose top-level `var`/`function` bindings
/// must be visible to every module in the graph. Pending promise jobs are
/// drained so a prelude that schedules microtasks settles before module
/// evaluation.
pub(super) fn eval_prelude_script(bytecode: &Bytecode, realm: &Realm) -> Result<(), RuntimeError> {
    {
        let mut globals = realm.borrow_mut();
        Vm::initialize_script_global_bindings(bytecode, &mut globals)?;
    }
    let env = CallEnv::new(Rc::clone(realm));
    let captured_env = Rc::new(RefCell::new(HashMap::new()));
    let mut vm = Vm::new_with_globals_and_captures(bytecode, env, captured_env);
    vm.run()?;
    vm.drain_promise_jobs()?;
    Ok(())
}

/// Builds a fresh realm for a module graph: built-ins, a graph-private
/// `globalThis`, and `this` = undefined (module top-level `this`).
pub(super) fn new_module_realm() -> Realm {
    let mut globals = HashMap::new();
    let global_this = Value::Object(ObjectRef::new(HashMap::new()));
    globals.insert("this".to_owned(), Value::Undefined);
    globals.insert(GLOBAL_THIS_BINDING.to_owned(), global_this.clone());
    globals.insert("undefined".to_owned(), Value::Undefined);
    let realm: Realm = Rc::new(RefCell::new(globals));
    let mut env = CallEnv::new(Rc::clone(&realm));
    initialize_builtins(&mut env, &global_this);
    realm
}

/// Evaluates a module body (compiled as global-scope bytecode) against the
/// shared graph `realm`, seeding `imports` as module-scope bindings first.
/// Returns the module's frame environment so the linker can read its exports.
///
/// All of a module's top-level `var`/`let`/`const`/`function`/`class` bindings
/// land in the shared graph realm; imported bindings are inserted there before
/// the body runs, so import references resolve through the ordinary global-load
/// path and functions defined in any module see them.
pub(super) fn eval_module_body(
    bytecode: &Bytecode,
    realm: &Realm,
    imports: HashMap<String, Value>,
    host: Option<crate::module::ModuleHostRef>,
    live_exports: ModuleLiveExports,
    drain: bool,
) -> Result<ModuleEvaluation, RuntimeError> {
    {
        let mut globals = realm.borrow_mut();
        Vm::initialize_script_global_bindings(bytecode, &mut globals)?;
        for (name, value) in imports {
            globals.insert(name, value);
        }
    }
    let mut env = CallEnv::new(Rc::clone(realm));
    // Wire the realm's dynamic-import host so an `import()` in this module body
    // (or in a closure it creates) can load further modules.
    if let Some(host) = host {
        env.set_module_host(host);
    }
    // A module body with top-level `await` evaluates like an async function body
    // (16.2.1.5.3 AsyncModuleExecution): it suspends at each `await` and resumes
    // through the realm job queue, settling a result promise on completion.
    if bytecode.contains_top_level_await() {
        return eval_async_module_body(bytecode, env, live_exports);
    }
    seed_live_bindings(
        &live_exports.bindings,
        bytecode,
        live_exports.names,
        live_exports.seed_tdz_markers,
    );
    let mut vm = Vm::new_with_globals_and_captures(bytecode, env, live_exports.bindings);
    vm.run()?;
    // The dynamic-import path defers job draining to the outer queue loop so the
    // module graph (borrowed while this body runs) is not re-borrowed by a
    // nested dynamic-import job mid-evaluation.
    if drain {
        vm.drain_promise_jobs()?;
    }
    Ok(ModuleEvaluation {
        env: vm.current_env(),
        captured_env: vm.captured_env.clone(),
    })
}

/// Evaluates a module body that contains top-level `await`. The body is staged
/// as a `SuspendedStart` async context and driven to completion, draining the
/// realm job queue so each `await` resumes and the module settles before its
/// dependents evaluate. A rejected completion is surfaced as a `RuntimeError`
/// (carrying the rejection reason) so the linker propagates it to the caller /
/// the import promise.
///
/// The body's top-level `var`/`function` bindings live in the shared realm and
/// its top-level `let`/`const` bindings write through to the shared
/// `captured_env` cell (see `Vm::store_local`); the returned `CallEnv` merges
/// both so the linker reads every export after the module has settled.
///
/// Residue: a top-level `await` of a *dynamic* `import()` cannot drain here
/// (the module graph is borrowed by the static evaluation, so a nested import
/// job would re-borrow it); that path is left to the outer queue loop.
fn eval_async_module_body(
    bytecode: &Bytecode,
    mut env: CallEnv,
    live_exports: ModuleLiveExports,
) -> Result<ModuleEvaluation, RuntimeError> {
    let realm = env.realm_rc();
    // Seed the shared captured-env cell with every top-level local name so each
    // `store_local` (notably a `let`/`const` export written after an `await`
    // resumes) writes through to it; the linker reads these settled lexical
    // exports back after the module's promise settles. `var`/`function` exports
    // live in the realm and are read from there.
    seed_live_bindings(
        &live_exports.bindings,
        bytecode,
        live_exports.names,
        live_exports.seed_tdz_markers,
    );
    {
        let mut captured = live_exports.bindings.borrow_mut();
        for name in bytecode.local_names().filter(|name| {
            bytecode
                .local_slot(name)
                .is_some_and(|slot| !bytecode.local_is_body_hoist_only(slot))
        }) {
            captured
                .entry(name.to_owned())
                .or_insert_with(|| Value::Function(Function::uninitialized_lexical_marker()));
        }
    }
    let captured_env = live_exports.bindings;
    let function_env = env.clone();
    let context = ObjectRef::new(HashMap::new());
    *context.generator_state().borrow_mut() =
        Some(GeneratorState::SuspendedStart(Box::new(GeneratorStart {
            bytecode: Rc::new(bytecode.clone()),
            env: function_env,
            captured_env: Rc::clone(&captured_env),
            with_stack: Vec::new(),
            refresh_captured_slots_on_resume: false,
            capture_writeback: None,
        })));

    let result_promise = crate::async_function::drive_async_module(&context, &mut env);
    // Resume the suspended `await`s and run any reactions the module scheduled.
    crate::promise::drain_promise_jobs(&mut env)?;
    if let Some(Err(reason)) = crate::promise::settled_outcome(&result_promise) {
        return Err(RuntimeError {
            thrown: Some(Box::new(reason)),
            message: "module top-level await rejected".to_owned(),
        });
    }
    // Materialize the settled module frame: realm bindings (var/function) plus
    // the captured lexical slots (let/const) written through during evaluation.
    // A top-level `var`/`function` export lives in the realm and must be read
    // from there, so drop any captured local that shadows a realm binding (the
    // seed leaves it `Undefined`); the remaining captured locals are the
    // module's lexical exports.
    let locals: HashMap<String, Value> = captured_env
        .borrow()
        .iter()
        .filter(|(name, _)| !realm.borrow().contains_key(name.as_str()))
        .map(|(name, value)| (name.clone(), value.clone()))
        .collect();
    Ok(ModuleEvaluation {
        env: CallEnv::with_locals(realm, locals),
        captured_env,
    })
}

pub(super) fn seed_live_bindings(
    live_bindings: &Rc<RefCell<HashMap<String, Value>>>,
    bytecode: &Bytecode,
    names: Vec<String>,
    seed_tdz_markers: bool,
) {
    let mut bindings = live_bindings.borrow_mut();
    for name in names {
        let value = match bytecode.local_slot(&name) {
            Some(slot) if bytecode.local_is_body_hoist_only(slot) => Value::Undefined,
            Some(slot) if seed_tdz_markers || bytecode.local_is_mutable(slot) => {
                Value::Function(Function::uninitialized_lexical_marker())
            }
            Some(_) => continue,
            None => Value::Undefined,
        };
        bindings.entry(name).or_insert(value);
    }
}
