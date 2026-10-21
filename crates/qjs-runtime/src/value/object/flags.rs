//! Ordinary-object brands and state bits that do not borrow property storage.
//!
//! The five less frequent booleans share one byte. Together with a u32 index
//! count (there are only u32::MAX possible array-index keys), this removes an
//! entire word from the object header. SymbolBrand and the two hot exotic flags keep individual bytes.

use std::cell::Cell;

#[derive(Clone, Copy)]
pub(super) enum Flag {
    Extensible = 1,
    RawJson = 2,
    ArrayPrototype = 4,
    ImmutablePrototype = 8,
    /// Lazily define a String wrapper's indexed properties from StringData
    /// before exposing its table; avoid rebuilding them on every sloppy call.
    StringIndicesPending = 16,
}

pub(super) struct ObjectFlags(Cell<u8>);

impl ObjectFlags {
    pub(super) fn new() -> Self {
        Self(Cell::new(Flag::Extensible as u8))
    }

    #[inline(always)]
    pub(super) fn get(&self, flag: Flag) -> bool {
        self.0.get() & flag as u8 != 0
    }

    #[inline(always)]
    pub(super) fn set(&self, flag: Flag, value: bool) {
        let bits = self.0.get();
        self.0.set(if value {
            bits | flag as u8
        } else {
            bits & !(flag as u8)
        });
    }
}

#[cfg(test)]
mod tests {
    use super::{Flag, ObjectFlags};

    #[test]
    fn object_flags_preserve_every_other_brand_and_state() {
        let flags = ObjectFlags::new();
        let all = [
            Flag::Extensible,
            Flag::RawJson,
            Flag::ArrayPrototype,
            Flag::ImmutablePrototype,
            Flag::StringIndicesPending,
        ];
        assert!(flags.get(Flag::Extensible));
        for flag in all.into_iter().skip(1) {
            assert!(!flags.get(flag));
            flags.set(flag, true);
        }
        for cleared in all {
            flags.set(cleared, false);
            for flag in all {
                assert_eq!(flags.get(flag), flag as u8 != cleared as u8);
            }
            flags.set(cleared, true);
        }
    }
}
