//! Host module-resolution callback.
//!
//! The engine knows nothing about file systems or Test262 directory layout: a
//! host supplies a [`ModuleResolver`] that maps a `(specifier, referrer)` pair
//! to a canonical module key and its source text. The S3 Test262 channel will
//! implement this against the test directory; unit tests use a small in-memory
//! map.

/// A failure to resolve or load a module specifier.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleResolveError {
    /// Human-readable message (surfaced as a `SyntaxError` during linking).
    pub message: String,
}

/// A resolved module: its canonical key (used to deduplicate the graph) and its
/// source text.
#[derive(Clone, Debug)]
pub struct ResolvedModule {
    /// Canonical, graph-unique key for the module (e.g. an absolute path).
    pub key: String,
    /// The module's source text.
    pub source: String,
}

/// Host callback that resolves a module specifier referenced from `referrer` to
/// a canonical key and source text.
pub trait ModuleResolver {
    /// Resolves `specifier` as referenced from the module identified by
    /// `referrer` (the canonical key of the importing module).
    ///
    /// # Errors
    ///
    /// Returns a [`ModuleResolveError`] when the specifier cannot be resolved or
    /// its source cannot be loaded.
    fn resolve(
        &mut self,
        specifier: &str,
        referrer: &str,
    ) -> Result<ResolvedModule, ModuleResolveError>;
}

/// An in-memory resolver backed by a map from canonical key to source text.
/// Specifiers are treated as canonical keys directly (no relative resolution).
/// Used by unit tests and as a simple embedding default.
#[derive(Clone, Debug, Default)]
pub struct MapResolver {
    sources: std::collections::HashMap<String, String>,
}

impl MapResolver {
    /// Builds an empty resolver.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `source` under the canonical key `key`.
    #[must_use]
    pub fn with(mut self, key: &str, source: &str) -> Self {
        self.sources.insert(key.to_owned(), source.to_owned());
        self
    }
}

impl ModuleResolver for MapResolver {
    fn resolve(
        &mut self,
        specifier: &str,
        _referrer: &str,
    ) -> Result<ResolvedModule, ModuleResolveError> {
        match self.sources.get(specifier) {
            Some(source) => Ok(ResolvedModule {
                key: specifier.to_owned(),
                source: source.clone(),
            }),
            None => Err(ModuleResolveError {
                message: format!("Cannot resolve module '{specifier}'"),
            }),
        }
    }
}
