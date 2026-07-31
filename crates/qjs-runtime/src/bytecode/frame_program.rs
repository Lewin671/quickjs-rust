//! What code and accelerators a frame runs, and who owns them.
//!
//! A frame used to borrow its bytecode, its selected instruction stream, and
//! four compiled loop-plan slices from one `&'a Bytecode` that outlived the
//! whole VM. That is why a callee could not run on its caller's VM: the
//! callee's bytecode is owned by a `Function` on the caller's stack, not by
//! anything with the root's lifetime.
//!
//! [`FrameBytecode`] lets a frame own that handle instead. The slices cannot
//! then live in the frame -- they would point into a value the frame also owns
//! -- so they move to [`FrameProgramView`], derived once per interpreter
//! activation from a *stack-local* owner. The view borrows that local rather
//! than the VM, which is what lets an instruction handler mutate the VM while
//! the current instruction stays borrowed.
//!
//! The invariant that keeps this sound is small and worth stating plainly:
//!
//! > While a `FrameProgramView` is alive, the frame it was derived from must
//! > not be replaced. A handler may *request* that the driver enter, leave, or
//! > suspend a frame; the driver drops the view before acting on it.
//!
//! Re-selecting a frame's instruction stream (`refresh_virtual_object_execution`)
//! therefore only updates selection inputs. The next activation derives the
//! stream those inputs imply. A future caller that needs the stream re-selected
//! from *inside* the dispatch loop must return a restart boundary rather than
//! replace code underneath a live view.

use std::ops::Deref;
use std::rc::Rc;

use super::ir::{Bytecode, Op};
use super::vm_loop_dispatch::LoopPlanView;

/// A frame's handle on the bytecode it runs.
///
/// Root scripts and eval bodies are entered through a public API that already
/// owns their bytecode for the duration, so they stay borrowed and pay
/// nothing. An ordinary call owns a shared handle, which is what makes the
/// frame independent of its caller's lifetime.
pub(super) enum FrameBytecode<'root> {
    Borrowed(&'root Bytecode),
    Shared(Rc<Bytecode>),
}

impl Clone for FrameBytecode<'_> {
    fn clone(&self) -> Self {
        match self {
            Self::Borrowed(bytecode) => Self::Borrowed(bytecode),
            Self::Shared(bytecode) => Self::Shared(Rc::clone(bytecode)),
        }
    }
}

impl Deref for FrameBytecode<'_> {
    type Target = Bytecode;

    fn deref(&self) -> &Self::Target {
        match self {
            Self::Borrowed(bytecode) => bytecode,
            Self::Shared(bytecode) => bytecode,
        }
    }
}

impl<'root> From<&'root Bytecode> for FrameBytecode<'root> {
    fn from(bytecode: &'root Bytecode) -> Self {
        Self::Borrowed(bytecode)
    }
}

impl From<Rc<Bytecode>> for FrameBytecode<'_> {
    fn from(bytecode: Rc<Bytecode>) -> Self {
        Self::Shared(bytecode)
    }
}

/// The instruction stream one interpreter activation runs.
///
/// Deliberately only the stream. Deriving it is cheap -- every compiled
/// artifact is behind a `OnceCell` the first activation fills -- but it is not
/// done per instruction, because the selection inputs cannot change during an
/// activation and re-deriving would cost a probe per dispatch.
///
/// The loop accelerators are *not* here. Four of the six pointers a combined
/// view would carry are theirs, and a call-heavy workload reaches a backward
/// edge in a small minority of frames: 12,700,004 frames against 100,000 edges
/// on the recursion sentinel. Holding them across the dispatch loop would make
/// every frame pay register pressure for something almost none of them use, so
/// they are derived where they are needed instead.
pub(super) struct FrameProgramView<'a> {
    pub(super) bytecode: &'a Bytecode,
    pub(super) execution_code: &'a [Op],
}

impl<'a> FrameProgramView<'a> {
    pub(super) fn new(
        bytecode: &'a Bytecode,
        authoritative_slots: u128,
        allow_virtual_functions: bool,
    ) -> Self {
        let virtual_object_program = bytecode
            .virtual_object_program
            .get_or_init(|| super::virtual_object::lower(bytecode));
        let execution_code = virtual_object_program.code_for_frame(
            &bytecode.code,
            authoritative_slots,
            allow_virtual_functions,
        );
        Self {
            bytecode,
            execution_code,
        }
    }

    /// The loop accelerators for this body, derived at the backward edge that
    /// needs them.
    pub(super) fn loop_plans(&self) -> LoopPlanView<'a> {
        let bytecode = self.bytecode;
        LoopPlanView {
            control: bytecode
                .control_loop_plans
                .get_or_init(|| super::vm_control_loop::ControlLoopPlan::compile_all(bytecode)),
            numeric: bytecode
                .numeric_loop_plans
                .get_or_init(|| super::vm_numeric_loop::NumericLoopPlan::compile_all(bytecode)),
            typed: bytecode
                .typed_loop_programs
                .get_or_init(|| super::typed_loop::compile_all(bytecode)),
            shared_numeric_mutation: bytecode.numeric_mutation_loop_plans.get_or_init(|| {
                super::vm_numeric_mutation_loop::NumericMutationLoopPlan::compile_all(bytecode)
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bytecode::ir::Bytecode;
    use std::rc::Weak;

    fn empty_bytecode() -> Bytecode {
        Bytecode::new(Vec::new(), Vec::new(), Vec::new())
    }

    #[test]
    fn a_shared_frame_handle_keeps_its_bytecode_alive_on_its_own() {
        // The whole point of the change: a frame's bytecode must outlive every
        // other strong reference, because a callee's bytecode is owned by a
        // `Function` the caller may drop while the callee is still running.
        let observer: Weak<Bytecode>;
        let frame_handle = {
            let owner = Rc::new(empty_bytecode());
            observer = Rc::downgrade(&owner);
            FrameBytecode::Shared(owner)
        };
        assert!(
            observer.upgrade().is_some(),
            "the frame handle must be the surviving owner"
        );
        assert_eq!(frame_handle.code.len(), 0);
        drop(frame_handle);
        assert!(
            observer.upgrade().is_none(),
            "dropping the frame handle must release the bytecode"
        );
    }

    #[test]
    fn a_borrowed_and_a_shared_handle_derive_the_same_program() {
        // Root scripts stay borrowed and ordinary calls are shared; both must
        // select the same instruction stream and plan set for the same body.
        let owner = Rc::new(empty_bytecode());
        let borrowed = FrameBytecode::Borrowed(&owner);
        let shared = FrameBytecode::Shared(Rc::clone(&owner));

        let from_borrowed = FrameProgramView::new(&borrowed, 0, true);
        let from_shared = FrameProgramView::new(&shared, 0, true);

        assert!(std::ptr::eq(from_borrowed.bytecode, from_shared.bytecode));
        assert!(std::ptr::eq(
            from_borrowed.execution_code,
            from_shared.execution_code
        ));
        assert!(std::ptr::eq(
            from_borrowed.loop_plans().numeric,
            from_shared.loop_plans().numeric
        ));
    }

    #[test]
    fn cloning_a_handle_shares_rather_than_copies_the_body() {
        let owner = Rc::new(empty_bytecode());
        let handle = FrameBytecode::Shared(Rc::clone(&owner));
        let clone = handle.clone();
        assert_eq!(Rc::strong_count(&owner), 3);
        assert!(std::ptr::eq(&*handle, &*clone));
    }
}
