//! ECMAScript module records, linking, and evaluation (T012 S2).
//!
//! This subsystem builds a Source Text Module Record per module, links a
//! (possibly cyclic) module graph by resolving every import to an exporting
//! module's binding, evaluates module bodies in dependency order, and exposes
//! Module Namespace exotic objects for `import * as ns`.
//!
//! # Environment model
//!
//! Each module is evaluated against its own fresh realm (see
//! [`crate::bytecode::eval_module_body`]): the module's top-level `var`/`let`/
//! `const`/`function`/`class` bindings live in that realm and never leak to a
//! shared `globalThis`. Resolved imports are seeded into the importing module's
//! realm as ordinary module-scope bindings before its body runs, so import
//! references resolve through the normal global-load path.
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

mod link;
mod namespace;
mod records;
mod resolver;

#[cfg(test)]
mod tests;

pub use resolver::{MapResolver, ModuleResolveError, ModuleResolver, ResolvedModule};

use crate::{EvalError, EvalErrorKind, Value};

/// Loads, links, and evaluates the module graph rooted at `source` (identified
/// by `specifier`), resolving further imports through `resolver`. Returns the
/// root module's namespace object on success.
///
/// # Errors
///
/// Returns a [`EvalError`] for parse failures, unresolvable imports or ambiguous
/// star exports (reported as `SyntaxError`), or runtime failures during module
/// evaluation.
pub fn eval_module(
    source: &str,
    specifier: &str,
    resolver: &mut dyn ModuleResolver,
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
    resolver: &mut dyn ModuleResolver,
) -> Result<Value, EvalError> {
    let mut graph = link::ModuleGraph::new();
    if let Some(prelude) = prelude {
        graph.eval_prelude(prelude).map_err(|error| EvalError {
            kind: EvalErrorKind::Runtime,
            message: error.message,
        })?;
    }
    let root = graph
        .load(specifier, source, resolver)
        .map_err(link_error)?;
    graph.link(&root).map_err(link_error)?;
    graph.evaluate(&root).map_err(|error| EvalError {
        kind: EvalErrorKind::Runtime,
        message: error.message,
    })?;
    Ok(graph.namespace(&root))
}

fn link_error(error: link::LinkError) -> EvalError {
    EvalError {
        kind: match error.kind {
            link::LinkErrorKind::Parse => EvalErrorKind::Parse,
            // Unresolvable imports and ambiguous star exports are early
            // (link-phase) SyntaxErrors per 16.2.1.5.
            link::LinkErrorKind::Syntax => EvalErrorKind::Early,
        },
        message: error.message,
    }
}
