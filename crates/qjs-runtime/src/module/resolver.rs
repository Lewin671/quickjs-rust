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
/// loaded source.
#[derive(Clone, Debug)]
pub struct ResolvedModule {
    /// Canonical, graph-unique key for the module (e.g. an absolute path).
    pub key: String,
    /// The module's source text.
    pub source: String,
    /// The module's raw bytes, used by import-bytes module records.
    pub bytes: Vec<u8>,
}

/// Host callback that resolves a module specifier referenced from `referrer` to
/// a canonical key and source text.
pub trait ModuleResolver {
    /// Resolves `specifier` as referenced from the module identified by
    /// `referrer` (the canonical key of the importing module). `specifier` is
    /// a lossless, opaque WTF-16 key: use [`module_specifier_code_units`] when a
    /// resolver must distinguish isolated surrogates, or
    /// [`module_specifier_to_utf8_lossy`] only for display/file-system fallback.
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
    sources: std::collections::HashMap<String, Vec<u8>>,
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
        self.sources.insert(
            crate::string::string_from_utf8_scalars(key),
            source.as_bytes().to_vec(),
        );
        self
    }

    /// Registers `source` under an exact UTF-16 module key, including keys with
    /// isolated surrogate code units that cannot be represented by host UTF-8.
    #[must_use]
    pub fn with_utf16_key(mut self, key: &[u16], source: &str) -> Self {
        self.sources.insert(
            crate::string::string_from_code_units(key),
            source.as_bytes().to_vec(),
        );
        self
    }

    /// Registers raw module bytes under the canonical key `key`.
    #[must_use]
    pub fn with_bytes(mut self, key: &str, bytes: &[u8]) -> Self {
        self.sources
            .insert(crate::string::string_from_utf8_scalars(key), bytes.to_vec());
        self
    }

    /// Registers raw module bytes under an exact UTF-16 module key.
    #[must_use]
    pub fn with_utf16_key_bytes(mut self, key: &[u16], bytes: &[u8]) -> Self {
        self.sources
            .insert(crate::string::string_from_code_units(key), bytes.to_vec());
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
            Some(bytes) => Ok(ResolvedModule {
                key: specifier.to_owned(),
                source: String::from_utf8_lossy(bytes).into_owned(),
                bytes: bytes.clone(),
            }),
            None => Err(ModuleResolveError {
                message: format!(
                    "Cannot resolve module '{}'",
                    module_specifier_to_utf8_lossy(specifier)
                ),
            }),
        }
    }
}

/// Returns the exact ECMAScript UTF-16 code units of the opaque specifier key.
#[must_use]
pub fn module_specifier_code_units(specifier: &str) -> Vec<u16> {
    crate::string::string_code_units(specifier)
}

/// Converts an opaque module specifier to host UTF-8 for display or a host API
/// that cannot represent isolated surrogates. Do not use this as an identity
/// key because isolated surrogates are replaced with U+FFFD.
#[must_use]
pub fn module_specifier_to_utf8_lossy(specifier: &str) -> String {
    crate::string::string_to_utf8_lossy(specifier)
}
