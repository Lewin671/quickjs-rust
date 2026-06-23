//! Generator object runtime: the suspend/resume state machine backing
//! `function*` bodies and the `next`/`return`/`throw` protocol methods.
//!
//! A generator object owns the resumable state of its body. Because the VM
//! keeps all execution state explicitly (instruction pointer, value stack,
//! locals, environment, and the try/finally stack), a suspended generator is
//! just that owned state plus the `Rc<Bytecode>` of its body. Resuming rebuilds
//! a [`Vm`] borrowing the held bytecode, delivers the resume value (or an
//! injected return/throw completion) at the yield point, and runs until the
//! next `Op::Yield`, an ordinary return, or an unwound abrupt completion.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{ObjectRef, RuntimeError, Value};

use super::ir::Bytecode;
use super::vm::{Slot, Vm};
use super::vm_dispose::DisposeResource;
use super::vm_result::Completion;
use super::vm_try::TryFrame;
use crate::CallEnv;

const DYNAMIC_FUNCTION_REALM_GLOBAL: &str = "__quickjsRustDynamicFunctionRealm";

/// The lifecycle state of a generator object (ES2023 27.5.3 [[GeneratorState]]).
pub(crate) enum GeneratorState {
    /// Created but never resumed: the first `next` runs the body from the top.
    SuspendedStart(Box<GeneratorStart>),
    /// Suspended at a `yield`: the next resume re-enters the saved VM.
    SuspendedYield(Box<GeneratorSnapshot>),
    /// Currently running: re-entrancy is a TypeError.
    Executing,
    /// Finished (return or uncaught throw): further `next` returns
    /// `{ value: undefined, done: true }`.
    Completed,
}

/// The captured call frame for a not-yet-started generator.
pub(crate) struct GeneratorStart {
    pub(crate) bytecode: Rc<Bytecode>,
    pub(crate) env: CallEnv,
    pub(crate) captured_env: Rc<RefCell<HashMap<String, Value>>>,
    pub(crate) upvalues: Vec<crate::function::Upvalue>,
    pub(crate) with_stack: Vec<Value>,
    pub(crate) refresh_captured_slots_on_resume: bool,
    pub(crate) capture_writeback: Option<CaptureWriteback>,
}

/// The original closure capture cell an async activation must update when it
/// mutates bindings captured from an enclosing function.
#[derive(Clone)]
pub(crate) struct CaptureWriteback {
    pub(crate) target: Rc<RefCell<HashMap<String, Value>>>,
    pub(crate) names: Vec<String>,
    pub(crate) aliases: Vec<(String, String)>,
    pub(crate) parent: Option<Box<CaptureWriteback>>,
}

/// A snapshot of a generator body's VM state, taken at a `yield`.
pub(crate) struct GeneratorSnapshot {
    bytecode: Rc<Bytecode>,
    ip: usize,
    stack: Vec<Value>,
    locals: Vec<Slot>,
    local_upvalues: Vec<Option<crate::function::Upvalue>>,
    upvalues: Vec<crate::function::Upvalue>,
    env: CallEnv,
    captured_env: Rc<RefCell<HashMap<String, Value>>>,
    with_stack: Vec<Value>,
    refresh_captured_slots_on_resume: bool,
    capture_writeback: Option<CaptureWriteback>,
    sloppy_global_names: Vec<String>,
    try_stack: Vec<TryFrame>,
    disposable_scopes: Vec<Vec<DisposeResource>>,
    pending_throw: Option<Value>,
    pending_return: Option<Value>,
    pending_jump: Option<usize>,
    suspension: SuspensionKind,
}

enum SuspensionKind {
    Ordinary,
    DelegateYield,
    DelegateYieldAsync,
    DelegateYieldReturnAwait,
    DelegateAwait,
    DelegateAwaitReturn,
    DelegateAwaitReturnValue,
}

/// How a suspended generator is resumed.
pub(crate) enum Resume {
    /// `next(v)`: deliver `v` as the value of the `yield` expression.
    Next(Value),
    /// `return(v)`: inject a return completion at the yield point so enclosing
    /// `finally` blocks run.
    Return(Value),
    /// `return(v)` after the async-generator driver has already awaited `v`.
    /// The injected return completion is the same, but the driver must not
    /// unwrap the resulting completion value a second time.
    ReturnAlreadyAwaited(Value),
    /// `throw(v)`: inject `v` as a thrown exception at the yield point.
    Throw(Value),
}

/// The outcome of resuming a generator, mapped to an iterator result by the
/// caller.
pub(crate) enum GeneratorOutcome {
    /// The body yielded: `{ value, done: false }`, state SuspendedYield.
    Yield(Value),
    /// The body suspended inside a `yield*`: the carried value is the inner
    /// iterator's result object, returned to the caller unwrapped (the spec
    /// hands back the inner result without rebuilding it). State SuspendedYield.
    YieldDelegate(Value),
    /// The body suspended at an `await` (`Op::Await`): the carried value is the
    /// operand being awaited. The async/async-generator driver resolves it and
    /// resumes the body via a promise reaction. State SuspendedYield. Plain
    /// generators never produce this (they emit no `Op::Await`).
    Await(Value),
    /// The body returned (or a `return(v)` completed it): `{ value, done: true }`.
    Return(Value),
    /// The body returned after async `yield*` already awaited the injected
    /// return value. Async-generator request settlement must not await it again.
    ReturnAlreadyAwaited(Value),
}

pub(crate) fn is_suspended_at_plain_yield(generator: &ObjectRef) -> bool {
    let slot = generator.generator_state().borrow();
    matches!(
        slot.as_ref(),
        Some(GeneratorState::SuspendedYield(snapshot))
            if matches!(snapshot.suspension, SuspensionKind::Ordinary)
    )
}

impl Vm<'_> {
    /// Propagates the body's final values for bindings it shares with the
    /// resuming caller back into the caller's environment, so a generator that
    /// mutates an outer `let`/`var` is observed by the resuming frame. Mirrors
    /// the caller-binding write-back performed for ordinary function calls.
    fn propagate_to_caller(&self, caller_env: &mut CallEnv) {
        let names: Vec<String> = caller_env.locals().keys().cloned().collect();
        for name in names {
            if crate::function::is_internal_binding_name(&name)
                || name == "this"
                || name == "arguments"
            {
                continue;
            }
            if let Some(value) = self.binding_value(&name)
                && let Some(slot) = caller_env.get_local_mut(&name)
            {
                *slot = value;
            }
        }
    }

    /// Refreshes the body's view of bindings it shares with the resuming
    /// caller. A generator captures its environment when the generator function
    /// is called, but it resumes later — the caller may have reassigned shared
    /// bindings in between (`var it = g()` itself is the common case), so the
    /// captured snapshot is stale by the time the body runs. Without this
    /// refresh the post-run write-back would clobber the caller's current
    /// values with the stale ones.
    fn refresh_from_caller(&mut self, caller_env: &CallEnv) {
        // The realm is shared by `Rc`, so realm bindings are already live in the
        // body; only the caller's frame *locals* need to be mirrored into the
        // body's own frame-local layer.
        for (name, value) in caller_env.locals() {
            // `this` (and `arguments`) belong to the generator's own frame,
            // never to the resuming caller; internal bindings likewise.
            if crate::function::is_internal_binding_name(name)
                || name == "this"
                || name == "arguments"
            {
                continue;
            }
            if self.env.locals().contains_key(&format!(
                "{}{}",
                crate::DIRECT_EVAL_PARAMETER_VAR_BINDING_PREFIX,
                name
            )) {
                continue;
            }
            if self.env.locals().contains_key(name) {
                self.env.insert(name.clone(), value.clone());
            }
        }
    }

    /// Refreshes this resumed VM from the shared capture cell. A suspended
    /// async/generator body keeps its own slot snapshot, but closures created by
    /// that body share `captured_env` and may run while the body is suspended
    /// (for example, a thenable's `then` callback during `await`). Pull those
    /// writes back before execution resumes so the snapshot does not roll them
    /// back.
    fn refresh_from_captured_env(&mut self) {
        let captured = self.captured_env.borrow();
        for (name, value) in captured.iter() {
            if let Some(slot) = self.bytecode.local_slot(name)
                && let Some(local) = self.locals.get_mut(slot)
            {
                *local = Some(value.clone());
            }
        }
    }

    /// Refreshes shared bindings captured from an enclosing function. Async
    /// functions may suspend while sibling closures mutate those bindings; the
    /// writeback target is the shared cell those sibling closures update.
    fn refresh_from_capture_writeback(&mut self, writeback: Option<&CaptureWriteback>) {
        let Some(writeback) = writeback else {
            return;
        };
        self.refresh_one_capture_writeback(writeback);
        self.refresh_from_capture_writeback(writeback.parent.as_deref());
    }

    fn refresh_one_capture_writeback(&mut self, writeback: &CaptureWriteback) {
        let target = writeback.target.borrow();
        for name in &writeback.names {
            if let Some(value) = target.get(name) {
                self.refresh_capture_slot(name, value);
            }
        }
        for (source_name, target_name) in &writeback.aliases {
            if let Some(value) = target.get(target_name) {
                self.refresh_capture_slot(source_name, value);
            }
        }
    }

    fn refresh_capture_slot(&mut self, name: &str, value: &Value) {
        if let Some(slot) = self.bytecode.local_slot(name)
            && let Some(local) = self.locals.get_mut(slot)
        {
            *local = Some(value.clone());
        }
        if self.env.locals().contains_key(name) {
            self.env.insert(name.to_owned(), value.clone());
        }
    }

    /// Writes this activation's current captured binding values back into the
    /// closure environment it was called from. Ordinary functions do this at
    /// return in `function::call`; async functions complete later through the
    /// generator driver, so the write-back has to travel with the resumable
    /// state.
    fn write_back_function_captures(&self, writeback: Option<&CaptureWriteback>) {
        let Some(writeback) = writeback else {
            return;
        };
        self.write_back_one_function_capture(writeback);
        self.write_back_function_captures(writeback.parent.as_deref());
    }

    fn write_back_one_function_capture(&self, writeback: &CaptureWriteback) {
        let realm_global = writeback
            .target
            .borrow()
            .get(DYNAMIC_FUNCTION_REALM_GLOBAL)
            .and_then(|value| match value {
                Value::Object(object) => Some(object.clone()),
                _ => None,
            });
        let mut target = writeback.target.borrow_mut();
        for name in &writeback.names {
            if !self.bytecode.writes_binding(name) {
                continue;
            }
            if crate::function::is_internal_binding_name(name)
                || matches!(
                    name.as_str(),
                    crate::GLOBAL_THIS_BINDING | "this" | "arguments"
                )
            {
                continue;
            }
            if let Some(value) = self.binding_value(name) {
                target.insert(name.clone(), value.clone());
                if let Some(global) = &realm_global
                    && global.has_own_property(name)
                {
                    global.define_property(name.clone(), crate::Property::enumerable(value));
                }
            }
        }
    }

    /// Reads a binding by name from the body's current locals (preferred) or
    /// frame environment.
    fn binding_value(&self, name: &str) -> Option<Value> {
        if let Some(index) = self.bytecode.local_slot(name) {
            match self.locals.get(index) {
                Some(Some(value)) => return Some(value.clone()),
                Some(None) if !self.bytecode.local_is_body_hoist_only(index) => return None,
                _ => {}
            }
        }
        self.env.get(name)
    }

    /// Captures the running generator body's state at a `yield`.
    fn into_snapshot(
        self,
        bytecode: Rc<Bytecode>,
        suspension: SuspensionKind,
        refresh_captured_slots_on_resume: bool,
        capture_writeback: Option<CaptureWriteback>,
    ) -> GeneratorSnapshot {
        GeneratorSnapshot {
            bytecode,
            ip: self.ip,
            stack: self.stack,
            locals: self.locals,
            local_upvalues: self.local_upvalues,
            upvalues: self.upvalues,
            env: self.env,
            captured_env: self.captured_env,
            with_stack: self.with_stack,
            refresh_captured_slots_on_resume,
            capture_writeback,
            sloppy_global_names: self.sloppy_global_names,
            try_stack: self.try_stack,
            disposable_scopes: self.disposable_scopes,
            pending_throw: self.pending_throw,
            pending_return: self.pending_return,
            pending_jump: self.pending_jump,
            suspension,
        }
    }
}

/// Runs a generator's parameter prologue synchronously, returning a snapshot
/// suspended at the start of the body. Mirrors `FunctionDeclarationInstantiation`
/// running at the call: a parameter-binding error (a destructuring failure or a
/// throwing default initializer) propagates here, before the generator object is
/// observable, instead of on the first `next`.
pub(crate) fn start_suspended_at_body(
    start: GeneratorStart,
    caller_env: &mut CallEnv,
) -> Result<GeneratorState, RuntimeError> {
    let GeneratorStart {
        bytecode,
        env,
        captured_env,
        upvalues,
        with_stack,
        refresh_captured_slots_on_resume,
        capture_writeback,
    } = start;
    let mut vm = Vm::new_with_globals_captures_upvalues_and_with_stack(
        &bytecode,
        env,
        captured_env,
        upvalues,
        with_stack,
    );
    vm.capture_writeback = capture_writeback.clone();
    vm.stop_at_prologue = true;
    vm.refresh_from_caller(caller_env);
    let result = vm.run_completion();
    vm.propagate_to_caller(caller_env);
    vm.write_back_function_captures(capture_writeback.as_ref());
    refresh_activation_captures_from_realm(&mut vm);
    match result {
        // Suspended exactly at the prologue boundary: capture the body-start
        // state for the first resume.
        Ok(Completion::PrologueEnd) => {
            Ok(GeneratorState::SuspendedYield(Box::new(vm.into_snapshot(
                bytecode.clone(),
                SuspensionKind::Ordinary,
                refresh_captured_slots_on_resume,
                capture_writeback,
            ))))
        }
        // A function with no executable prologue suspension (should not happen,
        // since every compiled function emits the marker) — treat a clean return
        // as an empty body that has already finished is wrong here, so surface a
        // structured error rather than silently mis-driving the generator.
        Ok(_) => Err(RuntimeError {
            thrown: None,
            message: "generator prologue did not reach the body boundary".to_owned(),
        }),
        Err(error) => Err(error),
    }
}

fn refresh_activation_captures_from_realm(vm: &mut Vm<'_>) {
    let names: Vec<String> = vm.captured_env.borrow().keys().cloned().collect();
    for name in names {
        if crate::function::is_internal_binding_name(&name)
            || matches!(name.as_str(), "this" | "arguments")
            || vm.env.is_immutable_function_name(&name)
        {
            continue;
        }
        if vm
            .bytecode
            .local_slot(&name)
            .is_some_and(|slot| vm.bytecode.local_is_parameter(slot))
        {
            continue;
        }
        if vm.env.locals().contains_key(&format!(
            "{}{}",
            crate::DIRECT_EVAL_PARAMETER_VAR_BINDING_PREFIX,
            name
        )) {
            continue;
        }
        let Some(value) = vm.realm.borrow().get(&name).cloned() else {
            continue;
        };
        vm.captured_env
            .borrow_mut()
            .insert(name.clone(), value.clone());
        if let Some(slot) = vm.bytecode.local_slot(&name)
            && let Some(local) = vm.locals.get_mut(slot)
        {
            *local = Some(value.clone());
        }
        if vm.env.locals().contains_key(&name) {
            vm.env.insert(name, value);
        }
    }
}

/// Drives a generator from `SuspendedStart`: builds the body VM and runs it.
fn run_from_start(
    start: GeneratorStart,
    caller_env: &mut CallEnv,
) -> Result<(GeneratorState, GeneratorOutcome), RuntimeError> {
    let GeneratorStart {
        bytecode,
        env,
        captured_env,
        upvalues,
        with_stack,
        refresh_captured_slots_on_resume,
        capture_writeback,
    } = start;
    let mut vm = Vm::new_with_globals_captures_upvalues_and_with_stack(
        &bytecode,
        env,
        captured_env,
        upvalues,
        with_stack,
    );
    vm.capture_writeback = capture_writeback.clone();
    vm.refresh_from_caller(caller_env);
    let result = vm.run_completion();
    drive(
        result,
        vm,
        &bytecode,
        caller_env,
        refresh_captured_slots_on_resume,
        capture_writeback,
    )
}

/// Resumes a generator suspended at a `yield`, delivering `resume`.
fn run_from_yield(
    snapshot: GeneratorSnapshot,
    resume: Resume,
    caller_env: &mut CallEnv,
) -> Result<(GeneratorState, GeneratorOutcome), RuntimeError> {
    let bytecode = snapshot.bytecode.clone();
    let mut vm = Vm::new_with_globals_captures_upvalues_and_with_stack(
        &bytecode,
        snapshot.env,
        snapshot.captured_env,
        snapshot.upvalues,
        snapshot.with_stack,
    );
    vm.capture_writeback = snapshot.capture_writeback.clone();
    vm.ip = snapshot.ip;
    vm.stack = snapshot.stack;
    vm.locals = snapshot.locals;
    vm.local_upvalues = snapshot.local_upvalues;
    vm.sloppy_global_names = snapshot.sloppy_global_names;
    vm.pending_throw = snapshot.pending_throw;
    vm.pending_return = snapshot.pending_return;
    vm.pending_jump = snapshot.pending_jump;
    vm.try_stack = snapshot.try_stack;
    vm.disposable_scopes = snapshot.disposable_scopes;
    let capture_writeback = snapshot.capture_writeback;
    if snapshot.refresh_captured_slots_on_resume {
        vm.refresh_from_captured_env();
        vm.refresh_from_capture_writeback(capture_writeback.as_ref());
    }
    vm.refresh_from_caller(caller_env);
    let refresh_captured_slots_on_resume = snapshot.refresh_captured_slots_on_resume;

    // A suspension inside a `yield*` forwards the resume to the inner iterator:
    // the re-entered `Op::YieldDelegate` reads `resume_mode` and decides how to
    // drive (next/return/throw) the inner iterator and whether the outer body
    // continues, suspends again, or completes.
    match snapshot.suspension {
        SuspensionKind::DelegateYield | SuspensionKind::DelegateYieldAsync => {
            vm.resume_mode = Some(match resume {
                Resume::Next(value) => super::vm_result::ResumeMode::Next(value),
                Resume::Return(value)
                    if matches!(snapshot.suspension, SuspensionKind::DelegateYieldAsync) =>
                {
                    let snapshot = vm.into_snapshot(
                        bytecode.clone(),
                        SuspensionKind::DelegateYieldReturnAwait,
                        refresh_captured_slots_on_resume,
                        capture_writeback,
                    );
                    return Ok((
                        GeneratorState::SuspendedYield(Box::new(snapshot)),
                        GeneratorOutcome::Await(value),
                    ));
                }
                Resume::Return(value) | Resume::ReturnAlreadyAwaited(value) => {
                    super::vm_result::ResumeMode::Return(value)
                }
                Resume::Throw(value) => super::vm_result::ResumeMode::Throw(value),
            });
            let result = vm.run_completion();
            return drive(
                result,
                vm,
                &bytecode,
                caller_env,
                refresh_captured_slots_on_resume,
                capture_writeback,
            );
        }
        SuspensionKind::DelegateAwait => {
            vm.resume_mode = Some(match resume {
                Resume::Next(value) => super::vm_result::ResumeMode::Awaited(value),
                Resume::Throw(value) => super::vm_result::ResumeMode::AwaitRejected(value),
                Resume::Return(value) | Resume::ReturnAlreadyAwaited(value) => {
                    super::vm_result::ResumeMode::Return(value)
                }
            });
            let result = vm.run_completion();
            return drive(
                result,
                vm,
                &bytecode,
                caller_env,
                refresh_captured_slots_on_resume,
                capture_writeback,
            );
        }
        SuspensionKind::DelegateYieldReturnAwait => {
            vm.resume_mode = Some(match resume {
                Resume::Next(value) => super::vm_result::ResumeMode::Return(value),
                Resume::Throw(value) => {
                    super::vm_result::ResumeMode::AwaitReturnValueRejected(value)
                }
                Resume::Return(value) | Resume::ReturnAlreadyAwaited(value) => {
                    super::vm_result::ResumeMode::Return(value)
                }
            });
            let result = vm.run_completion();
            return drive(
                result,
                vm,
                &bytecode,
                caller_env,
                refresh_captured_slots_on_resume,
                capture_writeback,
            );
        }
        SuspensionKind::DelegateAwaitReturn => {
            vm.resume_mode = Some(match resume {
                Resume::Next(value) => super::vm_result::ResumeMode::AwaitedReturn(value),
                Resume::Throw(value) => super::vm_result::ResumeMode::AwaitReturnRejected(value),
                Resume::Return(value) | Resume::ReturnAlreadyAwaited(value) => {
                    super::vm_result::ResumeMode::Return(value)
                }
            });
            let result = vm.run_completion();
            return drive(
                result,
                vm,
                &bytecode,
                caller_env,
                refresh_captured_slots_on_resume,
                capture_writeback,
            );
        }
        SuspensionKind::DelegateAwaitReturnValue => {
            vm.resume_mode = Some(match resume {
                Resume::Next(value) => super::vm_result::ResumeMode::AwaitedReturnValue(value),
                Resume::Throw(value) => {
                    super::vm_result::ResumeMode::AwaitReturnValueRejected(value)
                }
                Resume::Return(value) | Resume::ReturnAlreadyAwaited(value) => {
                    super::vm_result::ResumeMode::Return(value)
                }
            });
            let result = vm.run_completion();
            return drive_with_return_already_awaited(
                result,
                vm,
                &bytecode,
                caller_env,
                refresh_captured_slots_on_resume,
                capture_writeback,
            );
        }
        SuspensionKind::Ordinary => {}
    }

    let return_already_awaited = matches!(resume, Resume::ReturnAlreadyAwaited(_));
    let started = match resume {
        // The yield expression evaluates to the resume value.
        Resume::Next(value) => {
            vm.stack.push(value);
            Ok(())
        }
        // A `throw(v)` raises `v` at the yield point so the body's catch/finally
        // can handle it; an unwound throw is the generator's completion.
        Resume::Throw(value) => vm.throw_value(value),
        // A `return(v)` injects a return completion that runs enclosing finally
        // blocks; with no finally it completes the generator immediately.
        Resume::Return(value) | Resume::ReturnAlreadyAwaited(value) => match vm.return_value(value)
        {
            Ok(Some(returned)) => {
                vm.propagate_to_caller(caller_env);
                vm.write_back_function_captures(capture_writeback.as_ref());
                let outcome = if return_already_awaited {
                    GeneratorOutcome::ReturnAlreadyAwaited(returned)
                } else {
                    GeneratorOutcome::Return(returned)
                };
                return Ok((GeneratorState::Completed, outcome));
            }
            Ok(None) => Ok(()),
            Err(error) => Err(error),
        },
    };
    if let Err(error) = started {
        // The injected throw/return had no handler: the generator is done.
        vm.propagate_to_caller(caller_env);
        vm.write_back_function_captures(capture_writeback.as_ref());
        return Err(error);
    }
    let result = vm.run_completion();
    if return_already_awaited {
        drive_with_return_already_awaited(
            result,
            vm,
            &bytecode,
            caller_env,
            refresh_captured_slots_on_resume,
            capture_writeback,
        )
    } else {
        drive(
            result,
            vm,
            &bytecode,
            caller_env,
            refresh_captured_slots_on_resume,
            capture_writeback,
        )
    }
}

/// Maps a body run result to a generator state transition and outcome,
/// capturing a fresh snapshot on `yield`. Before returning, the body's writes
/// to bindings it shares with the resuming caller propagate back, mirroring the
/// caller-binding write-back ordinary function calls perform.
fn drive(
    result: Result<Completion, RuntimeError>,
    vm: Vm<'_>,
    bytecode: &Rc<Bytecode>,
    caller_env: &mut CallEnv,
    refresh_captured_slots_on_resume: bool,
    capture_writeback: Option<CaptureWriteback>,
) -> Result<(GeneratorState, GeneratorOutcome), RuntimeError> {
    vm.propagate_to_caller(caller_env);
    vm.write_back_function_captures(capture_writeback.as_ref());
    match result {
        Ok(Completion::Yield(value)) => {
            let snapshot = vm.into_snapshot(
                bytecode.clone(),
                SuspensionKind::Ordinary,
                refresh_captured_slots_on_resume,
                capture_writeback,
            );
            Ok((
                GeneratorState::SuspendedYield(Box::new(snapshot)),
                GeneratorOutcome::Yield(value),
            ))
        }
        Ok(Completion::Await(value)) => {
            // An `await` suspends like a non-delegating yield: the resume
            // delivers the fulfillment value (or injects a throw) at the await
            // site. Only the outcome tag differs, so the driver routes the
            // suspension to a promise reaction instead of to a consumer.
            let snapshot = vm.into_snapshot(
                bytecode.clone(),
                SuspensionKind::Ordinary,
                refresh_captured_slots_on_resume,
                capture_writeback,
            );
            Ok((
                GeneratorState::SuspendedYield(Box::new(snapshot)),
                GeneratorOutcome::Await(value),
            ))
        }
        Ok(Completion::YieldDelegate(value)) => {
            // Suspended inside a `yield*`: the yielded value is the inner
            // iterator's result object, returned to the outer caller unwrapped.
            let snapshot = vm.into_snapshot(
                bytecode.clone(),
                SuspensionKind::DelegateYield,
                refresh_captured_slots_on_resume,
                capture_writeback,
            );
            Ok((
                GeneratorState::SuspendedYield(Box::new(snapshot)),
                GeneratorOutcome::YieldDelegate(value),
            ))
        }
        Ok(Completion::YieldDelegateAsync(value)) => {
            let snapshot = vm.into_snapshot(
                bytecode.clone(),
                SuspensionKind::DelegateYieldAsync,
                refresh_captured_slots_on_resume,
                capture_writeback,
            );
            Ok((
                GeneratorState::SuspendedYield(Box::new(snapshot)),
                GeneratorOutcome::YieldDelegate(value),
            ))
        }
        Ok(Completion::YieldDelegateAwait(value)) => {
            let snapshot = vm.into_snapshot(
                bytecode.clone(),
                SuspensionKind::DelegateAwait,
                refresh_captured_slots_on_resume,
                capture_writeback,
            );
            Ok((
                GeneratorState::SuspendedYield(Box::new(snapshot)),
                GeneratorOutcome::Await(value),
            ))
        }
        Ok(Completion::YieldDelegateAwaitReturn(value)) => {
            let snapshot = vm.into_snapshot(
                bytecode.clone(),
                SuspensionKind::DelegateAwaitReturn,
                refresh_captured_slots_on_resume,
                capture_writeback,
            );
            Ok((
                GeneratorState::SuspendedYield(Box::new(snapshot)),
                GeneratorOutcome::Await(value),
            ))
        }
        Ok(Completion::YieldDelegateAwaitReturnValue(value)) => {
            let snapshot = vm.into_snapshot(
                bytecode.clone(),
                SuspensionKind::DelegateAwaitReturnValue,
                refresh_captured_slots_on_resume,
                capture_writeback,
            );
            Ok((
                GeneratorState::SuspendedYield(Box::new(snapshot)),
                GeneratorOutcome::Await(value),
            ))
        }
        Ok(Completion::Return(value)) => {
            Ok((GeneratorState::Completed, GeneratorOutcome::Return(value)))
        }
        // The prologue boundary only suspends a freshly created generator (via
        // `start_suspended_at_body`, which never routes through `drive`), so a
        // running body never observes it here.
        Ok(Completion::PrologueEnd) => Err(RuntimeError {
            thrown: None,
            message: "unexpected prologue boundary in a running generator body".to_owned(),
        }),
        Err(error) => Err(error),
    }
}

fn drive_with_return_already_awaited(
    result: Result<Completion, RuntimeError>,
    vm: Vm<'_>,
    bytecode: &Rc<Bytecode>,
    caller_env: &mut CallEnv,
    refresh_captured_slots_on_resume: bool,
    capture_writeback: Option<CaptureWriteback>,
) -> Result<(GeneratorState, GeneratorOutcome), RuntimeError> {
    match result {
        Ok(Completion::Return(value)) => {
            vm.propagate_to_caller(caller_env);
            vm.write_back_function_captures(capture_writeback.as_ref());
            Ok((
                GeneratorState::Completed,
                GeneratorOutcome::ReturnAlreadyAwaited(value),
            ))
        }
        other => drive(
            other,
            vm,
            bytecode,
            caller_env,
            refresh_captured_slots_on_resume,
            capture_writeback,
        ),
    }
}

/// Resumes the generator backing `generator`, applying `resume` and returning
/// the iterator-result outcome. Enforces the `Executing` re-entrancy guard and
/// transitions the stored state on every path.
pub(crate) fn resume_generator(
    generator: &ObjectRef,
    resume: Resume,
    caller_env: &mut CallEnv,
) -> Result<GeneratorOutcome, RuntimeError> {
    // Take the state out behind the re-entrancy guard: a nested `next` while the
    // body runs observes `Executing` and is rejected, and we never hold a borrow
    // of the suspended VM across the body run.
    let state = {
        let mut slot = generator.generator_state().borrow_mut();
        match slot.as_ref() {
            None => {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: not a generator object".to_owned(),
                });
            }
            Some(GeneratorState::Executing) => {
                return Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: generator is already running".to_owned(),
                });
            }
            Some(_) => {}
        }
        slot.replace(GeneratorState::Executing)
            .expect("state present")
    };

    match state {
        GeneratorState::Executing => unreachable!("guarded above"),
        GeneratorState::Completed => {
            *generator.generator_state().borrow_mut() = Some(GeneratorState::Completed);
            completed_outcome(resume)
        }
        GeneratorState::SuspendedStart(start) => {
            // The first `next`'s argument is ignored. A `return(v)` before start
            // completes the generator without running the body; a `throw(v)`
            // before start completes it and rethrows.
            match resume {
                Resume::Next(_) => finish(generator, run_from_start(*start, caller_env)),
                Resume::Return(value) => {
                    *generator.generator_state().borrow_mut() = Some(GeneratorState::Completed);
                    Ok(GeneratorOutcome::Return(value))
                }
                Resume::ReturnAlreadyAwaited(value) => {
                    *generator.generator_state().borrow_mut() = Some(GeneratorState::Completed);
                    Ok(GeneratorOutcome::ReturnAlreadyAwaited(value))
                }
                Resume::Throw(value) => {
                    *generator.generator_state().borrow_mut() = Some(GeneratorState::Completed);
                    Err(throw_completion(value))
                }
            }
        }
        GeneratorState::SuspendedYield(snapshot) => {
            finish(generator, run_from_yield(*snapshot, resume, caller_env))
        }
    }
}

/// Stores the post-run state on the generator and returns its outcome, marking
/// the generator `Completed` when the body run errors.
fn finish(
    generator: &ObjectRef,
    result: Result<(GeneratorState, GeneratorOutcome), RuntimeError>,
) -> Result<GeneratorOutcome, RuntimeError> {
    match result {
        Ok((state, outcome)) => {
            *generator.generator_state().borrow_mut() = Some(state);
            Ok(outcome)
        }
        Err(error) => {
            *generator.generator_state().borrow_mut() = Some(GeneratorState::Completed);
            Err(error)
        }
    }
}

/// The outcome of resuming an already-completed generator: `next`/`return`
/// produce `{ value, done: true }`, while `throw` rethrows.
fn completed_outcome(resume: Resume) -> Result<GeneratorOutcome, RuntimeError> {
    match resume {
        Resume::Next(_) => Ok(GeneratorOutcome::Return(Value::Undefined)),
        Resume::Return(value) => Ok(GeneratorOutcome::Return(value)),
        Resume::ReturnAlreadyAwaited(value) => Ok(GeneratorOutcome::ReturnAlreadyAwaited(value)),
        Resume::Throw(value) => Err(throw_completion(value)),
    }
}

/// Builds the runtime error that carries a thrown JavaScript value so an
/// enclosing `try` (or the host) observes it as an exception.
pub(crate) fn throw_completion(value: Value) -> RuntimeError {
    RuntimeError {
        thrown: Some(Box::new(value.clone())),
        message: format!("throw statement executed: {}", crate::error_value(value)),
    }
}
