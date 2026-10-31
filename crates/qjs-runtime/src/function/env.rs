//! Temporary service ABI over typed creation/activation state.
//! Declaratives live in bytecode slots/shared cells; realm ownership is in
//! context/realm.rs. FrameBindings/deopt value projections remain compatibility
//! inputs/publication for the atomic Context cut. Never add new snapshots here.
//! Storage borrows must end before getters, setters, traps or other JS callbacks.

use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    ops::Deref,
    rc::Rc,
};

use qjs_parser::EvalParseContext;

use crate::context::{ModuleImports, Realm, new_realm};
use crate::{
    Bytecode, Function, JsString, ObjectRef, Value, function::BindingCell,
    private::PrivateEnvironment,
};

const DYNAMIC_FUNCTION_REALM_GLOBAL: &str = "__quickjsRustDynamicFunctionRealm";

#[derive(Clone)]
enum FrameBindingValue {
    Direct(Value),
    Cell(BindingCell),
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

    fn cell(&self, name: &str) -> Option<BindingCell> {
        let mut bindings = self.0.borrow_mut();
        let index = bindings
            .iter()
            .rposition(|(candidate, _)| candidate == name)?;
        match &mut bindings[index].1 {
            FrameBindingValue::Direct(value) => {
                let cell = BindingCell::new(value.clone());
                // Replace only after cloning the direct value so promotion
                // preserves the exact binding value and future identity.
                let promoted = cell.clone();
                bindings[index].1 = FrameBindingValue::Cell(cell);
                Some(promoted)
            }
            FrameBindingValue::Cell(value) => Some(value.clone()),
        }
    }

    fn is_empty(&self) -> bool {
        self.0.borrow().is_empty()
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

    fn insert_cell(&self, name: String, value: BindingCell) {
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
                .map(|(name, value)| {
                    let forked = FrameBindingValue::Direct(value.get());
                    (name.clone(), forked)
                })
                .collect(),
        ))
    }
}

#[derive(Clone, Default)]
pub(crate) struct DynamicBindings(Rc<DynamicBindingsInner>);

#[derive(Default)]
pub(crate) struct DynamicBindingsInner {
    map: RefCell<crate::value::name_hash::NameMap<String, BindingCell>>,
    /// Advanced by every change to which cell a name maps to -- a new name,
    /// a replaced or removed cell -- but not by writes through a cell. Equal
    /// generations mean every name still maps to the cell it did.
    generation: std::cell::Cell<u64>,
    /// Closures created bypassing this scope (`bypass`), revoked on the next
    /// generation change.
    bypasses: RefCell<Vec<super::FunctionWeakRef>>,
}

/// Closures one scope lets bypass it at a time; past this, closures keep the
/// scope, so a frame creating closures in a loop does not grow the list.
const MAX_SCOPE_BYPASSES: usize = 64;

impl DynamicBindings {
    /// Lets `function` run without this scope until a name is added,
    /// removed or remapped here. The caller has proved that nothing the
    /// function resolves by name is in the scope now. `false` when the
    /// scope already has its fill of bypassing closures.
    pub(crate) fn bypass(&self, function: &super::Function) -> bool {
        let mut bypasses = self.0.bypasses.borrow_mut();
        if bypasses.len() >= MAX_SCOPE_BYPASSES {
            bypasses.retain(|weak| weak.upgrade().is_some());
            if bypasses.len() >= MAX_SCOPE_BYPASSES {
                return false;
            }
        }
        bypasses.push(function.downgrade());
        function.scope_bypassed.set(true);
        true
    }

    #[cold]
    #[inline(never)]
    fn revoke_bypasses(&self) {
        let bypasses = std::mem::take(&mut *self.0.bypasses.borrow_mut());
        for function in bypasses {
            if let Some(function) = function.upgrade() {
                function.revoke_scope_bypass();
            }
        }
    }

    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// The map for a change that may alter which cell a name maps to.
    fn mapping_mut(
        &self,
    ) -> std::cell::RefMut<'_, crate::value::name_hash::NameMap<String, BindingCell>> {
        self.0
            .generation
            .set(self.0.generation.get().wrapping_add(1));
        if !self.0.bypasses.borrow().is_empty() {
            self.revoke_bypasses();
        }
        self.0.map.borrow_mut()
    }

    /// This environment's identity, for a memo keyed on it.
    pub(crate) fn identity(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
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
                    .map(|(name, value)| (name, BindingCell::new(value)))
                    .collect(),
            ),
            generation: std::cell::Cell::new(0),
            bypasses: RefCell::default(),
        }))
    }

    pub(crate) fn fork_cells(&self) -> Self {
        Self(Rc::new(DynamicBindingsInner {
            map: RefCell::new(self.0.map.borrow().clone()),
            generation: std::cell::Cell::new(0),
            bypasses: RefCell::default(),
        }))
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
        self.0.map.borrow().get(name).map(BindingCell::get)
    }

    pub(crate) fn cell(&self, name: &str) -> Option<BindingCell> {
        self.0.map.borrow().get(name).cloned()
    }

    pub(crate) fn insert(&self, name: String, value: Value) -> Option<Value> {
        if let Some(binding) = self.0.map.borrow().get(&name) {
            let previous = binding.get();
            binding.set(value);
            return Some(previous);
        }
        self.mapping_mut().insert(name, BindingCell::new(value));
        None
    }

    pub(crate) fn insert_cell(&self, name: String, upvalue: BindingCell) {
        self.mapping_mut().insert(name, upvalue);
    }

    /// Overlays a live frame cell unless this shared dynamic environment
    /// already names that exact cell.
    ///
    /// Closure creation can ask the same frame to expose its locals many
    /// times. The map is shared by those closures, so replacing an entry with
    /// the identical [`BindingCell`] only clones and drops the owned name while
    /// leaving binding identity unchanged. A different cell must still win:
    /// it can represent a newly active lexical shadow with the same name.
    pub(crate) fn overlay_cell(&self, name: &str, upvalue: &BindingCell) -> bool {
        if self
            .0
            .map
            .borrow()
            .get(name)
            .is_some_and(|binding| binding.ptr_eq(upvalue))
        {
            return false;
        }
        self.mapping_mut().insert(name.to_owned(), upvalue.clone());
        true
    }

    pub(crate) fn set(&self, name: &str, value: Value) -> bool {
        let Some(binding) = self.cell(name) else {
            return false;
        };
        binding.set(value);
        true
    }

    pub(crate) fn remove(&self, name: &str) -> Option<Value> {
        self.mapping_mut().remove(name).map(|binding| binding.get())
    }

    pub(crate) fn remove_cell_if(&self, name: &str, expected: &BindingCell) -> bool {
        let matches = self
            .0
            .map
            .borrow()
            .get(name)
            .is_some_and(|binding| binding.ptr_eq(expected));
        if matches {
            self.mapping_mut().remove(name);
        }
        matches
    }

    pub(crate) fn contains_key(&self, name: &str) -> bool {
        self.0.map.borrow().contains_key(name)
    }

    pub(crate) fn snapshot(&self) -> HashMap<String, Value> {
        self.0
            .map
            .borrow()
            .iter()
            .map(|(name, binding)| (name.clone(), binding.get()))
            .collect()
    }

    pub(crate) fn names(&self) -> Vec<String> {
        self.0.map.borrow().keys().cloned().collect()
    }

    /// Calls `visit` with every binding name that carries `prefix`, stripped.
    ///
    /// The caller used to ask the opposite question -- build `prefix + name`
    /// for each candidate and look it up -- which allocated a `String` per
    /// candidate. Walking this map instead allocates nothing, and it is the
    /// smaller side: a frame has one binding per split parameter and many
    /// locals.
    pub(crate) fn for_each_prefixed_name(&self, prefix: &str, mut visit: impl FnMut(&str)) {
        for (name, _) in self.0.map.borrow().iter() {
            if let Some(rest) = name.strip_prefix(prefix) {
                visit(rest);
            }
        }
    }

    pub(crate) fn cells(&self) -> Vec<(String, BindingCell)> {
        self.0
            .map
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
    activation: crate::context::ActivationContext,
    frame_bindings: FrameBindings,
    /// Pending construct target until B5 introduces the borrowed NativeSite.
    construct_target: Option<Value>,
    deopt_bindings: Option<DynamicBindings>,
    global_binding_root: bool,
    global_function_configurable: bool,
    catch_bindings: Option<Rc<HashSet<String>>>,
    immutable_function_name: Option<String>,
    direct_eval_var_conflicts: Option<Rc<HashSet<String>>>,
    direct_eval_with_stack: Vec<Value>,
    #[cfg(feature = "agents")]
    agent_context: Option<crate::agent::AgentContextRef>,
}

impl Clone for CallEnv {
    fn clone(&self) -> Self {
        Self {
            activation: self.activation.retained_view(),
            frame_bindings: self.frame_bindings.clone(),
            construct_target: self.construct_target.clone(),
            deopt_bindings: self.deopt_bindings.clone(),
            global_binding_root: self.global_binding_root,
            global_function_configurable: self.global_function_configurable,
            catch_bindings: self.catch_bindings.clone(),
            immutable_function_name: self.immutable_function_name.clone(),
            direct_eval_var_conflicts: self.direct_eval_var_conflicts.clone(),
            direct_eval_with_stack: self.direct_eval_with_stack.clone(),
            #[cfg(feature = "agents")]
            agent_context: self.agent_context.clone(),
        }
    }
}

impl std::fmt::Debug for CallEnv {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CallEnv")
            .field("realm", &self.activation.creation.realm)
            .field("frame_bindings", &self.snapshot_locals())
            .field("deopt_bindings", &self.deopt_bindings.is_some())
            .field("catch_bindings", &self.catch_bindings)
            .field("direct_eval_var_conflicts", &self.direct_eval_var_conflicts)
            .field("direct_eval_with_stack", &self.direct_eval_with_stack.len())
            .field("module_host", &self.module_host_ref().is_some())
            .field("module_imports", &self.activation.creation.imports().keys())
            .field("module_live_bindings", &self.has_module_live_bindings())
            .finish()
    }
}

#[allow(dead_code)]
impl CallEnv {
    pub(crate) fn primitive_prototype(&self, name: &str) -> Option<ObjectRef> {
        let kind = crate::context::PrimitiveIntrinsic::from_name(name)?;
        self.activation.creation.intrinsics.primitive[kind.index()]
            .clone()
            .or_else(|| self.realm().primitive_prototype(kind))
    }
    pub(crate) fn creation_context(&self) -> Rc<crate::context::CreationContext> {
        self.activation.creation.clone()
    }
    pub(crate) fn creation_context_ref(&self) -> &Rc<crate::context::CreationContext> {
        &self.activation.creation
    }
    pub(crate) fn function_entry(
        &self,
        creation: Rc<crate::context::CreationContext>,
        capacity: usize,
    ) -> Self {
        Self {
            activation: crate::context::ActivationContext::new(
                creation,
                crate::context::ExecutionMode::Function,
            ),
            frame_bindings: FrameBindings::with_capacity(capacity),
            construct_target: None,
            deopt_bindings: None,
            global_binding_root: false,
            global_function_configurable: true,
            catch_bindings: None,
            immutable_function_name: None,
            direct_eval_var_conflicts: None,
            direct_eval_with_stack: Vec::new(),
            #[cfg(feature = "agents")]
            agent_context: self.agent_context.clone(),
        }
    }
    pub(crate) fn set_call_owner(&mut self, owner: crate::context::CallOwner) {
        self.activation.mode = owner.kind.into();
        if owner.kind == crate::context::OwnerKind::Module
            && !matches!(
                self.activation.creation.referrer,
                crate::context::ScriptOrModule::Module(_)
            )
        {
            Rc::make_mut(&mut self.activation.creation)
                .referrer
                .select_module();
        }
        *self.activation.call.borrow_mut() = crate::context::LexicalCallContext::Inline(owner);
    }
    pub(crate) fn inherit_call_owner(&mut self, owner: Rc<crate::context::CallOwner>) {
        *self.activation.call.borrow_mut() = crate::context::LexicalCallContext::Shared(owner);
    }
    pub(crate) fn capture_call_owner(&self) -> Option<Rc<crate::context::CallOwner>> {
        self.activation.call.borrow_mut().capture()
    }
    fn this_compatibility_value(&self) -> Option<Value> {
        self.activation.call.borrow().owner().map(|owner| {
            owner
                .this
                .read()
                .unwrap_or_else(|_| Value::Function(Function::uninitialized_lexical_marker()))
        })
    }
    pub(crate) fn this_value(&self) -> Result<Value, crate::RuntimeError> {
        self.activation
            .call
            .borrow()
            .owner()
            .map_or(Ok(Value::Undefined), |owner| owner.this.read())
    }
    pub(crate) fn lexical_new_target(&self) -> Option<Value> {
        self.activation
            .call
            .borrow()
            .owner()
            .and_then(|owner| owner.new_target.clone())
    }
    pub(crate) fn home_object(&self) -> Option<Value> {
        self.activation
            .call
            .borrow()
            .owner()
            .and_then(|owner| owner.home.clone())
    }
    pub(crate) fn super_constructor(&self) -> Option<Value> {
        self.activation
            .call
            .borrow()
            .owner()
            .and_then(|owner| owner.super_constructor.clone())
    }
    pub(crate) fn active_constructor(&self) -> Option<Value> {
        self.activation
            .call
            .borrow()
            .owner()
            .and_then(|owner| owner.active_constructor.clone())
    }
    pub(crate) fn owner_kind(&self) -> Option<crate::context::OwnerKind> {
        self.activation
            .call
            .borrow()
            .owner()
            .map(|owner| owner.kind)
    }

    pub(crate) fn set_global_selection(&mut self, value: Value) {
        let selected = Rc::make_mut(&mut self.activation.creation);
        selected.intrinsics = match &value {
            Value::Object(global) => crate::context::IntrinsicOverrides::for_marked_global(global),
            _ => Default::default(),
        };
        selected.global = crate::context::GlobalSelection::from_control(value);
    }
    pub(crate) fn explicit_global_selection_value(&self) -> Option<Value> {
        self.activation.creation.global.explicit_value()
    }
    pub(crate) fn dynamic_function_realm_value(&self) -> Option<Value> {
        self.explicit_global_selection_value().or_else(|| {
            self.realm()
                .dynamic_function_realm_global()
                .map(Value::Object)
        })
    }
    pub(crate) fn select_variable_boundary(&mut self, scope: crate::binding::ScopeId) {
        self.activation.variable.boundary = scope;
    }
    pub(crate) fn variable_identity(&self) -> crate::binding::VariableEnvIdentity {
        self.activation.variable
    }
    pub(crate) fn execution_state(&self) -> (crate::context::ExecutionMode, bool) {
        (self.activation.mode, self.activation.strict)
    }
    pub(crate) fn set_execution_state(
        &mut self,
        mode: crate::context::ExecutionMode,
        strict: bool,
    ) {
        self.activation.mode = mode;
        self.activation.strict = strict;
    }
    /// Builds an environment over `realm` with empty locals.
    pub(crate) fn new(realm: Realm) -> Self {
        Self {
            activation: crate::context::ActivationContext::new(
                crate::context::CreationContext::root(realm),
                crate::context::ExecutionMode::Script,
            ),
            frame_bindings: FrameBindings::default(),
            construct_target: None,
            deopt_bindings: None,
            global_binding_root: false,
            global_function_configurable: true,
            catch_bindings: None,
            immutable_function_name: None,
            direct_eval_var_conflicts: None,
            direct_eval_with_stack: Vec::new(),
            #[cfg(feature = "agents")]
            agent_context: None,
        }
    }

    /// The realm's dynamic-import host, if one was installed.
    pub(crate) fn module_host(&self) -> Option<crate::module::ModuleHostRef> {
        self.module_host_ref().cloned()
    }

    /// Borrows the dynamic-import host, for callers that only compare it.
    ///
    /// `module_host` hands out a clone, which is a reference-count round trip
    /// per question asked. The call-admission predicates ask on every call and
    /// only ever compare identity.
    pub(crate) fn module_host_ref(&self) -> Option<&crate::module::ModuleHostRef> {
        self.activation.creation.referrer.host()
    }

    /// Installs (or replaces) the dynamic-import host on this environment.
    pub(crate) fn set_module_host(&mut self, host: crate::module::ModuleHostRef) {
        if self
            .module_host_ref()
            .is_some_and(|old| Rc::ptr_eq(old, &host))
        {
            return;
        }
        Rc::make_mut(&mut self.activation.creation)
            .referrer
            .set_host(host);
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
        bindings: DynamicBindings,
        exported: String,
    ) {
        let module = Rc::make_mut(&mut self.activation.creation).module_mut();
        Rc::make_mut(&mut module.imports).insert(local_name, (bindings, exported));
    }

    /// Reads the current value of a live module import binding.
    pub(crate) fn module_import_value(&self, local_name: &str) -> Option<Value> {
        let (bindings, exported) = self.activation.creation.imports().get(local_name)?;
        Some(
            bindings
                .get(exported)
                .unwrap_or_else(|| Value::Function(Function::uninitialized_lexical_marker())),
        )
    }

    /// Returns the exporter's shared cell backing a live module import.
    /// Capturing an import must retain this identity instead of boxing the
    /// value currently observed by the importing module frame.
    pub(crate) fn module_import_cell(&self, local_name: &str) -> Option<BindingCell> {
        let (bindings, exported) = self.activation.creation.imports().get(local_name)?;
        bindings.cell(exported)
    }

    pub(crate) fn has_module_import(&self, local_name: &str) -> bool {
        self.activation.creation.imports().contains_key(local_name)
    }

    pub(crate) fn module_imports(&self) -> ModuleImports {
        self.activation.creation.imports().clone()
    }

    pub(crate) fn has_module_imports(&self) -> bool {
        !self.activation.creation.imports().is_empty()
    }

    pub(crate) fn set_module_imports(&mut self, imports: ModuleImports) {
        let old = self.activation.creation.imports();
        if Rc::ptr_eq(old, &imports) || old.is_empty() && imports.is_empty() {
            return;
        }
        Rc::make_mut(&mut self.activation.creation)
            .module_mut()
            .imports = imports;
    }

    pub(crate) fn set_module_live_bindings(&mut self, bindings: DynamicBindings) {
        self.activation.mode = crate::context::ExecutionMode::Module;
        self.activation.scopes.module_root = true;
        Rc::make_mut(&mut self.activation.creation)
            .module_mut()
            .live = Some(bindings);
    }

    pub(crate) fn has_module_live_bindings(&self) -> bool {
        self.activation.scopes.module_root
            && self
                .activation
                .creation
                .module
                .as_ref()
                .is_some_and(|module| module.live.is_some())
    }

    pub(crate) fn module_live_binding_cell(&self, name: &str) -> Option<BindingCell> {
        if !self.has_module_live_bindings() {
            return None;
        }
        self.activation
            .creation
            .module
            .as_ref()?
            .live
            .as_ref()?
            .cell(name)
    }

    /// Builds a standalone environment over a fresh, empty realm. Used by the
    /// no-context conversion helpers (`to_js_string`, `array_like_values`,
    /// `parse_json_text`) that run without a live VM; intrinsic lookups simply
    /// return `None`, matching the prior empty-`HashMap` behavior.
    pub(crate) fn detached() -> Self {
        Self::new(new_realm(HashMap::new()))
    }

    /// Wraps an owned flat map as a detached realm with no frame bindings.
    /// Used by native/dynamic compatibility entry points that receive an owned
    /// map rather than a live VM realm.
    pub(crate) fn from_map(map: HashMap<String, Value>) -> Self {
        Self::new(new_realm(map))
    }

    /// Builds an environment over `realm` with the given frame locals.
    pub(crate) fn with_locals(realm: Realm, locals: HashMap<String, Value>) -> Self {
        let mut env = Self::new(realm);
        env.frame_bindings = FrameBindings::from_values(locals);
        env
    }

    /// The shared realm cell, for sharing into a new frame or snapshot.
    pub(crate) fn realm(&self) -> &Realm {
        &self.activation.creation.realm
    }

    /// A clone of the realm `Rc` (shared cell, not a deep copy).
    pub(crate) fn realm_rc(&self) -> Realm {
        Rc::clone(&self.activation.creation.realm)
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
            activation: self.activation.fresh_function(),
            frame_bindings: FrameBindings::with_capacity(capacity),
            construct_target: None,
            deopt_bindings: None,
            global_binding_root: false,
            global_function_configurable: true,
            catch_bindings: self.catch_bindings.clone(),
            immutable_function_name: None,
            direct_eval_var_conflicts: self.direct_eval_var_conflicts.clone(),
            direct_eval_with_stack: self.direct_eval_with_stack.clone(),
            #[cfg(feature = "agents")]
            agent_context: self.agent_context.clone(),
        }
    }

    /// Builds the minimal context for an ordinary leaf function whose call
    /// contract excludes direct eval, `with`, closures, and special lexical
    /// bindings. Function-owned module routing is supplied directly and private
    /// state is installed by the caller, so copying the caller's transient maps
    /// here would only allocate them before immediately replacing them.
    pub(crate) fn new_direct_leaf_function_frame(&self, module_imports: ModuleImports) -> Self {
        let mut env = self.new_function_frame();
        env.catch_bindings = None;
        env.direct_eval_var_conflicts = None;
        env.direct_eval_with_stack.clear();
        env.set_module_imports(module_imports);
        env.set_private_environment(None);
        env
    }

    /// Builds an indirect-eval global frame. It has no caller locals, but it
    /// can read Script top-level lexical bindings through the global lexical
    /// environment instead of the global object.
    pub(crate) fn indirect_eval_frame(&self) -> Self {
        self.empty_frame()
    }

    pub(crate) fn declare_global_lexical(
        &self,
        meta: crate::binding::BindingMetaRef,
    ) -> BindingCell {
        if let Some(cell) = self.activation.creation.realm.cell(&meta.name) {
            if cell.metadata().kind == crate::binding::BindingKind::Lexical {
                return cell;
            }
            cell.set(Value::Function(Function::uninitialized_lexical_marker()));
        }
        let name = meta.name.clone();
        let cell = BindingCell::with_metadata(meta, None);
        self.activation
            .creation
            .realm
            .binding_cells()
            .insert_cell(name, cell.clone());
        cell
    }

    pub(crate) fn is_global_lexical_binding(&self, name: &str) -> bool {
        self.activation
            .creation
            .realm
            .cell(name)
            .is_some_and(|cell| cell.metadata().kind == crate::binding::BindingKind::Lexical)
    }

    pub(crate) fn is_immutable_lexical_binding(&self, name: &str) -> bool {
        self.activation
            .creation
            .realm
            .cell(name)
            .is_some_and(|cell| {
                cell.metadata().kind == crate::binding::BindingKind::Lexical
                    && !cell.metadata().mutable
            })
    }

    pub(crate) fn mark_catch_binding(&mut self, name: String) {
        Rc::make_mut(self.catch_bindings.get_or_insert_with(Default::default)).insert(name);
    }

    pub(crate) fn unmark_catch_binding(&mut self, name: &str) {
        if let Some(bindings) = &mut self.catch_bindings {
            Rc::make_mut(bindings).remove(name);
        }
    }

    pub(crate) fn is_catch_binding(&self, name: &str) -> bool {
        self.catch_bindings
            .as_ref()
            .is_some_and(|bindings| bindings.contains(name))
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
        // `None` is the empty set; dropping the handle detaches from any
        // frame that still shares the previous contents.
        self.direct_eval_var_conflicts = None;
    }

    pub(crate) fn mark_direct_eval_var_conflict(&mut self, name: String) {
        Rc::make_mut(
            self.direct_eval_var_conflicts
                .get_or_insert_with(Default::default),
        )
        .insert(name);
    }

    pub(crate) fn is_direct_eval_var_conflict(&self, name: &str) -> bool {
        self.direct_eval_var_conflicts
            .as_ref()
            .is_some_and(|conflicts| conflicts.contains(name))
    }

    /// Owned compatibility snapshot for the few dynamic-name consumers.
    pub(crate) fn binding_snapshot(&self) -> BindingSnapshot {
        BindingSnapshot(self.snapshot_locals())
    }

    /// Visits the currently visible bindings in the frame-local environment
    /// without materializing a name-keyed snapshot. This is reserved for cold
    /// dynamic-scope compatibility work such as direct eval and native-call
    /// writeback; ordinary bytecode bindings remain slot-indexed.
    pub(crate) fn for_each_visible_local_binding(&self, mut visit: impl FnMut(&str, &Value)) {
        let frame_bindings = self.frame_bindings.0.borrow();
        let mut visible_frame_names = HashSet::with_capacity(frame_bindings.len());
        for (name, value) in frame_bindings.iter().rev() {
            if visible_frame_names.insert(name.as_str()) {
                value.with_value(|value| visit(name, value));
            }
        }
        if let Some(deopt_bindings) = &self.deopt_bindings {
            for (name, value) in deopt_bindings.0.map.borrow().iter() {
                if !visible_frame_names.contains(name.as_str()) {
                    value.with_value(|value| visit(name, value));
                }
            }
        }
    }

    pub(crate) fn for_each_visible_local_value(&self, mut visit: impl FnMut(&Value)) {
        self.for_each_visible_local_binding(|_, value| visit(value));
    }

    /// Collects only binding names when values are irrelevant, avoiding the
    /// value clones and intermediate `HashMap` used by `binding_snapshot`.
    pub(crate) fn visible_local_names(&self) -> HashSet<String> {
        let mut names = HashSet::new();
        self.for_each_visible_local_binding(|name, _| {
            names.insert(name.to_owned());
        });
        names
    }

    /// Collects visible bindings in a sequential writeback buffer. Unlike a
    /// compatibility snapshot, this does not allocate or hash a temporary
    /// name-to-value map before the VM immediately iterates the result.
    /// Whether this frame carries any binding a write-back could observe. A
    /// realm-only environment -- what an ordinary frame now hands to builtins,
    /// accessors, coercion hooks, and iteration -- carries none, so its
    /// write-back has nothing to do.
    pub(crate) fn has_writable_frame_state(&self) -> bool {
        !self.frame_bindings.is_empty() || self.deopt_bindings.is_some()
    }

    pub(crate) fn visible_local_entries(&self) -> Vec<(String, Value)> {
        let mut entries = Vec::new();
        self.for_each_visible_local_binding(|name, value| {
            entries.push((name.to_owned(), value.clone()));
        });
        entries
    }

    /// Returns the lexical private-name environment for this frame, if any.
    pub(crate) fn private_environment(&self) -> Option<PrivateEnvironment> {
        self.activation.creation.private.clone()
    }

    /// Installs the lexical private-name environment for this frame.
    pub(crate) fn set_private_environment(&mut self, environment: Option<PrivateEnvironment>) {
        let same = match (&self.activation.creation.private, &environment) {
            (None, None) => true,
            (Some(a), Some(b)) => a.same(b),
            _ => false,
        };
        if !same {
            Rc::make_mut(&mut self.activation.creation).private = environment;
        }
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

    pub(crate) fn select_global_bindings(&mut self, global: bool) {
        self.global_binding_root = global;
        self.global_function_configurable = true;
        self.activation.scopes.entry = None;
    }
    pub(crate) fn select_script_bindings(&mut self) {
        self.select_global_bindings(true);
        self.global_function_configurable = false;
    }
    pub(crate) fn global_function_configurable(&self) -> bool {
        self.global_function_configurable
    }
    pub(crate) fn selected_global_this(&self) -> Option<Value> {
        match &self.activation.creation.global {
            crate::context::GlobalSelection::Override(global) => {
                Some(Value::Object(global.clone()))
            }
            crate::context::GlobalSelection::Default
            | crate::context::GlobalSelection::Disabled => self.realm().global_this(),
        }
    }

    pub(crate) fn can_cache_realm_globals(&self) -> bool {
        match &self.activation.creation.global {
            crate::context::GlobalSelection::Override(global) => {
                self.realm().is_global_object(global)
            }
            _ => true,
        }
    }

    pub(crate) fn has_global_binding_root(&self) -> bool {
        self.global_binding_root
    }
    pub(crate) fn entry_scope(&self) -> Option<&Rc<crate::binding::ScopeLease>> {
        self.activation.scopes.entry.as_ref()
    }
    pub(crate) fn set_eval_scope(&mut self, cursor: crate::binding::ScopeCursor) {
        self.activation.mode = crate::context::ExecutionMode::Eval;
        self.activation.scopes.module_root = false;
        if let Some(holder) = &cursor.variable.own_vars {
            self.activation.variable = holder.identity;
        }
        self.activation.scopes.entry = Some(cursor.head.clone());
        self.activation.scopes.cursor = Some(cursor);
        self.global_binding_root = false;
    }

    pub(crate) fn clear_deopt_bindings(&mut self) {
        self.deopt_bindings = None;
    }

    pub(crate) fn scope_cursor(&self) -> Option<&crate::binding::ScopeCursor> {
        self.activation.scopes.cursor.as_ref()
    }
    pub(crate) fn set_scope_cursor(&mut self, cursor: Option<crate::binding::ScopeCursor>) {
        self.activation.scopes.cursor = cursor;
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

    pub(crate) fn remove_deopt_cell_if(&mut self, name: &str, expected: &BindingCell) {
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

    /// Looks up `name`: frame locals first, then a short realm borrow. Returns
    /// an owned value because a value behind the realm `RefCell` cannot be
    /// handed out by reference.
    pub(crate) fn get(&self, name: &str) -> Option<Value> {
        if name == "this"
            && let Some(value) = self.this_compatibility_value()
        {
            return Some(value);
        }
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
        if let Some(value) = self.activation.creation.realm.get_value(name) {
            return Some(value);
        }
        None
    }

    /// The realm table a read of a name from this frame reaches first, when
    /// no frame or deoptimized layer sits above it -- then [`Self::get`] of any
    /// name the table binds, other than `new.target`, is that binding's cell.
    pub(crate) fn realm_read_layer(&self) -> Option<&DynamicBindings> {
        if !self.frame_bindings.is_empty()
            || self.deopt_bindings.is_some()
            || !self.can_cache_realm_globals()
        {
            return None;
        }
        Some(self.activation.creation.realm.binding_cells())
    }

    /// Looks up `name` in the shared realm layer only.
    pub(crate) fn get_realm(&self, name: &str) -> Option<Value> {
        self.activation.creation.realm.get_value(name)
    }

    /// Returns the active global object override used by indirect eval and
    /// dynamically constructed functions. A frame-local marker, including a
    /// non-object value that disables an outer override, wins before the
    /// realm's cached marker.
    pub(crate) fn dynamic_function_realm_global(&self) -> Option<ObjectRef> {
        match &self.activation.creation.global {
            crate::context::GlobalSelection::Override(global) => Some(global.clone()),
            crate::context::GlobalSelection::Disabled => None,
            crate::context::GlobalSelection::Default => {
                self.realm().dynamic_function_realm_global()
            }
        }
    }

    /// Whether [`Self::dynamic_function_realm_global`] would answer `Some`,
    /// without cloning the object. Frame setup asks this on every call.
    pub(crate) fn has_dynamic_function_realm_global(&self) -> bool {
        match &self.activation.creation.global {
            crate::context::GlobalSelection::Override(_) => true,
            crate::context::GlobalSelection::Disabled => false,
            crate::context::GlobalSelection::Default => {
                self.realm().has_dynamic_function_realm_global()
            }
        }
    }

    /// Preserve marked-realm compatibility views, including a frame marker
    /// whose non-object value deliberately disables an outer override.
    pub(crate) fn has_realm_override_context(&self) -> bool {
        !matches!(
            self.activation.creation.global,
            crate::context::GlobalSelection::Default
        ) || self.realm().has_dynamic_function_realm_global()
    }

    pub(crate) fn cached_direct_eval_bytecode(
        &self,
        source: &JsString,
        context: &EvalParseContext,
    ) -> Option<Rc<Bytecode>> {
        self.activation
            .creation
            .realm
            .cached_direct_eval_bytecode(source, context)
    }

    pub(crate) fn cache_direct_eval_bytecode(
        &self,
        source: JsString,
        context: EvalParseContext,
        bytecode: Rc<Bytecode>,
    ) {
        self.activation
            .creation
            .realm
            .cache_direct_eval_bytecode(source, context, bytecode);
    }

    #[cfg(test)]
    pub(crate) fn direct_eval_cache_len(&self) -> usize {
        self.activation.creation.realm.direct_eval_cache_len()
    }

    pub(crate) fn object_prototype_intrinsic_override(&self) -> Option<ObjectRef> {
        self.activation.creation.intrinsics.object.clone()
    }
    pub(crate) fn array_prototype_intrinsic_override(&self) -> Option<ObjectRef> {
        self.activation.creation.intrinsics.array.clone()
    }

    /// Returns the realm's internal global-this slot without hashing its
    /// private string key. The slot is fixed when a realm is created; writes
    /// to the public `globalThis` property do not replace this identity.
    pub(crate) fn global_this(&self) -> Option<Value> {
        self.selected_global_this()
    }

    /// Returns the one shared cell for a realm binding. Every realm binding is
    /// cell-backed from the moment it is created (`RealmState`'s single
    /// `DynamicBindings` map), so this is just a lookup — no more lazy
    /// cell creation on first capture.
    pub(crate) fn realm_binding_cell(&self, name: &str) -> Option<BindingCell> {
        self.activation.creation.realm.cell(name)
    }

    pub(crate) fn remove_realm(&self, name: &str) -> Option<Value> {
        // Capture the pre-removal value before marking the cell, matching the
        // old two-map model's behavior: a caller that already holds this
        // cell (e.g. a closure that captured the binding) observes the
        // uninitialized marker, while the returned value here is the value
        // that was live immediately before removal.
        let removed = self.activation.creation.realm.get_value(name);
        if let Some(cell) = self.activation.creation.realm.cell(name) {
            cell.set(Value::Function(Function::uninitialized_lexical_marker()));
        }
        self.activation.creation.realm.remove_value(name);
        self.activation
            .creation
            .realm
            .sync_dynamic_function_realm_binding(name, None);
        removed
    }

    pub(crate) fn is_realm_binding_cell(&self, name: &str, cell: &BindingCell) -> bool {
        self.activation.creation.realm.is_binding_cell(name, cell)
    }

    /// True if `name` is bound in either layer.
    pub(crate) fn contains_key(&self, name: &str) -> bool {
        self.frame_bindings.contains_key(name)
            || self
                .deopt_bindings
                .as_ref()
                .is_some_and(|bindings| bindings.contains_key(name))
            || self.activation.creation.realm.contains(name)
    }

    /// Construct dispatch target only. The lexical new.target stays in the
    /// call owner and cannot be overwritten by nested native calls.
    pub(crate) fn construct_target(&self) -> Option<Value> {
        self.construct_target.clone()
    }
    pub(crate) fn set_construct_target(&mut self, value: Option<Value>) -> Option<Value> {
        match value {
            Some(value) => self.construct_target.replace(value),
            None => self.construct_target.take(),
        }
    }

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
    pub(crate) fn insert_frame_cell(&self, name: String, value: BindingCell) {
        self.frame_bindings.insert_cell(name, value);
    }

    /// Inserts directly into the shared realm cell (builtin install and global
    /// definition). Visible to every frame sharing the realm.
    pub(crate) fn insert_realm(&self, name: String, value: Value) -> Option<Value> {
        self.activation
            .creation
            .realm
            .sync_dynamic_function_realm_binding(&name, Some(&value));
        self.activation.creation.realm.insert_value(name, value)
    }

    /// Replaces an existing realm binding. Hot global stores use this after
    /// proving that the corresponding global-object property is still an
    /// ordinary writable data property.
    pub(crate) fn replace_existing_realm(&self, name: &str, value: Value) -> bool {
        let Some(cell) = self.activation.creation.realm.cell(name) else {
            return false;
        };
        cell.set(value.clone());
        self.activation
            .creation
            .realm
            .sync_dynamic_function_realm_binding(name, Some(&value));
        true
    }

    /// Same as [`Self::replace_existing_realm`], but for a caller that already
    /// holds this name's realm-binding cell (e.g. via a slot's `local_upvalues`
    /// entry, gated by the `realm_binding_slots` bitset) — every realm binding
    /// is cell-backed from creation, so the caller's cell already *is* the
    /// canonical one and no lookup is needed at all.
    pub(crate) fn replace_existing_realm_with_cell(
        &self,
        name: &str,
        value: Value,
        cell: &BindingCell,
    ) -> bool {
        cell.set(value.clone());
        self.activation
            .creation
            .realm
            .sync_dynamic_function_realm_binding(name, Some(&value));
        true
    }

    /// Whether `object` is this realm's global object, without cloning it.
    pub(crate) fn is_realm_global_object(&self, object: &ObjectRef) -> bool {
        self.activation.creation.realm.is_global_object(object)
    }

    /// Mirrors a data-property definition on this realm's global object into
    /// the realm value table and any already-captured global cell.
    pub(crate) fn sync_realm_global_object_property(&self, object: &ObjectRef, name: &str) {
        if !self.activation.creation.realm.is_global_object(object)
            || !self.activation.creation.realm.contains(name)
            || self.is_global_lexical_binding(name)
        {
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

    /// Forgets the realm binding that mirrored a property just deleted from
    /// this realm's global object (`delete globalThis.x`,
    /// `Reflect.deleteProperty(globalThis, 'x')`), so the name stops
    /// resolving exactly as it does after `delete x`. A script-level lexical
    /// binding of the same name is a different binding and stays.
    pub(crate) fn forget_deleted_global_object_property(&self, object: &ObjectRef, name: &str) {
        if !self.activation.creation.realm.is_global_object(object)
            || !self.activation.creation.realm.contains(name)
            || self.is_global_lexical_binding(name)
        {
            return;
        }
        self.remove_realm(name);
    }

    /// Defines `name` in the shared realm only if it is not already bound there.
    /// Used by global-binding initialization (script and indirect-eval scopes).
    pub(crate) fn realm_entry_or_insert(&self, name: String, value: Value) {
        let refresh_dynamic_realm = name == DYNAMIC_FUNCTION_REALM_GLOBAL;
        self.activation
            .creation
            .realm
            .entry_or_insert_value(name, value);
        if refresh_dynamic_realm {
            self.activation
                .creation
                .realm
                .refresh_dynamic_function_realm_global();
        }
    }

    /// True if the shared realm cell binds `name`.
    pub(crate) fn realm_contains(&self, name: &str) -> bool {
        self.activation.creation.realm.contains(name)
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
        if name == "this" {
            return self.this_compatibility_value();
        }
        self.frame_bindings.get(name).or_else(|| {
            self.deopt_bindings
                .as_ref()
                .and_then(|bindings| bindings.get(name))
        })
    }

    pub(crate) fn has_frame_binding(&self, name: &str) -> bool {
        if name == "this" {
            return self.activation.call.borrow().owner().is_some();
        }
        self.frame_bindings.contains_key(name)
    }

    pub(crate) fn has_local_binding(&self, name: &str) -> bool {
        if name == "this" {
            return self.activation.call.borrow().owner().is_some();
        }
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
            && self.activation.creation.imports().is_empty()
            && !self.has_module_live_bindings()
            && self.immutable_function_name.is_none()
    }

    /// Whether this environment can supply a value for *any* name, which is
    /// what makes a frame's slots authoritative independently of the name being
    /// asked about. A frame with no bindings answers the whole slot mask at
    /// once, rather than one environment query per local.
    pub(crate) fn supplies_no_named_binding(&self) -> bool {
        self.frame_bindings.is_empty()
            && matches!(
                self.activation.creation.global,
                crate::context::GlobalSelection::Default
            )
            && self.activation.scopes.cursor.is_none()
            && self.deopt_bindings.is_none()
            && self.activation.creation.imports().is_empty()
            && !self.has_module_live_bindings()
            && self.immutable_function_name.is_none()
    }

    pub(crate) fn frame_binding_cell(&self, name: &str) -> Option<BindingCell> {
        if name == "this" {
            return self.derived_this_cell();
        }
        self.frame_bindings.cell(name)
    }

    /// Returns the live cell for a binding supplied by the current caller or
    /// dynamic environment, without falling through to the realm. Direct eval
    /// uses this to keep a same-named function local distinct from a global.
    pub(crate) fn local_binding_cell(&self, name: &str) -> Option<BindingCell> {
        if name == "this" {
            return self.derived_this_cell();
        }
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

    /// Builds an isolated dynamic view of this execution frame without a
    /// name-keyed intermediate map. Explicit direct-eval/with cells are added
    /// by the VM after it overlays live slot values.
    pub(crate) fn fork_current_frame_values(&self) -> Self {
        let mut env = self.clone();
        env.frame_bindings = self.frame_bindings.fork_values();
        env.deopt_bindings = None;
        env.activation.scopes.cursor = None;
        env.activation.scopes.entry = None;
        env
    }

    /// The implicit binding of one derived activation, never a realm cell.
    /// Its marker denotes TDZ; forks and lexical arrows must retain identity.
    pub(crate) fn derived_this_cell(&self) -> Option<BindingCell> {
        self.activation
            .call
            .borrow()
            .owner()
            .and_then(|owner| owner.this.derived_cell())
    }
}

#[cfg(test)]
mod tests;
