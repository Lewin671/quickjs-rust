//! The call-frame environment view threaded through the runtime.
//!
//! Historically every builtin and every call boundary received a fully
//! materialized `HashMap<String, Value>` holding the realm intrinsics, the true
//! globals, and the frame's own locals, rebuilt by cloning on each call. That
//! clone dominated call cost (see `tasks/T011-call-performance.md`).
//!
//! [`CallEnv`] replaces that flat map with a two-layer view:
//!
//! - `realm`: an `Rc<RefCell<HashMap>>` shared by `Rc::clone` into every frame.
//!   It owns the runtime intrinsics and the script's true global bindings.
//!   Sharing the cell means a reassigned builtin (`Array = X`) is visible
//!   everywhere for free, and a sloppy-mode global write is seen by every frame
//!   without a write-back scan.
//! - `locals`: the current frame's own bindings — `this`, `arguments`,
//!   parameters, captured closure variables, and caller-scope bindings the
//!   callee references. Only this layer is cloned per call.
//!
//! Reads check `locals` first, then take a *short* `realm` borrow and clone the
//! value out. A borrow is never held across a call back into user code
//! (getters, setters, Proxy traps, `valueOf`/`toString`, iterators): callers
//! copy the needed value out, drop the borrow, then call.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::Value;

/// The shared realm binding table: intrinsics plus the script's true globals.
pub(crate) type Realm = Rc<RefCell<HashMap<String, Value>>>;

/// A two-layer environment view: a shared realm cell plus this frame's locals.
///
/// Cloning a `CallEnv` shares the realm by `Rc::clone` and copies only the
/// (small) frame locals, so a per-call clone no longer copies the realm.
#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct CallEnv {
    realm: Realm,
    locals: HashMap<String, Value>,
}

// The realm-cell migration (tasks/T011-call-performance.md) wires these in
// across the crate; until then the foundation type is intentionally unused.
#[allow(dead_code)]
impl CallEnv {
    /// Builds an environment over `realm` with empty locals.
    pub(crate) fn new(realm: Realm) -> Self {
        Self {
            realm,
            locals: HashMap::new(),
        }
    }

    /// Builds an environment over `realm` with the given frame locals.
    pub(crate) fn with_locals(realm: Realm, locals: HashMap<String, Value>) -> Self {
        Self { realm, locals }
    }

    /// The shared realm cell, for sharing into a new frame or snapshot.
    pub(crate) fn realm(&self) -> &Realm {
        &self.realm
    }

    /// A clone of the realm `Rc` (shared cell, not a deep copy).
    pub(crate) fn realm_rc(&self) -> Realm {
        Rc::clone(&self.realm)
    }

    /// This frame's own locals layer.
    pub(crate) fn locals(&self) -> &HashMap<String, Value> {
        &self.locals
    }

    /// This frame's own locals layer, mutably.
    pub(crate) fn locals_mut(&mut self) -> &mut HashMap<String, Value> {
        &mut self.locals
    }

    /// Consumes the view, returning the frame locals.
    pub(crate) fn into_locals(self) -> HashMap<String, Value> {
        self.locals
    }

    /// Looks up `name`: frame locals first, then a short realm borrow. Returns
    /// an owned value because a value behind the realm `RefCell` cannot be
    /// handed out by reference.
    pub(crate) fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.locals.get(name) {
            return Some(value.clone());
        }
        self.realm.borrow().get(name).cloned()
    }

    /// True if `name` is bound in either layer.
    pub(crate) fn contains_key(&self, name: &str) -> bool {
        self.locals.contains_key(name) || self.realm.borrow().contains_key(name)
    }

    /// Inserts a frame-local binding (`this`, params, captures, caller-scope
    /// bindings). The VM write-back routes these to real locals-or-globals via
    /// `local_slot`. Realm/global definitions use [`CallEnv::insert_realm`].
    pub(crate) fn insert(&mut self, name: String, value: Value) -> Option<Value> {
        self.locals.insert(name, value)
    }

    /// Inserts directly into the shared realm cell (builtin install and global
    /// definition). Visible to every frame sharing the realm.
    pub(crate) fn insert_realm(&self, name: String, value: Value) -> Option<Value> {
        self.realm.borrow_mut().insert(name, value)
    }

    /// Removes a frame-local binding.
    pub(crate) fn remove(&mut self, name: &str) -> Option<Value> {
        self.locals.remove(name)
    }

    /// Mutates an existing frame-local binding in place, if present.
    pub(crate) fn get_local_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.locals.get_mut(name)
    }
}
