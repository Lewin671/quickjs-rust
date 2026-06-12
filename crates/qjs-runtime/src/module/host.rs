//! Dynamic-import host state threaded through the VM.
//!
//! A [`ModuleHost`] bundles the per-realm module graph, the owned host
//! [`ModuleResolver`], and the active referrer key (the canonical key of the
//! script or module whose code is running). It is shared by `Rc::clone` into
//! every call frame's [`crate::CallEnv`] and into the promise job queue, so a
//! dynamic `import()` reached from any depth — including a `.then` callback run
//! during job draining — can resolve a specifier against the running referrer
//! and reuse the realm's single module graph.
//!
//! The host owns the resolver (rather than borrowing it) because dynamic import
//! can happen at any point during evaluation, long after the top-level
//! `eval_module`/script call that installed the host has returned control to the
//! VM loop.

use std::{cell::RefCell, rc::Rc};

use crate::Value;

use super::link::{LinkError, LinkErrorKind, ModuleGraph};

/// Shared, mutable dynamic-import host state for one realm.
pub(crate) type ModuleHostRef = Rc<RefCell<ModuleHost>>;

/// The classification of a dynamic-import failure, mapped onto the right JS
/// error type when the import promise rejects.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ImportErrorKind {
    /// A parse or link (early) failure: a `SyntaxError`.
    Syntax,
    /// A runtime failure raised while evaluating the imported module body; the
    /// message already carries its own error type.
    Runtime,
}

/// A dynamic-import failure: the message plus how it should reject.
pub(crate) struct ImportError {
    pub(crate) kind: ImportErrorKind,
    pub(crate) message: String,
}

/// Per-realm dynamic-import host: the shared module graph (which owns the
/// resolver) and the active referrer key.
///
/// The graph lives behind its own `Rc<RefCell<…>>` so the brief borrow taken to
/// load/link/evaluate a dynamically imported module never overlaps the host
/// borrow held while a job runs.
pub(crate) struct ModuleHost {
    graph: Rc<RefCell<ModuleGraph>>,
    referrer: String,
}

impl ModuleHost {
    /// Builds a host over the shared module `graph`, running on behalf of
    /// `referrer`.
    pub(super) fn new(graph: Rc<RefCell<ModuleGraph>>, referrer: String) -> Self {
        Self { graph, referrer }
    }

    /// Wraps the host in a shared cell.
    pub(super) fn into_ref(self) -> ModuleHostRef {
        Rc::new(RefCell::new(self))
    }

    /// Resolves and evaluates `specifier` against the active referrer, returning
    /// the requested module's namespace object. The graph deduplicates by key,
    /// so importing an already-loaded module yields the same namespace.
    pub(crate) fn import(&mut self, specifier: &str) -> Result<Value, ImportError> {
        let referrer = self.referrer.clone();
        let mut graph = self.graph.borrow_mut();
        graph
            .import_dynamic_owned(specifier, &referrer)
            .map_err(import_error)
    }
}

fn import_error(error: LinkError) -> ImportError {
    ImportError {
        kind: match error.kind {
            LinkErrorKind::Parse | LinkErrorKind::Syntax => ImportErrorKind::Syntax,
            LinkErrorKind::Runtime => ImportErrorKind::Runtime,
        },
        message: error.message,
    }
}
