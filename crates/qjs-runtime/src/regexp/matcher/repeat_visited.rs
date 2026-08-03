use std::collections::HashMap;

type Capture = Option<(usize, usize)>;

struct RepeatKey {
    index: usize,
    count: usize,
    captures: Vec<Capture>,
    next: Option<usize>,
}

/// Exact visited-state storage whose capture buffers survive candidate-start
/// retries. A cheap fingerprint selects a collision chain, whose full state
/// comparisons preserve capture-sensitive backreference semantics.
#[derive(Default)]
pub(super) struct RepeatVisited {
    entries: Vec<RepeatKey>,
    heads: HashMap<u64, usize>,
    active: usize,
}

impl RepeatVisited {
    pub(super) fn insert(&mut self, index: usize, count: usize, captures: &[Capture]) -> bool {
        let fingerprint = repeat_fingerprint(index, count, captures);
        let mut cursor = self.heads.get(&fingerprint).copied();
        while let Some(entry_index) = cursor {
            let entry = &self.entries[entry_index];
            if entry.index == index && entry.count == count && entry.captures == captures {
                return false;
            }
            cursor = entry.next;
        }

        let next = self.heads.get(&fingerprint).copied();
        if self.active == self.entries.len() {
            self.entries.push(RepeatKey {
                index,
                count,
                captures: captures.to_vec(),
                next,
            });
        } else {
            let entry = &mut self.entries[self.active];
            entry.index = index;
            entry.count = count;
            entry.captures.clear();
            entry.captures.extend_from_slice(captures);
            entry.next = next;
        }
        self.heads.insert(fingerprint, self.active);
        self.active += 1;
        true
    }

    pub(super) fn clear(&mut self) {
        self.active = 0;
        self.heads.clear();
    }

    pub(super) fn is_empty(&self) -> bool {
        self.active == 0 && self.heads.is_empty()
    }
}

fn repeat_fingerprint(index: usize, count: usize, captures: &[Capture]) -> u64 {
    fn mix(hash: u64, value: usize) -> u64 {
        (hash.rotate_left(5) ^ value as u64).wrapping_mul(0x9e37_79b1_85eb_ca87)
    }

    let mut hash = mix(0x517c_c1b7_2722_0a95, index);
    hash = mix(hash, count);
    for capture in captures {
        match capture {
            Some((start, end)) => {
                hash = mix(hash, start.wrapping_add(1));
                hash = mix(hash, end.wrapping_add(1));
            }
            None => {
                hash = mix(hash, 0);
                hash = mix(hash, 0);
            }
        }
    }
    hash
}
