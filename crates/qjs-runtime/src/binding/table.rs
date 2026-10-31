//! Canonical realm/module binding tables. Name-to-cell mappings may change;
//! cells retain declaration authority. Mapping generations invalidate cell-route
//! caches, never values. Only diagnostics may snapshot the table's values.
use super::BindingCell;
use crate::Value;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

#[derive(Clone, Default)]
pub(crate) struct DynamicBindings(Rc<DynamicBindingsInner>);

#[derive(Default)]
struct DynamicBindingsInner {
    map: RefCell<crate::value::name_hash::NameMap<String, BindingCell>>,
    /// Advanced by every change to which cell a name maps to -- a new name,
    /// a replaced or removed cell -- but not by writes through a cell. Equal
    /// generations mean every name still maps to the cell it did.
    generation: std::cell::Cell<u64>,
}

impl DynamicBindings {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// The map for a change that may alter which cell a name maps to.
    fn mapping_mut(
        &self,
    ) -> std::cell::RefMut<'_, crate::value::name_hash::NameMap<String, BindingCell>> {
        self.0.generation.set(
            self.0
                .generation
                .get()
                .checked_add(1)
                .unwrap_or_else(|| std::process::abort()),
        );
        self.0.map.borrow_mut()
    }

    /// See `DynamicBindingsInner::generation`.
    pub(crate) fn generation(&self) -> u64 {
        self.0.generation.get()
    }

    /// Whether `other` is this same environment.
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    pub(crate) fn from_values(values: HashMap<String, Value>) -> Self {
        Self(Rc::new(DynamicBindingsInner {
            map: RefCell::new(
                values
                    .into_iter()
                    .map(|(name, value)| {
                        (name.clone(), BindingCell::new_object_storage(name, value))
                    })
                    .collect(),
            ),
            generation: std::cell::Cell::new(0),
        }))
    }

    pub(crate) fn get(&self, name: &str) -> Option<Value> {
        self.0
            .map
            .borrow()
            .get(name)
            .and_then(BindingCell::value_if_initialized)
    }

    pub(crate) fn cell(&self, name: &str) -> Option<BindingCell> {
        self.0.map.borrow().get(name).cloned()
    }

    /// Publishes object/host data only; existing declaratives keep their policy.
    pub(crate) fn insert(&self, name: String, value: Value) -> Option<Value> {
        if let Some(binding) = self.0.map.borrow().get(&name) {
            let previous = binding.value_if_initialized();
            binding.publish_object_value(value);
            return previous;
        }
        self.mapping_mut()
            .insert(name.clone(), BindingCell::new_object_storage(name, value));
        None
    }

    pub(crate) fn insert_cell(&self, name: String, upvalue: BindingCell) {
        self.mapping_mut().insert(name, upvalue);
    }

    pub(crate) fn remove(&self, name: &str) -> Option<Value> {
        self.mapping_mut()
            .remove(name)
            .and_then(|binding| binding.value_if_initialized())
    }

    pub(crate) fn contains_key(&self, name: &str) -> bool {
        self.0.map.borrow().contains_key(name)
    }

    pub(crate) fn snapshot(&self) -> HashMap<String, Value> {
        self.0
            .map
            .borrow()
            .iter()
            .filter_map(|(name, binding)| {
                binding
                    .value_if_initialized()
                    .map(|value| (name.clone(), value))
            })
            .collect()
    }
}
