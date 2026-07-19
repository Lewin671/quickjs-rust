//! The call-frame environment view threaded through the runtime.
//!
//! Ordinary JavaScript bindings do not live here: bytecode frames keep
//! slot-indexed values and captured slots share indexed [`Upvalue`] cells.
//! [`CallEnv`] carries only the cross-call runtime context that is still
//! naturally name-addressed:
//!
//! - `realm` is shared into every frame. It owns intrinsics, true global
//!   bindings, and the lazily allocated cells for captured globals, so global
//!   writes are immediately visible without copying or write-back.
//! - `frame_bindings` is a small cell vector for call metadata and native or
//!   dynamic compatibility consumers. Ordinary user-function setup inserts
//!   directly into it and never builds a per-call locals `HashMap`.
//! - `deopt_bindings` is the explicit name-to-cell map allocated only for
//!   direct `eval`, `with`, or a dynamic scope inherited from them.
//!
//! Reads take only short borrows and clone values out. A borrow is never held
//! across a call back into user code
//! (getters, setters, Proxy traps, `valueOf`/`toString`, iterators): callers
//! copy the needed value out, drop the borrow, then call.

use std::{
    cell::{Ref, RefCell, RefMut},
    collections::{HashMap, HashSet},
    fmt,
    ops::Deref,
    rc::Rc,
};

use crate::{Function, ObjectRef, Value, function::Upvalue, private::PrivateEnvironment};

const DYNAMIC_FUNCTION_REALM_GLOBAL: &str = "__quickjsRustDynamicFunctionRealm";

/// Shared realm state: intrinsics, true globals, and captured-global cells.
pub(crate) struct RealmState {
    bindings: RefCell<HashMap<String, Value>>,
    binding_cells: DynamicBindings,
    global_this: Option<Value>,
    dynamic_function_realm_global: RefCell<Option<ObjectRef>>,
}

impl RealmState {
    fn new(bindings: HashMap<String, Value>) -> Self {
        let global_this = bindings.get(crate::GLOBAL_THIS_BINDING).cloned();
        let dynamic_function_realm_global =
            bindings
                .get(DYNAMIC_FUNCTION_REALM_GLOBAL)
                .and_then(|value| match value {
                    Value::Object(global) => Some(global.clone()),
                    _ => None,
                });
        Self {
            bindings: RefCell::new(bindings),
            binding_cells: DynamicBindings::new(),
            global_this,
            dynamic_function_realm_global: RefCell::new(dynamic_function_realm_global),
        }
    }

    pub(crate) fn borrow(&self) -> Ref<'_, HashMap<String, Value>> {
        self.bindings.borrow()
    }

    pub(crate) fn borrow_mut(&self) -> RefMut<'_, HashMap<String, Value>> {
        self.bindings.borrow_mut()
    }

    pub(crate) fn global_this(&self) -> Option<Value> {
        self.global_this.clone()
    }

    /// Returns the internal global object used by dynamically constructed
    /// functions without hashing its private binding name for every closure.
    pub(crate) fn dynamic_function_realm_global(&self) -> Option<ObjectRef> {
        self.dynamic_function_realm_global.borrow().clone()
    }

    fn sync_dynamic_function_realm_binding(&self, name: &str, value: Option<&Value>) {
        if name != DYNAMIC_FUNCTION_REALM_GLOBAL {
            return;
        }
        *self.dynamic_function_realm_global.borrow_mut() = value.and_then(|value| match value {
            Value::Object(global) => Some(global.clone()),
            _ => None,
        });
    }

    /// Rebuilds cached internal metadata after a bulk initialization path has
    /// edited the binding map directly.
    pub(crate) fn refresh_dynamic_function_realm_global(&self) {
        let global = self
            .bindings
            .borrow()
            .get(DYNAMIC_FUNCTION_REALM_GLOBAL)
            .and_then(|value| match value {
                Value::Object(global) => Some(global.clone()),
                _ => None,
            });
        *self.dynamic_function_realm_global.borrow_mut() = global;
    }
}

impl fmt::Debug for RealmState {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("RealmState")
            .field("bindings", &self.bindings.borrow())
            .finish_non_exhaustive()
    }
}

pub(crate) type Realm = Rc<RealmState>;

pub(crate) fn new_realm(bindings: HashMap<String, Value>) -> Realm {
    Rc::new(RealmState::new(bindings))
}
pub(crate) type GlobalLexicalBindings = Rc<RefCell<HashSet<String>>>;
pub(crate) type GlobalLexicalValues = Rc<RefCell<HashMap<String, Value>>>;
pub(crate) type ImmutableLexicalBindings = Rc<RefCell<HashSet<String>>>;
/// Structurally immutable module-import routing shared by function objects and
/// call frames. Module setup uses copy-on-write so environment clones preserve
/// their previous routing table while ordinary scripts and nested functions
/// retain only one pointer-sized empty map.
pub(crate) type ModuleImports = Rc<HashMap<String, (DynamicBindings, String)>>;

#[derive(Clone)]
enum FrameBindingValue {
    Direct(Value),
    Cell(Upvalue),
}

impl FrameBindingValue {
    fn get(&self) -> Value {
        match self {
            Self::Direct(value) => value.clone(),
            Self::Cell(value) => value.get(),
        }
    }

    fn with_value<R>(&self, read: impl FnOnce(&Value) -> R) -> R {
        match self {
            Self::Direct(value) => read(value),
            Self::Cell(value) => value.with_value(read),
        }
    }

    fn set(&mut self, value: Value) -> Value {
        match self {
            Self::Direct(existing) => std::mem::replace(existing, value),
            Self::Cell(existing) => {
                let previous = existing.get();
                existing.set(value);
                previous
            }
        }
    }
}

#[derive(Default)]
struct FrameBindings(RefCell<Vec<(String, FrameBindingValue)>>);

impl Clone for FrameBindings {
    fn clone(&self) -> Self {
        Self(RefCell::new(self.0.borrow().clone()))
    }
}

impl FrameBindings {
    fn with_capacity(capacity: usize) -> Self {
        Self(RefCell::new(Vec::with_capacity(capacity)))
    }

    fn from_values(values: HashMap<String, Value>) -> Self {
        Self(RefCell::new(
            values
                .into_iter()
                .map(|(name, value)| (name, FrameBindingValue::Direct(value)))
                .collect(),
        ))
    }

    fn get(&self, name: &str) -> Option<Value> {
        self.0
            .borrow()
            .iter()
            .rev()
            .find(|(candidate, _)| candidate == name)
            .map(|(_, value)| value.get())
    }

    fn cell(&self, name: &str) -> Option<Upvalue> {
        let mut bindings = self.0.borrow_mut();
        let index = bindings
            .iter()
            .rposition(|(candidate, _)| candidate == name)?;
        match &mut bindings[index].1 {
            FrameBindingValue::Direct(value) => {
                let cell = Upvalue::new(value.clone());
                // Replace only after cloning the direct value so promotion
                // preserves the exact binding value and future identity.
                let promoted = cell.clone();
                bindings[index].1 = FrameBindingValue::Cell(cell);
                Some(promoted)
            }
            FrameBindingValue::Cell(value) => Some(value.clone()),
        }
    }

    fn contains_key(&self, name: &str) -> bool {
        self.0
            .borrow()
            .iter()
            .any(|(candidate, _)| candidate == name)
    }

    fn insert(&self, name: String, value: Value) -> Option<Value> {
        let mut bindings = self.0.borrow_mut();
        if let Some((_, existing)) = bindings
            .iter_mut()
            .rev()
            .find(|(candidate, _)| candidate == &name)
        {
            return Some(existing.set(value));
        }
        bindings.push((name, FrameBindingValue::Direct(value)));
        None
    }

    fn insert_cell(&self, name: String, value: Upvalue) {
        let mut bindings = self.0.borrow_mut();
        if let Some((_, existing)) = bindings
            .iter_mut()
            .rev()
            .find(|(candidate, _)| candidate == &name)
        {
            *existing = FrameBindingValue::Cell(value);
            return;
        }
        bindings.push((name, FrameBindingValue::Cell(value)));
    }

    fn push(&self, name: String, value: Value) {
        self.0
            .borrow_mut()
            .push((name, FrameBindingValue::Direct(value)));
    }

    fn set(&self, name: &str, value: Value) -> bool {
        let mut bindings = self.0.borrow_mut();
        let Some((_, existing)) = bindings
            .iter_mut()
            .rev()
            .find(|(candidate, _)| candidate == name)
        else {
            return false;
        };
        existing.set(value);
        true
    }

    fn remove(&self, name: &str) -> Option<Value> {
        let mut bindings = self.0.borrow_mut();
        let index = bindings
            .iter()
            .rposition(|(candidate, _)| candidate == name)?;
        Some(bindings.remove(index).1.get())
    }

    fn replace_value(&self, expected: &Value, replacement: &Value) {
        for (_, value) in self.0.borrow_mut().iter_mut() {
            if value.get() == *expected {
                value.set(replacement.clone());
            }
        }
    }

    fn snapshot(&self) -> HashMap<String, Value> {
        self.0
            .borrow()
            .iter()
            .map(|(name, value)| (name.clone(), value.get()))
            .collect()
    }

    fn fork_values(&self) -> Self {
        Self(RefCell::new(
            self.0
                .borrow()
                .iter()
                .map(|(name, value)| (name.clone(), FrameBindingValue::Direct(value.get())))
                .collect(),
        ))
    }
}

#[derive(Clone, Default)]
pub(crate) struct DynamicBindings(Rc<RefCell<HashMap<String, Upvalue>>>);

impl DynamicBindings {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn from_values(values: HashMap<String, Value>) -> Self {
        Self(Rc::new(RefCell::new(
            values
                .into_iter()
                .map(|(name, value)| (name, Upvalue::new(value)))
                .collect(),
        )))
    }

    pub(crate) fn fork_cells(&self) -> Self {
        Self(Rc::new(RefCell::new(self.0.borrow().clone())))
    }

    pub(crate) fn borrow(&self) -> BindingSnapshot {
        BindingSnapshot(self.snapshot())
    }

    pub(crate) fn borrow_mut(&self) -> DynamicBindingsMut<'_> {
        DynamicBindingsMut {
            bindings: self,
            values: self.snapshot(),
        }
    }

    pub(crate) fn get(&self, name: &str) -> Option<Value> {
        self.0.borrow().get(name).map(Upvalue::get)
    }

    pub(crate) fn cell(&self, name: &str) -> Option<Upvalue> {
        self.0.borrow().get(name).cloned()
    }

    pub(crate) fn insert(&self, name: String, value: Value) -> Option<Value> {
        let mut bindings = self.0.borrow_mut();
        if let Some(binding) = bindings.get(&name) {
            let previous = binding.get();
            binding.set(value);
            return Some(previous);
        }
        bindings.insert(name, Upvalue::new(value));
        None
    }

    pub(crate) fn insert_cell(&self, name: String, upvalue: Upvalue) {
        self.0.borrow_mut().insert(name, upvalue);
    }

    pub(crate) fn set(&self, name: &str, value: Value) -> bool {
        let Some(binding) = self.cell(name) else {
            return false;
        };
        binding.set(value);
        true
    }

    pub(crate) fn remove(&self, name: &str) -> Option<Value> {
        self.0
            .borrow_mut()
            .remove(name)
            .map(|binding| binding.get())
    }

    pub(crate) fn remove_cell_if(&self, name: &str, expected: &Upvalue) -> bool {
        let mut bindings = self.0.borrow_mut();
        let matches = bindings
            .get(name)
            .is_some_and(|binding| binding.ptr_eq(expected));
        if matches {
            bindings.remove(name);
        }
        matches
    }

    pub(crate) fn contains_key(&self, name: &str) -> bool {
        self.0.borrow().contains_key(name)
    }

    pub(crate) fn snapshot(&self) -> HashMap<String, Value> {
        self.0
            .borrow()
            .iter()
            .map(|(name, binding)| (name.clone(), binding.get()))
            .collect()
    }

    pub(crate) fn names(&self) -> Vec<String> {
        self.0.borrow().keys().cloned().collect()
    }

    pub(crate) fn cells(&self) -> Vec<(String, Upvalue)> {
        self.0
            .borrow()
            .iter()
            .map(|(name, binding)| (name.clone(), binding.clone()))
            .collect()
    }
}

/// Owned compatibility view for the few dynamic-name consumers that still
/// need to enumerate a frame. The authoritative storage is a name-to-cell map;
/// this value snapshot never participates in binding identity or write-back.
pub(crate) struct BindingSnapshot(HashMap<String, Value>);

pub(crate) struct DynamicBindingsMut<'a> {
    bindings: &'a DynamicBindings,
    values: HashMap<String, Value>,
}

impl Deref for DynamicBindingsMut<'_> {
    type Target = HashMap<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.values
    }
}

impl std::ops::DerefMut for DynamicBindingsMut<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.values
    }
}

impl Drop for DynamicBindingsMut<'_> {
    fn drop(&mut self) {
        let retained = self.values.keys().cloned().collect::<HashSet<_>>();
        for name in self.bindings.names() {
            if !retained.contains(&name) {
                self.bindings.remove(&name);
            }
        }
        for (name, value) in &self.values {
            self.bindings.insert(name.clone(), value.clone());
        }
    }
}

impl Deref for BindingSnapshot {
    type Target = HashMap<String, Value>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl IntoIterator for BindingSnapshot {
    type Item = (String, Value);
    type IntoIter = std::collections::hash_map::IntoIter<String, Value>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

/// Cross-call runtime context: shared realm metadata plus a small frame view.
///
/// Bytecode lexical bindings remain in VM slots/upvalue cells. Cloning this
/// value shares the realm and explicit deopt cells; it copies only the small
/// compatibility frame vector.
#[allow(dead_code)]
pub(crate) struct CallEnv {
    realm: Realm,
    global_lexical_bindings: GlobalLexicalBindings,
    global_lexical_values: GlobalLexicalValues,
    expose_global_lexical_values: bool,
    immutable_lexical_bindings: ImmutableLexicalBindings,
    frame_bindings: FrameBindings,
    deopt_bindings: Option<DynamicBindings>,
    /// Dynamic-scope metadata is inherited by nested execution views but is
    /// mutated only by catch setup. Share the ordinary call path and
    /// detach on those cold mutations instead of cloning a hash table per call.
    catch_bindings: Rc<HashSet<String>>,
    /// The immutable name binding of a named function expression, for this
    /// frame only. Assigning to it is a silent no-op in sloppy mode and a
    /// TypeError in strict mode (unless a parameter/`var`/lexical shadows it,
    /// in which case the assignment resolves to that local slot instead and
    /// never consults this field). New ordinary function frames reset this;
    /// lexical-this functions inherit it explicitly from their enclosing frame.
    immutable_function_name: Option<String>,
    /// Share the inherited declaration-conflict set across ordinary frames and
    /// detach only when direct-eval setup rebuilds the active conflict names.
    direct_eval_var_conflicts: Rc<HashSet<String>>,
    /// The lexical private-name environment active for this frame. This is
    /// separate from `\0home_object`: ordinary nested functions do not inherit
    /// `super`, but they do retain access to private names declared by enclosing
    /// classes.
    private_environment: Option<PrivateEnvironment>,
    /// Dynamic with-object chain active at a direct eval call site. This is
    /// intentionally not part of ordinary function environments; the VM sets it
    /// only while invoking the intrinsic eval function as a direct eval.
    direct_eval_with_stack: Vec<Value>,
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
    /// Exported bindings of the currently executing module body. This is set
    /// only on the module frame (and its suspended TLA continuation), so an
    /// ordinary nested function with a same-named local cannot accidentally
    /// reuse a module export cell.
    module_live_bindings: Option<DynamicBindings>,
    /// The Test262 `$262.agent` execution context for this thread's agent,
    /// threaded like `module_host` so native `Atomics`/`$262.agent` hooks reach
    /// it. `None` outside the agents harness. Gated so the default build's
    /// struct layout and per-call clone cost are unchanged.
    #[cfg(feature = "agents")]
    agent_context: Option<crate::agent::AgentContextRef>,
}

impl Clone for CallEnv {
    fn clone(&self) -> Self {
        Self {
            realm: Rc::clone(&self.realm),
            global_lexical_bindings: Rc::clone(&self.global_lexical_bindings),
            global_lexical_values: Rc::clone(&self.global_lexical_values),
            expose_global_lexical_values: self.expose_global_lexical_values,
            immutable_lexical_bindings: Rc::clone(&self.immutable_lexical_bindings),
            // A cloned execution view is an isolated dynamic environment. The
            // cells themselves are shared only when a direct-eval/with deopt
            // path explicitly requests that identity.
            frame_bindings: self.frame_bindings.clone(),
            deopt_bindings: self.deopt_bindings.clone(),
            catch_bindings: self.catch_bindings.clone(),
            immutable_function_name: self.immutable_function_name.clone(),
            direct_eval_var_conflicts: self.direct_eval_var_conflicts.clone(),
            private_environment: self.private_environment.clone(),
            direct_eval_with_stack: self.direct_eval_with_stack.clone(),
            module_host: self.module_host.clone(),
            module_imports: self.module_imports.clone(),
            module_live_bindings: self.module_live_bindings.clone(),
            #[cfg(feature = "agents")]
            agent_context: self.agent_context.clone(),
        }
    }
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
            .field("frame_bindings", &self.snapshot_locals())
            .field("deopt_bindings", &self.deopt_bindings.is_some())
            .field("catch_bindings", &self.catch_bindings)
            .field("direct_eval_var_conflicts", &self.direct_eval_var_conflicts)
            .field("direct_eval_with_stack", &self.direct_eval_with_stack.len())
            .field("module_host", &self.module_host.is_some())
            .field("module_imports", &self.module_imports.keys())
            .field("module_live_bindings", &self.module_live_bindings.is_some())
            .finish()
    }
}

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
            frame_bindings: FrameBindings::default(),
            deopt_bindings: None,
            catch_bindings: Default::default(),
            immutable_function_name: None,
            direct_eval_var_conflicts: Default::default(),
            private_environment: None,
            direct_eval_with_stack: Vec::new(),
            module_host: None,
            module_imports: Default::default(),
            module_live_bindings: None,
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
        exported_bindings: DynamicBindings,
        exported_local_name: String,
    ) {
        Rc::make_mut(&mut self.module_imports)
            .insert(local_name, (exported_bindings, exported_local_name));
    }

    /// Reads the current value of a live module import binding.
    pub(crate) fn module_import_value(&self, local_name: &str) -> Option<Value> {
        let (bindings, exported_local_name) = self.module_imports.get(local_name)?;
        Some(
            bindings
                .get(exported_local_name)
                .unwrap_or_else(|| Value::Function(Function::uninitialized_lexical_marker())),
        )
    }

    /// Returns the exporter's shared cell backing a live module import.
    /// Capturing an import must retain this identity instead of boxing the
    /// value currently observed by the importing module frame.
    pub(crate) fn module_import_cell(&self, local_name: &str) -> Option<Upvalue> {
        let (bindings, exported_local_name) = self.module_imports.get(local_name)?;
        bindings.cell(exported_local_name)
    }

    pub(crate) fn has_module_import(&self, local_name: &str) -> bool {
        self.module_imports.contains_key(local_name)
    }

    pub(crate) fn module_imports(&self) -> ModuleImports {
        self.module_imports.clone()
    }

    pub(crate) fn has_module_imports(&self) -> bool {
        !self.module_imports.is_empty()
    }

    pub(crate) fn set_module_imports(&mut self, imports: ModuleImports) {
        self.module_imports = imports;
    }

    pub(crate) fn set_module_live_bindings(&mut self, bindings: DynamicBindings) {
        self.module_live_bindings = Some(bindings);
    }

    pub(crate) fn module_live_binding_cell(&self, name: &str) -> Option<Upvalue> {
        self.module_live_bindings
            .as_ref()
            .and_then(|bindings| bindings.cell(name))
    }

    /// Builds a standalone environment over a fresh, empty realm. Used by the
    /// no-context conversion helpers (`to_js_string`, `array_like_values`,
    /// `parse_json_text`) that run without a live VM; intrinsic lookups simply
    /// return `None`, matching the prior empty-`HashMap` behavior.
    pub(crate) fn detached() -> Self {
        Self {
            realm: new_realm(HashMap::new()),
            global_lexical_bindings: Rc::new(RefCell::new(HashSet::new())),
            global_lexical_values: Rc::new(RefCell::new(HashMap::new())),
            expose_global_lexical_values: false,
            immutable_lexical_bindings: Rc::new(RefCell::new(HashSet::new())),
            frame_bindings: FrameBindings::default(),
            deopt_bindings: None,
            catch_bindings: Default::default(),
            immutable_function_name: None,
            direct_eval_var_conflicts: Default::default(),
            private_environment: None,
            direct_eval_with_stack: Vec::new(),
            module_host: None,
            module_imports: Default::default(),
            module_live_bindings: None,
            #[cfg(feature = "agents")]
            agent_context: None,
        }
    }

    /// Wraps an owned flat map as a detached realm with no frame bindings.
    /// Used by native/dynamic compatibility entry points that receive an owned
    /// map rather than a live VM realm.
    pub(crate) fn from_map(map: HashMap<String, Value>) -> Self {
        Self {
            realm: new_realm(map),
            global_lexical_bindings: Rc::new(RefCell::new(HashSet::new())),
            global_lexical_values: Rc::new(RefCell::new(HashMap::new())),
            expose_global_lexical_values: false,
            immutable_lexical_bindings: Rc::new(RefCell::new(HashSet::new())),
            frame_bindings: FrameBindings::default(),
            deopt_bindings: None,
            catch_bindings: Default::default(),
            immutable_function_name: None,
            direct_eval_var_conflicts: Default::default(),
            private_environment: None,
            direct_eval_with_stack: Vec::new(),
            module_host: None,
            module_imports: Default::default(),
            module_live_bindings: None,
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
            frame_bindings: FrameBindings::from_values(locals),
            deopt_bindings: None,
            catch_bindings: Default::default(),
            immutable_function_name: None,
            direct_eval_var_conflicts: Default::default(),
            private_environment: None,
            direct_eval_with_stack: Vec::new(),
            module_host: None,
            module_imports: Default::default(),
            module_live_bindings: None,
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
        self.new_function_frame()
    }

    /// Builds a new ordinary function frame over the same shared realm. Frame
    /// bindings are inserted directly into the small cell vector by the call
    /// setup path, avoiding an intermediate name-keyed `HashMap` allocation.
    pub(crate) fn new_function_frame(&self) -> Self {
        self.new_function_frame_with_capacity(0)
    }

    /// Builds a function frame with room for the bindings its call prologue
    /// will materialize, avoiding repeated small-vector growth on hot calls.
    pub(crate) fn new_function_frame_with_capacity(&self, capacity: usize) -> Self {
        Self {
            realm: Rc::clone(&self.realm),
            global_lexical_bindings: Rc::clone(&self.global_lexical_bindings),
            global_lexical_values: Rc::clone(&self.global_lexical_values),
            expose_global_lexical_values: false,
            immutable_lexical_bindings: Rc::clone(&self.immutable_lexical_bindings),
            frame_bindings: FrameBindings::with_capacity(capacity),
            deopt_bindings: None,
            catch_bindings: self.catch_bindings.clone(),
            immutable_function_name: None,
            direct_eval_var_conflicts: self.direct_eval_var_conflicts.clone(),
            private_environment: self.private_environment.clone(),
            direct_eval_with_stack: self.direct_eval_with_stack.clone(),
            module_host: self.module_host.clone(),
            module_imports: self.module_imports.clone(),
            module_live_bindings: None,
            #[cfg(feature = "agents")]
            agent_context: self.agent_context.clone(),
        }
    }

    /// Builds the minimal context for an ordinary leaf function whose call
    /// contract excludes direct eval, `with`, closures, and special lexical
    /// bindings. Function-owned private and module state is installed by the
    /// caller after construction, so copying the caller's transient maps here
    /// would only allocate them before immediately replacing them.
    pub(crate) fn new_direct_leaf_function_frame(&self) -> Self {
        Self {
            realm: Rc::clone(&self.realm),
            global_lexical_bindings: Rc::clone(&self.global_lexical_bindings),
            global_lexical_values: Rc::clone(&self.global_lexical_values),
            expose_global_lexical_values: false,
            immutable_lexical_bindings: Rc::clone(&self.immutable_lexical_bindings),
            frame_bindings: FrameBindings::default(),
            deopt_bindings: None,
            catch_bindings: Default::default(),
            immutable_function_name: None,
            direct_eval_var_conflicts: Default::default(),
            private_environment: None,
            direct_eval_with_stack: Vec::new(),
            module_host: self.module_host.clone(),
            module_imports: Default::default(),
            module_live_bindings: None,
            #[cfg(feature = "agents")]
            agent_context: self.agent_context.clone(),
        }
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
        Rc::make_mut(&mut self.catch_bindings).insert(name);
    }

    pub(crate) fn unmark_catch_binding(&mut self, name: &str) {
        Rc::make_mut(&mut self.catch_bindings).remove(name);
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
        Rc::make_mut(&mut self.direct_eval_var_conflicts).clear();
    }

    pub(crate) fn mark_direct_eval_var_conflict(&mut self, name: String) {
        Rc::make_mut(&mut self.direct_eval_var_conflicts).insert(name);
    }

    pub(crate) fn is_direct_eval_var_conflict(&self, name: &str) -> bool {
        self.direct_eval_var_conflicts.contains(name)
    }

    /// Owned compatibility snapshot for the few dynamic-name consumers.
    pub(crate) fn binding_snapshot(&self) -> BindingSnapshot {
        BindingSnapshot(self.snapshot_locals())
    }

    /// Visits the currently visible values in the frame-local environment
    /// without materializing a name-keyed snapshot. This is reserved for cold
    /// dynamic-scope compatibility work such as direct-eval capture repair;
    /// ordinary bytecode bindings remain slot-indexed.
    pub(crate) fn for_each_visible_local_value(&self, mut visit: impl FnMut(&Value)) {
        let frame_bindings = self.frame_bindings.0.borrow();
        if let Some(deopt_bindings) = &self.deopt_bindings {
            for (name, value) in deopt_bindings.0.borrow().iter() {
                if !frame_bindings
                    .iter()
                    .any(|(frame_name, _)| frame_name == name)
                {
                    value.with_value(&mut visit);
                }
            }
        }
        for (index, (name, value)) in frame_bindings.iter().enumerate() {
            if frame_bindings[index + 1..]
                .iter()
                .any(|(inner_name, _)| inner_name == name)
            {
                continue;
            }
            value.with_value(&mut visit);
        }
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

    pub(crate) fn set_deopt_bindings(&mut self, bindings: DynamicBindings) {
        self.deopt_bindings = Some(bindings);
    }

    pub(crate) fn deopt_bindings(&self) -> Option<&DynamicBindings> {
        self.deopt_bindings.as_ref()
    }

    pub(crate) fn fork_deopt_bindings(&mut self) {
        self.deopt_bindings = self
            .deopt_bindings
            .as_ref()
            .map(DynamicBindings::fork_cells);
    }

    pub(crate) fn remove_deopt_binding(&mut self, name: &str) {
        if let Some(bindings) = &self.deopt_bindings {
            bindings.remove(name);
        }
    }

    pub(crate) fn remove_deopt_cell_if(&mut self, name: &str, expected: &Upvalue) {
        if let Some(bindings) = &self.deopt_bindings {
            bindings.remove_cell_if(name, expected);
        }
    }

    pub(crate) fn remove_frame_binding(&mut self, name: &str) {
        self.frame_bindings.remove(name);
    }

    /// This frame's own locals layer, mutably.
    pub(crate) fn replace_local_value(&self, expected: &Value, replacement: &Value) {
        self.frame_bindings.replace_value(expected, replacement);
    }

    /// Consumes the view, returning a dynamic-name value snapshot.
    pub(crate) fn into_binding_snapshot(self) -> HashMap<String, Value> {
        self.snapshot_locals()
    }

    /// Looks up `name`: frame locals first, then a short realm borrow. Returns
    /// an owned value because a value behind the realm `RefCell` cannot be
    /// handed out by reference.
    pub(crate) fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.frame_bindings.get(name) {
            return Some(value);
        }
        if let Some(value) = self
            .deopt_bindings
            .as_ref()
            .and_then(|bindings| bindings.get(name))
        {
            return Some(value);
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

    /// Returns the active global object override used by indirect eval and
    /// dynamically constructed functions. A frame-local marker, including a
    /// non-object value that disables an outer override, wins before the
    /// realm's cached marker.
    pub(crate) fn dynamic_function_realm_global(&self) -> Option<ObjectRef> {
        let frame_value = self
            .frame_bindings
            .get(DYNAMIC_FUNCTION_REALM_GLOBAL)
            .or_else(|| {
                self.deopt_bindings
                    .as_ref()
                    .and_then(|bindings| bindings.get(DYNAMIC_FUNCTION_REALM_GLOBAL))
            });
        if let Some(value) = frame_value {
            return match value {
                Value::Object(global) => Some(global),
                _ => None,
            };
        }
        self.realm.dynamic_function_realm_global()
    }

    /// Returns the realm's internal global-this slot without hashing its
    /// private string key. The slot is fixed when a realm is created; writes
    /// to the public `globalThis` property do not replace this identity.
    pub(crate) fn global_this(&self) -> Option<Value> {
        self.realm.global_this()
    }

    /// Returns the one shared cell for a captured realm binding, creating it
    /// lazily from the realm's current value on first capture.
    pub(crate) fn realm_binding_cell(&self, name: &str) -> Option<Upvalue> {
        let value = self.realm.borrow().get(name).cloned()?;
        if let Some(cell) = self.realm.binding_cells.cell(name) {
            return Some(cell);
        }
        let cell = Upvalue::new(value);
        self.realm
            .binding_cells
            .insert_cell(name.to_owned(), cell.clone());
        Some(cell)
    }

    pub(crate) fn remove_realm(&self, name: &str) -> Option<Value> {
        let removed = self.realm.borrow_mut().remove(name);
        self.realm.sync_dynamic_function_realm_binding(name, None);
        if let Some(cell) = self.realm.binding_cells.cell(name) {
            cell.set(Value::Function(Function::uninitialized_lexical_marker()));
        }
        removed
    }

    pub(crate) fn is_realm_binding_cell(&self, name: &str, cell: &Upvalue) -> bool {
        self.realm
            .binding_cells
            .cell(name)
            .is_some_and(|candidate| candidate.ptr_eq(cell))
    }

    /// True if `name` is bound in either layer.
    pub(crate) fn contains_key(&self, name: &str) -> bool {
        self.frame_bindings.contains_key(name)
            || self
                .deopt_bindings
                .as_ref()
                .is_some_and(|bindings| bindings.contains_key(name))
            || self.realm.borrow().contains_key(name)
    }

    /// Inserts a frame-local binding (`this`, params, captures, caller-scope
    /// bindings). The VM write-back routes these to real locals-or-globals via
    /// `local_slot`. Realm/global definitions use [`CallEnv::insert_realm`].
    pub(crate) fn insert(&mut self, name: String, value: Value) -> Option<Value> {
        if let Some(bindings) = &self.deopt_bindings
            && bindings.contains_key(&name)
        {
            return bindings.insert(name, value);
        }
        self.frame_bindings.insert(name, value)
    }

    /// Inserts into the explicit dynamic-name environment, creating a binding
    /// there when direct eval introduces a new function-scope `var`.
    pub(crate) fn insert_deopt(&mut self, name: String, value: Value) -> Option<Value> {
        if let Some(bindings) = &self.deopt_bindings {
            return bindings.insert(name, value);
        }
        self.frame_bindings.insert(name, value)
    }

    /// Inserts into this execution frame even when an outer deopt environment
    /// has a same-named binding. Strict eval declarations use their own frame.
    pub(crate) fn insert_frame(&mut self, name: String, value: Value) -> Option<Value> {
        self.frame_bindings.push(name, value);
        None
    }

    /// Installs an existing VM slot cell in this frame view. Native operations
    /// and callbacks then observe the same binding identity as bytecode, so no
    /// value snapshot can overwrite a callback's update on return.
    pub(crate) fn insert_frame_cell(&self, name: String, value: Upvalue) {
        self.frame_bindings.insert_cell(name, value);
    }

    /// Inserts directly into the shared realm cell (builtin install and global
    /// definition). Visible to every frame sharing the realm.
    pub(crate) fn insert_realm(&self, name: String, value: Value) -> Option<Value> {
        if let Some(cell) = self.realm.binding_cells.cell(&name) {
            cell.set(value.clone());
        }
        self.realm
            .sync_dynamic_function_realm_binding(&name, Some(&value));
        self.realm.borrow_mut().insert(name, value)
    }

    /// Replaces an existing realm binding without allocating an owned lookup
    /// key. Hot global stores use this after proving that the corresponding
    /// global-object property is still an ordinary writable data property.
    pub(crate) fn replace_existing_realm(&self, name: &str, value: Value) -> bool {
        let mut bindings = self.realm.borrow_mut();
        let Some(binding) = bindings.get_mut(name) else {
            return false;
        };
        *binding = value.clone();
        drop(bindings);
        if let Some(cell) = self.realm.binding_cells.cell(name) {
            cell.set(value.clone());
        }
        self.realm
            .sync_dynamic_function_realm_binding(name, Some(&value));
        true
    }

    /// Mirrors a data-property definition on this realm's global object into
    /// the realm value table and any already-captured global cell.
    pub(crate) fn sync_realm_global_object_property(&self, object: &ObjectRef, name: &str) {
        let is_global_object = self
            .realm
            .borrow()
            .get(crate::GLOBAL_THIS_BINDING)
            .is_some_and(|global| global.same_value(&Value::Object(object.clone())));
        if !is_global_object || !self.realm.borrow().contains_key(name) {
            return;
        }
        let Some(property) = object.own_property(name) else {
            return;
        };
        if property.is_accessor() {
            self.remove_realm(name);
        } else {
            self.insert_realm(name.to_owned(), property.value);
        }
    }

    /// Defines `name` in the shared realm only if it is not already bound there.
    /// Used by global-binding initialization (script and indirect-eval scopes).
    pub(crate) fn realm_entry_or_insert(&self, name: String, value: Value) {
        let refresh_dynamic_realm = name == DYNAMIC_FUNCTION_REALM_GLOBAL;
        self.realm.borrow_mut().entry(name).or_insert(value);
        if refresh_dynamic_realm {
            self.realm.refresh_dynamic_function_realm_global();
        }
    }

    /// True if the shared realm cell binds `name`.
    pub(crate) fn realm_contains(&self, name: &str) -> bool {
        self.realm.borrow().contains_key(name)
    }

    /// Removes a frame-local binding.
    pub(crate) fn remove(&mut self, name: &str) -> Option<Value> {
        if let Some(bindings) = &self.deopt_bindings
            && bindings.contains_key(name)
        {
            return bindings.remove(name);
        }
        self.frame_bindings.remove(name)
    }

    /// Mutates an existing frame-local binding in place, if present.
    pub(crate) fn get_local(&self, name: &str) -> Option<Value> {
        self.frame_bindings.get(name).or_else(|| {
            self.deopt_bindings
                .as_ref()
                .and_then(|bindings| bindings.get(name))
        })
    }

    pub(crate) fn has_frame_binding(&self, name: &str) -> bool {
        self.frame_bindings.contains_key(name)
    }

    pub(crate) fn has_local_binding(&self, name: &str) -> bool {
        self.frame_bindings.contains_key(name)
            || self
                .deopt_bindings
                .as_ref()
                .is_some_and(|bindings| bindings.contains_key(name))
    }

    /// Whether ordinary bytecode locals can use their indexed slots without
    /// consulting any name-addressed compatibility environment.
    pub(crate) fn slot_is_authoritative(&self, name: &str) -> bool {
        !self.frame_bindings.contains_key(name)
            && self.deopt_bindings.is_none()
            && self.module_imports.is_empty()
            && self.module_live_bindings.is_none()
            && self.immutable_function_name.is_none()
    }

    pub(crate) fn frame_binding_cell(&self, name: &str) -> Option<Upvalue> {
        self.frame_bindings.cell(name)
    }

    /// Returns the live cell for a binding supplied by the current caller or
    /// dynamic environment, without falling through to the realm. Direct eval
    /// uses this to keep a same-named function local distinct from a global.
    pub(crate) fn local_binding_cell(&self, name: &str) -> Option<Upvalue> {
        self.frame_bindings.cell(name).or_else(|| {
            self.deopt_bindings
                .as_ref()
                .and_then(|bindings| bindings.cell(name))
        })
    }

    pub(crate) fn set_local(&self, name: &str, value: Value) -> bool {
        self.frame_bindings.set(name, value.clone())
            || self
                .deopt_bindings
                .as_ref()
                .is_some_and(|bindings| bindings.set(name, value))
    }

    /// A snapshot of just the frame locals layer.
    pub(crate) fn snapshot_locals(&self) -> HashMap<String, Value> {
        let mut locals = self
            .deopt_bindings
            .as_ref()
            .map_or_else(HashMap::new, DynamicBindings::snapshot);
        locals.extend(self.frame_bindings.snapshot());
        locals
    }

    /// A cold compatibility snapshot merging the realm and frame view.
    /// Lexical binding identity never depends on this materialized map.
    pub(crate) fn to_flat_map(&self) -> HashMap<String, Value> {
        let mut map = self.realm.borrow().clone();
        for (name, value) in self.snapshot_locals() {
            map.insert(name, value);
        }
        map
    }

    /// Builds an isolated dynamic view of this execution frame without a
    /// name-keyed intermediate map. Explicit direct-eval/with cells are added
    /// by the VM after it overlays live slot values.
    pub(crate) fn fork_current_frame_values(&self) -> Self {
        Self {
            realm: Rc::clone(&self.realm),
            global_lexical_bindings: Rc::clone(&self.global_lexical_bindings),
            global_lexical_values: Rc::clone(&self.global_lexical_values),
            expose_global_lexical_values: false,
            immutable_lexical_bindings: Rc::clone(&self.immutable_lexical_bindings),
            frame_bindings: self.frame_bindings.fork_values(),
            deopt_bindings: None,
            catch_bindings: self.catch_bindings.clone(),
            immutable_function_name: self.immutable_function_name.clone(),
            direct_eval_var_conflicts: self.direct_eval_var_conflicts.clone(),
            private_environment: self.private_environment.clone(),
            direct_eval_with_stack: self.direct_eval_with_stack.clone(),
            module_host: self.module_host.clone(),
            module_imports: self.module_imports.clone(),
            module_live_bindings: self.module_live_bindings.clone(),
            #[cfg(feature = "agents")]
            agent_context: self.agent_context.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CallEnv, DYNAMIC_FUNCTION_REALM_GLOBAL, DynamicBindings, FrameBindingValue, FrameBindings,
        new_realm,
    };
    use crate::{ObjectRef, Value};
    use std::{collections::HashMap, rc::Rc};

    #[test]
    fn frame_binding_promotes_to_a_cell_only_when_identity_is_requested() {
        let bindings = FrameBindings::default();
        bindings.insert("value".to_owned(), Value::Number(1.0));
        assert!(matches!(
            &bindings.0.borrow()[0].1,
            FrameBindingValue::Direct(Value::Number(1.0))
        ));

        let cell = bindings.cell("value").expect("binding should promote");
        assert!(matches!(
            &bindings.0.borrow()[0].1,
            FrameBindingValue::Cell(_)
        ));
        cell.set(Value::Number(2.0));
        assert!(matches!(bindings.get("value"), Some(Value::Number(2.0))));
        assert!(cell.ptr_eq(&bindings.cell("value").expect("cell is stable")));
    }

    #[test]
    fn function_frame_eval_conflict_sets_are_copy_on_write() {
        let mut caller = CallEnv::new(new_realm(HashMap::new()));
        caller.mark_catch_binding("caller_catch".to_owned());
        caller.mark_direct_eval_var_conflict("caller_lexical".to_owned());

        let mut callee = caller.new_function_frame();
        callee.unmark_catch_binding("caller_catch");
        callee.mark_catch_binding("callee_catch".to_owned());
        callee.clear_direct_eval_var_conflicts();
        callee.mark_direct_eval_var_conflict("callee_lexical".to_owned());

        assert!(caller.is_catch_binding("caller_catch"));
        assert!(!caller.is_catch_binding("callee_catch"));
        assert!(caller.is_direct_eval_var_conflict("caller_lexical"));
        assert!(!caller.is_direct_eval_var_conflict("callee_lexical"));
        assert!(!callee.is_catch_binding("caller_catch"));
        assert!(callee.is_catch_binding("callee_catch"));
        assert!(!callee.is_direct_eval_var_conflict("caller_lexical"));
        assert!(callee.is_direct_eval_var_conflict("callee_lexical"));
    }

    #[test]
    fn unrelated_frame_binding_does_not_disable_an_authoritative_slot() {
        let mut env = CallEnv::new(new_realm(HashMap::new()));
        env.insert("this".to_owned(), Value::Undefined);

        assert!(env.slot_is_authoritative("value"));
        assert!(!env.slot_is_authoritative("this"));

        env.set_deopt_bindings(DynamicBindings::new());
        assert!(!env.slot_is_authoritative("value"));
    }

    #[test]
    fn visible_local_value_scan_skips_shadowed_bindings() {
        let mut env = CallEnv::new(new_realm(HashMap::new()));
        let deopt = DynamicBindings::new();
        deopt.insert("deopt-only".to_owned(), Value::Number(3.0));
        deopt.insert("shadowed".to_owned(), Value::Number(1.0));
        env.set_deopt_bindings(deopt);
        env.frame_bindings
            .push("shadowed".to_owned(), Value::Number(2.0));
        env.frame_bindings
            .push("shadowed".to_owned(), Value::Number(4.0));
        env.frame_bindings
            .push("frame-only".to_owned(), Value::Number(5.0));

        let mut visited = Vec::new();
        env.for_each_visible_local_value(|value| {
            if let Value::Number(number) = value {
                visited.push(*number as i32);
            }
        });
        visited.sort_unstable();

        assert_eq!(visited, vec![3, 4, 5]);
    }

    #[test]
    fn module_import_routes_share_until_environment_mutation() {
        let mut env = CallEnv::new(new_realm(HashMap::new()));
        let cloned = env.clone();
        assert!(Rc::ptr_eq(&env.module_imports, &cloned.module_imports));

        env.set_module_import(
            "local".to_owned(),
            DynamicBindings::new(),
            "exported".to_owned(),
        );
        assert!(env.has_module_import("local"));
        assert!(!cloned.has_module_import("local"));
        assert!(!Rc::ptr_eq(&env.module_imports, &cloned.module_imports));
    }

    #[test]
    fn dynamic_function_realm_cache_tracks_bulk_and_incremental_mutations() {
        let first = ObjectRef::new(HashMap::new());
        let mut bindings = HashMap::new();
        bindings.insert(
            DYNAMIC_FUNCTION_REALM_GLOBAL.to_owned(),
            Value::Object(first.clone()),
        );
        let realm = new_realm(bindings);
        assert!(
            realm
                .dynamic_function_realm_global()
                .is_some_and(|global| global.ptr_eq(&first))
        );

        realm
            .borrow_mut()
            .insert(DYNAMIC_FUNCTION_REALM_GLOBAL.to_owned(), Value::Undefined);
        realm.refresh_dynamic_function_realm_global();
        assert!(realm.dynamic_function_realm_global().is_none());

        let second = ObjectRef::new(HashMap::new());
        let env = CallEnv::new(Rc::clone(&realm));
        env.insert_realm(
            DYNAMIC_FUNCTION_REALM_GLOBAL.to_owned(),
            Value::Object(second.clone()),
        );
        assert!(
            realm
                .dynamic_function_realm_global()
                .is_some_and(|global| global.ptr_eq(&second))
        );

        env.remove_realm(DYNAMIC_FUNCTION_REALM_GLOBAL);
        assert!(realm.dynamic_function_realm_global().is_none());
    }
}
