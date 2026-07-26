use std::{cell::Cell, cell::RefCell, collections::HashMap, fmt, rc::Rc};

use super::{CollectionKey, ObjectRef, Prototype, Value};

/// Set storage reference.
#[derive(Clone)]
pub struct SetRef(Rc<SetData>);

/// Insertion-ordered values plus a value index.
///
/// A `Set` must iterate in insertion order, so the values stay in a vector; a
/// deleted value becomes a hole rather than shifting every later index. The
/// index answers `has`/`add`/`delete` in constant time — the vector alone made
/// every operation a scan, so filling a set was quadratic.
struct SetData {
    entries: RefCell<Vec<Option<Value>>>,
    index: RefCell<HashMap<CollectionKey, usize>>,
    live: Cell<usize>,
    object: ObjectRef,
}

impl SetRef {
    pub(crate) fn new(prototype: Option<ObjectRef>) -> Self {
        Self::with_prototype_slot(prototype.map(Prototype::Object))
    }

    pub(crate) fn with_prototype_slot(prototype: Option<Prototype>) -> Self {
        let object = ObjectRef::with_prototype_slot(HashMap::new(), prototype);
        object.set_to_string_tag("Set");
        Self(Rc::new(SetData {
            entries: RefCell::new(Vec::new()),
            index: RefCell::new(HashMap::new()),
            live: Cell::new(0),
            object,
        }))
    }

    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    /// Identity of the shared storage, for hashing this reference as a key.
    pub(crate) fn address(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }

    pub(crate) fn object(&self) -> ObjectRef {
        self.0.object.clone()
    }

    pub(crate) fn len(&self) -> usize {
        self.0.live.get()
    }

    pub(crate) fn has(&self, value: &Value) -> bool {
        self.0
            .index
            .borrow()
            .contains_key(&CollectionKey::new(value))
    }

    pub(crate) fn add(&self, value: Value) {
        let value = canonical_set_value(value);
        let key = CollectionKey::new(&value);
        let mut index = self.0.index.borrow_mut();
        if index.contains_key(&key) {
            return;
        }
        let mut entries = self.0.entries.borrow_mut();
        index.insert(key, entries.len());
        entries.push(Some(value));
        self.0.live.set(self.0.live.get() + 1);
    }

    pub(crate) fn delete(&self, value: &Value) -> bool {
        let mut index = self.0.index.borrow_mut();
        let Some(position) = index.remove(&CollectionKey::new(value)) else {
            return false;
        };
        self.0.entries.borrow_mut()[position] = None;
        self.0.live.set(self.0.live.get() - 1);
        drop(index);
        self.compact_if_sparse();
        true
    }

    pub(crate) fn clear(&self) {
        self.0.entries.borrow_mut().clear();
        self.0.index.borrow_mut().clear();
        self.0.live.set(0);
    }

    pub(crate) fn values(&self) -> Vec<Value> {
        self.0.entries.borrow().iter().flatten().cloned().collect()
    }

    /// Rebuilds the storage once holes outnumber live values, so repeated
    /// add/delete cycles cannot grow the vector without bound.
    fn compact_if_sparse(&self) {
        let live = self.0.live.get();
        let mut entries = self.0.entries.borrow_mut();
        if entries.len() <= 8 || entries.len() <= live * 2 {
            return;
        }
        entries.retain(Option::is_some);
        let mut index = self.0.index.borrow_mut();
        index.clear();
        for (position, entry) in entries.iter().enumerate() {
            let Some(value) = entry else { continue };
            index.insert(CollectionKey::new(value), position);
        }
    }
}

impl fmt::Debug for SetRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SetRef")
            .field("len", &self.0.live.get())
            .finish()
    }
}

fn canonical_set_value(value: Value) -> Value {
    if matches!(value, Value::Number(number) if number == 0.0) {
        Value::Number(0.0)
    } else {
        value
    }
}
