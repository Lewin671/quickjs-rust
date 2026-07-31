//! Driving more than one frame on a single VM.
//!
//! Every ordinary call today constructs a whole nested [`Vm`] on the Rust
//! stack -- 12,700,004 of them for a workload performing 12,700,004 calls --
//! and recurses into it. That is the cost the call-frame migration exists to
//! remove, and it is also why JavaScript recursion past roughly a thousand
//! frames aborts the process with a native stack overflow instead of throwing
//! a catchable `RangeError`.
//!
//! This module installs the driver that makes a second frame possible: the VM
//! keeps its callers on a heap stack, runs one frame at a time, and resumes
//! the caller when the frame above it finishes. **Nothing routes production
//! calls here yet.** The point of landing it alone is that the completion and
//! unwinding protocol can be proved correct before any call depends on it.
//!
//! The load-bearing part is the error path. When a callee fails, the caller
//! must get exactly what it gets today from a nested `Vm` returning `Err`:
//! its own `try`/`catch`/`finally` machinery, reached through
//! `handle_runtime_error`, and propagation outward only when the caller has no
//! handler. Reusing that function rather than re-implementing unwinding is
//! what keeps this a representation change.

use crate::RuntimeError;

use super::vm::{FrameState, Vm};
use super::vm_result::Completion;

/// What a caller does with the completion of the frame above it.
///
/// Only one shape exists while nothing routes: an ordinary call expression
/// leaves its result on the caller's operand stack. Constructors, `super`
/// calls, and completion policies arrive with the stage that needs them.
// Constructed by the stage that routes ordinary calls onto this VM. It exists
// now so the completion protocol can be proved before any call depends on it.
#[allow(dead_code)]
pub(super) enum FrameContinuation {
    PushResult,
}

/// A caller waiting for the frame above it to finish.
pub(super) struct SuspendedFrame<'a> {
    frame: FrameState<'a>,
    continuation: FrameContinuation,
}

impl<'a> Vm<'a> {
    /// Suspends the current frame and installs `frame` above it.
    ///
    /// The caller's program view must already have been dropped: replacing the
    /// current frame while a view derived from it is alive would leave the
    /// dispatch loop reading code that no longer belongs to the frame it is
    /// executing.
    #[allow(dead_code)]
    pub(super) fn push_frame(&mut self, frame: FrameState<'a>, continuation: FrameContinuation) {
        let caller = std::mem::replace(&mut self.current, frame);
        self.callers.push(SuspendedFrame {
            frame: caller,
            continuation,
        });
    }

    /// Runs frames until the bottom one completes.
    ///
    /// With no callers -- which is every execution until calls are routed --
    /// this is exactly one activation, and the loop below runs once.
    pub(super) fn run_completion(&mut self) -> Result<Completion, RuntimeError> {
        loop {
            match self.run_current_activation() {
                Ok(completion) => match self.callers.pop() {
                    None => return Ok(completion),
                    Some(caller) => self.resume_caller(caller, completion)?,
                },
                Err(error) => match self.callers.pop() {
                    None => return Err(error),
                    Some(caller) => self.resume_caller_with_error(caller, error)?,
                },
            }
        }
    }

    /// Restores `caller` and delivers the completion of the frame above it.
    fn resume_caller(
        &mut self,
        caller: SuspendedFrame<'a>,
        completion: Completion,
    ) -> Result<(), RuntimeError> {
        let Completion::Return(value) = completion else {
            // A routed frame cannot suspend: generators and async functions
            // keep their own drivers and stay on the nested-`Vm` fallback.
            // Failing loudly here is deliberate -- silently discarding a
            // suspension would corrupt the caller's stack.
            return Err(RuntimeError {
                thrown: None,
                message: "a suspended frame cannot resume its caller".to_owned(),
            });
        };
        let SuspendedFrame {
            frame,
            continuation,
        } = caller;
        // Dropping the finished frame here returns its operand stack to the
        // body's recycler and releases its bytecode handle.
        self.current = frame;
        match continuation {
            FrameContinuation::PushResult => self.stack.push(value),
        }
        Ok(())
    }

    /// Restores `caller` and raises `error` inside it.
    ///
    /// This is the same path a nested `Vm`'s `Err` takes today: the caller's
    /// `try`/`catch`/`finally` claims it when the caller has a handler, and it
    /// propagates outward when it does not.
    fn resume_caller_with_error(
        &mut self,
        caller: SuspendedFrame<'a>,
        error: RuntimeError,
    ) -> Result<(), RuntimeError> {
        self.current = caller.frame;
        self.raise_in_current_frame(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;
    use crate::bytecode::compile_script;
    use crate::eval;
    use qjs_parser::parse_script;

    /// An ordinary evaluation, which must still take exactly one activation
    /// and be unaffected by the driver until calls are routed.
    fn undriven(source: &str) -> Result<Value, RuntimeError> {
        eval(source)
    }

    #[test]
    fn a_script_with_no_callers_still_runs_exactly_one_activation() {
        // The driver must be invisible until something routes into it.
        assert_eq!(undriven("1 + 2;").unwrap(), Value::Number(3.0));
        assert_eq!(
            undriven("var total = 0; for (var i = 0; i < 5; i++) { total += i; } total;").unwrap(),
            Value::Number(10.0),
        );
    }

    #[test]
    fn a_throw_with_no_caller_still_propagates_out_of_the_driver() {
        let error = undriven("throw new TypeError('boom');").unwrap_err();
        assert!(error.message.contains("boom"), "{}", error.message);
    }

    #[test]
    fn a_returning_frame_hands_its_value_to_the_caller() {
        let outer = parse_script("0;").expect("outer parses");
        let outer = compile_script(&outer).expect("outer compiles");
        let inner = parse_script("41 + 1;").expect("inner parses");
        let inner = compile_script(&inner).expect("inner compiles");

        let mut vm = Vm::new(&outer).expect("root frame");
        let inner_frame = Vm::new(&inner).expect("second frame").into_frame();
        vm.push_frame(inner_frame, FrameContinuation::PushResult);
        assert_eq!(vm.callers.len(), 1);

        let completion = vm.run_completion().expect("driver completes");
        assert!(matches!(completion, Completion::Return(_)));
        assert!(vm.callers.is_empty(), "the caller stack must be drained");
    }

    #[test]
    fn a_throwing_frame_without_a_caller_handler_propagates() {
        // No live `try` at the caller, so the error must leave the driver --
        // the same rule a nested `Vm`'s `Err` follows today.
        let outer = parse_script("0;").expect("outer parses");
        let outer = compile_script(&outer).expect("outer compiles");
        let inner =
            parse_script("throw new TypeError('from the frame above');").expect("inner parses");
        let inner = compile_script(&inner).expect("inner compiles");

        let mut vm = Vm::new(&outer).expect("root frame");
        let inner_frame = Vm::new(&inner).expect("second frame").into_frame();
        vm.push_frame(inner_frame, FrameContinuation::PushResult);

        let error = match vm.run_completion() {
            Err(error) => error,
            Ok(_) => panic!("the throw must propagate"),
        };
        assert!(
            error.message.contains("from the frame above"),
            "{}",
            error.message
        );
        assert!(vm.callers.is_empty(), "the caller stack must be drained");
    }

    #[test]
    fn a_throwing_frame_is_claimed_by_the_callers_live_handler() {
        // This is the property the migration depends on: a callee failing must
        // re-enter its caller's own unwinding machinery rather than escaping
        // to the root. The caller is parked inside a protected region, so the
        // driver must hand the error to that region.
        let outer = parse_script("var seen = 1; seen;").expect("outer parses");
        let outer = compile_script(&outer).expect("outer compiles");
        let inner = parse_script("throw new TypeError('claimed');").expect("inner parses");
        let inner = compile_script(&inner).expect("inner compiles");

        let mut vm = Vm::new(&outer).expect("root frame");
        // Park the caller inside a try whose catch is the end of its own code,
        // which is where an ordinary `try { f(); } catch {}` would leave it.
        let catch_target = outer.code.len() - 1;
        vm.enter_try(Some(catch_target), None, None, Vec::new());
        let inner_frame = Vm::new(&inner).expect("second frame").into_frame();
        vm.push_frame(inner_frame, FrameContinuation::PushResult);

        let completion = match vm.run_completion() {
            Ok(completion) => completion,
            Err(error) => panic!(
                "the caller's handler must claim the throw: {}",
                error.message
            ),
        };
        assert!(matches!(completion, Completion::Return(_)));
        assert!(vm.callers.is_empty(), "the caller stack must be drained");
    }

    #[test]
    fn a_frame_built_from_a_shared_handle_outlives_its_only_other_owner() {
        // The property a routed call depends on: a callee's bytecode is owned
        // by a `Function` on the caller's operand stack, which the caller may
        // release while the callee is still running.
        use crate::bytecode::frame_program::FrameBytecode;
        use std::rc::Rc;

        let outer = parse_script("0;").expect("outer parses");
        let outer = compile_script(&outer).expect("outer compiles");
        let inner = parse_script("41 + 1;").expect("inner parses");
        let inner = Rc::new(compile_script(&inner).expect("inner compiles"));
        let observer = Rc::downgrade(&inner);

        let mut vm = Vm::new(&outer).expect("root frame");
        let env = vm.env.clone();
        let callee = Vm::with_frame_bytecode(
            FrameBytecode::Shared(Rc::clone(&inner)),
            env,
            Vec::new(),
            Vec::new(),
            None,
        )
        .into_frame();
        vm.push_frame(callee, FrameContinuation::PushResult);

        // Every owner outside the frame is gone; the frame must still run.
        drop(inner);
        assert!(observer.upgrade().is_some(), "the frame must be the owner");
        let completion = match vm.run_completion() {
            Ok(completion) => completion,
            Err(error) => panic!("the routed frame must complete: {}", error.message),
        };
        assert!(matches!(completion, Completion::Return(_)));
        assert!(vm.callers.is_empty());
    }

    #[test]
    fn a_suspension_cannot_resume_a_caller() {
        // Generators and async functions keep their own drivers and stay on
        // the nested-`Vm` fallback. If one ever reached here, discarding the
        // suspension would corrupt the caller's stack, so the driver refuses.
        let outer = parse_script("0;").expect("outer parses");
        let outer = compile_script(&outer).expect("outer compiles");
        let inner = parse_script("0;").expect("inner parses");
        let inner = compile_script(&inner).expect("inner compiles");

        let mut vm = Vm::new(&outer).expect("root frame");
        let inner_frame = Vm::new(&inner).expect("second frame").into_frame();
        vm.push_frame(inner_frame, FrameContinuation::PushResult);
        let caller = vm.callers.pop().expect("one caller");
        let error = match vm.resume_caller(caller, Completion::Yield(Value::Undefined)) {
            Err(error) => error,
            Ok(()) => panic!("a suspension must be rejected"),
        };
        assert!(
            error.message.contains("suspended frame"),
            "{}",
            error.message
        );
    }
}
