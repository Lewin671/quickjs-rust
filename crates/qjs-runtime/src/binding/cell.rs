//! A shared declarative location and its immutable declaration metadata.
//!
//! State and value have one owner; every captured route retains this same Rc.
//! Initialization and assignment are distinct operations, and failed writes
//! leave the state unchanged. No storage borrow may survive a JS callback.

use super::{
    BindingId, BindingKind, BindingMeta, BindingMetaRef, BindingMetadata, InitPolicy, LocalSlot,
    WritePolicy,
};
use crate::{RuntimeError, Value};
use std::{cell::RefCell, rc::Rc};

#[derive(Clone, Debug)]
pub(crate) enum BindingState {
    Uninitialized,
    Initialized(Value),
}

struct CellStorage {
    meta: BindingMetaRef,
    state: RefCell<BindingState>,
}

#[derive(Clone)]
pub(crate) struct BindingCell(Rc<CellStorage>);

impl BindingMetaRef {
    pub(crate) fn dynamic(name: String, mutable: bool, deletable: bool) -> Self {
        Self {
            owner: Rc::new(BindingMetadata(
                vec![BindingMeta {
                    id: BindingId(0),
                    name,
                    kind: BindingKind::Var,
                    mutable,
                    strict_write: if mutable {
                        WritePolicy::Mutable
                    } else {
                        WritePolicy::Throw
                    },
                    deletable,
                    initialization: InitPolicy::Undefined,
                    slot: LocalSlot(0),
                }]
                .into_boxed_slice(),
                super::MetadataOrigin::Dynamic,
            )),
            id: BindingId(0),
        }
    }
}

impl BindingCell {
    pub(crate) fn new(value: Value) -> Self {
        Self::with_metadata(
            BindingMetaRef::dynamic(String::new(), true, false),
            Some(value),
        )
    }

    #[cfg(test)]
    pub(crate) fn undefined() -> Self {
        Self::new(Value::Undefined)
    }

    pub(crate) fn with_metadata(meta: BindingMetaRef, value: Option<Value>) -> Self {
        let state = match value {
            Some(value) if !value.is_uninitialized_lexical_marker() => {
                BindingState::Initialized(value)
            }
            _ => BindingState::Uninitialized,
        };
        Self(Rc::new(CellStorage {
            meta,
            state: RefCell::new(state),
        }))
    }

    pub(crate) fn has_compiler_metadata(&self) -> bool {
        self.0.meta.owner.1 == super::MetadataOrigin::Compiled
    }

    pub(crate) fn fresh_uninitialized(&self) -> Self {
        Self::with_metadata(self.0.meta.clone(), None)
    }
    pub(crate) fn fresh_with_value(&self, value: Value) -> Self {
        Self::with_metadata(self.0.meta.clone(), Some(value))
    }

    pub(crate) fn metadata(&self) -> &BindingMeta {
        &self.0.meta
    }

    pub(crate) fn is_initialized(&self) -> bool {
        matches!(*self.0.state.borrow(), BindingState::Initialized(_))
    }

    pub(crate) fn read(&self) -> Result<Value, RuntimeError> {
        match &*self.0.state.borrow() {
            BindingState::Initialized(value) => Ok(value.clone()),
            BindingState::Uninitialized => Err(self.tdz_error()),
        }
    }

    pub(crate) fn initialize(&self, value: Value) -> Result<(), RuntimeError> {
        let mut state = self.0.state.borrow_mut();
        if !matches!(*state, BindingState::Uninitialized) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: binding already initialized".to_owned(),
            });
        }
        *state = BindingState::Initialized(value);
        Ok(())
    }

    pub(crate) fn check_assignment(&self, _strict: bool) -> Result<(), RuntimeError> {
        if !self.is_initialized() {
            return Err(self.tdz_error());
        }
        if !self.metadata().mutable {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        Ok(())
    }

    pub(crate) fn assign(&self, value: Value, strict: bool) -> Result<(), RuntimeError> {
        self.check_assignment(strict)?;
        *self.0.state.borrow_mut() = BindingState::Initialized(value);
        Ok(())
    }

    fn tdz_error(&self) -> RuntimeError {
        RuntimeError {
            thrown: None,
            message: format!(
                "ReferenceError: undefined identifier `{}`",
                self.0.meta.name
            ),
        }
    }

    // Legacy execution views are removed in B3. Their marker-facing read API
    // continues to project state without maintaining a second value location.
    pub(crate) fn get(&self) -> Value {
        self.with_value(Clone::clone)
    }

    pub(crate) fn with_value<R>(&self, read: impl FnOnce(&Value) -> R) -> R {
        match &*self.0.state.borrow() {
            BindingState::Initialized(value) => read(value),
            BindingState::Uninitialized => read(&Value::Function(
                crate::Function::uninitialized_lexical_marker(),
            )),
        }
    }

    pub(crate) fn with_value_mut<R>(&self, mutate: impl FnOnce(&mut Value) -> R) -> R {
        let mut state = self.0.state.borrow_mut();
        match &mut *state {
            BindingState::Initialized(value) => mutate(value),
            BindingState::Uninitialized => {
                let mut value = Value::Function(crate::Function::uninitialized_lexical_marker());
                let result = mutate(&mut value);
                if !value.is_uninitialized_lexical_marker() {
                    *state = BindingState::Initialized(value);
                }
                result
            }
        }
    }

    // Transitional service-view publication; B3 deletes it with apply-back.
    pub(crate) fn set(&self, value: Value) {
        *self.0.state.borrow_mut() = if value.is_uninitialized_lexical_marker() {
            BindingState::Uninitialized
        } else {
            BindingState::Initialized(value)
        };
    }

    pub(crate) fn identity(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
    pub(crate) fn is_shared(&self) -> bool {
        Rc::strong_count(&self.0) > 1
    }
}

impl std::fmt::Debug for BindingCell {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("BindingCell")
            .field("meta", &self.0.meta)
            .field("state", &self.0.state.borrow())
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_then_get_roundtrips() {
        let cell = BindingCell::new(Value::Number(1.0));
        assert!(matches!(cell.get(), Value::Number(n) if n == 1.0));
    }

    #[test]
    fn undefined_starts_undefined() {
        assert!(matches!(BindingCell::undefined().get(), Value::Undefined));
    }

    #[test]
    fn set_is_visible_through_a_shared_clone() {
        let declaring = BindingCell::new(Value::Number(0.0));
        // A closure capturing the same binding clones the cell.
        let captured = declaring.clone();
        captured.set(Value::Number(42.0));
        // The declaring frame observes the closure's write — the whole point of
        // a shared cell vs. the old snapshot model.
        assert!(matches!(declaring.get(), Value::Number(n) if n == 42.0));
    }

    #[test]
    fn with_value_reads_without_changing_the_cell() {
        let cell = BindingCell::new(Value::Number(42.0));
        let number = cell.with_value(|value| match value {
            Value::Number(number) => Some(*number),
            _ => None,
        });
        assert_eq!(number, Some(42.0));
        assert!(matches!(cell.get(), Value::Number(number) if number == 42.0));
    }

    #[test]
    fn ptr_eq_tracks_cell_identity_not_value() {
        let cell = BindingCell::new(Value::Number(1.0));
        let same = cell.clone();
        let distinct = BindingCell::new(Value::Number(1.0));
        assert!(cell.ptr_eq(&same));
        assert!(!cell.ptr_eq(&distinct));
    }
}

#[cfg(test)]
mod policy_tests {
    use super::*;

    #[test]
    fn tdz_assign_cannot_initialize_and_init_keeps_shared_identity() {
        let cell =
            BindingCell::with_metadata(BindingMetaRef::dynamic("value".into(), true, false), None);
        let captured = cell.clone();
        assert!(
            captured
                .read()
                .unwrap_err()
                .message
                .starts_with("ReferenceError")
        );
        assert!(
            captured
                .assign(Value::Number(2.0), false)
                .unwrap_err()
                .message
                .starts_with("ReferenceError")
        );
        assert!(!cell.is_initialized());
        cell.initialize(Value::Number(3.0)).unwrap();
        assert_eq!(captured.read(), Ok(Value::Number(3.0)));
        assert!(cell.ptr_eq(&captured));
        assert!(cell.initialize(Value::Number(4.0)).is_err());
        assert_eq!(captured.read(), Ok(Value::Number(3.0)));
    }

    #[test]
    fn const_assign_rejects_same_value_without_changing_the_cell() {
        let cell = BindingCell::with_metadata(
            BindingMetaRef::dynamic("constant".into(), false, false),
            None,
        );
        cell.initialize(Value::Number(3.0)).unwrap();
        for strict in [false, true] {
            assert!(
                cell.assign(Value::Number(3.0), strict)
                    .unwrap_err()
                    .message
                    .starts_with("TypeError")
            );
            assert_eq!(cell.read(), Ok(Value::Number(3.0)));
        }
    }
}
