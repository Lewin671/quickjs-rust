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
    users: RefCell<Vec<Weak<PrototypeState>>>,
    direct_trackable: Cell<bool>,
    trackable: Cell<bool>,
    sweep_at: Cell<usize>,
}
impl PrototypeState {
    fn new(parent: Weak<Self>, direct_trackable: bool) -> Rc<Self> {
        let trackable = direct_trackable && parent.upgrade().is_none_or(|p| p.trackable.get());
        let state = Rc::new(Self {
            cell: RefCell::new(StructuralDependency::new()),
            parent: RefCell::new(parent),
            users: RefCell::new(Vec::new()),
            direct_trackable: Cell::new(direct_trackable),
            trackable: Cell::new(trackable),
            sweep_at: Cell::new(64),
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
            users.retain(|u| {
                u.upgrade()
                    .is_some_and(|u| u.parent.borrow().as_ptr() == Rc::as_ptr(&parent))
            });
            parent
                .sweep_at
                .set(users.len().saturating_mul(2).saturating_add(64));
        }
        users.push(Rc::downgrade(this));
    }
    fn reparent(this: &Rc<Self>, parent: Weak<Self>, direct_trackable: bool) {
        if !this.parent.borrow().ptr_eq(&parent) {
            if let Some(old) = this.parent.borrow().upgrade() {
                old.users
                    .borrow_mut()
                    .retain(|w| w.as_ptr() != Rc::as_ptr(this) && w.strong_count() > 0);
            }
            *this.parent.borrow_mut() = parent;
            Self::register(this);
        }
        this.direct_trackable.set(direct_trackable);
        Self::invalidate(this);
    }
    pub(crate) fn invalidate(this: &Rc<Self>) {
        let mut pending = vec![this.clone()];
        while let Some(state) = pending.pop() {
            state.cell.borrow().invalidate();
            let tracked = state.direct_trackable.get()
                && state
                    .parent
                    .borrow()
                    .upgrade()
                    .is_none_or(|p| p.trackable.get());
            state.trackable.set(tracked);
            *state.cell.borrow_mut() = StructuralDependency::new();
            let mut users = state.users.borrow_mut();
            users.retain(|weak| {
                let Some(user) = weak.upgrade() else {
                    return false;
                };
                if user.parent.borrow().as_ptr() != Rc::as_ptr(&state) {
                    return false;
                }
                pending.push(user);
                true
            });
        }
    }
    pub(super) fn token(&self) -> Option<StructuralDependency> {
        self.trackable.get().then(|| self.cell.borrow().clone())
    }
}
impl Drop for PrototypeState {
    fn drop(&mut self) {
        self.cell.get_mut().invalidate();
    }
}

impl Layout {
    fn root_shape(layout: &Rc<Self>) -> &Rc<Self> {
        layout.root.as_ref().unwrap_or(layout)
    }
    pub(crate) fn matches_prototype(&self, prototype: Option<&Prototype>, flavor: u16) -> bool {
        self.flavor == flavor
            && self.prototype.as_ref().map(WeakPrototype::identity)
                == prototype.map(Prototype::identity)
    }
}
impl PropertyOwner {
    pub(super) fn root_for(&self, prototype: Option<&Prototype>, flavor: u16) -> Rc<Layout> {
        if prototype.is_none() && flavor == 0 {
            return self.0.root.clone();
        }
        let (kind, address) = prototype.map(Prototype::identity).unwrap_or((0, 0));
        let key = (kind, address, flavor);
        if let Some(layout) = self.0.roots.borrow().get(&key).and_then(Weak::upgrade) {
            return layout;
        }
        let parent_state = prototype.and_then(Prototype::enter_prototype_mode);
        let budget = self.0.root.budget.clone();
        budget.add(size_of::<Layout>());
        let root = Rc::new(Layout {
            budget: budget.clone(),
            domain: self.domain().clone(),
            owner: Rc::downgrade(&self.0),
            data: Rc::new(RefCell::new(storage::LayoutData::new(budget))),
            len: 0,
            parent: None,
            incoming: None,
            dictionary: false,
            edits: 0,
            next_order: 0,
            transitions: RefCell::default(),
            root: None,
            prototype: prototype.map(WeakPrototype::new),
            prototype_value: RefCell::new(Weak::new()),
            parent_state: parent_state.as_ref().map(Rc::downgrade).unwrap_or_default(),
            mode: None,
            flavor,
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
        let layout = owner.root_for(prototype.as_ref(), flavor);
        let strong = Self::retain_prototype(&layout, prototype);
        Self {
            layout,
            values: ValueSlots::Empty,
            accessors: None,
            epoch: 0,
            prototype: strong,
        }
    }
    pub(super) fn retain_prototype(
        layout: &Rc<Layout>,
        prototype: Option<Prototype>,
    ) -> Option<Rc<Prototype>> {
        let prototype = prototype?;
        let root = Layout::root_shape(layout);
        if let Some(value) = root.prototype_value.borrow().upgrade() {
            return Some(value);
        }
        let value = Rc::new(prototype);
        *root.prototype_value.borrow_mut() = Rc::downgrade(&value);
        Some(value)
    }
    pub(crate) fn first_prototype_data(&self, version: &LayoutVersion, slot: u32) -> Option<Value> {
        if !self.matches_version(version) {
            return None;
        }
        self.prototype.as_deref()?.load_guarded_data(slot)
    }
    pub(crate) fn prototype(&self) -> Option<Prototype> {
        self.prototype.as_deref().cloned()
    }
    pub(crate) fn root_version(&self) -> LayoutVersion {
        LayoutVersion::shared(Layout::root_shape(&self.layout))
    }
    pub(crate) fn root_matches(&self, token: &LayoutVersion) -> bool {
        token.matches_layout(Layout::root_shape(&self.layout), 0)
    }
    pub(crate) fn chain_dependency(&self) -> Option<StructuralDependency> {
        Layout::root_shape(&self.layout)
            .parent_state
            .upgrade()?
            .token()
    }
    pub(crate) fn prototype_mode(&mut self) -> Rc<PrototypeState> {
        if let Some(mode) = &self.layout.mode {
            return mode.clone();
        }
        let root = Layout::root_shape(&self.layout).clone();
        let tracked = root.prototype.is_none() || root.parent_state.strong_count() > 0;
        let state = PrototypeState::new(root.parent_state.clone(), tracked);
        let mut layout = self.layout.copied(true);
        layout.root = Some(root);
        layout.mode = Some(state.clone());
        self.layout = Rc::new(layout);
        self.epoch = 1;
        state
    }
    pub(crate) fn set_prototype(&mut self, prototype: Option<Prototype>) {
        self.rebase(prototype, self.layout.flavor);
    }
    pub(crate) fn set_flavor(&mut self, flavor: u16) {
        self.rebase(self.prototype(), flavor);
    }
    pub(super) fn rebase(&mut self, prototype: Option<Prototype>, flavor: u16) {
        if self.layout.matches_prototype(prototype.as_ref(), flavor) {
            return;
        }
        let owner = PropertyOwner::new(self.domain().clone());
        let root = owner.root_for(prototype.as_ref(), flavor);
        let mode = self.layout.mode.clone();
        if self.layout.dictionary {
            self.invalidate_dependencies();
            let mut next = self.layout.copied(true);
            next.root = Some(root.clone());
            next.prototype = root.prototype.clone();
            next.parent_state = root.parent_state.clone();
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
            self.epoch = 0;
        }
        self.prototype = Self::retain_prototype(&root, prototype);
        if let Some(mode) = mode {
            PrototypeState::reparent(
                &mode,
                root.parent_state.clone(),
                root.prototype.is_none() || root.parent_state.strong_count() > 0,
            );
        }
    }
    pub(crate) fn invalidate_dependencies(&self) {
        if let Some(mode) = &self.layout.mode {
            PrototypeState::invalidate(mode);
        }
    }
}

impl Prototype {
    pub(crate) fn enter_prototype_mode(&self) -> Option<Rc<PrototypeState>> {
        match self {
            Self::Object(o) => Some(o.with_named_mut(NamedStore::prototype_mode)),
            Self::Array(a) => Some(a.array_ref().with_named_mut(NamedStore::prototype_mode)),
            Self::Function(f) => Some(f.with_named_mut(NamedStore::prototype_mode)),
            Self::Map(m) => Some(m.object_ref().with_named_mut(NamedStore::prototype_mode)),
            Self::Set(s) => Some(s.object_ref().with_named_mut(NamedStore::prototype_mode)),
            Self::Proxy(_) => None,
        }
    }
}
