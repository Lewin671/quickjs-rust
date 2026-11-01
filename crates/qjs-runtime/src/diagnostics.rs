//! Feature-gated execution counters for performance diagnosis.
//!
//! A benchmark's wall time says how long a workload took; it does not say
//! which of the engine's execution paths ran. A case can report nanoseconds
//! per nominal call while performing no calls at all, because a closed-form
//! tier folded the callee into the surrounding loop. Timing alone cannot
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
//! would burden those paths for no diagnostic benefit. The exemption is
//! deliberate and bounded to this module.

/// Declares the counter set once, so the struct, its deterministic report
/// order, and the thread-local storage cannot drift apart as fields are added.
macro_rules! declare_counters {
    ($($(#[$doc:meta])* $field:ident,)+) => {
        /// Execution event counts and explicitly documented current-size gauges.
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
    property_array_seed_headers,
    property_array_seed_releases,
    property_array_cold_stores,
    property_array_named_materializations,
    property_constructor_prototype_hits,
    property_constructor_prototype_misses,
    /// Lazy implicit arguments values prepared at function entry.
    arguments_recipes,
    /// Arguments objects constructed by a binding value read.
    arguments_materializations,
    property_arguments_template_hits,
    property_arguments_template_misses,
    property_arguments_templates_inserted,
    /// Calls a VM call opcode dispatched to a user function with a bytecode
    /// body, before any tier decides how to run it. A workload that claims N
    /// calls must report about N here, or the workload is not measuring calls.
    ordinary_call_attempts,
    /// Host/callback forwarding to the direct-leaf entry; not a second JS call.
    ordinary_call_forwardings,
    /// Calls answered by a closed-form leaf evaluator without building any
    /// frame. This is the tier that makes a foldable benchmark look free.
    closed_form_leaf_evaluations,
    /// Calls that built a slot-seeded direct-leaf frame.
    direct_leaf_frames,
    /// Calls that took the general `call_function` path with a bytecode body.
    generic_call_frames,
    /// Calls dispatched to a native builtin.
    native_calls,
    /// Nested `Vm` instances constructed to run a callee.
    nested_vm_constructions,
    /// Callee frames entered on the caller's own VM instead of a nested `Vm`.
    /// Always zero: no counting site exists, because no call routes through
    /// the frame-stack driver (`bytecode/frame_stack.rs`).
    same_vm_frame_entries,
    /// Successful numeric-plan root executions (already attributed at entry).
    numeric_plan_roots,
    /// Numeric-plan internal call probes, including discarded speculation.
    numeric_plan_call_probes,
    /// Internal callee entries retained by a successful numeric plan; root excluded.
    numeric_plan_call_entries,
    /// Actual non-recycled variable-identity tokens minted in the atom domain.
    typed_variable_identities,
    /// Rc allocations for nonempty typed native closure payloads.
    native_capture_payload_allocations,
    /// Actual slot-to-cell authority transfers at first exposure.
    binding_promotions,
    /// Actual Rc<CellStorage> allocations, including canonical object caches.
    binding_cell_allocations,
    /// Actual Rc<ScopeLease> allocations; cached reuse does not increment.
    scope_lease_allocations,
    /// Function creations retaining a lexical scope, not lease allocations.
    function_scope_retentions,
    /// Actual inline lexical call owners promoted into Rc storage.
    lexical_owner_allocations,
    /// Existing local-vector pool requests, including hits and zero-length requests.
    local_slot_pool_requests,
    /// New nonempty local-vector storage, separate from pool requests.
    local_slot_allocations,
    /// Operand-stack bank requests and initial capacity allocations; excludes growth.
    operand_stack_pool_requests,
    operand_stack_pool_allocations,
    /// Cold-frame recycler requests and actual new Box<ColdFrame> allocations.
    cold_frame_pool_requests,
    cold_frame_allocations,
    /// Retired CallEnv construction event; no counting site remains.
    call_env_constructions,
    /// Retired CallEnv Clone event; no counting site remains.
    call_env_clones,
    /// Retired caller/callee environment equality proof; no counting site remains.
    call_env_equality_proofs,
    /// Retired execution projection/fork/apply-back boundaries; no sites remain.
    legacy_binding_snapshot_calls,
    legacy_frame_fork_calls,
    legacy_applyback_calls,
    /// S3a-retired native argument ownership/family dispatch; no sites remain.
    argument_vec_native_pop,
    native_family_probes,
    native_family_declines,
    /// Retired direct-eval marker scrub event, retained during the Context
    /// migration audit. Typed eval sites have no counting site for this event.
    call_env_marker_scrubs,
    /// Statically named property reads and writes.
    named_property_reads,
    named_property_writes,
    /// Shape IC results, generic own probes (including hook declines), and text-index queries by receiver family.
    ic_object_hits,
    ic_object_misses,
    ic_object_probes,
    ic_object_hashes,
    ic_typed_array_hits,
    ic_typed_array_misses,
    ic_typed_array_probes,
    ic_typed_array_hashes,
    ic_array_prototype_hits,
    ic_array_prototype_misses,
    ic_array_prototype_probes,
    ic_array_prototype_hashes,
    ic_string_wrapper_hits,
    ic_string_wrapper_misses,
    ic_string_wrapper_probes,
    ic_string_wrapper_hashes,
    ic_array_hits,
    ic_array_misses,
    ic_array_probes,
    ic_array_hashes,
    ic_function_hits,
    ic_function_misses,
    ic_function_probes,
    ic_function_hashes,
    ic_map_hits,
    ic_map_misses,
    ic_map_probes,
    ic_map_hashes,
    ic_set_hits,
    ic_set_misses,
    ic_set_probes,
    ic_set_hashes,
    ic_exotic_hits,
    ic_exotic_misses,
    ic_exotic_probes,
    ic_exotic_hashes,
    /// Property-name atoms created by an interning domain. Steady-state
    /// named access must add none: linked code reuses its atoms.
    property_atoms_created,
    /// Calls to string or symbol interning, including existing-key hits.
    property_key_interns,
    /// Content/identity index searches at host key boundaries, including finds.
    property_text_probes,
    property_symbol_probes,
    /// ID lookup probes performed by the named-store authority.
    property_id_probes,
    /// Cold ID-index reconstruction work, separate from operation lookup probes.
    property_id_index_rebuilds,
    property_id_index_rebuilt_entries,
    /// Shape transition hits and misses; hits copy no fields.
    property_transition_hits,
    property_transition_misses,
    property_shape_field_copies,
    /// Append nodes created on transition misses (private edits/root excluded).
    property_shape_nodes_created,
    /// Gauge: shape nodes retained by live domain owners, including roots.
    /// Live uncached layouts are excluded; this returns to zero on owner drop.
    property_cached_shape_nodes,
    /// Bodies whose named operands were linked to an atom domain. One per
    /// body and domain; a count that grows with executed operations means
    /// linking is not cached.
    named_operand_links,
    /// Computed (bracket) property reads and writes.
    computed_property_reads,
    computed_property_writes,
    /// Ordinary bytecode backward edges taken.
    loop_backedges,
    /// Backward edges where a loop accelerator -- a numeric-mutation plan or
    /// a typed loop program -- ran the region.
    loop_plan_entries,
    /// Backward edges where both accelerators were consulted and both
    /// declined. On such an edge the entire probe chain is overhead.
    declined_loop_plan_edges,
    /// Backward edges never offered to an accelerator because the activation
    /// resolves names through a dynamic scope chain.
    scope_blocked_loop_edges,
    /// Backward edges never offered to an accelerator because a reference
    /// site in the region could not be elided at entry.
    reference_blocked_loop_edges,
    /// Scope proofs that kept a closure over an extensible chain on the
    /// general path: a holder or fixed cell binds one of its free names, a
    /// with object or a suspendable holder is in the chain.
    lexical_scope_entry_rejections,
    /// Activation admissions that need the VM's dynamic reference resolver
    /// (with/eval/parameter sites or a dynamic scope chain).
    reference_admission_dynamic,
    /// Activation admissions that run with per-start reference checkpoints
    /// because some checked global site is bound but configurable, or absent.
    reference_admission_checkpoints,
    /// Structural reference proofs answered from a plan's cache.
    reference_admission_cache_hits,
    /// Structural reference proofs recomputed by name.
    reference_admission_cache_misses,
    /// Tier checkpoints whose start could not be elided and selected its
    /// reference through ordered resolution instead.
    reference_checkpoint_selections,
    /// Standalone tier entries declined because the activation can still
    /// resolve a name dynamically (`indexed_bindings_only`).
    indexed_binding_admission_rejections,
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
    /// Retired caller-environment tier substitution; no counting site remains.
    compact_caller_env_calls,
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
