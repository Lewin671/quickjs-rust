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

use std::cell::RefCell;
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
pub(crate) struct Upvalue(Rc<RefCell<Value>>);

#[allow(dead_code)] // Consumed starting at T016 S2 (cell slots); see module docs.
impl Upvalue {
    /// Creates a cell initialized to `value`.
    pub(crate) fn new(value: Value) -> Self {
        Self(Rc::new(RefCell::new(value)))
    }

    /// Creates a cell initialized to `undefined` — the state of a hoisted or
    /// not-yet-initialized captured binding.
    pub(crate) fn undefined() -> Self {
        Self::new(Value::Undefined)
    }

    /// Reads the current value (a `Value` clone — a refcount bump for the
    /// heap-backed variants, never a deep copy).
    pub(crate) fn get(&self) -> Value {
        self.0.borrow().clone()
    }

    /// Overwrites the cell's value; visible through every handle to this cell.
    pub(crate) fn set(&self, value: Value) {
        *self.0.borrow_mut() = value;
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
            .field(&*self.0.borrow())
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
    fn ptr_eq_tracks_cell_identity_not_value() {
        let cell = Upvalue::new(Value::Number(1.0));
        let same = cell.clone();
        let distinct = Upvalue::new(Value::Number(1.0));
        assert!(cell.ptr_eq(&same));
        assert!(!cell.ptr_eq(&distinct));
    }
}
