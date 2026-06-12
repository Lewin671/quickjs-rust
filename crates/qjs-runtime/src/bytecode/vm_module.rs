//! Module-body evaluation entry points for the bytecode VM.
//!
//! A module graph shares one realm (see [`new_module_realm`]) so that a
//! function defined in one module resolves its free names — and a sibling
//! cyclic module's hoisted functions — against the same bindings. The realm is
//! distinct from any script realm, so module code never leaks names to the
//! process-wide `globalThis` used by [`crate::eval`].

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    GLOBAL_THIS_BINDING, ObjectRef, RuntimeError, Value, function::CallEnv, function::Realm,
    initialize_builtins,
};

use super::ir::Bytecode;
use super::vm::Vm;

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
        Vm::initialize_script_global_bindings(bytecode, &mut globals);
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
    drain: bool,
) -> Result<CallEnv, RuntimeError> {
    {
        let mut globals = realm.borrow_mut();
        Vm::initialize_script_global_bindings(bytecode, &mut globals);
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
    let captured_env = Rc::new(RefCell::new(HashMap::new()));
    let mut vm = Vm::new_with_globals_and_captures(bytecode, env, captured_env);
    vm.run()?;
    // The dynamic-import path defers job draining to the outer queue loop so the
    // module graph (borrowed while this body runs) is not re-borrowed by a
    // nested dynamic-import job mid-evaluation.
    if drain {
        vm.drain_promise_jobs()?;
    }
    Ok(vm.current_env())
}
