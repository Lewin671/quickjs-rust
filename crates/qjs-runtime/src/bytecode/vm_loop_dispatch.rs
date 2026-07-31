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

use super::vm::Vm;

impl Vm<'_> {
    /// Frame-local numeric mutation loop plans, materialized from the shared
    /// bytecode plans on first deoptimization. Suppressing or rewriting a plan
    /// must not affect other invocations of the same function, so the frame
    /// takes its own copy exactly when it first needs to diverge.
    pub(super) fn frame_numeric_mutation_loop_plans(
        &mut self,
    ) -> &mut Vec<super::vm_numeric_mutation_loop::NumericMutationLoopPlan> {
        let shared = self.current.shared_numeric_mutation_loop_plans;
        self.current
            .numeric_mutation_loop_plans
            .get_or_insert_with(|| shared.to_vec())
    }

    /// Performs one bytecode jump while preserving the shared counted-loop
    /// accelerators attached to ordinary backward edges.
    pub(super) fn jump_with_loop_plans(&mut self, target: usize, backedge: usize) {
        if target >= backedge {
            self.ip = target;
            return;
        }
        crate::diagnostics::count!(loop_backedges);
        let entered =
            super::vm_numeric_mutation_loop::try_run_numeric_mutation_loop(self, target, backedge)
                || super::vm_numeric_loop::try_run_numeric_loop(self, target, backedge)
                || super::vm_control_loop::try_run_control_loop(self, target, backedge)
                || super::typed_loop::try_run_typed_loop(self, target, backedge);
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
