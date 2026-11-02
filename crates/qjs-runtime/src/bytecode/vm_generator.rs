//! Generator object runtime: the suspend/resume state machine backing
//! `function*` bodies and the `next`/`return`/`throw` protocol methods.
//!
//! Suspendable bodies allocate their full FrameState before the prologue.
//! A suspension retains that same Box; resume only reattaches runtime services
//! and resets per-segment decisions. Cells, handlers and pending completions
//! stay in the owning record. Ordinary VM frames remain unboxed values.

use std::rc::Rc;

use crate::{ObjectRef, RuntimeError, Value};

use super::ir::Bytecode;
use super::vm::Vm;
use super::vm_result::Completion;

/// The lifecycle state of a generator object (ES2023 27.5.3 [[GeneratorState]]).
pub(crate) enum GeneratorState {
    /// Body not started: first `next` runs from the retained initial/prologue PC.
    SuspendedStart(SuspendedActivation),
    /// Suspended at a `yield`: the next resume re-enters the saved VM.
    SuspendedYield(SuspendedActivation),
    /// Currently running: re-entrancy is a TypeError.
    Executing,
    /// Finished (return or uncaught throw): further `next` returns
    /// `{ value: undefined, done: true }`.
    Completed,
}

/// A suspended owning record, including semantic state and its resume kind.
pub(crate) struct SuspendedActivation {
    frame: Box<super::vm::FrameState<'static>>,
    suspension: SuspensionKind,
}

#[derive(Clone, Copy)]
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
        Some(GeneratorState::SuspendedYield(record))
            if matches!(record.suspension, SuspensionKind::Ordinary)
    )
}

impl Vm<'static, '_, Box<super::vm::FrameState<'static>>> {
    /// Detach the exact owning record; no semantic state is reconstructed.
    fn into_record(self, suspension: SuspensionKind) -> SuspendedActivation {
        SuspendedActivation {
            frame: self.into_frame(),
            suspension,
        }
    }
}

impl SuspendedActivation {
    /// Attach services for one segment, consuming the unique frame owner.
    /// The code handle and every semantic field already belong to that frame.
    fn attach<'s>(
        self,
        runtime: &'s mut crate::context::RuntimeServices,
    ) -> Vm<'static, 's, Box<super::vm::FrameState<'static>>> {
        #[cfg(debug_assertions)]
        self.frame
            .stack
            .pool()
            .set_poison(runtime.binding_debug.poisoning());
        let mut vm = Vm {
            runtime,
            current: self.frame,
            callers: Vec::new(),
            pending_frame_entry: None,
            frame_lifetime: std::marker::PhantomData,
        };
        vm.reset_segment();
        vm.stop_at_prologue = false;
        vm.refresh_authoritative_slots();
        vm.verify_binding_authority();
        vm
    }
}

/// Prepare all owned call state before publishing a suspendable activation.
/// Argument slices are consumed into slots/owned recipes here; parameter
/// initializers still execute at the driver's existing prologue boundary.
pub(crate) fn prepare_suspended_start(
    bytecode: Rc<Bytecode>,
    activation: crate::context::ActivationContext,
    upvalues: Vec<crate::function::BindingCell>,
    with_stack: Vec<Value>,
    inputs: Option<crate::function::CallInputs<'_>>,
    runtime: &mut crate::context::RuntimeServices,
) -> SuspendedActivation {
    Vm::with_frame_bytecode(
        super::frame_program::FrameBytecode::Shared(bytecode),
        runtime,
        activation,
        upvalues,
        with_stack,
        None,
        inputs,
    )
    .into_suspendable()
    .into_record(SuspensionKind::Ordinary)
}

/// Runs a generator's parameter prologue synchronously, returning a record
/// suspended at the start of the body. Mirrors `FunctionDeclarationInstantiation`
/// running at the call: a parameter-binding error (a destructuring failure or a
/// throwing default initializer) propagates here, before the generator object is
/// observable, instead of on the first `next`.
pub(crate) fn start_suspended_at_body(
    start: SuspendedActivation,
    runtime: &mut crate::context::RuntimeServices,
) -> Result<GeneratorState, RuntimeError> {
    let mut vm = start.attach(runtime);
    vm.stop_at_prologue = true;
    let result = vm.run_completion();
    match result {
        // Suspended exactly at the prologue boundary: capture the body-start
        // state for the first resume.
        Ok(Completion::PrologueEnd) => Ok(GeneratorState::SuspendedStart(
            vm.into_record(SuspensionKind::Ordinary),
        )),
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

/// Drives the prepared owner from its initial PC or completed prologue.
fn run_from_start(
    start: SuspendedActivation,
    runtime: &mut crate::context::RuntimeServices,
) -> Result<(GeneratorState, GeneratorOutcome), RuntimeError> {
    let mut vm = start.attach(runtime);
    let result = vm.run_completion();
    drive(result, vm)
}

/// Resumes a generator suspended at a `yield`, delivering `resume`.
fn run_from_yield(
    record: SuspendedActivation,
    resume: Resume,
    runtime: &mut crate::context::RuntimeServices,
) -> Result<(GeneratorState, GeneratorOutcome), RuntimeError> {
    let suspension = record.suspension;
    let mut vm = record.attach(runtime);

    // A suspension inside a `yield*` forwards the resume to the inner iterator:
    // the re-entered `Op::YieldDelegate` reads `resume_mode` and decides how to
    // drive (next/return/throw) the inner iterator and whether the outer body
    // continues, suspends again, or completes.
    match suspension {
        SuspensionKind::DelegateYield | SuspensionKind::DelegateYieldAsync => {
            vm.cold_mut().resume_mode = Some(match resume {
                Resume::Next(value) => super::vm_result::ResumeMode::Next(value),
                Resume::Return(value)
                    if matches!(suspension, SuspensionKind::DelegateYieldAsync) =>
                {
                    let record = vm.into_record(SuspensionKind::DelegateYieldReturnAwait);
                    return Ok((
                        GeneratorState::SuspendedYield(record),
                        GeneratorOutcome::Await(value),
                    ));
                }
                Resume::Return(value) | Resume::ReturnAlreadyAwaited(value) => {
                    super::vm_result::ResumeMode::Return(value)
                }
                Resume::Throw(value) => super::vm_result::ResumeMode::Throw(value),
            });
            let result = vm.run_completion();
            return drive(result, vm);
        }
        SuspensionKind::DelegateAwait => {
            vm.cold_mut().resume_mode = Some(match resume {
                Resume::Next(value) => super::vm_result::ResumeMode::Awaited(value),
                Resume::Throw(value) => super::vm_result::ResumeMode::AwaitRejected(value),
                Resume::Return(value) | Resume::ReturnAlreadyAwaited(value) => {
                    super::vm_result::ResumeMode::Return(value)
                }
            });
            let result = vm.run_completion();
            return drive(result, vm);
        }
        SuspensionKind::DelegateYieldReturnAwait => {
            vm.cold_mut().resume_mode = Some(match resume {
                Resume::Next(value) => super::vm_result::ResumeMode::Return(value),
                Resume::Throw(value) => {
                    super::vm_result::ResumeMode::AwaitReturnValueRejected(value)
                }
                Resume::Return(value) | Resume::ReturnAlreadyAwaited(value) => {
                    super::vm_result::ResumeMode::Return(value)
                }
            });
            let result = vm.run_completion();
            return drive(result, vm);
        }
        SuspensionKind::DelegateAwaitReturn => {
            vm.cold_mut().resume_mode = Some(match resume {
                Resume::Next(value) => super::vm_result::ResumeMode::AwaitedReturn(value),
                Resume::Throw(value) => super::vm_result::ResumeMode::AwaitReturnRejected(value),
                Resume::Return(value) | Resume::ReturnAlreadyAwaited(value) => {
                    super::vm_result::ResumeMode::Return(value)
                }
            });
            let result = vm.run_completion();
            return drive(result, vm);
        }
        SuspensionKind::DelegateAwaitReturnValue => {
            vm.cold_mut().resume_mode = Some(match resume {
                Resume::Next(value) => super::vm_result::ResumeMode::AwaitedReturnValue(value),
                Resume::Throw(value) => {
                    super::vm_result::ResumeMode::AwaitReturnValueRejected(value)
                }
                Resume::Return(value) | Resume::ReturnAlreadyAwaited(value) => {
                    super::vm_result::ResumeMode::Return(value)
                }
            });
            let result = vm.run_completion();
            return drive_with_return_already_awaited(result, vm);
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
    // An injected throw/return with no handler completes the generator.
    started?;
    let result = vm.run_completion();
    if return_already_awaited {
        drive_with_return_already_awaited(result, vm)
    } else {
        drive(result, vm)
    }
}

/// Maps a body run result to a generator state transition and outcome,
/// retaining the same owning record on `yield`. Shared captures already point at the
/// same indexed cells held by their declaring frames, so suspension needs no
/// name-based capture write-back.
fn drive(
    result: Result<Completion, RuntimeError>,
    vm: Vm<'static, '_, Box<super::vm::FrameState<'static>>>,
) -> Result<(GeneratorState, GeneratorOutcome), RuntimeError> {
    match result {
        Ok(Completion::Yield(value)) => {
            let record = vm.into_record(SuspensionKind::Ordinary);
            Ok((
                GeneratorState::SuspendedYield(record),
                GeneratorOutcome::Yield(value),
            ))
        }
        Ok(Completion::Await(value)) => {
            // An `await` suspends like a non-delegating yield: the resume
            // delivers the fulfillment value (or injects a throw) at the await
            // site. Only the outcome tag differs, so the driver routes the
            // suspension to a promise reaction instead of to a consumer.
            let record = vm.into_record(SuspensionKind::Ordinary);
            Ok((
                GeneratorState::SuspendedYield(record),
                GeneratorOutcome::Await(value),
            ))
        }
        Ok(Completion::YieldDelegate(value)) => {
            // Suspended inside a `yield*`: the yielded value is the inner
            // iterator's result object, returned to the outer caller unwrapped.
            let record = vm.into_record(SuspensionKind::DelegateYield);
            Ok((
                GeneratorState::SuspendedYield(record),
                GeneratorOutcome::YieldDelegate(value),
            ))
        }
        Ok(Completion::YieldDelegateAsync(value)) => {
            let record = vm.into_record(SuspensionKind::DelegateYieldAsync);
            Ok((
                GeneratorState::SuspendedYield(record),
                GeneratorOutcome::YieldDelegate(value),
            ))
        }
        Ok(Completion::YieldDelegateAwait(value)) => {
            let record = vm.into_record(SuspensionKind::DelegateAwait);
            Ok((
                GeneratorState::SuspendedYield(record),
                GeneratorOutcome::Await(value),
            ))
        }
        Ok(Completion::YieldDelegateAwaitReturn(value)) => {
            let record = vm.into_record(SuspensionKind::DelegateAwaitReturn);
            Ok((
                GeneratorState::SuspendedYield(record),
                GeneratorOutcome::Await(value),
            ))
        }
        Ok(Completion::YieldDelegateAwaitReturnValue(value)) => {
            let record = vm.into_record(SuspensionKind::DelegateAwaitReturnValue);
            Ok((
                GeneratorState::SuspendedYield(record),
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
    vm: Vm<'static, '_, Box<super::vm::FrameState<'static>>>,
) -> Result<(GeneratorState, GeneratorOutcome), RuntimeError> {
    match result {
        Ok(Completion::Return(value)) => Ok((
            GeneratorState::Completed,
            GeneratorOutcome::ReturnAlreadyAwaited(value),
        )),
        other => drive(other, vm),
    }
}

/// Resumes the generator backing `generator`, applying `resume` and returning
/// the iterator-result outcome. Enforces the `Executing` re-entrancy guard and
/// transitions the stored state on every path.
pub(crate) fn resume_generator(
    generator: &ObjectRef,
    resume: Resume,
    caller_env: &mut crate::context::Context<'_>,
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
                Resume::Next(_) => finish(generator, run_from_start(start, caller_env.runtime)),
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
        GeneratorState::SuspendedYield(record) => finish(
            generator,
            run_from_yield(record, resume, caller_env.runtime),
        ),
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

#[cfg(test)]
mod owner_tests;
