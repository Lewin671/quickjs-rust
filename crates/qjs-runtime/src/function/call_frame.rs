//! An ordinary bytecode call, prepared but not yet run.
//!
//! [`call_function`](super::call::call_function) currently interleaves three
//! separate jobs: deciding *what kind of callable* this is (proxy, bound,
//! native, generator, async function, constructor, ordinary body), building
//! *the environment* the callee will run in, and *executing* the bytecode.
//! Interleaving them is why a callee cannot yet be entered on the caller's own
//! VM: there is no point at which "everything the callee needs" exists as a
//! value that some other executor could take.
//!
//! [`PreparedBytecodeCall`] is that point. It is deliberately only the
//! ordinary synchronous shape. Generators, async functions, proxies, bound and
//! native callables, and construction all remain dispatch decisions made
//! before preparation, because each of them needs something other than "run
//! this body to completion" and encoding that here would recreate the
//! interleaving this module exists to remove.

use std::rc::Rc;

use crate::bytecode::{Bytecode, DirectCallSlots};
use crate::value::Value;
use crate::{CallEnv, RuntimeError};

use super::upvalue::Upvalue;

/// What must happen after an ordinary bytecode call returns -- and nothing
/// about how it runs.
///
/// Both members are caller-side obligations, which is exactly why they belong
/// with the prepared call rather than in the executor: whoever runs the body
/// must be able to hand back what the caller is owed without knowing why.
pub(crate) struct CallCompletionPolicy {
    /// A named function expression binds its own name for the duration of the
    /// call, shadowing whatever the caller had. This is the caller's value,
    /// captured before the call and restored after it.
    restored_caller_binding: Option<(String, Value)>,
    /// A derived constructor implicitly returns its super-bound `this` when
    /// the body returns no object, and finishing without having called
    /// `super(...)` is a ReferenceError.
    finishes_derived_construct: bool,
}

impl CallCompletionPolicy {
    pub(crate) fn new(
        restored_caller_binding: Option<(String, Value)>,
        finishes_derived_construct: bool,
    ) -> Self {
        Self {
            restored_caller_binding,
            finishes_derived_construct,
        }
    }

    pub(crate) fn finishes_derived_construct(&self) -> bool {
        self.finishes_derived_construct
    }

    /// Returns the caller binding to restore, consuming it so a completion
    /// policy cannot be applied twice.
    pub(crate) fn take_restored_caller_binding(&mut self) -> Option<(String, Value)> {
        self.restored_caller_binding.take()
    }
}

/// Everything an ordinary synchronous bytecode call needs in order to run.
///
/// The borrow in `slots` is the current representation's, not this type's: the
/// slot-seeded path points at the caller's argument slice rather than copying
/// it. Making a frame own its inputs is a separate step; separating *what a
/// call is* from *how it is dispatched and executed* is this one.
pub(crate) struct PreparedBytecodeCall<'a> {
    pub(crate) bytecode: Rc<Bytecode>,
    pub(crate) env: CallEnv,
    /// Present when the callee's parameters and receiver can be seeded
    /// directly into frame slots, absent when it needs the general
    /// name-keyed prologue.
    pub(crate) slots: Option<DirectCallSlots<'a>>,
    /// Only the general path needs these; a slot-seeded call carries its
    /// upvalues inside `slots`.
    pub(crate) upvalues: Vec<Upvalue>,
    pub(crate) with_stack: Vec<Value>,
    pub(crate) completion: CallCompletionPolicy,
}

impl PreparedBytecodeCall<'_> {
    /// True when this call can be seeded straight into frame slots.
    ///
    /// A slot-seeded call is never a derived constructor: both seeding
    /// predicates exclude class constructors, and the base-class one
    /// additionally excludes derived ones. The frame-stack migration relies on
    /// that, because it is what makes the completion policy uniform across
    /// both execution shapes.
    pub(crate) fn is_slot_seeded(&self) -> bool {
        debug_assert!(
            !(self.slots.is_some() && self.completion.finishes_derived_construct()),
            "a slot-seeded call cannot be a derived constructor"
        );
        self.slots.is_some()
    }
}

/// Restores the caller binding an ordinary call shadowed, if any.
pub(crate) fn apply_completion_to_caller(
    completion: &mut CallCompletionPolicy,
    caller_env: &mut CallEnv,
) {
    let Some((name, value)) = completion.take_restored_caller_binding() else {
        return;
    };
    caller_env.set_local(&name, value.clone());
    if caller_env.realm_contains(&name) {
        caller_env.insert_realm(name, value);
    }
}

/// Signals a malformed callable reaching the ordinary bytecode path.
pub(crate) fn missing_bytecode_body() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "user function has no bytecode body".to_owned(),
    }
}
