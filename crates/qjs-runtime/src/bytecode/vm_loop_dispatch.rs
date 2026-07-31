//! Loop-plan dispatch at ordinary backward edges.
//!
//! Four independent accelerators can claim a loop region: the numeric mutation
//! loop, the numeric loop, the control loop, and the typed loop. Each is free
//! to decline at run time -- a higher-priority plan that matched an
//! instruction range may still refuse once the loop's values are known -- so
//! the edge consults them in priority order and falls through to a plain jump
//! when none applies.
//!
//! Keeping that decision in its own module separates "which accelerator runs
//! this region" from the interpreter's opcode dispatch, and gives the probe
//! chain one place to be measured and, later, narrowed.

use super::typed_loop::TypedLoopProgram;
use super::vm::Vm;
use super::vm_control_loop::ControlLoopPlan;
use super::vm_numeric_loop::NumericLoopPlan;
use super::vm_numeric_mutation_loop::NumericMutationLoopPlan;

/// The compiled loop accelerators available at one backward edge.
///
/// The engines used to read these straight off `FrameState`, which is why the
/// frame had to borrow them from its bytecode for its whole lifetime. Passing
/// them in instead means the slices can come from wherever the caller can
/// prove they live -- today the frame, and next from a stack-local bytecode
/// owner the frame owns rather than borrows.
#[derive(Clone, Copy)]
pub(super) struct LoopPlanView<'a> {
    pub(super) control: &'a [ControlLoopPlan],
    pub(super) numeric: &'a [NumericLoopPlan],
    pub(super) typed: &'a [TypedLoopProgram],
    /// The body's shared plans. A frame that has diverged holds its own
    /// override in `FrameState`, which still wins over this.
    pub(super) shared_numeric_mutation: &'a [NumericMutationLoopPlan],
}

impl Vm<'_> {
    /// Frame-local numeric mutation loop plans, materialized from the shared
    /// bytecode plans on first deoptimization. Suppressing or rewriting a plan
    /// must not affect other invocations of the same function, so the frame
    /// takes its own copy exactly when it first needs to diverge.
    pub(super) fn frame_numeric_mutation_loop_plans(
        &mut self,
        shared: &[super::vm_numeric_mutation_loop::NumericMutationLoopPlan],
    ) -> &mut Vec<super::vm_numeric_mutation_loop::NumericMutationLoopPlan> {
        self.current
            .numeric_mutation_loop_plans
            .get_or_insert_with(|| shared.to_vec())
    }

    /// Performs one bytecode jump while preserving the counted-loop
    /// accelerators attached to ordinary backward edges.
    pub(super) fn jump_with_loop_plans(
        &mut self,
        plans: LoopPlanView<'_>,
        target: usize,
        backedge: usize,
    ) {
        if target >= backedge {
            self.ip = target;
            return;
        }
        crate::diagnostics::count!(loop_backedges);
        let entered =
            super::vm_numeric_mutation_loop::try_run_numeric_mutation_loop(
                self, plans, target, backedge,
            ) || super::vm_numeric_loop::try_run_numeric_loop(self, plans, target, backedge)
                || super::vm_control_loop::try_run_control_loop(self, plans, target, backedge)
                || super::typed_loop::try_run_typed_loop(self, plans, target, backedge);
        if entered {
            crate::diagnostics::count!(loop_plan_entries);
            return;
        }
        // Reaching here means all four engines were consulted and all four
        // declined, so the whole probe chain was overhead on this edge.
        crate::diagnostics::count!(declined_loop_plan_edges);
        self.ip = target;
    }
}
