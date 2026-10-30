//! Key-ID index with consumable vacant-bucket proofs.
//!
//! A lookup searches once. Cloning an immutable layout preserves bucket
//! positions, so an insertion into its copy consumes the same proof without
//! searching again. Each insertion reserves space for the following one;
//! rebuilds are cold structural work and explicitly counted separately.

use crate::property::atom::KeyId;

const TOMBSTONE: u32 = u32::MAX;
#[derive(Clone, Copy)]
struct Bucket {
    key: Option<KeyId>,
    position: u32,
}
const EMPTY: Bucket = Bucket {
    key: None,
    position: 0,
};

#[derive(Clone)]
pub(super) struct IdIndex {
    buckets: Vec<Bucket>,
    len: usize,
    tombstones: usize,
}

#[derive(Clone, Copy)]
pub(super) struct IdProbe {
    key: KeyId,
    bucket: usize,
    capacity: usize,
    pub(super) position: Option<usize>,
}

impl IdIndex {
    pub(super) fn new() -> Self {
        Self {
            buckets: vec![EMPTY; 8],
            len: 0,
            tombstones: 0,
        }
    }
    pub(super) fn bytes(&self) -> usize {
        self.buckets.capacity() * size_of::<Bucket>()
    }
    fn bucket(key: KeyId, capacity: usize) -> usize {
        (key.get().wrapping_mul(0x9e37_79b1) >> (32 - capacity.trailing_zeros())) as usize
    }
    pub(super) fn probe(&self, key: KeyId) -> IdProbe {
        crate::diagnostics::count!(property_id_probes);
        self.probe_uncounted(key)
    }
    pub(super) fn probe_uncounted(&self, key: KeyId) -> IdProbe {
        let mask = self.buckets.len() - 1;
        let mut bucket = Self::bucket(key, self.buckets.len());
        let mut vacancy = None;
        loop {
            let entry = self.buckets[bucket];
            match entry.key {
                Some(candidate) if candidate == key => {
                    return IdProbe {
                        key,
                        bucket,
                        capacity: self.buckets.len(),
                        position: Some(entry.position as usize),
                    };
                }
                Some(_) => {}
                None if entry.position == TOMBSTONE => {
                    vacancy.get_or_insert(bucket);
                }
                None => {
                    return IdProbe {
                        key,
                        bucket: vacancy.unwrap_or(bucket),
                        capacity: self.buckets.len(),
                        position: None,
                    };
                }
            }
            bucket = (bucket + 1) & mask;
        }
    }
    #[cfg(test)]
    pub(super) fn get(&self, key: KeyId) -> Option<usize> {
        self.probe(key).position
    }
    /// Debug reconstruction is outside the operation's diagnostic work count.
    pub(super) fn verify_position(&self, key: KeyId) -> Option<usize> {
        self.probe_uncounted(key).position
    }
    pub(super) fn insert_at(&mut self, proof: IdProbe, position: usize) {
        assert_eq!(proof.capacity, self.buckets.len(), "stale index proof");
        assert!(proof.position.is_none(), "insertion requires a miss proof");
        let entry = &mut self.buckets[proof.bucket];
        assert!(entry.key.is_none(), "vacant bucket changed under proof");
        if entry.position == TOMBSTONE {
            self.tombstones -= 1;
        }
        *entry = Bucket {
            key: Some(proof.key),
            position: u32::try_from(position).expect("property slot space exhausted"),
        };
        self.len += 1;
        // No insertion performs a second search for the inserted key. Grow
        // after publication, leaving the following lookup's proof usable.
        if self.len * 2 >= self.buckets.len() {
            self.rebuild(self.buckets.len() * 2);
        } else if (self.len + self.tombstones) * 2 >= self.buckets.len() {
            self.rebuild(self.buckets.len());
        }
    }
    pub(super) fn remove_at(&mut self, proof: IdProbe) {
        assert_eq!(proof.capacity, self.buckets.len(), "stale index proof");
        let position = proof.position.expect("removal requires a hit proof");
        let entry = &mut self.buckets[proof.bucket];
        assert_eq!(entry.key, Some(proof.key));
        assert_eq!(entry.position as usize, position);
        *entry = Bucket {
            key: None,
            position: TOMBSTONE,
        };
        self.len -= 1;
        self.tombstones += 1;
    }
    pub(super) fn set_position(&mut self, key: KeyId, position: usize) {
        let proof = self.probe(key);
        assert!(
            proof.position.is_some(),
            "moving a live slot requires an indexed key"
        );
        self.buckets[proof.bucket].position = position as u32;
    }
    pub(super) fn compact_if_due(&mut self) {
        if self.tombstones > self.len + 32 {
            self.rebuild((self.len * 4).max(8).next_power_of_two());
        }
    }
    fn rebuild(&mut self, capacity: usize) {
        crate::diagnostics::count!(property_id_index_rebuilds);
        let previous = std::mem::replace(&mut self.buckets, vec![EMPTY; capacity]);
        self.tombstones = 0;
        for entry in previous {
            if let Some(key) = entry.key {
                // Rebuild insertion cannot invoke normal insertion/resizing.
                let proof = self.probe_uncounted(key);
                self.buckets[proof.bucket] = entry;
                crate::diagnostics::count!(property_id_index_rebuilt_entries);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::property::atom::AtomDomain;

    #[test]
    fn strided_ids_disperse_across_high_product_bits() {
        let domain = AtomDomain::new();
        let mut held = Vec::new();
        let mut starts = std::collections::HashSet::new();
        for n in 0..128 {
            for gap in 0..255 {
                held.push(domain.intern(&format!("gap{n}_{gap}")));
            }
            let key = domain.intern(&format!("key{n}"));
            starts.insert(IdIndex::bucket(key.id(), 256));
            held.push(key);
        }
        // Low-bit multiplication puts every one of these keys in one bucket.
        assert!(starts.len() >= 64, "{} distinct buckets", starts.len());
    }

    #[test]
    fn colliding_keys_survive_delete_reinsert_and_rebuild() {
        let domain = AtomDomain::new();
        let mut gaps = Vec::new();
        let keys: Vec<_> = (0..200)
            .map(|n| {
                // Strided IDs exercise high-bit distribution rather than only
                // sequential IDs; removal/rebuild must retain every mapping.
                for gap in 0..255 {
                    gaps.push(domain.intern(&format!("gap{n}_{gap}")));
                }
                domain.intern(&format!("k{n}"))
            })
            .collect();
        let mut index = IdIndex::new();
        for (n, key) in keys.iter().enumerate() {
            let proof = index.probe(key.id());
            index.insert_at(proof, n);
        }
        for key in keys.iter().step_by(2) {
            let proof = index.probe(key.id());
            index.remove_at(proof);
        }
        index.compact_if_due();
        for (n, key) in keys.iter().enumerate() {
            assert_eq!(index.get(key.id()), (n % 2 != 0).then_some(n));
        }
        for (n, key) in keys.iter().enumerate().step_by(2) {
            let proof = index.probe(key.id());
            index.insert_at(proof, n + 1000);
        }
        for (n, key) in keys.iter().enumerate() {
            assert_eq!(
                index.get(key.id()),
                Some(if n % 2 == 0 { n + 1000 } else { n })
            );
        }
    }

    #[test]
    fn short_lived_distinct_keys_always_leave_an_empty_probe_terminator() {
        let domain = AtomDomain::new();
        let mut index = IdIndex::new();
        for n in 0..10_000 {
            let key = domain.intern(&format!("transient{n}"));
            index.insert_at(index.probe(key.id()), n);
            index.remove_at(index.probe(key.id()));
            assert_eq!(index.len, 0);
            assert!(index.tombstones < index.buckets.len());
        }
    }

    #[test]
    fn cloned_layout_consumes_the_original_vacancy_without_searching_again() {
        let domain = AtomDomain::new();
        let a = domain.intern("a");
        let b = domain.intern("b");
        let mut index = IdIndex::new();
        index.insert_at(index.probe(a.id()), 0);
        let proof = index.probe(b.id());
        let mut copied = index.clone();
        copied.insert_at(proof, 1);
        assert_eq!(index.get(b.id()), None);
        assert_eq!(copied.get(a.id()), Some(0));
        assert_eq!(copied.get(b.id()), Some(1));
    }
}
