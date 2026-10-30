//! Weak transition lookup never evicts a live child. Only strong keep-alive
//! ownership is bounded; conservative byte charges include ancestor chains.
use super::*;
use crate::value::name_hash::NameMap;

#[derive(Default)]
pub(super) struct Transitions {
    small: Vec<(Transition, Weak<Layout>)>,
    large: Option<NameMap<Transition, Weak<Layout>>>,
    sweep_at: usize,
    inserts: usize,
    budget: Option<storage::Budget>,
    charged: usize,
}
impl Transitions {
    pub(super) fn get(&self, label: Transition) -> Option<Rc<Layout>> {
        match &self.large {
            Some(map) => map.get(&label).and_then(Weak::upgrade),
            None => self
                .small
                .iter()
                .find(|(key, _)| *key == label)
                .and_then(|(_, child)| child.upgrade()),
        }
    }
    pub(super) fn is_empty(&self) -> bool {
        self.small.is_empty() && self.large.as_ref().is_none_or(NameMap::is_empty)
    }
    fn insert(&mut self, label: Transition, child: &Rc<Layout>) {
        self.budget.get_or_insert_with(|| child.budget.clone());
        if let Some(map) = &mut self.large {
            self.inserts += 1;
            if self.inserts >= self.sweep_at {
                map.retain(|_, child| child.strong_count() > 0);
                map.shrink_to_fit();
                self.sweep_at = map.len().max(16);
                self.inserts = 0;
            }
            map.insert(label, Rc::downgrade(child));
        } else {
            self.small.retain(|(_, child)| child.strong_count() > 0);
            if let Some((_, old)) = self.small.iter_mut().find(|(key, _)| *key == label) {
                *old = Rc::downgrade(child);
            } else if self.small.len() < 8 {
                self.small.push((label, Rc::downgrade(child)));
            } else {
                let mut map: NameMap<_, _> = self.small.drain(..).collect();
                map.insert(label, Rc::downgrade(child));
                self.large = Some(map);
                self.sweep_at = 16;
            }
        }
        self.reaccount();
    }
    pub(super) fn remove(&mut self, label: Transition, address: *const Layout) {
        if let Some(map) = &mut self.large {
            if map
                .get(&label)
                .is_some_and(|child| child.as_ptr() == address)
            {
                map.remove(&label);
                if map.len() * 4 < map.capacity() {
                    map.shrink_to_fit();
                }
            }
        } else {
            self.small
                .retain(|(key, child)| *key != label || child.as_ptr() != address);
        }
        self.reaccount();
    }
    fn reaccount(&mut self) {
        let capacity = self.small.capacity() + self.large.as_ref().map_or(0, NameMap::capacity);
        let bytes = capacity * (size_of::<Layout>() + 2 * size_of::<(Transition, Weak<Layout>)>());
        if let Some(budget) = &self.budget {
            budget.sub(self.charged);
            budget.add(bytes);
        }
        self.charged = bytes;
    }
}
impl Drop for Transitions {
    fn drop(&mut self) {
        if let Some(budget) = &self.budget {
            budget.sub(self.charged);
        }
    }
}
#[derive(Default)]
pub(super) struct Retention {
    pub(super) nodes: VecDeque<Rc<Layout>>,
}
impl Layout {
    pub(super) fn cache(&self, label: Transition, child: &Rc<Self>) {
        self.transitions.borrow_mut().insert(label, child);
        if let Some(owner) = self.owner.upgrade() {
            let mut retained = owner.retained.borrow_mut();
            while child.budget.bytes() > MAX_CACHED_BYTES {
                let Some(node) = retained.nodes.pop_front() else {
                    break;
                };
                drop(node);
                #[cfg(feature = "perf-counters")]
                crate::diagnostics::update(|c| {
                    c.property_cached_shape_nodes = c.property_cached_shape_nodes.saturating_sub(1)
                });
            }
            if child.budget.bytes() <= MAX_CACHED_BYTES {
                retained.nodes.push_back(child.clone());
                crate::diagnostics::count!(property_cached_shape_nodes);
            }
        }
    }
}
