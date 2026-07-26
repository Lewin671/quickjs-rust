use std::{cell::Cell, cell::RefCell, collections::HashMap, fmt, rc::Rc};

use super::{CollectionKey, ObjectRef, Prototype, Value};

/// Map storage reference.
#[derive(Clone)]
pub struct MapRef(Rc<MapData>);

/// Insertion-ordered entries plus a key index.
///
/// A `Map` must iterate in insertion order, so the entries stay in a vector;
/// a deleted entry becomes a hole rather than shifting every later index. The
/// index answers `get`/`has`/`set`/`delete` in constant time — the vector alone
/// made every operation a scan, so filling a map was quadratic.
struct MapData {
    entries: RefCell<Vec<Option<(Value, Value)>>>,
    index: RefCell<HashMap<CollectionKey, usize>>,
    live: Cell<usize>,
    object: ObjectRef,
}

impl MapRef {
    pub(crate) fn new(prototype: Option<ObjectRef>) -> Self {
        Self::with_prototype_slot(prototype.map(Prototype::Object))
    }

    pub(crate) fn with_prototype_slot(prototype: Option<Prototype>) -> Self {
        let object = ObjectRef::with_prototype_slot(HashMap::new(), prototype);
        object.set_to_string_tag("Map");
        Self(Rc::new(MapData {
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

    pub(crate) fn get(&self, key: &Value) -> Option<Value> {
        let position = *self.0.index.borrow().get(&CollectionKey::new(key))?;
        self.0.entries.borrow()[position]
            .as_ref()
            .map(|(_, value)| value.clone())
    }

    pub(crate) fn has(&self, key: &Value) -> bool {
        self.0.index.borrow().contains_key(&CollectionKey::new(key))
    }

    pub(crate) fn set(&self, key: Value, value: Value) {
        let key = canonical_map_key(key);
        let mut index = self.0.index.borrow_mut();
        let mut entries = self.0.entries.borrow_mut();
        if let Some(position) = index.get(&CollectionKey::new(&key)) {
            if let Some(entry) = entries[*position].as_mut() {
                entry.1 = value;
                return;
            }
        }
        index.insert(CollectionKey::new(&key), entries.len());
        entries.push(Some((key, value)));
        self.0.live.set(self.0.live.get() + 1);
    }

    pub(crate) fn delete(&self, key: &Value) -> bool {
        let mut index = self.0.index.borrow_mut();
        let Some(position) = index.remove(&CollectionKey::new(key)) else {
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

    pub(crate) fn entries(&self) -> Vec<(Value, Value)> {
        self.0.entries.borrow().iter().flatten().cloned().collect()
    }

    /// Rebuilds the storage once holes outnumber live entries, so repeated
    /// insert/delete cycles cannot grow the vector without bound.
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
            let Some((key, _)) = entry else { continue };
            index.insert(CollectionKey::new(key), position);
        }
    }
}

impl fmt::Debug for MapRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("MapRef")
            .field("len", &self.0.live.get())
            .finish()
    }
}

fn canonical_map_key(key: Value) -> Value {
    if matches!(key, Value::Number(value) if value == 0.0) {
        Value::Number(0.0)
    } else {
        key
    }
}
