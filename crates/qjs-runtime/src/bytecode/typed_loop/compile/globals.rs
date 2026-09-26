//! The region's global bindings: hoisted reads, and the sloppy fallback
//! globals it writes.

use super::{Builder, Op};

impl Builder<'_> {
    /// Boxed register seeded from the global binding `name`, allocating one if
    /// new.
    pub(super) fn global_register(&mut self, name: &str) -> Option<u16> {
        if let Some((register, _)) = self
            .boxed_global_reads
            .iter()
            .find(|(_, candidate)| candidate == name)
        {
            return Some(*register);
        }
        let register = self.fresh_boxed()?;
        self.boxed_global_reads.push((register, name.to_owned()));
        Some(register)
    }

    /// Whether the region contains an instruction that could write `name`,
    /// which would make a hoisted read of that same binding observably stale.
    pub(super) fn region_writes_global_named(&self, name: &str) -> bool {
        self.bytecode.code[self.header..=self.backedge]
            .iter()
            .any(|op| match op {
                Op::StoreGlobalStrict(candidate) | Op::DefineGlobalVar(candidate) => {
                    candidate == name
                }
                Op::StoreGlobalSloppy {
                    name: candidate, ..
                }
                | Op::StoreLocalOrGlobalSloppy {
                    name: candidate, ..
                }
                | Op::AppendStringLiteralGlobal {
                    name: candidate, ..
                } => candidate == name,
                _ => false,
            })
    }

    /// A named-property write could target `globalThis` and mutate a hoisted
    /// global binding. A region with a sloppy fallback sink therefore keeps
    /// those writes out of this tier; dense array writes remain separately
    /// guarded and cannot alter a global object's binding descriptors.
    pub(super) fn region_writes_a_sloppy_global(&self) -> bool {
        self.bytecode.code[self.header..=self.backedge]
            .iter()
            .any(|op| matches!(op, Op::StoreLocalOrGlobalSloppy { .. }))
    }

    /// Returns the frame slot for an unresolved sloppy binding only when this
    /// bytecode owns the compiler-emitted fallback slot for the same name.
    pub(super) fn sloppy_global_fallback_slot(&self, name: &str) -> Option<usize> {
        let slot = self.bytecode.local_slot(name)?;
        let local = self.bytecode.locals.get(slot)?;
        (local.name == name && local.sloppy_global_fallback && !local.compiler_temporary)
            .then_some(slot)
    }

    /// Index of the prepared sink for a sloppy fallback write, adding it once
    /// per slot/name pair. The runtime validates the dynamic binding and
    /// property identities before entering the program.
    pub(super) fn sloppy_global_write_index(&mut self, slot: usize, name: &str) -> Option<u16> {
        if self.sloppy_global_fallback_slot(name) != Some(slot) {
            return None;
        }
        let slot = u32::try_from(slot).ok()?;
        if let Some(index) =
            self.sloppy_global_writes
                .iter()
                .position(|(candidate_slot, candidate_name)| {
                    *candidate_slot == slot && candidate_name == name
                })
        {
            return u16::try_from(index).ok();
        }
        self.sloppy_global_writes.push((slot, name.to_owned()));
        u16::try_from(self.sloppy_global_writes.len() - 1).ok()
    }
}
