//! Bounded compilation blueprints for simple direct `eval` sources.
//!
//! Cached entries are immutable bytecode blueprints shared through [`Rc`].
//! Eligibility excludes every bytecode feature with per-evaluation identity or
//! declaration-instantiation state, so each invocation still constructs its
//! own VM frame, operand stack, and runtime values while recurring
//! expression-only sources skip parsing and compilation.

use std::rc::Rc;

use qjs_parser::EvalParseContext;

use crate::{Bytecode, JsString};

const MAX_ENTRIES: usize = 32;
const MAX_SOURCE_BYTES: usize = 4096;

#[derive(Clone)]
struct Entry {
    source: JsString,
    context: EvalParseContext,
    bytecode: Rc<Bytecode>,
}

/// Per-realm, FIFO-bounded cache of compilation blueprints.
///
/// A linear scan is deliberate here: the cache is small, avoids allocating a
/// second owned key for every lookup, and keeps source retention bounded even
/// when an application evaluates many one-off strings.
#[derive(Default)]
pub(super) struct DirectEvalCache {
    entries: Vec<Entry>,
}

impl DirectEvalCache {
    pub(super) fn lookup(
        &self,
        source: &JsString,
        context: &EvalParseContext,
    ) -> Option<Rc<Bytecode>> {
        self.entries
            .iter()
            .find(|entry| entry.source == *source && entry.context == *context)
            .map(|entry| Rc::clone(&entry.bytecode))
    }

    pub(super) fn insert(
        &mut self,
        source: JsString,
        context: EvalParseContext,
        bytecode: Rc<Bytecode>,
    ) {
        if source.len() > MAX_SOURCE_BYTES {
            return;
        }
        if self.entries.len() >= MAX_ENTRIES {
            self.entries.remove(0);
        }
        self.entries.push(Entry {
            source,
            context,
            bytecode,
        });
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }
}
