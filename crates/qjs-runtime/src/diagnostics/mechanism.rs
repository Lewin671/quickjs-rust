//! Feature-gated mechanism-event scopes.
//!
//! Mechanism counters have deliberately separate denominators. `value_clones`
//! counts every explicit Value clone; `value_heap_clones` is its owning subset.
//! `heap_handle_*_drops` counts releases of the seven runtime handle types,
//! including handles held outside a Value; it is **not** an exact Value-destructor
//! total. `value_transport_replacements` and `_heap_retirements` cover compact
//! register stores only. Scalar values have no ownership destructor in production.
//! `heap_<kind>_allocations`/`_frees` count outer payload lifetimes for all eight
//! Value heap kinds, including string COW; frees mean payload destruction, not the
//! release of a weakly retained Rc allocation. BigInt destruction is observed by
//! weak-handle sweeps at counter snapshots. Internal Vec/HashMap buffers, frame
//! buffers and allocator metadata are outside these counts.
//!
//! `property_name_hashes` and `_comparisons` count actual ordinary property-storage
//! hash finishes and equality calls across Small/Dynamic/Shaped/ShapedPair,
//! including computed queries and mutation. This covers the four ObjectRef stores,
//! function maps and array named-property maps; function own-property special
//! name checks are counted too. Hash bytes cover byte-slice writes, excluding
//! the string delimiter; comparison bytes are the shorter operand length,
//! not bytes CPU-loaded.
//! Diagnostic-only owned query keys expose standard HashMap equality; their
//! allocation is excluded from JS payload counts. `ordinary_property_name_*` and
//! `accelerated_property_name_*` scope those events to the innermost VM or
//! compact/typed named handler. Unscoped setup/computed work remains only in totals.
//! Named read/write counters distinguish VM from compact/typed **attempts**;
//! a deoptimized attempt may be replayed. Closed-form helper access is not in that
//! split. `ic_<kind>_{probes,hits,rejects}` partitions each marked cache helper;
//! prototype/full-walk hits mean a guarded candidate, while hot reads are answers.
//! Write hits mean a returned slot result, including read-only/slow-path metadata.
//! Kinds and helpers can overlap and must not be summed as access counts.
//! `virtual_field_dispatches` counts selected virtual-object handler invocations.
//! `frame_constructions` includes root and callee frames. `frame_stack_transfers`
//! and `_bytes` count driver transfers. `frame_aggregate_transfers` and `_bytes`
//! additionally include consumed Vms transferred to generator snapshots. These
//! are source-level events, never counts of emitted memcpy.
//! Wiring tests live in `tests/diagnostics/` and cache/property-map modules.

use std::cell::Cell;

use super::update;

thread_local! {
    static NAMED_PATH: Cell<u8> = const { Cell::new(0) };
    static PROPERTY_DEPTH: Cell<usize> = const { Cell::new(0) };
}

pub(crate) struct PropertyScope;
impl PropertyScope {
    pub(crate) fn enter() -> Self {
        PROPERTY_DEPTH.with(|depth| depth.set(depth.get() + 1));
        super::count!(property_storage_queries);
        Self
    }
}
impl Drop for PropertyScope {
    fn drop(&mut self) {
        let _ = PROPERTY_DEPTH.try_with(|depth| depth.set(depth.get().saturating_sub(1)));
    }
}

pub(crate) fn name_hash() {
    if PROPERTY_DEPTH.get() > 0 {
        super::count!(property_name_hashes);
        update(|c| match NAMED_PATH.get() {
            1 => c.ordinary_property_name_hashes += 1,
            2 => c.accelerated_property_name_hashes += 1,
            _ => {}
        });
    }
}
pub(crate) fn name_hash_bytes(bytes: usize) {
    if PROPERTY_DEPTH.get() > 0 {
        update(|c| c.property_name_hash_bytes += bytes as u64);
    }
}
pub(crate) fn name_eq(left: &str, right: &str) -> bool {
    update(|c| {
        c.property_name_comparisons += 1;
        match NAMED_PATH.get() {
            1 => c.ordinary_property_name_comparisons += 1,
            2 => c.accelerated_property_name_comparisons += 1,
            _ => {}
        }
        // Bytes offered to equality, not a claim about CPU-loaded bytes.
        c.property_name_compared_bytes += left.len().min(right.len()) as u64;
    });
    left == right
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum HeapKind {
    Array,
    String,
    Function,
    Map,
    Set,
    Proxy,
}

#[derive(Debug)]
pub(crate) struct HeapLifetime(HeapKind);
impl HeapLifetime {
    pub(crate) fn new(kind: HeapKind) -> Self {
        update(|c| match kind {
            HeapKind::Array => c.heap_array_allocations += 1,
            HeapKind::String => c.heap_string_allocations += 1,
            HeapKind::Function => c.heap_function_allocations += 1,
            HeapKind::Map => c.heap_map_allocations += 1,
            HeapKind::Set => c.heap_set_allocations += 1,
            HeapKind::Proxy => c.heap_proxy_allocations += 1,
        });
        Self(kind)
    }
}
impl Drop for HeapLifetime {
    fn drop(&mut self) {
        update(|c| match self.0 {
            HeapKind::Array => c.heap_array_frees += 1,
            HeapKind::String => c.heap_string_frees += 1,
            HeapKind::Function => c.heap_function_frees += 1,
            HeapKind::Map => c.heap_map_frees += 1,
            HeapKind::Set => c.heap_set_frees += 1,
            HeapKind::Proxy => c.heap_proxy_frees += 1,
        });
    }
}

pub(crate) fn record_ic(kind: &str, hit: bool) {
    update(|c| match kind {
        "write" => {
            c.ic_write_probes += 1;
            if hit {
                c.ic_write_hits += 1;
            } else {
                c.ic_write_rejects += 1;
            }
        }
        "hot_write" => {
            c.ic_hot_write_probes += 1;
            if hit {
                c.ic_hot_write_hits += 1;
            } else {
                c.ic_hot_write_rejects += 1;
            }
        }
        "hot" => {
            c.ic_hot_probes += 1;
            if hit {
                c.ic_hot_hits += 1;
            } else {
                c.ic_hot_rejects += 1;
            }
        }
        "exact" => {
            c.ic_exact_probes += 1;
            if hit {
                c.ic_exact_hits += 1;
            } else {
                c.ic_exact_rejects += 1;
            }
        }
        "literal" => {
            c.ic_literal_probes += 1;
            if hit {
                c.ic_literal_hits += 1;
            } else {
                c.ic_literal_rejects += 1;
            }
        }
        "own_slot" => {
            c.ic_own_slot_probes += 1;
            if hit {
                c.ic_own_slot_hits += 1;
            } else {
                c.ic_own_slot_rejects += 1;
            }
        }
        "shared_slot" => {
            c.ic_shared_slot_probes += 1;
            if hit {
                c.ic_shared_slot_hits += 1;
            } else {
                c.ic_shared_slot_rejects += 1;
            }
        }
        "prototype" => {
            c.ic_prototype_probes += 1;
            if hit {
                c.ic_prototype_hits += 1;
            } else {
                c.ic_prototype_rejects += 1;
            }
        }
        "grandprototype" => {
            c.ic_grandprototype_probes += 1;
            if hit {
                c.ic_grandprototype_hits += 1;
            } else {
                c.ic_grandprototype_rejects += 1;
            }
        }
        _ => unreachable!("known diagnostic cache kind"),
    });
}

/// Innermost named-operation handler, restored across re-entry and unwinds.
pub(crate) struct NamedPath(u8);
impl NamedPath {
    pub(crate) fn enter(path: u8) -> Self {
        Self(NAMED_PATH.replace(path))
    }
}
impl Drop for NamedPath {
    fn drop(&mut self) {
        let _ = NAMED_PATH.try_with(|path| path.set(self.0));
    }
}

// Immutable BigInt is the one payload supplied directly by a dependency.
// Weak tracking preserves payload lifetime/identity and does not inspect Rc's
// representation. Sweeps count dead payloads once, at snapshot boundaries.
thread_local! {
    static BIGINTS: std::cell::RefCell<Vec<std::rc::Weak<num_bigint::BigInt>>> = const { std::cell::RefCell::new(Vec::new()) };
}
pub(crate) fn track_bigint(value: &std::rc::Rc<num_bigint::BigInt>) {
    super::count!(heap_bigint_allocations);
    BIGINTS.with(|values| values.borrow_mut().push(std::rc::Rc::downgrade(value)));
}
pub(crate) fn sweep_bigints() {
    let _ = BIGINTS.try_with(|values| {
        if let Ok(mut values) = values.try_borrow_mut() {
            values.retain(|value| {
                if value.strong_count() == 0 {
                    super::count!(heap_bigint_frees);
                    false
                } else {
                    true
                }
            });
        }
    });
}
pub(crate) fn reset_bigints() {
    BIGINTS.with(|values| values.borrow_mut().clear());
}

use crate::{ArrayRef, Function, JsString, MapRef, ObjectRef, SetRef, proxy::ProxyRef};

impl Drop for ObjectRef {
    fn drop(&mut self) {
        crate::diagnostics::count!(heap_handle_drops);
        crate::diagnostics::count!(heap_handle_object_drops);
    }
}

impl Drop for ArrayRef {
    fn drop(&mut self) {
        crate::diagnostics::count!(heap_handle_drops);
        crate::diagnostics::count!(heap_handle_array_drops);
    }
}

impl Drop for JsString {
    fn drop(&mut self) {
        crate::diagnostics::count!(heap_handle_drops);
        crate::diagnostics::count!(heap_handle_string_drops);
    }
}

impl Drop for MapRef {
    fn drop(&mut self) {
        crate::diagnostics::count!(heap_handle_drops);
        crate::diagnostics::count!(heap_handle_map_drops);
    }
}

impl Drop for SetRef {
    fn drop(&mut self) {
        crate::diagnostics::count!(heap_handle_drops);
        crate::diagnostics::count!(heap_handle_set_drops);
    }
}

impl Drop for Function {
    fn drop(&mut self) {
        crate::diagnostics::count!(heap_handle_drops);
        crate::diagnostics::count!(heap_handle_function_drops);
    }
}

impl Drop for ProxyRef {
    fn drop(&mut self) {
        crate::diagnostics::count!(heap_handle_drops);
        crate::diagnostics::count!(heap_handle_proxy_drops);
    }
}
