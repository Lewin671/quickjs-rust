//! Module graph loading, linking, and evaluation.
//!
//! The graph is keyed by canonical module key. Loading is depth-first and
//! deduplicating; linking resolves every import to an exporting binding (or a
//! namespace object) and reports unresolvable imports / ambiguous star exports
//! as SyntaxErrors; evaluation is a post-order DFS over the dependency graph,
//! mirroring the instantiation/evaluation state machine of ECMAScript 16.2.1.5
//! (`Unlinked` -> `Linking` -> `Linked` -> `Evaluating` -> `Evaluated`).

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{ModuleNamespaceBindings, RuntimeError, Value, bytecode};

use super::namespace::{empty_namespace, populate_namespace};
use super::records::{ImportName, ModuleRecord, NAMESPACE_BINDING, build_record};
use super::resolver::ModuleResolver;

/// A linking-phase failure: a parse error or a SyntaxError-class link error.
#[derive(Clone, Debug)]
pub(super) struct LinkError {
    pub(super) kind: LinkErrorKind,
    pub(super) message: String,
    pub(super) thrown: Option<Box<Value>>,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum LinkErrorKind {
    Parse,
    Syntax,
    /// A runtime failure raised while evaluating a module body (used only on the
    /// dynamic-import path, where it rejects the import promise).
    Runtime,
}

impl LinkError {
    fn syntax(message: impl Into<String>) -> Self {
        Self {
            kind: LinkErrorKind::Syntax,
            message: message.into(),
            thrown: None,
        }
    }
}

/// Per-module instantiation/evaluation status.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Status {
    Unlinked,
    Linking,
    Linked,
    Evaluating,
    Evaluated,
}

/// A loaded module: its static record, the canonical keys its requested
/// specifiers resolve to, its status, and (once evaluated) its bindings.
struct Module {
    record: ModuleRecord,
    status: Status,
    /// Resolved canonical key per requested specifier.
    resolved_requests: HashMap<String, String>,
    /// The module's exported binding values, populated after evaluation:
    /// export name -> value.
    exports: HashMap<String, Value>,
    /// Live lexical binding storage shared with closures created by this
    /// module body. Export reads consult this before falling back to the realm.
    live_lexical: Rc<RefCell<HashMap<String, Value>>>,
    /// Cached namespace object, built lazily.
    namespace: Option<Value>,
}

/// The module graph: a registry of modules keyed by canonical key, plus the
/// single realm shared by every module in the graph.
pub(super) struct ModuleGraph {
    modules: HashMap<String, Module>,
    realm: bytecode::ModuleRealm,
    /// The realm's dynamic-import host, set once the graph is wrapped in a host
    /// so module bodies can reach it for nested `import()`. `None` until then.
    host: Option<super::host::ModuleHostRef>,
    /// Shared handle to this graph, copied from the installed host so module
    /// evaluation can create per-module hosts without borrowing the root host.
    host_graph: Option<std::rc::Rc<std::cell::RefCell<ModuleGraph>>>,
    /// The owned host resolver, retained for the realm's lifetime so a dynamic
    /// `import()` can resolve specifiers long after the initial load.
    resolver: Option<Box<dyn ModuleResolver>>,
}

impl ModuleGraph {
    pub(super) fn new() -> Self {
        Self::with_realm(bytecode::new_module_realm())
    }

    pub(super) fn with_realm(realm: bytecode::ModuleRealm) -> Self {
        Self {
            modules: HashMap::new(),
            realm,
            host: None,
            host_graph: None,
            resolver: None,
        }
    }

    /// Records the dynamic-import host so evaluated module bodies carry it.
    pub(super) fn set_host(&mut self, host: super::host::ModuleHostRef) {
        self.host_graph = Some(host.borrow().graph_ref());
        self.host = Some(host);
    }

    /// Installs the owned host resolver.
    pub(super) fn set_resolver(&mut self, resolver: Box<dyn ModuleResolver>) {
        self.resolver = Some(resolver);
    }

    /// Loads the root module using the graph's own (owned) resolver.
    pub(super) fn load_root(&mut self, specifier: &str, source: &str) -> Result<String, LinkError> {
        let mut resolver = self.resolver.take().expect("resolver installed");
        let result = self.load(specifier, source, resolver.as_mut());
        self.resolver = Some(resolver);
        result
    }

    /// Resolves and evaluates `specifier` against `referrer` using the graph's
    /// own resolver, returning the requested module's namespace object.
    pub(super) fn import_dynamic_owned(
        &mut self,
        specifier: &str,
        referrer: &str,
    ) -> Result<Value, LinkError> {
        let mut resolver = self.resolver.take().expect("resolver installed");
        let result = self.import_dynamic(specifier, referrer, resolver.as_mut());
        self.resolver = Some(resolver);
        result
    }

    /// The shared graph realm (`Rc::clone`), used to wire the dynamic-import
    /// host into a module's evaluation environment.
    pub(super) fn realm(&self) -> bytecode::ModuleRealm {
        self.realm.clone()
    }

    /// Loads, links, and evaluates the module reached by resolving `specifier`
    /// from `referrer`, returning its namespace object. Reuses any module
    /// already present in the graph so the same key yields the same namespace.
    /// This is the dynamic-`import()` entry point.
    pub(super) fn import_dynamic(
        &mut self,
        specifier: &str,
        referrer: &str,
        resolver: &mut dyn ModuleResolver,
    ) -> Result<Value, LinkError> {
        let resolved = resolver.resolve(specifier, referrer).map_err(|error| {
            LinkError::syntax(format!(
                "{} (importing '{specifier}' from '{referrer}')",
                error.message
            ))
        })?;
        let key = resolved.key.clone();
        if !self.modules.contains_key(&key) {
            let record = build_record(&resolved.source).map_err(|message| LinkError {
                kind: LinkErrorKind::Parse,
                message,
                thrown: None,
            })?;
            self.insert_and_load_dependencies(key.clone(), record, resolver)?;
        }
        self.link(&key)?;
        self.evaluate_with_drain(&key, false)
            .map_err(|error| LinkError {
                // A runtime failure during module evaluation rejects the import
                // promise. Preserve the original JS throw completion when the
                // VM provides one so `throw 'x'` rejects with `'x'`, not a
                // synthesized Error object.
                kind: LinkErrorKind::Runtime,
                message: error.message,
                thrown: error.thrown,
            })?;
        Ok(self.namespace(&key))
    }

    /// Evaluates a prelude *script* against the graph's shared realm before any
    /// module body runs, so its top-level bindings (e.g. Test262 harness
    /// includes) are visible to every module in the graph.
    pub(super) fn eval_prelude(&self, source: &str) -> Result<(), RuntimeError> {
        bytecode::eval_prelude_script(source, &self.realm)
    }

    /// Loads the module identified by `(specifier, source)` and, depth-first,
    /// every module it requests, returning the root module's canonical key.
    pub(super) fn load(
        &mut self,
        specifier: &str,
        source: &str,
        resolver: &mut dyn ModuleResolver,
    ) -> Result<String, LinkError> {
        let record = build_record(source).map_err(|message| LinkError {
            kind: LinkErrorKind::Parse,
            message,
            thrown: None,
        })?;
        let key = specifier.to_owned();
        self.insert_and_load_dependencies(key.clone(), record, resolver)?;
        Ok(key)
    }

    fn insert_and_load_dependencies(
        &mut self,
        key: String,
        record: ModuleRecord,
        resolver: &mut dyn ModuleResolver,
    ) -> Result<(), LinkError> {
        if self.modules.contains_key(&key) {
            return Ok(());
        }
        let requested = record.requested_modules.clone();
        self.modules.insert(
            key.clone(),
            Module {
                record,
                status: Status::Unlinked,
                resolved_requests: HashMap::new(),
                exports: HashMap::new(),
                live_lexical: Rc::new(RefCell::new(HashMap::new())),
                namespace: None,
            },
        );
        for specifier in requested {
            let resolved = resolver.resolve(&specifier, &key).map_err(|error| {
                LinkError::syntax(format!(
                    "SyntaxError: {} (importing '{specifier}' from '{key}')",
                    error.message
                ))
            })?;
            self.modules
                .get_mut(&key)
                .expect("module just inserted")
                .resolved_requests
                .insert(specifier, resolved.key.clone());
            if !self.modules.contains_key(&resolved.key) {
                let dep_record = build_record(&resolved.source).map_err(|message| LinkError {
                    kind: LinkErrorKind::Parse,
                    message,
                    thrown: None,
                })?;
                self.insert_and_load_dependencies(resolved.key, dep_record, resolver)?;
            }
        }
        Ok(())
    }

    /// Links the graph rooted at `key`: validates that every import resolves to
    /// an exporting binding (or namespace) and that no star export is ambiguous.
    pub(super) fn link(&mut self, key: &str) -> Result<(), LinkError> {
        match self.status(key) {
            Status::Linking | Status::Linked | Status::Evaluating | Status::Evaluated => {
                return Ok(());
            }
            Status::Unlinked => {}
        }
        self.set_status(key, Status::Linking);
        let deps = self.dependency_keys(key);
        for dep in deps {
            self.link(&dep)?;
        }
        // Validate imports resolve to a concrete export or a namespace.
        let imports = self.import_specs(key);
        for (module_request, import_name, local) in imports {
            let target = self.resolved(key, &module_request);
            if let ImportName::Named(name) = &import_name
                && self
                    .resolve_export(&target, name, &mut Vec::new())?
                    .is_none()
            {
                return Err(LinkError::syntax(format!(
                    "SyntaxError: module '{target}' has no export '{name}' \
                     (imported as '{local}' in '{key}')"
                )));
            }
        }
        // ModuleDeclarationInstantiation calls ResolveExport on every exported
        // name; an indirect re-export (`export { x } from './m'`) that resolves
        // to nothing (a re-export cycle) or to two distinct bindings (ambiguous)
        // is a SyntaxError. `resolve_export` already reports the ambiguous case;
        // the cycle case surfaces here as an unresolved (`None`) result.
        let indirect_export_names: Vec<String> = self.modules[key]
            .record
            .indirect_exports
            .iter()
            .map(|indirect| indirect.export_name.clone())
            .collect();
        for name in indirect_export_names {
            if self.resolve_export(key, &name, &mut Vec::new())?.is_none() {
                return Err(LinkError::syntax(format!(
                    "SyntaxError: module '{key}' re-export '{name}' could not be resolved"
                )));
            }
        }
        self.set_status(key, Status::Linked);
        Ok(())
    }

    /// Resolves an export name against module `key` per ResolveExport,
    /// following star re-exports. Returns the resolved `(module_key,
    /// export_name)` binding, `None` when not found, or a SyntaxError on an
    /// ambiguous star resolution.
    fn resolve_export(
        &self,
        key: &str,
        name: &str,
        visited: &mut Vec<(String, String)>,
    ) -> Result<Option<(String, String)>, LinkError> {
        let probe = (key.to_owned(), name.to_owned());
        if visited.contains(&probe) {
            // A cycle in star resolution: this path contributes no binding.
            return Ok(None);
        }
        visited.push(probe);
        let module = &self.modules[key];
        for local in &module.record.local_exports {
            if local.export_name == name {
                return Ok(Some((key.to_owned(), name.to_owned())));
            }
        }
        for indirect in &module.record.indirect_exports {
            if indirect.export_name == name {
                let target = self.resolved(key, &indirect.module_request);
                return match &indirect.import_name {
                    ImportName::Namespace => Ok(Some((target, NAMESPACE_BINDING.to_owned()))),
                    ImportName::Named(inner) => self.resolve_export(&target, inner, visited),
                };
            }
        }
        if name == "default" {
            return Err(LinkError::syntax(format!(
                "SyntaxError: module '{key}' has no default export"
            )));
        }
        // `export * from` aggregation: a name found in exactly one star target
        // resolves; found in two distinct bindings is ambiguous.
        let mut found: Option<(String, String)> = None;
        for star in &module.record.star_exports {
            let target = self.resolved(key, star);
            if let Some(resolution) = self.resolve_export(&target, name, visited)? {
                match &found {
                    Some(existing) if *existing != resolution => {
                        return Err(LinkError::syntax(format!(
                            "SyntaxError: ambiguous star export '{name}' in module '{key}'"
                        )));
                    }
                    _ => found = Some(resolution),
                }
            }
        }
        Ok(found)
    }

    /// Evaluates the graph rooted at `key` with a post-order DFS over
    /// dependencies, then evaluates `key` itself. `drain` controls whether each
    /// module body drains its promise jobs: the top-level static path drains;
    /// the dynamic-import path defers to the outer job-queue loop so the (then
    /// borrowed) graph is not re-entered mid-evaluation.
    pub(super) fn evaluate(&mut self, key: &str) -> Result<(), RuntimeError> {
        // Defer promise-job draining (including dynamic `import()`) to the
        // top-level loop run with the graph borrow released, so a job can
        // re-borrow the graph without a double-borrow panic.
        self.evaluate_with_drain(key, false)
    }

    fn evaluate_with_drain(&mut self, key: &str, drain: bool) -> Result<(), RuntimeError> {
        match self.status(key) {
            Status::Evaluating | Status::Evaluated => return Ok(()),
            _ => {}
        }
        self.set_status(key, Status::Evaluating);
        let deps = self.dependency_keys(key);
        for dep in deps {
            if let Err(error) = self.evaluate_with_drain(&dep, drain) {
                self.set_status(key, Status::Linked);
                return Err(error);
            }
        }
        if let Err(error) = self.evaluate_body(key, drain) {
            self.set_status(key, Status::Linked);
            return Err(error);
        }
        self.set_status(key, Status::Evaluated);
        Ok(())
    }

    /// Compiles and runs one module body against a fresh realm seeded with its
    /// resolved imports, then records its exported binding values.
    fn evaluate_body(&mut self, key: &str, drain: bool) -> Result<(), RuntimeError> {
        let compiled = {
            let module = &self.modules[key];
            bytecode::compile_module(&module.record.body)?
        };
        let live_names: Vec<String> = self.modules[key]
            .record
            .local_exports
            .iter()
            .map(|export| export.local_name.clone())
            .collect();
        let live_bindings = self.modules[key].live_lexical.clone();
        let seed_tdz_markers = self.needs_module_live_tdz_seed(key);
        let imports = self.import_bindings(key)?;
        let live_imports = self.import_live_bindings(key);
        let live_exports = bytecode::ModuleLiveExports {
            names: live_names,
            bindings: live_bindings,
            seed_tdz_markers,
            imports: live_imports,
        };
        bytecode::seed_module_live_bindings(&compiled, &live_exports);
        let host = self
            .host_graph
            .as_ref()
            .map(|graph| super::host::ModuleHost::new(graph.clone(), key.to_owned()).into_ref());
        let evaluation =
            bytecode::eval_module_body(&compiled, &self.realm, imports, host, live_exports, drain)?;
        {
            let mut live = evaluation.captured_env.borrow_mut();
            for (name, value) in evaluation.env.locals() {
                live.entry(name.clone()).or_insert_with(|| value.clone());
            }
        }
        // Snapshot exported binding values from the module's frame environment.
        let export_pairs = self.collect_export_values(key, &evaluation.env)?;
        {
            let mut live = evaluation.captured_env.borrow_mut();
            for local in &self.modules[key].record.local_exports {
                if let Some((_, value)) = export_pairs
                    .iter()
                    .find(|(export_name, _)| export_name == &local.export_name)
                {
                    live.insert(local.local_name.clone(), value.clone());
                }
            }
        }
        let module = self.modules.get_mut(key).expect("module exists");
        module.live_lexical = evaluation.captured_env;
        module.exports = export_pairs.into_iter().collect();
        Ok(())
    }

    /// Builds the local-name -> value map seeded into the module realm: each
    /// import entry resolved to the exporting module's already-evaluated value
    /// (or namespace object).
    fn import_bindings(&mut self, key: &str) -> Result<HashMap<String, Value>, RuntimeError> {
        let entries = self.modules[key].record.import_entries.to_vec();
        let mut bindings = HashMap::new();
        for entry in entries {
            let target = self.resolved(key, &entry.module_request);
            let value = match &entry.import_name {
                ImportName::Namespace => self.namespace(&target),
                ImportName::Named(name) => self.export_value(&target, name),
            };
            bindings.insert(entry.local_name, value);
        }
        Ok(bindings)
    }

    fn import_live_bindings(&mut self, key: &str) -> Vec<bytecode::ModuleLiveImport> {
        let entries = self.modules[key].record.import_entries.to_vec();
        let mut imports = Vec::new();
        for entry in entries {
            let target = self.resolved(key, &entry.module_request);
            if matches!(entry.import_name, ImportName::Namespace) {
                let mut bindings = HashMap::new();
                bindings.insert(NAMESPACE_BINDING.to_owned(), self.namespace(&target));
                imports.push(bytecode::ModuleLiveImport {
                    local_name: entry.local_name,
                    bindings: Rc::new(RefCell::new(bindings)),
                    binding_name: NAMESPACE_BINDING.to_owned(),
                });
                continue;
            }
            let ImportName::Named(name) = &entry.import_name else {
                continue;
            };
            let Ok(Some((module_key, export_name))) =
                self.resolve_export(&target, name, &mut Vec::new())
            else {
                continue;
            };
            if export_name == NAMESPACE_BINDING {
                continue;
            }
            let Some(binding_name) = self.modules[&module_key]
                .record
                .local_exports
                .iter()
                .find(|local| local.export_name == export_name)
                .map(|local| local.local_name.clone())
            else {
                continue;
            };
            imports.push(bytecode::ModuleLiveImport {
                local_name: entry.local_name,
                bindings: self.modules[&module_key].live_lexical.clone(),
                binding_name,
            });
        }
        imports
    }

    fn needs_module_live_tdz_seed(&self, key: &str) -> bool {
        self.modules[key]
            .record
            .import_entries
            .iter()
            .any(|entry| self.resolved(key, &entry.module_request) == key)
            || self.modules[key].namespace.is_some()
    }

    /// Reads the value of `name` exported by `key`, following indirect/star
    /// resolution to the binding that actually holds it.
    fn export_value(&mut self, key: &str, name: &str) -> Value {
        match self.resolve_export(key, name, &mut Vec::new()) {
            Ok(Some((module_key, export_name))) if export_name == NAMESPACE_BINDING => {
                self.namespace(&module_key)
            }
            Ok(Some((module_key, export_name))) => {
                self.live_export_value(&module_key, &export_name)
            }
            // A namespace-valued indirect export resolves to the target's
            // namespace object.
            _ => Value::Undefined,
        }
    }

    fn live_export_value(&self, key: &str, export_name: &str) -> Value {
        let module = &self.modules[key];
        if let Some(local_name) = module
            .record
            .local_exports
            .iter()
            .find(|local| local.export_name == export_name)
            .map(|local| local.local_name.as_str())
        {
            if let Some(value) = module.live_lexical.borrow().get(local_name) {
                return value.clone();
            }
            if let Some(value) = self.realm.borrow().get(local_name) {
                return value.clone();
            }
        }
        module
            .exports
            .get(export_name)
            .cloned()
            .unwrap_or(Value::Undefined)
    }

    /// Reads the local export values from a freshly evaluated module frame.
    fn collect_export_values(
        &self,
        key: &str,
        env: &crate::CallEnv,
    ) -> Result<Vec<(String, Value)>, RuntimeError> {
        let module = &self.modules[key];
        let mut pairs = Vec::new();
        for local in &module.record.local_exports {
            let mut value = module
                .live_lexical
                .borrow()
                .get(&local.local_name)
                .filter(|value| **value != Value::Undefined)
                .cloned()
                .or_else(|| env.get(&local.local_name))
                .unwrap_or(Value::Undefined);
            if value == Value::Undefined
                && let Some(global_value) = self.global_this_property(&local.local_name)
            {
                value = global_value;
            }
            pairs.push((local.export_name.clone(), value));
        }
        Ok(pairs)
    }

    fn global_this_property(&self, name: &str) -> Option<Value> {
        let global_this = match self.realm.borrow().get(crate::GLOBAL_THIS_BINDING) {
            Some(Value::Object(global_this)) => Some(global_this.clone()),
            _ => None,
        }?;
        global_this
            .own_property(name)
            .map(|property| property.value)
    }

    /// The module namespace object for `key`, built (and cached) lazily from its
    /// resolved export names.
    pub(super) fn namespace(&mut self, key: &str) -> Value {
        if let Some(namespace) = &self.modules[key].namespace {
            return namespace.clone();
        }
        let env = crate::CallEnv::new(self.realm.clone());
        let live_bindings =
            ModuleNamespaceBindings::new(self.modules[key].live_lexical.clone(), HashMap::new());
        let namespace = empty_namespace(live_bindings);
        self.modules.get_mut(key).expect("module exists").namespace =
            Some(Value::Object(namespace.clone()));

        let names = self.export_names(key, &mut Vec::new());
        let mut bindings = Vec::new();
        let mut aliases = HashMap::new();
        for name in names {
            // A namespace omits any name whose resolution is ambiguous.
            if let Some((module_key, export_name)) = self
                .resolve_export(key, &name, &mut Vec::new())
                .ok()
                .flatten()
            {
                let namespace_name = name.clone();
                let value = self.export_value(key, &name);
                bindings.push((name, value));
                if export_name != NAMESPACE_BINDING
                    && let Some(local_name) = self.modules[&module_key]
                        .record
                        .local_exports
                        .iter()
                        .find(|local| local.export_name == export_name)
                        .map(|local| local.local_name.clone())
                {
                    aliases.insert(
                        namespace_name,
                        (self.modules[&module_key].live_lexical.clone(), local_name),
                    );
                }
            }
        }
        let live_bindings =
            ModuleNamespaceBindings::new(self.modules[key].live_lexical.clone(), aliases);
        namespace.set_module_namespace_bindings(live_bindings);
        populate_namespace(&namespace, &mut bindings, &env);
        Value::Object(namespace)
    }

    /// The exported names of `key` (GetExportedNames), including names
    /// contributed by `export * from` targets, excluding `default` from star
    /// aggregation.
    fn export_names(&self, key: &str, visited: &mut Vec<String>) -> Vec<String> {
        if visited.contains(&key.to_owned()) {
            return Vec::new();
        }
        visited.push(key.to_owned());
        let module = &self.modules[key];
        let mut names = Vec::new();
        let push = |name: &str, names: &mut Vec<String>| {
            if !names.iter().any(|existing| existing == name) {
                names.push(name.to_owned());
            }
        };
        for local in &module.record.local_exports {
            push(&local.export_name, &mut names);
        }
        for indirect in &module.record.indirect_exports {
            push(&indirect.export_name, &mut names);
        }
        for star in &module.record.star_exports {
            let target = self.resolved(key, star);
            for name in self.export_names(&target, visited) {
                if name != "default" {
                    push(&name, &mut names);
                }
            }
        }
        names
    }

    // --- small accessors -------------------------------------------------

    fn status(&self, key: &str) -> Status {
        self.modules[key].status
    }

    fn set_status(&mut self, key: &str, status: Status) {
        self.modules.get_mut(key).expect("module exists").status = status;
    }

    fn dependency_keys(&self, key: &str) -> Vec<String> {
        self.modules[key]
            .record
            .requested_modules
            .iter()
            .map(|specifier| self.resolved(key, specifier))
            .collect()
    }

    fn resolved(&self, key: &str, specifier: &str) -> String {
        self.modules[key]
            .resolved_requests
            .get(specifier)
            .cloned()
            .unwrap_or_else(|| specifier.to_owned())
    }

    fn import_specs(&self, key: &str) -> Vec<(String, ImportName, String)> {
        self.modules[key]
            .record
            .import_entries
            .iter()
            .map(|entry| {
                (
                    entry.module_request.clone(),
                    entry.import_name.clone(),
                    entry.local_name.clone(),
                )
            })
            .collect()
    }
}
