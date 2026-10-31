//! Prototype-sensitive root layouts and per-prototype validity generations.
//! Layouts hold weak identities; only actual receivers own prototype values.
//! The user graph is weak, uncapped, and invalidation drains it iteratively.
use super::*;
use crate::Prototype;
use std::cell::Cell;

#[derive(Clone, Debug)]
pub(crate) enum WeakPrototype {
    Object(crate::value::ObjectWeakRef),
    Array(crate::value::ArrayWeakRef),
    Function(crate::function::FunctionWeakRef),
    Proxy(crate::proxy::ProxyWeakRef),
    Map(crate::value::map::MapWeakRef),
    Set(crate::value::set::SetWeakRef),
}
impl WeakPrototype {
    fn new(value: &Prototype) -> Self {
        match value {
            Prototype::Object(o) => Self::Object(o.downgrade()),
            Prototype::Array(a) => Self::Array(a.array_ref().downgrade()),
            Prototype::Function(f) => Self::Function(f.downgrade()),
            Prototype::Proxy(p) => Self::Proxy(p.downgrade()),
            Prototype::Map(m) => Self::Map(m.downgrade()),
            Prototype::Set(s) => Self::Set(s.downgrade()),
        }
    }
    pub(crate) fn identity(&self) -> (u8, usize) {
        match self {
            Self::Object(v) => (1, v.address()),
            Self::Array(v) => (2, v.address()),
            Self::Function(v) => (3, v.address()),
            Self::Proxy(v) => (4, v.address()),
            Self::Map(v) => (5, v.address()),
            Self::Set(v) => (6, v.address()),
        }
    }
    pub(crate) fn upgrade(&self) -> Option<Prototype> {
        match self {
            Self::Object(v) => v.upgrade().map(Prototype::Object),
            Self::Array(v) => v.upgrade().map(Prototype::array),
            Self::Function(v) => v.upgrade().map(Prototype::Function),
            Self::Proxy(v) => v.upgrade().map(Prototype::Proxy),
            Self::Map(v) => v.upgrade().map(Prototype::Map),
            Self::Set(v) => v.upgrade().map(Prototype::Set),
        }
    }
}

pub(crate) struct PrototypeState {
    cell: RefCell<StructuralDependency>,
    parent: RefCell<Weak<PrototypeState>>,
    users: RefCell<Vec<(Weak<PrototypeState>, u64)>>,
    direct_trackable: Cell<bool>,
    trackable: Cell<bool>,
    sweep_at: Cell<usize>,
    value_writes: Cell<u8>,
    initialized: Cell<bool>,
    registration: Cell<u64>,
    #[cfg(test)]
    visits: Cell<usize>,
}
impl PrototypeState {
    fn new(parent: Weak<Self>, direct_trackable: bool) -> Rc<Self> {
        let trackable = direct_trackable && parent.upgrade().is_none_or(|p| p.trackable.get());
        let cell = StructuralDependency::new();
        cell.invalidate();
        let state = Rc::new(Self {
            cell: RefCell::new(cell),
            parent: RefCell::new(parent),
            users: RefCell::new(Vec::new()),
            direct_trackable: Cell::new(direct_trackable),
            trackable: Cell::new(trackable),
            sweep_at: Cell::new(64),
            value_writes: Cell::new(0),
            initialized: Cell::new(false),
            registration: Cell::new(0),
            #[cfg(test)]
            visits: Cell::new(0),
        });
        Self::register(&state);
        state
    }
    fn register(this: &Rc<Self>) {
        let Some(parent) = this.parent.borrow().upgrade() else {
            return;
        };
        let mut users = parent.users.borrow_mut();
        if users.len() >= parent.sweep_at.get() {
            users.retain(|(weak, registration)| {
                weak.upgrade().is_some_and(|u| {
                    u.parent.borrow().as_ptr() == Rc::as_ptr(&parent)
                        && u.registration.get() == *registration
                })
            });
            parent
                .sweep_at
                .set(users.len().saturating_mul(2).saturating_add(64));
        }
        let registration = this
            .registration
            .get()
            .checked_add(1)
            .expect("prototype registration space exhausted");
        this.registration.set(registration);
        users.push((Rc::downgrade(this), registration));
    }
    fn reparent(this: &Rc<Self>, parent: Weak<Self>, direct_trackable: bool) {
        if !this.parent.borrow().ptr_eq(&parent) {
            *this.parent.borrow_mut() = parent;
            Self::register(this);
        }
        this.direct_trackable.set(direct_trackable);
        Self::invalidate(this);
    }
    pub(crate) fn invalidate(this: &Rc<Self>) {
        let mut pending = vec![this.clone()];
        while let Some(state) = pending.pop() {
            #[cfg(test)]
            state.visits.set(state.visits.get() + 1);
            let tracked = state.direct_trackable.get()
                && state
                    .parent
                    .borrow()
                    .upgrade()
                    .is_none_or(|p| p.trackable.get());
            // A fresh descendant token arms its ancestors. An invalid cell
            // with unchanged trackability therefore proves the subtree inert.
            if !state.cell.borrow().is_valid() && tracked == state.trackable.get() {
                continue;
            }
            state.cell.borrow().invalidate();
            state.trackable.set(tracked);
            let mut users = state.users.borrow_mut();
            users.retain(|(weak, registration)| {
                let Some(user) = weak.upgrade() else {
                    return false;
                };
                if user.parent.borrow().as_ptr() != Rc::as_ptr(&state)
                    || user.registration.get() != *registration
                {
                    return false;
                }
                pending.push(user);
                true
            });
        }
    }
    // After four invalidations the holder uses live slots permanently.
    fn values_stable(&self) -> bool {
        self.value_writes.get() < 4
    }
    #[cold]
    #[inline(never)]
    fn value_write(this: &Rc<Self>) {
        if this.values_stable() {
            this.value_writes.set(this.value_writes.get() + 1);
            Self::invalidate(this);
        }
    }
    pub(super) fn token(&self) -> Option<StructuralDependency> {
        if !self.trackable.get() {
            return None;
        }
        Self::arm(self);
        let mut parent = self.parent.borrow().upgrade();
        while let Some(state) = parent {
            if state.cell.borrow().is_valid() {
                break;
            }
            Self::arm(&state);
            parent = state.parent.borrow().upgrade();
        }
        Some(self.cell.borrow().clone())
    }
    fn arm(&self) {
        if !self.cell.borrow().is_valid() {
            *self.cell.borrow_mut() = StructuralDependency::new();
        }
    }
}
impl Drop for PrototypeState {
    fn drop(&mut self) {
        self.cell.get_mut().invalidate();
    }
}

impl Layout {
    pub(super) fn root_shape(layout: &Rc<Self>) -> &Rc<Self> {
        if let Some(root) = layout.known_root() {
            return root;
        }
        if layout.len == 0 && !layout.transient {
            return layout;
        }
        let prototype = layout.prototype.as_ref().and_then(WeakPrototype::upgrade);
        if layout.prototype.is_some() && prototype.is_none() {
            return layout;
        }
        layout.links().root.get_or_init(|| {
            PropertyOwner::new(layout.domain.clone()).root_for(prototype.as_ref(), layout.flavor)
        })
    }
    pub(crate) fn matches_prototype(&self, prototype: Option<&Prototype>, flavor: u16) -> bool {
        !self.deferred_seed
            && self.flavor == flavor
            && self.prototype.as_ref().map(WeakPrototype::identity)
                == prototype.map(Prototype::identity)
    }
}
impl PropertyOwner {
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
    fn empty_layout(&self, prototype: Option<&Prototype>, flavor: u16) -> Rc<Layout> {
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
        if roots.len() >= 64 && roots.len().is_power_of_two() {
            roots.retain(|_, r| r.strong_count() > 0);
        }
        roots.insert(key, Rc::downgrade(&root));
        drop(roots);
        root.retain();
        root
    }
}
impl NamedStore {
    pub(crate) fn empty_with_prototype(
        domain: &AtomDomain,
        prototype: Option<Prototype>,
        flavor: u16,
    ) -> Self {
        let owner = PropertyOwner::new(domain.clone());
        let layout = owner.empty_layout(prototype.as_ref(), flavor);
        Self {
            layout,
            values: ValueSlots::Empty,
            cold: ColdStorage::default(),
            prototype,
        }
    }
    /// Compact empty authorities keep their smaller boxed-edge representation.
    /// The root memo is weak: sharing an edge box must never pin a JS value.
    pub(super) fn compact_prototype(
        layout: &Rc<Layout>,
        prototype: Option<Prototype>,
    ) -> Option<Rc<Prototype>> {
        let prototype = prototype?;
        let root = Layout::root_shape(layout);
        if let Some(edge) = root.links().compact_prototype.borrow().upgrade() {
            return Some(edge);
        }
        let edge = Rc::new(prototype);
        *root.links().compact_prototype.borrow_mut() = Rc::downgrade(&edge);
        Some(edge)
    }
    fn deferred(&self) -> bool {
        self.layout.deferred_seed
    }
    /// An empty seed carries no cacheable prototype identity until admission.
    pub(super) fn guard_layout(&self) -> &Rc<Layout> {
        if !self.needs_publication() {
            return &self.layout;
        }
        self.cold.get().guard.get_or_init(|| {
            let owner = PropertyOwner::new(self.domain().clone());
            let mut next = owner.root_for(self.prototype.as_ref(), self.layout.flavor);
            for (key, entry) in self.layout.keys().iter().zip(self.layout.entries().iter()) {
                let proof = next.probe(key.id());
                next = next.append(key, entry.attrs, proof);
            }
            next
        })
    }
    pub(super) fn matched_layout(&self) -> Option<&Rc<Layout>> {
        if self.needs_publication() {
            self.cold.guard()
        } else {
            Some(&self.layout)
        }
    }
    pub(super) fn needs_publication(&self) -> bool {
        self.layout.deferred_seed || (self.layout.transient && !self.layout.dictionary)
    }
    /// Publish observed metadata without changing properties or their slots.
    pub(crate) fn materialize_layout(&mut self) {
        if self.needs_publication() {
            self.layout = self.guard_layout().clone();
            self.cold.clear_guard();
        }
    }
    pub(super) fn materialize_write_layout(&mut self) {
        if self.deferred() {
            self.layout = self.cold.guard().cloned().unwrap_or_else(|| {
                PropertyOwner::new(self.domain().clone())
                    .write_root(self.prototype.as_ref(), self.layout.flavor)
            });
            self.cold.clear_guard();
        }
    }
    pub(crate) fn first_prototype_data(&self, version: &LayoutVersion, slot: u32) -> Option<Value> {
        if !self.matches_version(version) {
            return None;
        }
        self.prototype.as_ref()?.load_guarded_data(slot)
    }
    /// Pure identity check without acquiring a prototype owner.
    #[inline(always)]
    pub(crate) fn prototype_is_object(&self, expected: &crate::ObjectRef) -> bool {
        matches!(self.prototype.as_ref(), Some(Prototype::Object(actual)) if actual.ptr_eq(expected))
    }
    pub(crate) fn prototype(&self) -> Option<Prototype> {
        self.prototype.clone()
    }
    pub(crate) fn root_version(&self) -> LayoutVersion {
        LayoutVersion::shared(Layout::root_shape(self.guard_layout()))
    }
    pub(crate) fn root_matches(&self, token: &LayoutVersion) -> bool {
        self.matched_layout()
            .is_some_and(|layout| token.matches_layout(Layout::root_shape(layout), 0))
    }
    pub(crate) fn chain_dependency(&self) -> Option<StructuralDependency> {
        self.prototype.as_ref()?.enter_prototype_mode()?.token()
    }
    pub(crate) fn prototype_mode(&mut self) -> Rc<PrototypeState> {
        self.cold
            .get()
            .mode
            .get_or_init(|| PrototypeState::new(Weak::new(), true))
            .clone()
    }
    pub(crate) fn set_prototype(&mut self, prototype: Option<Prototype>) {
        self.rebase(prototype, self.layout.flavor);
    }
    pub(crate) fn set_flavor(&mut self, flavor: u16) {
        self.rebase(self.prototype(), flavor);
    }
    pub(super) fn rebase(&mut self, prototype: Option<Prototype>, flavor: u16) {
        if self.layout.flavor == flavor
            && self.prototype.as_ref().map(Prototype::identity)
                == prototype.as_ref().map(Prototype::identity)
        {
            return;
        }
        let owner = PropertyOwner::new(self.domain().clone());
        let root = if self.is_empty() {
            owner.empty_layout(prototype.as_ref(), flavor)
        } else {
            owner.root_for(prototype.as_ref(), flavor)
        };
        let mode = self.cold.mode().cloned();
        self.invalidate_dependencies();
        self.cold.clear_guard();
        if self.is_empty() {
            self.layout = root.clone();
        } else if self.layout.dictionary {
            let mut next = self.layout.copied(true);
            next.set_root(root.clone());
            next.prototype = root.prototype.clone();
            next.flavor = flavor;
            self.layout = Rc::new(next);
            self.bump_epoch();
        } else {
            let mut next = root.clone();
            let keys = self.layout.keys();
            let fields = self.layout.entries();
            for (key, entry) in keys.iter().zip(fields.iter()) {
                let proof = next.probe(key.id());
                next = next.append(key, entry.attrs, proof);
            }
            drop(fields);
            drop(keys);
            self.layout = next;
        }
        self.prototype = prototype.clone();
        if let Some(mode) = mode {
            PrototypeState::reparent(
                &mode,
                prototype
                    .as_ref()
                    .and_then(Prototype::enter_prototype_mode)
                    .as_ref()
                    .map(Rc::downgrade)
                    .unwrap_or_default(),
                prototype.is_none() || !matches!(prototype, Some(Prototype::Proxy(_))),
            );
        }
    }
    /// Single barrier before observable named value/accessor mutation.
    #[inline(always)]
    pub(super) fn before_value_write(&self, value_only: bool) {
        // Additions have already invalidated through edit_layout/append.
        if value_only && let Some(mode) = self.cold.mode() {
            PrototypeState::value_write(mode);
        }
    }
    pub(crate) fn prototype_values_stable(&self) -> bool {
        self.cold.mode().is_some_and(|m| m.values_stable())
    }
    pub(crate) fn invalidate_dependencies(&self) {
        if let Some(mode) = self.cold.mode() {
            PrototypeState::invalidate(mode);
        }
    }
}

impl Prototype {
    /// Install only the demanded path, iteratively, without holding a store
    /// borrow while following another receiver. Proxy ancestry is untrackable.
    pub(crate) fn enter_prototype_mode(&self) -> Option<Rc<PrototypeState>> {
        let mut next = Some(self.clone());
        let mut path = Vec::new();
        let mut parent = None;
        let mut trackable = true;
        while let Some(prototype) = next {
            let local = |s: &mut NamedStore| (s.prototype_mode(), s.prototype());
            let (state, above) = match prototype {
                Self::Object(o) => o.with_named_mut(local),
                Self::Array(a) => a.array_ref().with_named_mut(local),
                Self::Function(f) => f.with_named_mut(local),
                Self::Map(m) => m.object_ref().with_named_mut(local),
                Self::Set(s) => s.object_ref().with_named_mut(local),
                Self::Proxy(_) => {
                    trackable = false;
                    break;
                }
            };
            if state.initialized.get() {
                parent = Some(state);
                break;
            }
            path.push(state);
            next = above;
        }
        for state in path.into_iter().rev() {
            *state.parent.borrow_mut() = parent.as_ref().map(Rc::downgrade).unwrap_or_default();
            state.direct_trackable.set(trackable);
            state
                .trackable
                .set(trackable && parent.as_ref().is_none_or(|p| p.trackable.get()));
            state.initialized.set(true);
            PrototypeState::register(&state);
            parent = Some(state);
            trackable = true;
        }
        parent
    }
}

#[cfg(test)]
#[path = "prototype_tests.rs"]
mod tests;
