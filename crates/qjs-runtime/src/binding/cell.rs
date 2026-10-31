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
    LazyArguments(Rc<crate::function::LazyArguments>),
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
            )),
            id: BindingId(0),
        }
    }
}

impl BindingMetaRef {
    pub(crate) fn function_name(name: String) -> Self {
        let mut meta = Self::dynamic(name, false, false).owner[0].clone();
        meta.kind = BindingKind::FunctionName;
        meta.strict_write = WritePolicy::IgnoreSloppy;
        meta.initialization = InitPolicy::Uninitialized;
        Self {
            owner: Rc::new(BindingMetadata(vec![meta].into_boxed_slice())),
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
        !matches!(*self.0.state.borrow(), BindingState::Uninitialized)
    }

    pub(crate) fn read(&self) -> Result<Value, RuntimeError> {
        let state = self.0.state.borrow();
        match &*state {
            BindingState::Initialized(value) => Ok(value.clone()),
            BindingState::Uninitialized => Err(self.tdz_error()),
            BindingState::LazyArguments(_) => {
                drop(state);
                self.materialize_arguments();
                self.read()
            }
        }
    }

    pub(crate) fn initialize_arguments(
        &self,
        recipe: crate::function::ArgumentsRecipe,
    ) -> Result<(), RuntimeError> {
        let mut state = self.0.state.borrow_mut();
        if !matches!(*state, BindingState::Uninitialized) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: arguments binding already initialized".into(),
            });
        }
        *state = BindingState::LazyArguments(Rc::new(crate::function::LazyArguments::new(recipe)));
        Ok(())
    }

    /// A compiler-recorded parameter/body initialization copy. Lazy object
    /// identity is shared while the target binding remains independently mutable.
    pub(crate) fn copy_initialized_from(&self, source: &Self) -> Result<(), RuntimeError> {
        self.check_assignment(false)?;
        let source = source.0.state.borrow().clone();
        if matches!(source, BindingState::Uninitialized) {
            return Err(self.tdz_error());
        }
        *self.0.state.borrow_mut() = source;
        Ok(())
    }

    fn materialize_arguments(&self) {
        let recipe = {
            let mut state = self.0.state.borrow_mut();
            if !matches!(*state, BindingState::LazyArguments(_)) {
                return;
            }
            let BindingState::LazyArguments(recipe) =
                std::mem::replace(&mut *state, BindingState::Uninitialized)
            else {
                unreachable!("checked recipe")
            };
            recipe
        };
        // The factory reads only captured intrinsics/cells and installs own
        // descriptors. It performs no coercion, getter, trap, or JS callback.
        let value = recipe.value();
        *self.0.state.borrow_mut() = BindingState::Initialized(value);
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

    pub(crate) fn check_assignment(&self, strict: bool) -> Result<(), RuntimeError> {
        if !self.is_initialized() {
            return Err(self.tdz_error());
        }
        if self.metadata().strict_write == WritePolicy::Throw
            || strict && self.metadata().strict_write == WritePolicy::IgnoreSloppy
        {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: assignment to constant variable".to_owned(),
            });
        }
        Ok(())
    }

    pub(crate) fn assign(&self, value: Value, strict: bool) -> Result<(), RuntimeError> {
        self.check_assignment(strict)?;
        if self.metadata().strict_write == WritePolicy::IgnoreSloppy {
            return Ok(());
        }
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

    // The remaining VM marker-facing reads project TDZ without storing a
    // second value; checked declarative access uses read/initialize/assign.
    pub(crate) fn get(&self) -> Value {
        self.with_value(Clone::clone)
    }

    pub(crate) fn with_value<R>(&self, read: impl FnOnce(&Value) -> R) -> R {
        let state = self.0.state.borrow();
        match &*state {
            BindingState::Initialized(value) => read(value),
            BindingState::Uninitialized => read(&Value::Function(
                crate::Function::uninitialized_lexical_marker(),
            )),
            BindingState::LazyArguments(_) => {
                drop(state);
                self.materialize_arguments();
                self.with_value(read)
            }
        }
    }

    pub(crate) fn with_value_mut<R>(&self, mutate: impl FnOnce(&mut Value) -> R) -> R {
        let mut state = self.0.state.borrow_mut();
        match &mut *state {
            BindingState::Initialized(value) => mutate(value),
            BindingState::LazyArguments(_) => {
                drop(state);
                self.materialize_arguments();
                self.with_value_mut(mutate)
            }
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

    // Remaining raw VM/global publishers are retired in the final cell audit.
    pub(crate) fn set(&self, value: Value) {
        *self.0.state.borrow_mut() = if value.is_uninitialized_lexical_marker() {
            BindingState::Uninitialized
        } else {
            BindingState::Initialized(value)
        };
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

#[cfg(test)]
mod arguments_tests {
    use super::*;
    use crate::Function;

    fn arguments_cell(function: &Function) -> &BindingCell {
        function
            .upvalues
            .iter()
            .find(|cell| cell.metadata().kind == BindingKind::Arguments)
            .expect("closure retains its declaring arguments binding")
    }

    #[test]
    fn escaping_arguments_materialize_one_identity_on_first_read() {
        let Value::Function(read) =
            crate::eval("function make(value) { return () => arguments; } make(8);").unwrap()
        else {
            panic!("arguments reader");
        };
        let cell = arguments_cell(&read);
        assert!(cell.is_initialized());
        assert!(matches!(
            *cell.0.state.borrow(),
            BindingState::LazyArguments(_)
        ));
        let _ = format!("{cell:?}");
        assert!(matches!(
            *cell.0.state.borrow(),
            BindingState::LazyArguments(_)
        ));
        let first = cell.read().unwrap();
        assert!(matches!(first, Value::Object(_)));
        assert_eq!(cell.read(), Ok(first));
    }

    #[test]
    fn overwriting_an_unread_arguments_binding_discards_its_recipe() {
        let Value::Function(read) =
            crate::eval("function make(value) { return () => arguments; } make(8);").unwrap()
        else {
            panic!("arguments reader");
        };
        let cell = arguments_cell(&read);
        assert!(matches!(
            *cell.0.state.borrow(),
            BindingState::LazyArguments(_)
        ));
        cell.assign(Value::Number(7.0), false).unwrap();
        assert_eq!(cell.read(), Ok(Value::Number(7.0)));
    }

    #[test]
    fn suspended_arguments_keep_mapped_cells_before_first_read() {
        let Value::Array(values) = crate::eval(
            "function* g(value) { yield () => arguments; value = 9; } const it = g(2); [it.next().value, it];"
        ).unwrap() else { panic!("generator and reader"); };
        let values = values.to_vec();
        let Value::Function(read) = &values[0] else {
            panic!("reader");
        };
        let cell = arguments_cell(read);
        assert!(matches!(
            *cell.0.state.borrow(),
            BindingState::LazyArguments(_)
        ));
        let mut root_root = crate::context::HostRoot::new(read.creation_realm().clone());
        let mut root = root_root.context();
        let next = crate::property_value(values[1].clone(), "next", &mut root).unwrap();
        crate::call_function(next, values[1].clone(), Vec::new(), &mut root, false).unwrap();
        assert!(matches!(
            *cell.0.state.borrow(),
            BindingState::LazyArguments(_)
        ));
        let args = cell.read().unwrap();
        assert_eq!(
            crate::property_value(args, "0", &mut root),
            Ok(Value::Number(9.0))
        );
    }

    #[cfg(feature = "perf-counters")]
    #[test]
    fn bytecode_and_eval_overwrites_create_no_arguments_object() {
        for source in [
            "function f(value) { if (value < 0) return arguments; arguments = 5; return value; } f(2);",
            "function f(value) { eval('arguments = 5'); return value; } f(2);",
            "function f(value = 0) { var arguments; if (value < 0) return arguments; arguments = 5; return value; } f(2);",
            "function f(read = () => arguments) { var arguments; arguments = 5; return 2; } f();",
        ] {
            crate::reset_counters();
            assert_eq!(crate::eval(source), Ok(Value::Number(2.0)));
            let counters = crate::counters().unwrap();
            assert_eq!(counters.arguments_recipes, 1, "{source}");
            assert_eq!(counters.arguments_materializations, 0, "{source}");
        }
    }

    #[cfg(feature = "perf-counters")]
    #[test]
    fn copied_parameter_and_body_values_run_the_factory_once() {
        crate::reset_counters();
        assert_eq!(
            crate::eval(
                "function f(read = () => arguments) { var arguments; const own = arguments; return read() === own; } f();"
            ),
            Ok(Value::Boolean(true))
        );
        let counters = crate::counters().unwrap();
        assert_eq!(counters.arguments_recipes, 1);
        assert_eq!(counters.arguments_materializations, 1);
        crate::reset_counters();
        assert_eq!(
            crate::eval("function f(value) { return item => value + item; } f(2)(3);"),
            Ok(Value::Number(5.0))
        );
        let counters = crate::counters().unwrap();
        assert_eq!(counters.arguments_recipes, 0);
        assert_eq!(counters.arguments_materializations, 0);
    }
}
