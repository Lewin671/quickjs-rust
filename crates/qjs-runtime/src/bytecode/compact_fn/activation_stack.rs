//! The activations of a frame driver's inline callees. The root body runs in
//! the activation its caller lent the driver; the frame at depth `d > 0` (the
//! number of suspended frames below it) runs in slot `d - 1`, so entering and
//! returning move no activation. Slots above the current frame hold released
//! ordinary activations (`ActivationContext::release_for_reuse`) that the next
//! call at that depth rebinds instead of rebuilding; a depth without one draws
//! from the services' released activations, and a run that returns normally
//! hands its released slots back there. Every call still owns one canonical
//! activation; a released slot keeps only its creation and class handles.
use crate::context::ActivationContext;
use crate::function::Function;

/// Released slots kept above the current frame after a deep return.
const MAX_PARKED: usize = 64;

#[derive(Default)]
pub(in crate::bytecode) struct ActivationStack {
    slots: Vec<ActivationContext>,
}

impl ActivationStack {
    /// The activation of the frame at `depth`.
    #[inline(always)]
    pub(in crate::bytecode) fn current<'a>(
        &'a mut self,
        root: &'a mut ActivationContext,
        depth: usize,
    ) -> &'a mut ActivationContext {
        match depth.checked_sub(1) {
            None => root,
            Some(slot) => &mut self.slots[slot],
        }
    }

    /// The activation a call from the frame at `depth` binds for its callee:
    /// a released one rebound when it is exactly what a fresh indexed entry
    /// of `function` would build, else a fresh one.
    #[inline]
    pub(in crate::bytecode) fn callee_slot(
        &mut self,
        depth: usize,
        function: &Function,
        runtime: &mut crate::context::RuntimeServices,
    ) -> &mut ActivationContext {
        debug_assert!(self.slots.len() >= depth, "one slot per suspended frame");
        if let Some(slot) = self.slots.get_mut(depth) {
            if !crate::function::reenter_indexed_activation(slot, function) {
                *slot = crate::function::released_indexed_activation(runtime, function);
            }
        } else {
            self.slots
                .push(crate::function::released_indexed_activation(
                    runtime, function,
                ));
        }
        &mut self.slots[depth]
    }

    /// After the callee of the frame at `depth` returned: its activation
    /// stays for reuse when it holds only ordinary call state; anything it
    /// retained or exposed is dropped with it.
    #[inline]
    pub(in crate::bytecode) fn release(&mut self, depth: usize) {
        if !self.slots[depth].release_for_reuse() {
            self.slots.truncate(depth);
        } else if self.slots.len() > depth + MAX_PARKED {
            self.slots.truncate(depth + MAX_PARKED);
        }
    }

    pub(in crate::bytecode) fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }

    /// Ends a run: after a normal return every slot is released and goes
    /// back to the services; after a terminal error the slots are dropped.
    pub(in crate::bytecode) fn finish(
        &mut self,
        runtime: &mut crate::context::RuntimeServices,
        returned: bool,
    ) {
        if returned {
            for activation in self.slots.drain(..) {
                crate::function::keep_released_activation(runtime, activation);
            }
        } else {
            self.slots.clear();
        }
    }
}
