//! Feature-gated execution counters for performance diagnosis.
//!
//! A benchmark's wall time says how long a workload took; it does not say
//! which of the engine's execution paths ran. That distinction is exactly what
//! the broad micro portfolio lost: its `plain_function_call` case reported
//! nanoseconds per nominal call while performing no calls at all, because the
//! callee had been folded into the surrounding loop. Timing alone could not
//! detect that. A count of the calls the engine actually attempted can.
//!
//! These counters therefore answer "which path did this workload take", not
//! "how fast was it". A build with `perf-counters` enabled is a diagnostic
//! build and must never be used for timing: the counters add work to the hot
//! paths they observe.
//!
//! ## Why this is thread-local state
//!
//! `AGENTS.md` forbids global mutable state in runtime data structures. These
//! counters are not runtime data structures: JavaScript cannot observe them,
//! they hold no engine state, and with the feature off they do not exist —
//! every counting site expands to nothing and the storage is not compiled in.
//! Threading a counter handle through every call, property, and loop path
//! would be a large diff through code this campaign is about to restructure,
//! for no diagnostic benefit. The exemption is deliberate and bounded to this
//! module.

/// Declares the counter set once, so the struct, its deterministic report
/// order, and the thread-local storage cannot drift apart as fields are added.
macro_rules! declare_counters {
    ($($(#[$doc:meta])* $field:ident,)+) => {
        /// One monotonic count per diagnosed execution event.
        #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
        pub struct Counters {
            $($(#[$doc])* pub $field: u64,)+
        }

        impl Counters {
            /// Returns every counter paired with its field name, in
            /// declaration order, so a report stays stable across builds.
            #[must_use]
            pub fn entries(&self) -> Vec<(&'static str, u64)> {
                vec![$((stringify!($field), self.$field),)+]
            }
        }

        #[cfg(feature = "perf-counters")]
        thread_local! {
            static COUNTERS: std::cell::RefCell<Counters> =
                const { std::cell::RefCell::new(Counters { $($field: 0,)+ }) };
        }
    };
}

declare_counters! {
    /// Calls a VM call opcode dispatched to a user function with a bytecode
    /// body, before any tier decides how to run it. A workload that claims N
    /// calls must report about N here, or the workload is not measuring calls.
    ordinary_call_attempts,
    /// Calls answered by a closed-form leaf evaluator without building any
    /// frame. This is the tier that makes a foldable benchmark look free.
    closed_form_leaf_evaluations,
    /// Calls that built a slot-seeded direct-leaf frame.
    direct_leaf_frames,
    /// Calls that took the general `call_function` path with a bytecode body.
    generic_call_frames,
    /// Calls dispatched to a native builtin.
    native_calls,
    /// Nested `Vm` instances constructed to run a callee. The call-frame
    /// migration exists to drive this toward zero for ordinary calls.
    nested_vm_constructions,
    /// Callee frames entered on the caller's own VM instead of a nested `Vm`.
    /// Stays zero until the frame-stack driver routes ordinary calls.
    same_vm_frame_entries,
    /// Generic calls whose environment could carry a direct-eval marker and
    /// therefore paid the pre-call scrub. An ordinary workload should report
    /// zero: every environment it hands over is built empty.
    call_env_marker_scrubs,
    /// Statically named property reads and writes.
    named_property_reads,
    named_property_writes,
    /// Immutable static property-name comparisons answered by shared
    /// compilation-graph identity, or forced to the cross-graph textual
    /// fallback. These are mechanism counters, not timing counters.
    static_property_name_identity_hits,
    static_property_name_text_fallbacks,
    /// Computed (bracket) property reads and writes.
    computed_property_reads,
    computed_property_writes,
    /// Ordinary bytecode backward edges taken.
    loop_backedges,
    /// Backward edges where a loop-plan engine actually ran a region.
    loop_plan_entries,
    /// Backward edges where all four loop engines were consulted and all four
    /// declined. On such an edge the entire probe chain is overhead, so this
    /// counter -- not a raw probe count -- is what a dispatch-table unit would
    /// have to justify itself against.
    declined_loop_plan_edges,
    /// Bytecode instructions actually dispatched by the interpreter loop.
    executed_ops,
    /// Generic-dispatch opcode families. Together these counters partition
    /// `executed_ops`, so profiles can distinguish instruction-count pressure
    /// from one particularly frequent handler family without adding a counter
    /// to every individual opcode.
    dispatched_load_const_ops,
    dispatched_local_binding_ops,
    dispatched_load_local_ops,
    dispatched_store_local_ops,
    dispatched_assign_local_ops,
    authoritative_load_local_hits,
    authoritative_store_local_hits,
    authoritative_assign_local_hits,
    dispatched_global_binding_ops,
    dispatched_named_property_ops,
    dispatched_computed_property_ops,
    dispatched_call_construct_ops,
    dispatched_stack_ops,
    dispatched_numeric_ops,
    dispatched_branch_return_ops,
    dispatched_general_ops,
    /// Register operations the compact executor dispatched. Read against
    /// `executed_ops`: work moved to this tier must leave the generic loop, not
    /// merely add to it.
    compact_function_ops,
    /// Activations that ran with no `Vm` at all. Read against
    /// `nested_vm_constructions`: this tier exists to move calls from one
    /// counter to the other. This is a mechanism counter, not a tier
    /// attribution -- a call entered through `call_direct_leaf_function` is
    /// counted both there and here.
    compact_standalone_activations,
    /// Calls a compact body dispatched straight to another compact body,
    /// building neither a frame nor an environment. This is a tier
    /// attribution: such a call is counted here and nowhere else.
    compact_direct_calls,
}

/// Applies `update` to the calling thread's counters.
///
/// Prefer the [`count!`] macro, which compiles away entirely when the feature
/// is off.
#[cfg(feature = "perf-counters")]
pub(crate) fn update(update: impl FnOnce(&mut Counters)) {
    // A counting site must never change behavior, so a re-entrant borrow is
    // silently skipped rather than panicking.
    let _ = COUNTERS.try_with(|counters| {
        if let Ok(mut counters) = counters.try_borrow_mut() {
            update(&mut counters);
        }
    });
}

/// Returns the calling thread's counters, or `None` in a build without the
/// `perf-counters` feature.
#[must_use]
pub fn counters() -> Option<Counters> {
    #[cfg(feature = "perf-counters")]
    {
        COUNTERS.try_with(|counters| *counters.borrow()).ok()
    }
    #[cfg(not(feature = "perf-counters"))]
    {
        None
    }
}

/// Resets the calling thread's counters. Used by tests that assert an exact
/// path count for one evaluation.
#[cfg(feature = "perf-counters")]
pub fn reset_counters() {
    update(|counters| *counters = Counters::default());
}

/// Adds one to a counter, or expands to nothing without the feature.
macro_rules! count {
    ($field:ident) => {
        #[cfg(feature = "perf-counters")]
        $crate::diagnostics::update(|counters| counters.$field += 1);
    };
}

pub(crate) use count;
