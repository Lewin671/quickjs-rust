//! A remembered add: the layout an object had before an operation created
//! one plain data property, and the layout that creation led to.
//!
//! Layouts are shared tokens, so an object whose layout is the remembered
//! source provably lacks the key, and appending it as an enumerable,
//! writable, configurable data property provably leads to the remembered
//! target. A site that adds the same key to many fresh objects -- `this.x =
//! x` in a constructor, a class field -- replays the add by identity: no
//! probe, no edge search. Only shapes are remembered (a dictionary is edited
//! in place), only over a descriptor payload, and only for keys whose
//! creation has no side effect beyond the append (no array index, not
//! `globalThis`). A replay checks extensibility itself; whether the receiver
//! is ordinary for the key, and whether the prototype chain lets the
//! operation create it, stay the caller's proof.

use std::cell::RefCell;
use std::rc::Rc;

use super::storage::{Payload, PropertyStorage};
use super::{ObjectRef, is_array_index_key};
use crate::Property;
use crate::property::layout::{Attributes, Layout};
use crate::value::Value;

#[derive(Clone, Debug)]
pub(crate) struct AddTransition {
    from: Layout,
    to: Layout,
}

/// The receiver's layout before an add that may be remembered.
pub(crate) struct AddSource(Option<Layout>);

/// One site's last remembered add.
#[derive(Clone, Debug, Default)]
pub(crate) struct AddCache(RefCell<Option<AddTransition>>);

fn source_of(storage: &PropertyStorage, key: &str) -> AddSource {
    let shape = matches!(storage.payload, Payload::Slots(_))
        && !storage.layout.is_dictionary()
        && !is_array_index_key(key)
        && key != "globalThis";
    AddSource(shape.then(|| Rc::clone(&storage.layout)))
}

fn transition_of(storage: &PropertyStorage, source: AddSource, key: &str) -> Option<AddTransition> {
    let from = source.0?;
    let to = &storage.layout;
    if to.is_dictionary() || !matches!(storage.payload, Payload::Slots(_)) {
        return None;
    }
    let field = to.field(from.len())?;
    (to.len() == from.len() + 1
        && field.attributes == Attributes::DEFAULT
        && field.text().as_ref() == key
        && to.fields()[..from.len()] == from.fields()[..])
        .then(|| AddTransition {
            from,
            to: Rc::clone(to),
        })
}

/// Appends `value` along `transition` when `storage` is on its source.
#[inline]
fn replay(storage: &mut PropertyStorage, transition: &AddTransition, value: &Value) -> bool {
    if !Rc::ptr_eq(&storage.layout, &transition.from) {
        return false;
    }
    let Payload::Slots(properties) = &mut storage.payload else {
        return false;
    };
    properties.push(Property::enumerable(value.clone()));
    storage.layout = Rc::clone(&transition.to);
    true
}

impl AddCache {
    /// Replays the remembered add on `storage`; `false` changes nothing.
    #[inline]
    pub(super) fn replay_in(&self, storage: &mut PropertyStorage, value: &Value) -> bool {
        self.0
            .borrow()
            .as_ref()
            .is_some_and(|transition| replay(storage, transition, value))
    }

    /// Remembers the add from `source` that `storage` now reflects.
    pub(super) fn record_in(&self, storage: &PropertyStorage, source: AddSource, key: &str) {
        if let Some(transition) = transition_of(storage, source, key) {
            *self.0.borrow_mut() = Some(transition);
        }
    }

    /// [`ObjectRef::add_by_transition`] with the remembered add.
    #[inline]
    pub(crate) fn replay(&self, object: &ObjectRef, value: &Value) -> bool {
        self.0
            .borrow()
            .as_ref()
            .is_some_and(|transition| object.add_by_transition(transition, value))
    }

    /// Remembers the add from `source` that `object` now reflects.
    pub(crate) fn record(&self, object: &ObjectRef, source: AddSource, key: &str) {
        self.record_in(&object.0.properties.borrow(), source, key);
    }

    /// Whether `object` is on the remembered add's source layout.
    #[inline]
    pub(crate) fn is_source_of(&self, object: &ObjectRef) -> bool {
        self.0.borrow().as_ref().is_some_and(|transition| {
            Rc::ptr_eq(&object.0.properties.borrow().layout, &transition.from)
        })
    }
}

impl ObjectRef {
    /// The layout to remember as the source of an add of `key`.
    pub(crate) fn add_source(&self, key: &str) -> AddSource {
        source_of(&self.0.properties.borrow(), key)
    }

    /// Replays `transition` on this object, appending `value`; `false`
    /// leaves the object untouched for the general path.
    pub(crate) fn add_by_transition(&self, transition: &AddTransition, value: &Value) -> bool {
        if self.0.array_prototype_exotic.get() || !self.0.extensible.get() {
            return false;
        }
        if !replay(&mut self.0.properties.borrow_mut(), transition, value) {
            return false;
        }
        self.bump_property_revision();
        true
    }
}

impl PropertyStorage {
    /// [`source_of`] for a segment installer holding the borrow.
    pub(super) fn add_source(&self, key: &str) -> AddSource {
        source_of(self, key)
    }
}
