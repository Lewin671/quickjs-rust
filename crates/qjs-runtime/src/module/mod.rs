//! ECMAScript module records, linking, and evaluation (T012 S2 + S4).
//!
//! This subsystem builds a Source Text Module Record per module, links a
//! (possibly cyclic) module graph by resolving every import to an exporting
//! module's binding, evaluates module bodies in dependency order, and exposes
//! Module Namespace exotic objects for `import * as ns`.
//!
//! # Dynamic import (S4)
//!
//! A realm carries a [`ModuleHost`] (the shared module graph plus the owned host
//! resolver), reachable from every call frame and the promise job queue through
//! [`crate::CallEnv`]. A dynamic `import(specifier)` builds a `%Promise%`
//! capability, schedules a host load job, and — when that job runs as a
//! microtask — resolves/links/evaluates the requested module against the same
//! graph (so the same key yields the same namespace) and settles the promise
//! with the namespace object or a load/link/evaluate error. The call is valid
//! under both the Script and Module goals.
//!
//! # Environment model
//!
//! Each module graph evaluates against one shared realm: a static module entry
//! creates a module realm, while script-goal dynamic import reuses the script
//! realm. The module's top-level `var`/`let`/`const`/`function`/`class`
//! bindings live in that graph realm. Resolved imports are seeded into the
//! importing module's realm as ordinary module-scope bindings before its body
//! runs, so import references resolve through the normal global-load path.
//!
//! # Live bindings (current limitation)
//!
//! Function and object exports are shared by reference, so the common live
//! pattern — an importer calling an exported function that reads the exporter's
//! own (live) top-level binding — observes updates. A *primitive* binding
//! re-exported by value is seeded as a snapshot at link time: a later
//! reassignment of an exported `let`/`var` in the exporter is not yet reflected
//! at the importer's binding. Full primitive live-binding indirection is
//! deferred; see `tasks/T012-modules-campaign.md`.

mod host;
mod link;
mod namespace;
mod records;
mod resolver;

#[cfg(test)]
mod tests;

pub(crate) use host::{ImportErrorKind, ModuleHost, ModuleHostRef};
pub use resolver::{MapResolver, ModuleResolveError, ModuleResolver, ResolvedModule};

use crate::{EvalError, EvalErrorKind, Value};

/// Loads, links, and evaluates the module graph rooted at `source` (identified
/// by `specifier`), resolving further imports through `resolver`. Returns the
/// root module's namespace object on success.
///
/// `resolver` is taken by value because it is retained by the realm's
/// dynamic-import host for the lifetime of the evaluation: a dynamic `import()`
/// reached from any module body or microtask resolves further specifiers
/// through the same resolver and reuses the same module graph.
///
/// # Errors
///
/// Returns a [`EvalError`] for parse failures, unresolvable imports or ambiguous
/// star exports (reported as `SyntaxError`), or runtime failures during module
/// evaluation.
pub fn eval_module(
    source: &str,
    specifier: &str,
    resolver: Box<dyn ModuleResolver>,
) -> Result<Value, EvalError> {
    eval_module_with_prelude(None, source, specifier, resolver)
}

/// Like [`eval_module`], but first evaluates `prelude` as a *script* in the
/// module graph's shared realm, so its top-level bindings are visible to every
/// module. Used by the Test262 module channel to install harness includes
/// (`assert.js`, `sta.js`, the `$DONE` handler) as a module-scope prelude.
///
/// # Errors
///
/// Returns a [`EvalError`] for a prelude/parse/link failure or a runtime
/// failure during module evaluation (same classification as [`eval_module`]).
pub fn eval_module_with_prelude(
    prelude: Option<&str>,
    source: &str,
    specifier: &str,
    resolver: Box<dyn ModuleResolver>,
) -> Result<Value, EvalError> {
    use std::{cell::RefCell, rc::Rc};

    let mut graph = link::ModuleGraph::new();
    if let Some(prelude) = prelude {
        graph.eval_prelude(prelude).map_err(|error| EvalError {
            kind: EvalErrorKind::Runtime,
            message: error.message,
        })?;
    }
    // Share the graph behind a cell so both the static root evaluation here and
    // any later dynamic `import()` (which runs as a microtask through the host)
    // operate on the same registry, giving same-key namespace identity. The
    // graph owns the resolver for the realm's lifetime.
    graph.set_resolver(resolver);
    let graph = Rc::new(RefCell::new(graph));
    let host = ModuleHost::new(Rc::clone(&graph), specifier.to_owned()).into_ref();
    graph.borrow_mut().set_host(Rc::clone(&host));

    // Load, link, and evaluate the root with short-lived graph borrows. Module
    // bodies defer their promise jobs (including any dynamic `import()`), so the
    // graph is never borrowed across a job that would re-borrow it.
    let (root, realm) = {
        let mut graph_mut = graph.borrow_mut();
        let root = graph_mut.load_root(specifier, source).map_err(link_error)?;
        graph_mut.link(&root).map_err(link_error)?;
        graph_mut.evaluate(&root).map_err(|error| EvalError {
            kind: EvalErrorKind::Runtime,
            message: error.message,
        })?;
        let realm = graph_mut.realm();
        (root, realm)
    };
    // Drain the deferred jobs (reactions and dynamic imports) with the graph
    // borrow released, so a dynamic-import job can re-borrow the graph.
    let mut drain_env = crate::CallEnv::new(realm);
    drain_env.set_module_host(host);
    crate::promise::drain_promise_jobs(&mut drain_env).map_err(|error| EvalError {
        kind: EvalErrorKind::Runtime,
        message: error.message,
    })?;
    Ok(graph.borrow_mut().namespace(&root))
}

/// Builds a dynamic-import host for *script*-goal evaluation: a fresh module
/// graph sharing the script's realm, the owned `resolver`, and `referrer` as
/// the active referrer key. The returned handle is installed on the script's
/// environment so a dynamic `import()` inside the script resolves and loads
/// modules through it.
pub(crate) fn new_script_module_host(
    referrer: &str,
    resolver: Box<dyn ModuleResolver>,
    realm: crate::bytecode::ModuleRealm,
) -> ModuleHostRef {
    use std::{cell::RefCell, rc::Rc};

    let mut graph = link::ModuleGraph::with_realm(realm);
    graph.set_resolver(resolver);
    let graph = Rc::new(RefCell::new(graph));
    let host = ModuleHost::new(Rc::clone(&graph), referrer.to_owned()).into_ref();
    graph.borrow_mut().set_host(Rc::clone(&host));
    host
}

fn link_error(error: link::LinkError) -> EvalError {
    EvalError {
        kind: match error.kind {
            link::LinkErrorKind::Parse => EvalErrorKind::Parse,
            // Unresolvable imports and ambiguous star exports are early
            // (link-phase) SyntaxErrors per 16.2.1.5.
            link::LinkErrorKind::Syntax => EvalErrorKind::Early,
            // A runtime module-evaluation failure should not reach the static
            // top-level path (it uses `evaluate`, which surfaces RuntimeError
            // directly); treat it as a runtime error defensively.
            link::LinkErrorKind::Runtime => EvalErrorKind::Runtime,
        },
        message: error.message,
    }
}
