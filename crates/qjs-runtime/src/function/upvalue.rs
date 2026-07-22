//! Shared upvalue cell for the environment-model rewrite (T016 / S1).
//!
//! An [`Upvalue`] is the single heap cell that backs one *captured* binding.
//! The declaring frame and every closure that closes over the binding hold a
//! clone of the same `Rc`, so a write through any handle is observed by all the
//! others with no snapshot, no shared-`captured_env` HashMap, and no
//! `CaptureWriteback` write-back pass. This is the vocabulary that the cell-slot
//! migration in `docs/design/env-model-rewrite.md` builds on; it is introduced
//! ahead of its first consumer (T016 S2), so the items are `dead_code`-allowed
//! until then.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use crate::Value;

/// A shared, mutable cell holding one captured binding's current value.
///
/// Cloning an `Upvalue` shares the underlying cell (an `Rc` bump), which is the
/// whole point: two closures created from the same frame that capture the same
/// binding clone the same cell and therefore see each other's writes. Identity
/// is by cell, not by value — use [`Upvalue::ptr_eq`] to ask whether two handles
/// name the same binding.
#[derive(Clone)]
#[allow(dead_code)] // Consumed starting at T016 S2 (cell slots); see module docs.
pub(crate) struct Upvalue(Rc<UpvalueData>);

struct UpvalueData {
    value: RefCell<Value>,
    global_data_state: Cell<GlobalDataState>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum GlobalDataState {
    Unlinked,
    LinkedWritable,
    LinkedReadOnly,
    Detached,
}

/// Result of attempting the slot-only store used by a linked realm global.
/// `NotLinked` deliberately includes permanently detached cells so callers
/// fail closed to the complete property/environment path.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LinkedGlobalStore {
    Written,
    ReadOnly,
    NotLinked,
}

#[allow(dead_code)] // Consumed starting at T016 S2 (cell slots); see module docs.
impl Upvalue {
    /// Creates a cell initialized to `value`.
    pub(crate) fn new(value: Value) -> Self {
        Self(Rc::new(UpvalueData {
            value: RefCell::new(value),
            global_data_state: Cell::new(GlobalDataState::Unlinked),
        }))
    }

    /// Creates a cell initialized to `undefined` — the state of a hoisted or
    /// not-yet-initialized captured binding.
    pub(crate) fn undefined() -> Self {
        Self::new(Value::Undefined)
    }

    /// Reads the current value (a `Value` clone — a refcount bump for the
    /// heap-backed variants, never a deep copy).
    pub(crate) fn get(&self) -> Value {
        self.0.value.borrow().clone()
    }

    /// Borrows the current value for a read that can copy only the payload it
    /// needs. This avoids an otherwise unnecessary reference-count update when
    /// a guarded fast path merely classifies or extracts an inline primitive.
    pub(crate) fn with_value<R>(&self, read: impl FnOnce(&Value) -> R) -> R {
        read(&self.0.value.borrow())
    }

    /// Mutates the cell's value in place (e.g. an in-place `Rc::make_mut`
    /// string append). A detached global cell has returned to ordinary cell
    /// semantics: callers that know it once represented a global must still
    /// choose the complete object path, but environment invalidation is free
    /// to install its uninitialized marker instead of silently losing it.
    pub(crate) fn with_value_mut<R>(&self, mutate: impl FnOnce(&mut Value) -> R) -> Option<R> {
        match self.0.global_data_state.get() {
            GlobalDataState::Unlinked
            | GlobalDataState::LinkedWritable
            | GlobalDataState::Detached => Some(mutate(&mut self.0.value.borrow_mut())),
            GlobalDataState::LinkedReadOnly => None,
        }
    }

    /// Overwrites an ordinary, detached, or writable-linked cell; visible
    /// through every handle. Only a currently read-only link rejects this
    /// unchecked path. Detached cells must accept realm invalidation markers
    /// and slow-path synchronization; link-aware optimizers separately reject
    /// them through [`Self::is_detached_global`].
    pub(crate) fn set(&self, value: Value) {
        if self.0.global_data_state.get() == GlobalDataState::LinkedReadOnly {
            return;
        }
        *self.0.value.borrow_mut() = value;
    }

    /// Marks this cell as the single value storage for one ordinary global
    /// data property. A cell can be linked only once; detached cells never
    /// relink, keeping invalidation monotonic and fail closed.
    pub(crate) fn try_link_global_data(&self, writable: bool) -> bool {
        if self.0.global_data_state.get() != GlobalDataState::Unlinked {
            return false;
        }
        self.0.global_data_state.set(if writable {
            GlobalDataState::LinkedWritable
        } else {
            GlobalDataState::LinkedReadOnly
        });
        true
    }

    /// Stores through a linked global property without consulting its name or
    /// object descriptor. The link installation and descriptor mutation paths
    /// maintain the writable state used by this slot-only guard.
    pub(crate) fn try_store_linked_global(&self, value: Value) -> LinkedGlobalStore {
        match self.0.global_data_state.get() {
            GlobalDataState::LinkedWritable => {
                *self.0.value.borrow_mut() = value;
                LinkedGlobalStore::Written
            }
            GlobalDataState::LinkedReadOnly => LinkedGlobalStore::ReadOnly,
            GlobalDataState::Unlinked | GlobalDataState::Detached => LinkedGlobalStore::NotLinked,
        }
    }

    pub(crate) fn set_linked_global_writable(&self, writable: bool) -> bool {
        let next = match self.0.global_data_state.get() {
            GlobalDataState::LinkedWritable => {
                if writable {
                    GlobalDataState::LinkedWritable
                } else {
                    GlobalDataState::LinkedReadOnly
                }
            }
            // A non-configurable data property cannot become writable again.
            // Treat an unchecked internal attempt as incompatible so its
            // caller detaches rather than reviving a stale fast-path guard.
            GlobalDataState::LinkedReadOnly if writable => return false,
            GlobalDataState::LinkedReadOnly => GlobalDataState::LinkedReadOnly,
            GlobalDataState::Unlinked | GlobalDataState::Detached => return false,
        };
        self.0.global_data_state.set(next);
        true
    }

    pub(crate) fn detach_linked_global(&self) {
        if matches!(
            self.0.global_data_state.get(),
            GlobalDataState::LinkedWritable | GlobalDataState::LinkedReadOnly
        ) {
            self.0.global_data_state.set(GlobalDataState::Detached);
        }
    }

    pub(crate) fn is_linked_global(&self) -> bool {
        matches!(
            self.0.global_data_state.get(),
            GlobalDataState::LinkedWritable | GlobalDataState::LinkedReadOnly
        )
    }

    pub(crate) fn is_linked_global_writable(&self) -> bool {
        self.0.global_data_state.get() == GlobalDataState::LinkedWritable
    }

    /// Whether this cell once shared storage with a realm-global property but
    /// must now use full object/environment resolution. Detachment is
    /// permanent even though ordinary cell updates remain valid.
    pub(crate) fn is_detached_global(&self) -> bool {
        self.0.global_data_state.get() == GlobalDataState::Detached
    }

    /// Whether `self` and `other` are handles to the *same* cell (binding
    /// identity), independent of the values they currently hold.
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn is_shared(&self) -> bool {
        Rc::strong_count(&self.0) > 1
    }
}

impl std::fmt::Debug for Upvalue {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("Upvalue")
            .field(&*self.0.value.borrow())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_then_get_roundtrips() {
        let cell = Upvalue::new(Value::Number(1.0));
        assert!(matches!(cell.get(), Value::Number(n) if n == 1.0));
    }

    #[test]
    fn undefined_starts_undefined() {
        assert!(matches!(Upvalue::undefined().get(), Value::Undefined));
    }

    #[test]
    fn set_is_visible_through_a_shared_clone() {
        let declaring = Upvalue::new(Value::Number(0.0));
        // A closure capturing the same binding clones the cell.
        let captured = declaring.clone();
        captured.set(Value::Number(42.0));
        // The declaring frame observes the closure's write — the whole point of
        // a shared cell vs. the old snapshot model.
        assert!(matches!(declaring.get(), Value::Number(n) if n == 42.0));
    }

    #[test]
    fn with_value_reads_without_changing_the_cell() {
        let cell = Upvalue::new(Value::Number(42.0));
        let number = cell.with_value(|value| match value {
            Value::Number(number) => Some(*number),
            _ => None,
        });
        assert_eq!(number, Some(42.0));
        assert!(matches!(cell.get(), Value::Number(number) if number == 42.0));
    }

    #[test]
    fn ptr_eq_tracks_cell_identity_not_value() {
        let cell = Upvalue::new(Value::Number(1.0));
        let same = cell.clone();
        let distinct = Upvalue::new(Value::Number(1.0));
        assert!(cell.ptr_eq(&same));
        assert!(!cell.ptr_eq(&distinct));
    }

    #[test]
    fn ordinary_set_cannot_bypass_a_read_only_global_link() {
        let cell = Upvalue::new(Value::Number(1.0));
        assert!(cell.try_link_global_data(false));
        cell.set(Value::Number(2.0));
        assert_eq!(cell.get(), Value::Number(1.0));
        assert_eq!(
            cell.try_store_linked_global(Value::Number(3.0)),
            LinkedGlobalStore::ReadOnly
        );
    }

    #[test]
    fn detached_global_returns_to_ordinary_cell_semantics_without_relinking() {
        let cell = Upvalue::new(Value::Number(1.0));
        assert!(cell.try_link_global_data(true));
        cell.detach_linked_global();

        assert!(cell.is_detached_global());
        assert_eq!(
            cell.try_store_linked_global(Value::Number(2.0)),
            LinkedGlobalStore::NotLinked
        );
        cell.set(Value::Number(3.0));
        assert_eq!(cell.get(), Value::Number(3.0));
        assert_eq!(
            cell.with_value_mut(|value| *value = Value::Number(4.0)),
            Some(())
        );
        assert_eq!(cell.get(), Value::Number(4.0));
        assert!(!cell.try_link_global_data(true));
    }
}
