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

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
};

use crate::{Function, Value, private::PrivateEnvironment};

/// The shared realm binding table: intrinsics plus the script's true globals.
pub(crate) type Realm = Rc<RefCell<HashMap<String, Value>>>;
pub(crate) type GlobalLexicalBindings = Rc<RefCell<HashSet<String>>>;
pub(crate) type GlobalLexicalValues = Rc<RefCell<HashMap<String, Value>>>;
pub(crate) type ImmutableLexicalBindings = Rc<RefCell<HashSet<String>>>;
pub(crate) type ModuleImports = HashMap<String, (Realm, String)>;

/// A two-layer environment view: a shared realm cell plus this frame's locals.
///
/// Cloning a `CallEnv` shares the realm by `Rc::clone` and copies only the
/// (small) frame locals, so a per-call clone no longer copies the realm.
#[derive(Clone)]
#[allow(dead_code)]
pub(crate) struct CallEnv {
    realm: Realm,
    global_lexical_bindings: GlobalLexicalBindings,
    global_lexical_values: GlobalLexicalValues,
    expose_global_lexical_values: bool,
    immutable_lexical_bindings: ImmutableLexicalBindings,
    locals: HashMap<String, Value>,
    catch_bindings: HashSet<String>,
    /// The immutable name binding of a named function expression, for this
    /// frame only. Assigning to it is a silent no-op in sloppy mode and a
    /// TypeError in strict mode (unless a parameter/`var`/lexical shadows it,
    /// in which case the assignment resolves to that local slot instead and
    /// never consults this field). New ordinary function frames reset this;
    /// lexical-this functions inherit it explicitly from their enclosing frame.
    immutable_function_name: Option<String>,
    direct_eval_var_conflicts: HashSet<String>,
    /// The lexical private-name environment active for this frame. This is
    /// separate from `\0home_object`: ordinary nested functions do not inherit
    /// `super`, but they do retain access to private names declared by enclosing
    /// classes.
    private_environment: Option<PrivateEnvironment>,
    /// Dynamic with-object chain active at a direct eval call site. This is
    /// intentionally not part of ordinary function environments; the VM sets it
    /// only while invoking the intrinsic eval function as a direct eval.
    direct_eval_with_stack: Vec<Value>,
    /// The activation captured-env cell for the frame that produced this view.
    /// User callbacks use it to distinguish same-activation closure write-back
    /// from an unrelated caller's same-named local binding.
    activation_captured_env: Option<Rc<RefCell<HashMap<String, Value>>>>,
    /// The captured-env cell that supplied this frame's closure bindings.
    /// Sibling closures called through an intermediate user frame compare
    /// against this source so they can still write through shared outer
    /// bindings.
    captured_binding_source_env: Option<Rc<RefCell<HashMap<String, Value>>>>,
    parameter_captured_envs: Vec<Rc<RefCell<HashMap<String, Value>>>>,
    /// The realm's dynamic-import host (module graph + resolver + active
    /// referrer), shared by `Rc::clone` into every frame and the job queue so a
    /// dynamic `import()` reached at any depth can load and cache modules. `None`
    /// when the code was not entered with a module host (the host then reports a
    /// dynamic-import-unsupported rejection).
    module_host: Option<crate::module::ModuleHostRef>,
    /// Live module imports keyed by this module's local import binding name.
    /// Each entry points at the exporting module's shared lexical export cell
    /// and the local binding name that backs that export.
    module_imports: ModuleImports,
    /// The Test262 `$262.agent` execution context for this thread's agent,
    /// threaded like `module_host` so native `Atomics`/`$262.agent` hooks reach
    /// it. `None` outside the agents harness. Gated so the default build's
    /// struct layout and per-call clone cost are unchanged.
    #[cfg(feature = "agents")]
    agent_context: Option<crate::agent::AgentContextRef>,
}

impl std::fmt::Debug for CallEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallEnv")
            .field("realm", &self.realm)
            .field(
                "global_lexical_bindings",
                &self.global_lexical_bindings.borrow().len(),
            )
            .field(
                "global_lexical_values",
                &self.global_lexical_values.borrow().len(),
            )
            .field(
                "expose_global_lexical_values",
                &self.expose_global_lexical_values,
            )
            .field(
                "immutable_lexical_bindings",
                &self.immutable_lexical_bindings.borrow().len(),
            )
            .field("locals", &self.locals)
            .field("catch_bindings", &self.catch_bindings)
            .field("direct_eval_var_conflicts", &self.direct_eval_var_conflicts)
            .field("direct_eval_with_stack", &self.direct_eval_with_stack.len())
            .field("module_host", &self.module_host.is_some())
            .field(
                "activation_captured_env",
                &self.activation_captured_env.is_some(),
            )
            .field(
                "captured_binding_source_env",
                &self.captured_binding_source_env.is_some(),
            )
            .field(
                "parameter_captured_envs",
                &self.parameter_captured_envs.len(),
            )
            .field("module_imports", &self.module_imports.keys())
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
            global_lexical_bindings: Rc::new(RefCell::new(HashSet::new())),
            global_lexical_values: Rc::new(RefCell::new(HashMap::new())),
            expose_global_lexical_values: false,
            immutable_lexical_bindings: Rc::new(RefCell::new(HashSet::new())),
            locals: HashMap::new(),
            catch_bindings: HashSet::new(),
            immutable_function_name: None,
            direct_eval_var_conflicts: HashSet::new(),
            private_environment: None,
            direct_eval_with_stack: Vec::new(),
            activation_captured_env: None,
            captured_binding_source_env: None,
            parameter_captured_envs: Vec::new(),
            module_host: None,
            module_imports: HashMap::new(),
            #[cfg(feature = "agents")]
            agent_context: None,
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

    /// The Test262 `$262.agent` context for this thread's agent, if installed.
    #[cfg(feature = "agents")]
    pub(crate) fn agent_context(&self) -> Option<crate::agent::AgentContextRef> {
        self.agent_context.clone()
    }

    /// Installs the `$262.agent` context on this environment. Threaded into
    /// every frame the VM builds so native hooks reach it.
    #[cfg(feature = "agents")]
    pub(crate) fn set_agent_context(&mut self, context: crate::agent::AgentContextRef) {
        self.agent_context = Some(context);
    }

    /// Installs a live module import binding.
    pub(crate) fn set_module_import(
        &mut self,
        local_name: String,
        exported_bindings: Realm,
        exported_local_name: String,
    ) {
        self.module_imports
            .insert(local_name, (exported_bindings, exported_local_name));
    }

    /// Reads the current value of a live module import binding.
    pub(crate) fn module_import_value(&self, local_name: &str) -> Option<Value> {
        let (bindings, exported_local_name) = self.module_imports.get(local_name)?;
        Some(
            bindings
                .borrow()
                .get(exported_local_name)
                .cloned()
                .unwrap_or_else(|| Value::Function(Function::uninitialized_lexical_marker())),
        )
    }

    pub(crate) fn has_module_import(&self, local_name: &str) -> bool {
        self.module_imports.contains_key(local_name)
    }

    pub(crate) fn module_imports(&self) -> ModuleImports {
        self.module_imports.clone()
    }

    pub(crate) fn set_module_imports(&mut self, imports: ModuleImports) {
        self.module_imports = imports;
    }

    /// Builds a standalone environment over a fresh, empty realm. Used by the
    /// no-context conversion helpers (`to_js_string`, `array_like_values`,
    /// `parse_json_text`) that run without a live VM; intrinsic lookups simply
    /// return `None`, matching the prior empty-`HashMap` behavior.
    pub(crate) fn detached() -> Self {
        Self {
            realm: Rc::new(RefCell::new(HashMap::new())),
            global_lexical_bindings: Rc::new(RefCell::new(HashSet::new())),
            global_lexical_values: Rc::new(RefCell::new(HashMap::new())),
            expose_global_lexical_values: false,
            immutable_lexical_bindings: Rc::new(RefCell::new(HashSet::new())),
            locals: HashMap::new(),
            catch_bindings: HashSet::new(),
            immutable_function_name: None,
            direct_eval_var_conflicts: HashSet::new(),
            private_environment: None,
            direct_eval_with_stack: Vec::new(),
            activation_captured_env: None,
            captured_binding_source_env: None,
            parameter_captured_envs: Vec::new(),
            module_host: None,
            module_imports: HashMap::new(),
            #[cfg(feature = "agents")]
            agent_context: None,
        }
    }

    /// Wraps an owned flat map as a detached environment: the whole map becomes
    /// the realm layer with empty locals. Used by capture-env paths (function
    /// creation env, snapshots) that still carry a flat `HashMap`.
    pub(crate) fn from_map(map: HashMap<String, Value>) -> Self {
        Self {
            realm: Rc::new(RefCell::new(map)),
            global_lexical_bindings: Rc::new(RefCell::new(HashSet::new())),
            global_lexical_values: Rc::new(RefCell::new(HashMap::new())),
            expose_global_lexical_values: false,
            immutable_lexical_bindings: Rc::new(RefCell::new(HashSet::new())),
            locals: HashMap::new(),
            catch_bindings: HashSet::new(),
            immutable_function_name: None,
            direct_eval_var_conflicts: HashSet::new(),
            private_environment: None,
            direct_eval_with_stack: Vec::new(),
            activation_captured_env: None,
            captured_binding_source_env: None,
            parameter_captured_envs: Vec::new(),
            module_host: None,
            module_imports: HashMap::new(),
            #[cfg(feature = "agents")]
            agent_context: None,
        }
    }

    /// Builds an environment over `realm` with the given frame locals.
    pub(crate) fn with_locals(realm: Realm, locals: HashMap<String, Value>) -> Self {
        Self {
            realm,
            global_lexical_bindings: Rc::new(RefCell::new(HashSet::new())),
            global_lexical_values: Rc::new(RefCell::new(HashMap::new())),
            expose_global_lexical_values: false,
            immutable_lexical_bindings: Rc::new(RefCell::new(HashSet::new())),
            locals,
            catch_bindings: HashSet::new(),
            immutable_function_name: None,
            direct_eval_var_conflicts: HashSet::new(),
            private_environment: None,
            direct_eval_with_stack: Vec::new(),
            activation_captured_env: None,
            captured_binding_source_env: None,
            parameter_captured_envs: Vec::new(),
            module_host: None,
            module_imports: HashMap::new(),
            #[cfg(feature = "agents")]
            agent_context: None,
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

    /// Builds an empty frame that shares this environment's realm metadata.
    pub(crate) fn empty_frame(&self) -> Self {
        self.with_frame_locals(HashMap::new())
    }

    /// Builds an indirect-eval global frame. It has no caller locals, but it
    /// can read Script top-level lexical bindings through the global lexical
    /// environment instead of the global object.
    pub(crate) fn indirect_eval_frame(&self) -> Self {
        let mut env = self.empty_frame();
        env.expose_global_lexical_values = true;
        env
    }

    pub(crate) fn mark_global_lexical_binding(&self, name: String) {
        self.global_lexical_bindings.borrow_mut().insert(name);
    }

    pub(crate) fn is_global_lexical_binding(&self, name: &str) -> bool {
        self.global_lexical_bindings.borrow().contains(name)
    }

    pub(crate) fn set_global_lexical_value(&self, name: String, value: Value) {
        self.global_lexical_values.borrow_mut().insert(name, value);
    }

    pub(crate) fn mark_immutable_lexical_binding(&self, name: String) {
        self.immutable_lexical_bindings.borrow_mut().insert(name);
    }

    pub(crate) fn is_immutable_lexical_binding(&self, name: &str) -> bool {
        self.immutable_lexical_bindings.borrow().contains(name)
    }

    pub(crate) fn mark_catch_binding(&mut self, name: String) {
        self.catch_bindings.insert(name);
    }

    pub(crate) fn unmark_catch_binding(&mut self, name: &str) {
        self.catch_bindings.remove(name);
    }

    pub(crate) fn is_catch_binding(&self, name: &str) -> bool {
        self.catch_bindings.contains(name)
    }

    /// Marks `name` as this frame's immutable named-function-expression binding.
    pub(crate) fn set_immutable_function_name(&mut self, name: String) {
        self.immutable_function_name = Some(name);
    }

    /// Returns true when `name` is this frame's immutable function-expression
    /// name binding, so an assignment to it must be rejected (a silent no-op in
    /// sloppy mode, a TypeError in strict mode).
    pub(crate) fn is_immutable_function_name(&self, name: &str) -> bool {
        self.immutable_function_name.as_deref() == Some(name)
    }

    pub(crate) fn immutable_function_name(&self) -> Option<&str> {
        self.immutable_function_name.as_deref()
    }

    pub(crate) fn clear_direct_eval_var_conflicts(&mut self) {
        self.direct_eval_var_conflicts.clear();
    }

    pub(crate) fn mark_direct_eval_var_conflict(&mut self, name: String) {
        self.direct_eval_var_conflicts.insert(name);
    }

    pub(crate) fn is_direct_eval_var_conflict(&self, name: &str) -> bool {
        self.direct_eval_var_conflicts.contains(name)
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

    pub(crate) fn set_direct_eval_with_stack(&mut self, with_stack: Vec<Value>) {
        self.direct_eval_with_stack = with_stack;
    }

    pub(crate) fn direct_eval_with_stack(&self) -> Vec<Value> {
        self.direct_eval_with_stack.clone()
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

    pub(crate) fn set_parameter_captured_envs(
        &mut self,
        envs: Vec<Rc<RefCell<HashMap<String, Value>>>>,
    ) {
        self.parameter_captured_envs = envs;
    }

    pub(crate) fn parameter_captured_envs(&self) -> &[Rc<RefCell<HashMap<String, Value>>>] {
        &self.parameter_captured_envs
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
        if let Some(value) = self.realm.borrow().get(name).cloned() {
            return Some(value);
        }
        if self.expose_global_lexical_values {
            return self.global_lexical_values.borrow().get(name).cloned();
        }
        None
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
            global_lexical_bindings: Rc::clone(&self.global_lexical_bindings),
            global_lexical_values: Rc::clone(&self.global_lexical_values),
            expose_global_lexical_values: false,
            immutable_lexical_bindings: Rc::clone(&self.immutable_lexical_bindings),
            locals,
            catch_bindings: self.catch_bindings.clone(),
            // A new call frame's function-name binding is set explicitly by
            // function_env for the callee; it is never inherited from the caller.
            immutable_function_name: None,
            direct_eval_var_conflicts: self.direct_eval_var_conflicts.clone(),
            private_environment: self.private_environment.clone(),
            direct_eval_with_stack: self.direct_eval_with_stack.clone(),
            activation_captured_env: self.activation_captured_env.clone(),
            captured_binding_source_env: self.captured_binding_source_env.clone(),
            parameter_captured_envs: self.parameter_captured_envs.clone(),
            module_host: self.module_host.clone(),
            module_imports: self.module_imports.clone(),
            #[cfg(feature = "agents")]
            agent_context: self.agent_context.clone(),
        }
    }

    /// Builds a new view over the current execution frame. Unlike a new
    /// ordinary function call frame, this preserves the named-function-expression
    /// immutable name marker for direct eval and lexical arrow calls.
    pub(crate) fn with_current_frame_locals(&self, locals: HashMap<String, Value>) -> Self {
        let mut env = self.with_frame_locals(locals);
        env.immutable_function_name = self.immutable_function_name.clone();
        env
    }
}
