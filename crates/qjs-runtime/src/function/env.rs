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

use crate::{Value, private::PrivateEnvironment};

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
    /// The lexical private-name environment active for this frame. This is
    /// separate from `\0home_object`: ordinary nested functions do not inherit
    /// `super`, but they do retain access to private names declared by enclosing
    /// classes.
    private_environment: Option<PrivateEnvironment>,
    /// The activation captured-env cell for the frame that produced this view.
    /// User callbacks use it to distinguish same-activation closure write-back
    /// from an unrelated caller's same-named local binding.
    activation_captured_env: Option<Rc<RefCell<HashMap<String, Value>>>>,
    /// The captured-env cell that supplied this frame's closure bindings.
    /// Sibling closures called through an intermediate user frame compare
    /// against this source so they can still write through shared outer
    /// bindings.
    captured_binding_source_env: Option<Rc<RefCell<HashMap<String, Value>>>>,
    /// The realm's dynamic-import host (module graph + resolver + active
    /// referrer), shared by `Rc::clone` into every frame and the job queue so a
    /// dynamic `import()` reached at any depth can load and cache modules. `None`
    /// when the code was not entered with a module host (the host then reports a
    /// dynamic-import-unsupported rejection).
    module_host: Option<crate::module::ModuleHostRef>,
}

impl std::fmt::Debug for CallEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallEnv")
            .field("realm", &self.realm)
            .field("locals", &self.locals)
            .field("module_host", &self.module_host.is_some())
            .field(
                "activation_captured_env",
                &self.activation_captured_env.is_some(),
            )
            .field(
                "captured_binding_source_env",
                &self.captured_binding_source_env.is_some(),
            )
            .finish()
    }
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
            private_environment: None,
            activation_captured_env: None,
            captured_binding_source_env: None,
            module_host: None,
        }
    }

    /// The realm's dynamic-import host, if one was installed.
    pub(crate) fn module_host(&self) -> Option<crate::module::ModuleHostRef> {
        self.module_host.clone()
    }

    /// Installs (or replaces) the dynamic-import host on this environment.
    pub(crate) fn set_module_host(&mut self, host: crate::module::ModuleHostRef) {
        self.module_host = Some(host);
    }

    /// Builds a standalone environment over a fresh, empty realm. Used by the
    /// no-context conversion helpers (`to_js_string`, `array_like_values`,
    /// `parse_json_text`) that run without a live VM; intrinsic lookups simply
    /// return `None`, matching the prior empty-`HashMap` behavior.
    pub(crate) fn detached() -> Self {
        Self {
            realm: Rc::new(RefCell::new(HashMap::new())),
            locals: HashMap::new(),
            private_environment: None,
            activation_captured_env: None,
            captured_binding_source_env: None,
            module_host: None,
        }
    }

    /// Wraps an owned flat map as a detached environment: the whole map becomes
    /// the realm layer with empty locals. Used by capture-env paths (function
    /// creation env, snapshots) that still carry a flat `HashMap`.
    pub(crate) fn from_map(map: HashMap<String, Value>) -> Self {
        Self {
            realm: Rc::new(RefCell::new(map)),
            locals: HashMap::new(),
            private_environment: None,
            activation_captured_env: None,
            captured_binding_source_env: None,
            module_host: None,
        }
    }

    /// Builds an environment over `realm` with the given frame locals.
    pub(crate) fn with_locals(realm: Realm, locals: HashMap<String, Value>) -> Self {
        Self {
            realm,
            locals,
            private_environment: None,
            activation_captured_env: None,
            captured_binding_source_env: None,
            module_host: None,
        }
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

    /// Returns the lexical private-name environment for this frame, if any.
    pub(crate) fn private_environment(&self) -> Option<PrivateEnvironment> {
        self.private_environment.clone()
    }

    /// Installs the lexical private-name environment for this frame.
    pub(crate) fn set_private_environment(&mut self, environment: Option<PrivateEnvironment>) {
        self.private_environment = environment;
    }

    /// Installs the activation captured-env identity for this frame.
    pub(crate) fn set_activation_captured_env(&mut self, env: Rc<RefCell<HashMap<String, Value>>>) {
        self.activation_captured_env = Some(env);
    }

    /// Returns the activation captured-env identity for this frame, if known.
    pub(crate) fn activation_captured_env(&self) -> Option<&Rc<RefCell<HashMap<String, Value>>>> {
        self.activation_captured_env.as_ref()
    }

    /// Installs the captured-env cell that supplied this frame's closure
    /// bindings.
    pub(crate) fn set_captured_binding_source_env(
        &mut self,
        env: Rc<RefCell<HashMap<String, Value>>>,
    ) {
        self.captured_binding_source_env = Some(env);
    }

    /// Returns the captured-env cell that supplied this frame's closure
    /// bindings, if known.
    pub(crate) fn captured_binding_source_env(
        &self,
    ) -> Option<&Rc<RefCell<HashMap<String, Value>>>> {
        self.captured_binding_source_env.as_ref()
    }

    pub(crate) fn captures_binding(&self, name: &str) -> bool {
        self.activation_captured_env
            .as_ref()
            .is_some_and(|activation| activation.borrow().contains_key(name))
            || self
                .captured_binding_source_env
                .as_ref()
                .is_some_and(|source| source.borrow().contains_key(name))
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

    /// Looks up `name` in the shared realm layer only.
    pub(crate) fn get_realm(&self, name: &str) -> Option<Value> {
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

    /// Defines `name` in the shared realm only if it is not already bound there.
    /// Used by global-binding initialization (script and indirect-eval scopes).
    pub(crate) fn realm_entry_or_insert(&self, name: String, value: Value) {
        self.realm.borrow_mut().entry(name).or_insert(value);
    }

    /// True if the shared realm cell binds `name`.
    pub(crate) fn realm_contains(&self, name: &str) -> bool {
        self.realm.borrow().contains_key(name)
    }

    /// Removes a frame-local binding.
    pub(crate) fn remove(&mut self, name: &str) -> Option<Value> {
        self.locals.remove(name)
    }

    /// Mutates an existing frame-local binding in place, if present.
    pub(crate) fn get_local(&self, name: &str) -> Option<Value> {
        self.locals.get(name).cloned()
    }

    pub(crate) fn get_local_mut(&mut self, name: &str) -> Option<&mut Value> {
        self.locals.get_mut(name)
    }

    /// A snapshot of just the frame locals layer.
    pub(crate) fn snapshot_locals(&self) -> HashMap<String, Value> {
        self.locals.clone()
    }

    /// A fully materialized map merging the shared realm and this frame's
    /// locals (locals shadow realm). Used by the legacy generator/async capture
    /// paths that still snapshot a flat `HashMap`; prefer keeping the realm
    /// shared where possible.
    pub(crate) fn to_flat_map(&self) -> HashMap<String, Value> {
        let mut map = self.realm.borrow().clone();
        for (name, value) in &self.locals {
            map.insert(name.clone(), value.clone());
        }
        map
    }

    /// Builds a `CallEnv` over the same realm, replacing the locals layer.
    pub(crate) fn with_frame_locals(&self, locals: HashMap<String, Value>) -> Self {
        Self {
            realm: Rc::clone(&self.realm),
            locals,
            private_environment: self.private_environment.clone(),
            activation_captured_env: self.activation_captured_env.clone(),
            captured_binding_source_env: self.captured_binding_source_env.clone(),
            module_host: self.module_host.clone(),
        }
    }
}
