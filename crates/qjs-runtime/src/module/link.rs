//! Module graph loading, linking, and evaluation.
//!
//! The graph is keyed by canonical module key. Loading is depth-first and
//! deduplicating; linking resolves every import to an exporting binding (or a
//! namespace object) and reports unresolvable imports / ambiguous star exports
//! as SyntaxErrors; evaluation is a post-order DFS over the dependency graph,
//! mirroring the instantiation/evaluation state machine of ECMAScript 16.2.1.5
//! (`Unlinked` -> `Linking` -> `Linked` -> `Evaluating` -> `Evaluated`).

use std::collections::HashMap;

use crate::{RuntimeError, Value, bytecode};

use super::namespace::build_namespace;
use super::records::{ImportName, ModuleRecord, build_record};
use super::resolver::ModuleResolver;

/// A linking-phase failure: a parse error or a SyntaxError-class link error.
#[derive(Clone, Debug)]
pub(super) struct LinkError {
    pub(super) kind: LinkErrorKind,
    pub(super) message: String,
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
    /// The owned host resolver, retained for the realm's lifetime so a dynamic
    /// `import()` can resolve specifiers long after the initial load.
    resolver: Option<Box<dyn ModuleResolver>>,
}

impl ModuleGraph {
    pub(super) fn new() -> Self {
        Self {
            modules: HashMap::new(),
            realm: bytecode::new_module_realm(),
            host: None,
            resolver: None,
        }
    }

    /// Records the dynamic-import host so evaluated module bodies carry it.
    pub(super) fn set_host(&mut self, host: super::host::ModuleHostRef) {
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
            })?;
            self.insert_and_load_dependencies(key.clone(), record, resolver)?;
        }
        self.link(&key)?;
        self.evaluate_with_drain(&key, false)
            .map_err(|error| LinkError {
                // A runtime failure during module evaluation rejects the import
                // promise; reuse the Runtime variant as a transport and carry
                // the message through verbatim.
                kind: LinkErrorKind::Runtime,
                message: error.message,
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
                    ImportName::Namespace => Ok(Some((target, name.to_owned()))),
                    ImportName::Named(inner) => self.resolve_export(&target, inner, visited),
                };
            }
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
            self.evaluate_with_drain(&dep, drain)?;
        }
        self.evaluate_body(key, drain)?;
        self.set_status(key, Status::Evaluated);
        Ok(())
    }

    /// Compiles and runs one module body against a fresh realm seeded with its
    /// resolved imports, then records its exported binding values.
    fn evaluate_body(&mut self, key: &str, drain: bool) -> Result<(), RuntimeError> {
        let imports = self.import_bindings(key)?;
        let compiled = {
            let module = &self.modules[key];
            bytecode::compile_module(&module.record.body)?
        };
        let host = self.host.clone();
        let env = bytecode::eval_module_body(&compiled, &self.realm, imports, host, drain)?;
        // Snapshot exported binding values from the module's frame environment.
        let export_pairs = self.collect_export_values(key, &env)?;
        let module = self.modules.get_mut(key).expect("module exists");
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

    /// Reads the value of `name` exported by `key`, following indirect/star
    /// resolution to the binding that actually holds it.
    fn export_value(&self, key: &str, name: &str) -> Value {
        match self.resolve_export(key, name, &mut Vec::new()) {
            Ok(Some((module_key, export_name))) => self.modules[&module_key]
                .exports
                .get(&export_name)
                .cloned()
                .unwrap_or(Value::Undefined),
            // A namespace-valued indirect export resolves to the target's
            // namespace object.
            _ => Value::Undefined,
        }
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
            let value = env.get(&local.local_name).unwrap_or(Value::Undefined);
            pairs.push((local.export_name.clone(), value));
        }
        Ok(pairs)
    }

    /// The module namespace object for `key`, built (and cached) lazily from its
    /// resolved export names.
    pub(super) fn namespace(&mut self, key: &str) -> Value {
        if let Some(namespace) = &self.modules[key].namespace {
            return namespace.clone();
        }
        let names = self.export_names(key, &mut Vec::new());
        let mut bindings = Vec::new();
        for name in names {
            // A namespace omits any name whose resolution is ambiguous.
            if let Ok(Some((module_key, export_name))) =
                self.resolve_export(key, &name, &mut Vec::new())
            {
                let value = self.modules[&module_key]
                    .exports
                    .get(&export_name)
                    .cloned()
                    .unwrap_or(Value::Undefined);
                bindings.push((name, value));
            }
        }
        let namespace = build_namespace(bindings);
        self.modules.get_mut(key).expect("module exists").namespace = Some(namespace.clone());
        namespace
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
