//! Realm-owned cache of compiled regular-expression programs.
//!
//! A `PreparedRegexp` is a pure function of its source and the four flags
//! that change matching, and matching only reads it, so every RegExp object
//! with the same source and flags can share one. Without the cache each
//! `exec`/`test` rebuilt the program from the source text, and every
//! evaluation of a literal inside a loop re-validated its pattern. Entries
//! hold no objects or realm references; both tables are bounded and simply
//! cleared when full.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use super::matcher::PreparedRegexp;

/// Distinct patterns kept per table before the table is cleared.
const MAX_ENTRIES: usize = 64;

#[derive(Default)]
pub(crate) struct ProgramCache {
    /// Programs by source, one table per combination of the matching flags
    /// (`i`, `u`/`v`, `s`, `m`).
    programs: [HashMap<Box<str>, Rc<PreparedRegexp>>; 16],
    /// Sources that passed pattern validation, by their complete flags text.
    validated: HashMap<Box<str>, HashSet<Box<str>>>,
}

impl ProgramCache {
    pub(super) fn program(
        &mut self,
        source: &str,
        ignore_case: bool,
        unicode: bool,
        dot_all: bool,
        multiline: bool,
    ) -> Rc<PreparedRegexp> {
        let table = &mut self.programs[usize::from(ignore_case)
            | usize::from(unicode) << 1
            | usize::from(dot_all) << 2
            | usize::from(multiline) << 3];
        if let Some(program) = table.get(source) {
            return Rc::clone(program);
        }
        if table.len() >= MAX_ENTRIES {
            table.clear();
        }
        let program = Rc::new(PreparedRegexp::new(
            source,
            ignore_case,
            unicode,
            dot_all,
            multiline,
        ));
        table.insert(source.into(), Rc::clone(&program));
        program
    }

    pub(super) fn is_validated(&self, source: &str, flags: &str) -> bool {
        self.validated
            .get(flags)
            .is_some_and(|sources| sources.contains(source))
    }

    pub(super) fn mark_validated(&mut self, source: &str, flags: &str) {
        if self.validated.len() >= MAX_ENTRIES {
            self.validated.clear();
        }
        let sources = self.validated.entry(flags.into()).or_default();
        if sources.len() >= MAX_ENTRIES {
            sources.clear();
        }
        sources.insert(source.into());
    }
}
