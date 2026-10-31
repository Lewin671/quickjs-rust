//! Canonical prototype/flavor bindings and amortized weak-prototype cache cleanup.
//! Live receivers own prototype values; cached roots own only weak identities.
use super::*;
use crate::Prototype;

impl PropertyOwner {
    fn sweep_dead_roots_if_due(&self) {
        let left = self.0.roots_until_sweep.get();
        if left > 0 {
            self.0.roots_until_sweep.set(left - 1);
            return;
        }
        // A live receiver owns its prototype, so a dead prototype proves no
        // receiver can reuse this binding. Sweep only on root misses, spaced
        // by live root count to keep large live prototype graphs amortized.
        {
            let mut retained = self.0.retained.borrow_mut();
            #[cfg(feature = "perf-counters")]
            let before = retained.nodes.len();
            retained.nodes.retain(|layout| {
                layout
                    .prototype
                    .as_ref()
                    .is_none_or(|prototype| prototype.upgrade().is_some())
            });
            #[cfg(feature = "perf-counters")]
            crate::diagnostics::update(|c| {
                c.property_cached_shape_nodes = c
                    .property_cached_shape_nodes
                    .saturating_sub((before - retained.nodes.len()) as u64);
            });
        }
        let mut roots = self.0.roots.borrow_mut();
        roots.retain(|_, root| root.strong_count() > 0);
        self.0
            .roots_until_sweep
            .set(roots.len().saturating_mul(2).saturating_add(64));
    }

    fn cached_root(&self, prototype: Option<&Prototype>, flavor: u16) -> Option<Rc<Layout>> {
        if prototype.is_none() && flavor == 0 {
            return Some(self.0.root.clone());
        }
        let (kind, address) = prototype.map(Prototype::identity).unwrap_or((0, 0));
        self.0
            .roots
            .borrow()
            .get(&(kind, address, flavor))
            .and_then(Weak::upgrade)
    }
    /// These seeds are never issued as guards, including for null prototypes.
    pub(super) fn empty_layout(&self, prototype: Option<&Prototype>, flavor: u16) -> Rc<Layout> {
        if let Some(root) = self.cached_root(prototype, flavor) {
            return root;
        }
        if prototype.is_none() {
            return self.root_for(None, flavor);
        }
        if let Some(seed) = self.0.seeds.borrow().get(&flavor) {
            return seed.clone();
        }
        let mut seed = self.0.root.copied(false);
        seed.data = self.0.root.data.clone();
        seed.flavor = flavor;
        seed.deferred_seed = true;
        let seed = Rc::new(seed);
        self.0.seeds.borrow_mut().insert(flavor, seed.clone());
        seed
    }
    pub(super) fn write_root(&self, prototype: Option<&Prototype>, flavor: u16) -> Rc<Layout> {
        if let Some(root) = self.cached_root(prototype, flavor) {
            return root;
        }
        let budget = self.0.root.budget.clone();
        budget.add(size_of::<Layout>());
        Rc::new(Layout {
            budget,
            domain: self.domain().clone(),
            owner: Rc::downgrade(&self.0),
            data: self.0.root.data.clone(),
            len: 0,
            links: OnceCell::new(),
            dictionary: false,
            epoch: std::cell::Cell::new(0),
            deferred_seed: false,
            transient: true,
            edits: 0,
            prototype: prototype.map(WeakPrototype::new),
            flavor,
            capacity_hint: std::cell::Cell::new(0),
            append_misses: std::cell::Cell::new(0),
        })
    }

    pub(super) fn root_for(&self, prototype: Option<&Prototype>, flavor: u16) -> Rc<Layout> {
        if prototype.is_none() && flavor == 0 {
            return self.0.root.clone();
        }
        let (kind, address) = prototype.map(Prototype::identity).unwrap_or((0, 0));
        let key = (kind, address, flavor);
        if let Some(layout) = self.0.roots.borrow().get(&key).and_then(Weak::upgrade) {
            return layout;
        }
        self.sweep_dead_roots_if_due();
        let budget = self.0.root.budget.clone();
        budget.add(size_of::<Layout>());
        let root = Rc::new(Layout {
            budget: budget.clone(),
            domain: self.domain().clone(),
            owner: Rc::downgrade(&self.0),
            data: self.0.root.data.clone(),
            len: 0,
            links: OnceCell::new(),
            dictionary: false,
            epoch: std::cell::Cell::new(0),
            deferred_seed: false,
            transient: false,
            edits: 0,
            prototype: prototype.map(WeakPrototype::new),
            flavor,
            capacity_hint: std::cell::Cell::new(0),
            append_misses: std::cell::Cell::new(0),
        });
        let mut roots = self.0.roots.borrow_mut();
        roots.insert(key, Rc::downgrade(&root));
        drop(roots);
        root.retain();
        root
    }
}
