//! Hasher for the engine's internal string-keyed tables.
//!
//! Property tables and binding tables are probed on every named property
//! access and every dynamic binding lookup, so their hash function is on the
//! hottest path in the runtime. The standard library's default hasher is
//! SipHash-1-3, a keyed pseudorandom function chosen to make hash-flooding
//! attacks impractical on collections built from untrusted input. That is the
//! wrong trade here: it costs a per-byte round of ARX mixing where a property
//! name is a handful of bytes, and the tables are engine-internal.
//!
//! This is the same FxHash construction rustc uses for its own interner
//! tables: multiply-rotate-xor over machine words, seeded by a fixed constant.
//! It has no cryptographic strength — a script that deliberately constructs
//! colliding property names can degrade a single object's lookups to a linear
//! scan of that object's own properties. That matches what other production
//! JavaScript engines (including the QuickJS reference this engine is compared
//! against) accept for property tables, and it cannot be used to attack any
//! table outside the script's own realm.

use std::hash::{BuildHasherDefault, Hasher};

/// [`std::collections::HashMap`] over engine-internal string keys.
pub(crate) type NameMap<K, V> = std::collections::HashMap<K, V, BuildHasherDefault<NameHasher>>;

/// [`std::collections::HashSet`] over engine-internal string keys.
pub(crate) type NameSet<K> = std::collections::HashSet<K, BuildHasherDefault<NameHasher>>;

/// The golden-ratio-derived odd multiplier used by FxHash.
const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;
const ROTATE: u32 = 5;

#[derive(Default)]
pub(crate) struct NameHasher {
    hash: u64,
}

impl NameHasher {
    #[inline]
    fn add_word(&mut self, word: u64) {
        self.hash = (self.hash.rotate_left(ROTATE) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for NameHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        let mut rest = bytes;
        while rest.len() >= 8 {
            let (word, tail) = rest.split_at(8);
            self.add_word(u64::from_ne_bytes(word.try_into().expect("eight bytes")));
            rest = tail;
        }
        if rest.len() >= 4 {
            let (word, tail) = rest.split_at(4);
            self.add_word(u64::from(u32::from_ne_bytes(
                word.try_into().expect("four bytes"),
            )));
            rest = tail;
        }
        if rest.len() >= 2 {
            let (word, tail) = rest.split_at(2);
            self.add_word(u64::from(u16::from_ne_bytes(
                word.try_into().expect("two bytes"),
            )));
            rest = tail;
        }
        if let Some(byte) = rest.first() {
            self.add_word(u64::from(*byte));
        }
    }

    #[inline]
    fn write_u8(&mut self, value: u8) {
        self.add_word(u64::from(value));
    }

    #[inline]
    fn write_usize(&mut self, value: usize) {
        self.add_word(value as u64);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::Hash;

    fn hash_of(value: &str) -> u64 {
        let mut hasher = NameHasher::default();
        value.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn distinct_names_of_every_tail_length_hash_apart() {
        // The word/half-word/byte tail arms must all contribute, so names that
        // differ only in a tail byte must not collide.
        let names = [
            "a",
            "b",
            "ab",
            "ac",
            "abc",
            "abd",
            "abcd",
            "abce",
            "abcdefg",
            "abcdefh",
            "abcdefgh",
            "abcdefgi",
            "abcdefghijk",
            "abcdefghijl",
            "",
        ];
        for (index, name) in names.iter().enumerate() {
            for other in &names[index + 1..] {
                assert_ne!(hash_of(name), hash_of(other), "{name} vs {other}");
            }
        }
    }

    #[test]
    fn hashing_is_deterministic_across_hashers() {
        assert_eq!(hash_of("prototype"), hash_of("prototype"));
        let mut map: NameMap<String, u32> = NameMap::default();
        map.insert("length".to_owned(), 1);
        map.insert("prototype".to_owned(), 2);
        assert_eq!(map.get("length"), Some(&1));
        assert_eq!(map.get("prototype"), Some(&2));
        assert_eq!(map.get("missing"), None);
    }
}
