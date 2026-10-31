//! Key-ID index with consumable vacant-bucket proofs.
//!
//! A write lookup yields a bucket proof; read lookups return only positions.
//! Full index copies preserve that proof;
//! prefix forks rebuild and consume a new proof as cold structural work.
//! Each insertion reserves space for the next one; index rebuilds are counted
//! separately from operation probes. Shared-tail admission lives in storage.
//! The hash shift is derived from bucket capacity only at allocation/rebuild.

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
    hash_shift: u32,
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

impl IdProbe {
    pub(super) fn linear(key: KeyId, position: Option<usize>) -> Self {
        Self {
            key,
            bucket: position.unwrap_or(0),
            capacity: 0,
            position,
        }
    }
}
impl IdIndex {
    pub(super) fn new() -> Self {
        Self {
            buckets: vec![EMPTY; 8],
            hash_shift: Self::shift_for(8),
            len: 0,
            tombstones: 0,
        }
    }
    pub(super) fn bytes(&self) -> usize {
        self.buckets.capacity() * size_of::<Bucket>()
    }
    pub(super) fn unallocated() -> Self {
        Self {
            buckets: Vec::new(),
            hash_shift: 0,
            len: 0,
            tombstones: 0,
        }
    }
    pub(super) fn allocated(&self) -> bool {
        !self.buckets.is_empty()
    }
    fn shift_for(capacity: usize) -> u32 {
        32 - capacity.trailing_zeros().min(32)
    }
    #[cfg(test)]
    fn bucket(key: KeyId, capacity: usize) -> usize {
        (key.get().wrapping_mul(0x9e37_79b1) >> Self::shift_for(capacity)) as usize
    }
    pub(super) fn probe(&self, key: KeyId) -> IdProbe {
        crate::diagnostics::count!(property_id_probes);
        self.probe_uncounted(key)
    }
    /// A read needs neither a vacant bucket nor an insertion-capacity proof.
    #[inline]
    pub(super) fn position(&self, key: KeyId) -> Option<usize> {
        let mask = self.buckets.len() - 1;
        let mut bucket = (key.get().wrapping_mul(0x9e37_79b1) >> self.hash_shift) as usize;
        loop {
            let entry = self.buckets[bucket];
            match entry.key {
                Some(candidate) if candidate == key => return Some(entry.position as usize),
                None if entry.position != TOMBSTONE => return None,
                _ => bucket = (bucket + 1) & mask,
            }
        }
    }
    pub(super) fn probe_uncounted(&self, key: KeyId) -> IdProbe {
        let mask = self.buckets.len() - 1;
        let mut bucket = (key.get().wrapping_mul(0x9e37_79b1) >> self.hash_shift) as usize;
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
        crate::diagnostics::count!(property_id_probes);
        self.position(key)
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
        if !self.allocated() {
            debug_assert_eq!(proof.capacity, 0);
            return;
        }
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
        if !self.allocated() {
            return;
        }
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
        self.hash_shift = Self::shift_for(capacity);
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

    #[cfg(target_pointer_width = "64")]
    #[test]
    fn capacities_beyond_the_id_namespace_do_not_overflow_the_hash_shift() {
        let domain = AtomDomain::new();
        let key = domain.intern("x");
        // Exercise the hash without allocating a multi-gigabyte bucket array.
        assert!(IdIndex::bucket(key.id(), 1usize << 33) < 1usize << 33);
    }

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
