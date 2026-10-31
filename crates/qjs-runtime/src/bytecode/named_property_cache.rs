//! Per-site mono/four-way/mega caches over complete pure lookup outcomes.
//! Layout identity implies [[Prototype]]; inherited hits use one prototype
//! generation plus a weak prototype constant or live unstable-holder slot.
//! Own slots always load live; ICs never own a receiver or heap constant.
//! The first mono entry is immutable. Disable its marker before replacement;
//! its inactive weak metadata survives only until this bounded site is dropped.
//! Shared bytecode sites use Rc handles; loop-owned sites keep this same data
//! inline in their existing program vector. Code clones start local sites cold.
use crate::property::atom::{KeyLease, WeakKeyLease};
use crate::value::{
    OwnDataPropertyWrite,
    layout::{LayoutVersion, StructuralDependency},
    named::{NamedTarget, Probe, Virtual, WeakTarget},
};
use crate::{CallEnv, ObjectRef, Property, Value};
use std::{
    cell::{Cell, OnceCell, RefCell},
    ops::Deref,
    rc::Rc,
};
mod constants;
mod misses;
mod stores;
use constants::Constant;
#[cfg(test)]
mod constant_tests;
#[cfg(test)]
mod tests;
use stores::AddEntry;
#[derive(Clone, Debug)]
pub(super) struct NamedPropertyCache(Rc<CacheData>);
#[derive(Debug, Default)]
pub(super) struct CacheData {
    first: OnceCell<Entry>,
    first_active: Cell<bool>,
    state: RefCell<State>,
    local_slot: Option<usize>,
}
#[derive(Debug, Default)]
struct State {
    entries: Entries,
    key: Option<WeakKeyLease>,
    add: Option<Box<AddEntry>>,
    #[cfg(debug_assertions)]
    name: Option<String>,
}
#[derive(Debug, Default)]
enum Entries {
    #[default]
    Empty,
    MonoFirst,
    Mono(Box<Entry>),
    Poly(Box<[Entry; 4]>),
    SharedOwnPoly(Box<[Entry; 4]>),
    Mega,
}
#[derive(Clone, Debug)]
struct Entry {
    version: LayoutVersion,
    slot: u32,
    flags: u32,
    chain: Option<Box<Chain>>,
}
#[derive(Clone, Debug)]
struct Chain {
    holder: Option<WeakTarget>,
    validity: StructuralDependency,
    constant: Option<Constant>,
}
const DATA: u32 = 0;
const ACCESSOR: u32 = 1;
const MISSING: u32 = 2;
const VIRTUAL: u32 = 3;
const OWN: u32 = 1 << 3;
const WRITABLE: u32 = 1 << 4;
const WRITE_FORBIDDEN: u32 = 1 << 5;
const VERIFY: u32 = 1 << 6;
const FIRST_HOLDER: u32 = 1 << 7;
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<Entry>() == 32);
#[cfg(all(target_pointer_width = "64", not(debug_assertions)))]
const _: () = assert!(std::mem::size_of::<CacheData>() == 96);
#[derive(Debug)]
pub(super) enum ReadOutcome {
    Value(Value),
    Accessor(Property),
}
impl ReadOutcome {
    fn value(self) -> Option<Value> {
        match self {
            Self::Value(v) => Some(v),
            _ => None,
        }
    }
}
#[derive(Clone)]
struct Resolution {
    probe: Probe,
    holder: Option<WeakTarget>,
    inherited: bool,
    first_holder: bool,
}
impl Resolution {
    fn identity(&self) -> Option<(u8, usize)> {
        if self.inherited {
            self.holder.as_ref().map(WeakTarget::identity)
        } else {
            None
        }
    }
    fn fetch(&self, target: NamedTarget<'_>) -> Option<ReadOutcome> {
        if self.probe == Probe::Missing {
            return Some(ReadOutcome::Value(Value::Undefined));
        }
        if self.inherited {
            self.holder
                .as_ref()?
                .upgrade()?
                .with_named_target(|t| fetch_probe(t, self.probe))?
        } else {
            fetch_probe(target, self.probe)
        }
    }
}
fn fetch_probe(target: NamedTarget<'_>, probe: Probe) -> Option<ReadOutcome> {
    Some(match probe {
        Probe::Data { slot, .. } => ReadOutcome::Value(target.load_data(slot)?),
        Probe::Accessor { slot } => ReadOutcome::Accessor(target.load_accessor(slot)?),
        Probe::Virtual(hook) => ReadOutcome::Value(target.virtual_value(hook)?),
        Probe::Missing => ReadOutcome::Value(Value::Undefined),
        Probe::Exotic => return None,
    })
}
impl Default for NamedPropertyCache {
    fn default() -> Self {
        Self(Rc::new(CacheData::default()))
    }
}
impl Deref for NamedPropertyCache {
    type Target = CacheData;
    #[inline(always)]
    fn deref(&self) -> &CacheData {
        &self.0
    }
}
impl NamedPropertyCache {
    pub(super) fn for_local(slot: usize) -> Self {
        Self(Rc::new(CacheData {
            local_slot: Some(slot),
            ..CacheData::default()
        }))
    }
}
impl Clone for CacheData {
    /// A code clone keeps its immutable configuration and learns its own sites.
    fn clone(&self) -> Self {
        Self {
            local_slot: self.local_slot,
            ..Self::local_site()
        }
    }
}
impl Entry {
    fn empty() -> Self {
        Self {
            version: LayoutVersion::empty(),
            slot: 0,
            flags: DATA | OWN,
            chain: None,
        }
    }
    fn shared_own(&self) -> bool {
        self.flags & (7 | OWN) == DATA | OWN && self.chain.is_none() && self.version.is_shared()
    }
    fn probe(&self) -> Probe {
        match self.flags & 7 {
            DATA => Probe::Data {
                slot: self.slot,
                writable: self.flags & WRITABLE != 0,
            },
            ACCESSOR => Probe::Accessor { slot: self.slot },
            MISSING => Probe::Missing,
            VIRTUAL => Probe::Virtual(match self.slot {
                0 => Virtual::ArrayLength,
                1 => Virtual::FunctionName,
                2 => Virtual::FunctionLength,
                3 => Virtual::FunctionPrototype,
                4 => Virtual::FunctionCaller,
                _ => Virtual::FunctionArguments,
            }),
            _ => unreachable!(),
        }
    }
    fn matches(&self, target: NamedTarget<'_>) -> bool {
        target.matches_version(&self.version)
            && self.chain.as_ref().is_none_or(|c| c.validity.is_valid())
    }
    #[cfg(debug_assertions)]
    fn resolution(&self) -> Resolution {
        Resolution {
            probe: self.probe(),
            holder: self.chain.as_ref().and_then(|c| c.holder.clone()),
            inherited: self.chain.is_some(),
            first_holder: self.flags & FIRST_HOLDER != 0,
        }
    }
    fn fetch(&self, target: NamedTarget<'_>) -> Option<ReadOutcome> {
        if self.probe() == Probe::Missing {
            return Some(ReadOutcome::Value(Value::Undefined));
        }
        if let Some(chain) = &self.chain {
            if let Some(constant) = &chain.constant {
                return constant.fetch();
            }
            chain
                .holder
                .as_ref()?
                .upgrade()?
                .with_named_target(|t| fetch_probe(t, self.probe()))?
        } else {
            fetch_probe(target, self.probe())
        }
    }
    fn same_guard(&self, e: &Self) -> bool {
        self.version == e.version
    }
    fn stale(&self) -> bool {
        !self.version.is_alive() || self.chain.as_ref().is_some_and(|c| !c.validity.is_valid())
    }
}
impl Entries {
    fn mono<'a>(&'a self, first: &'a OnceCell<Entry>) -> Option<&'a Entry> {
        match self {
            Self::MonoFirst => first.get(),
            Self::Mono(e) => Some(e),
            _ => None,
        }
    }
    fn find<'a>(
        &'a self,
        target: NamedTarget<'_>,
        first: &'a OnceCell<Entry>,
    ) -> Option<&'a Entry> {
        match self {
            Self::MonoFirst | Self::Mono(_) => self.mono(first).filter(|e| e.matches(target)),
            Self::Poly(entries) | Self::SharedOwnPoly(entries) => {
                entries.iter().find(|e| e.matches(target))
            }
            _ => None,
        }
    }
    fn poly(ways: Box<[Entry; 4]>) -> Self {
        if ways.iter().all(Entry::shared_own) {
            Self::SharedOwnPoly(ways)
        } else {
            Self::Poly(ways)
        }
    }
    fn record(&mut self, entry: Entry, first: &OnceCell<Entry>) {
        match self {
            Self::Empty if first.get().is_none() => {
                first.set(entry).expect("first monomorphic entry");
                *self = Self::MonoFirst;
            }
            Self::Empty => *self = Self::Mono(Box::new(entry)),
            Self::MonoFirst => {
                let old = first.get().expect("first entry");
                if old.same_guard(&entry)
                    && old.slot == entry.slot
                    && old.flags == entry.flags
                    && !old.stale()
                {
                    return;
                }
                if old.same_guard(&entry) || old.stale() {
                    *self = Self::Mono(Box::new(entry));
                } else {
                    *self = Self::poly(Box::new([
                        old.clone(),
                        entry,
                        Entry::empty(),
                        Entry::empty(),
                    ]));
                }
            }
            Self::Mono(old) if old.same_guard(&entry) || old.stale() => **old = entry,
            Self::Mono(_) => {
                let Self::Mono(old) = std::mem::take(self) else {
                    unreachable!()
                };
                *self = Self::poly(Box::new([*old, entry, Entry::empty(), Entry::empty()]));
            }
            Self::Poly(_) | Self::SharedOwnPoly(_) => {
                let (Self::Poly(mut ways) | Self::SharedOwnPoly(mut ways)) = std::mem::take(self)
                else {
                    unreachable!()
                };
                if let Some(slot) = ways.iter_mut().find(|e| e.same_guard(&entry) || e.stale()) {
                    *slot = entry;
                    *self = Self::poly(ways);
                } else {
                    *self = Self::Mega;
                }
            }
            Self::Mega => {}
        }
    }
}

impl CacheData {
    /// Fresh metadata owned directly by one compiled program site.
    pub(super) fn local_site() -> Self {
        Self::default()
    }
    #[inline(always)]
    fn hot_mono(&self) -> Option<&Entry> {
        if self.first_active.get() {
            self.first.get()
        } else {
            None
        }
    }
    #[cfg(debug_assertions)]
    #[cold]
    #[inline(never)]
    fn verify_hot(&self, entry: &Entry, target: NamedTarget<'_>) {
        assert!(self.first_active.get());
        assert!(std::ptr::eq(entry, self.first.get().expect("active first")));
        if entry.flags & VERIFY != 0 {
            self.verify_entry(&self.state.borrow(), entry, target);
        }
    }
    pub(super) fn local_slot(&self) -> Option<usize> {
        self.local_slot
    }
    fn cached(&self, target: NamedTarget<'_>, accessor: bool) -> Option<ReadOutcome> {
        let state = self.state.borrow();
        // The monomorphic own-data path has one authority borrow: token compare
        // and a slot load, without Weak creation or descriptor/index access.
        if !accessor
            && let Some(entry) = state.entries.mono(&self.first)
            && entry.flags & 7 == DATA
            && entry.flags & OWN != 0
        {
            let Some(value) = target.load_matched_data(&entry.version, entry.slot) else {
                count_event(target, Event::Miss);
                return None;
            };
            #[cfg(debug_assertions)]
            self.verify_entry(&state, entry, target);
            count_event(target, Event::Hit);
            return Some(ReadOutcome::Value(value));
        }
        let Some(entry) = state.entries.find(target, &self.first) else {
            count_event(target, Event::Miss);
            return None;
        };
        #[cfg(debug_assertions)]
        self.verify_entry(&state, entry, target);
        if (entry.flags & 7 == ACCESSOR) != accessor {
            return None;
        }
        count_event(target, Event::Hit);
        entry.fetch(target)
    }
    #[cfg(debug_assertions)]
    #[cold]
    #[inline(never)]
    fn verify_entry(&self, state: &State, entry: &Entry, target: NamedTarget<'_>) {
        if entry.flags & VERIFY == 0 {
            return;
        }
        let name = state.name.as_deref().expect("verified site name");
        let expected = entry.resolution();
        let generic = resolve(target, name, None);
        assert!(
            generic.as_ref().is_some_and(|g| g.probe == expected.probe
                && g.inherited == expected.inherited
                && g.first_holder == expected.first_holder
                && g.identity() == expected.identity()),
            "IC complete outcome differs from pure generic lookup: {name}"
        );
        if let Some(constant) = entry.chain.as_ref().and_then(|c| c.constant.as_ref()) {
            let actual = constant.fetch();
            let expected = generic.and_then(|g| g.fetch(target));
            match (actual, expected) {
                (Some(ReadOutcome::Value(a)), Some(ReadOutcome::Value(b))) => {
                    assert!(a.same_value(&b))
                }
                (Some(ReadOutcome::Accessor(a)), Some(ReadOutcome::Accessor(b))) => {
                    assert_eq!(a.getter(), b.getter());
                    assert_eq!(a.setter(), b.setter());
                }
                (None, _) => (), // Dead weak payload must miss, never resurrect.
                _ => panic!("prototype constant differs from live lookup: {name}"),
            }
        }
    }
    // Keep the data-only hit ABI small. The descriptor outcome belongs to the
    // cold accessor path and must not inflate every typed/compact read frame.
    #[inline(always)]
    fn data_value(&self, target: NamedTarget<'_>) -> Option<Value> {
        if let Some(entry) = self.hot_mono() {
            if entry.flags & (7 | OWN) == DATA | OWN {
                let value = target.load_matched_data(&entry.version, entry.slot);
                #[cfg(debug_assertions)]
                if value.is_some() {
                    self.verify_hot(entry, target);
                }
                count_event(
                    target,
                    if value.is_some() {
                        Event::Hit
                    } else {
                        Event::Miss
                    },
                );
                return value;
            }
            if entry.flags & 7 == DATA
                && entry.matches(target)
                && let Some(constant) = entry.chain.as_ref().and_then(|c| c.constant.as_ref())
            {
                let value = constant.data();
                #[cfg(debug_assertions)]
                if value.is_some() {
                    self.verify_hot(entry, target);
                }
                count_event(
                    target,
                    if value.is_some() {
                        Event::Hit
                    } else {
                        Event::Miss
                    },
                );
                return value;
            }
        }
        self.data_value_other(target)
    }
    #[cold]
    #[inline(never)]
    fn data_value_other(&self, target: NamedTarget<'_>) -> Option<Value> {
        let state = self.state.borrow();
        if let Some(entry) = state.entries.mono(&self.first)
            && entry.flags & (7 | OWN) == DATA | OWN
        {
            let value = target.load_matched_data(&entry.version, entry.slot);
            #[cfg(debug_assertions)]
            if value.is_some() {
                self.verify_entry(&state, entry, target);
            }
            count_event(
                target,
                if value.is_some() {
                    Event::Hit
                } else {
                    Event::Miss
                },
            );
            return value;
        }
        if let Some(entry) = state.entries.mono(&self.first)
            && entry.flags & 7 == DATA
            && entry.matches(target)
            && let Some(constant) = entry.chain.as_ref().and_then(|c| c.constant.as_ref())
        {
            let value = constant.data();
            #[cfg(debug_assertions)]
            if value.is_some() {
                self.verify_entry(&state, entry, target);
            }
            count_event(
                target,
                if value.is_some() {
                    Event::Hit
                } else {
                    Event::Miss
                },
            );
            return value;
        }
        if let Some(entry) = state.entries.mono(&self.first)
            && entry.flags & (7 | FIRST_HOLDER) == DATA | FIRST_HOLDER
            && entry.chain.as_ref().is_some_and(|c| c.validity.is_valid())
            && let Some(value) = target.first_prototype_data(&entry.version, entry.slot)
        {
            #[cfg(debug_assertions)]
            self.verify_entry(&state, entry, target);
            count_event(target, Event::Hit);
            return Some(value);
        }
        self.data_value_multiple(&state, target)
    }
    #[cold]
    #[inline(never)]
    fn data_value_multiple(&self, state: &State, target: NamedTarget<'_>) -> Option<Value> {
        if let Entries::SharedOwnPoly(entries) = &state.entries {
            let value = target
                .with_store(|store| {
                    // Independent pointer comparisons expose parallel loads and
                    // avoid a serial branch chain at a polymorphic own-data site.
                    let mask = u32::from(store.matches_shared_identity(&entries[0].version))
                        | (u32::from(store.matches_shared_identity(&entries[1].version)) << 1)
                        | (u32::from(store.matches_shared_identity(&entries[2].version)) << 2)
                        | (u32::from(store.matches_shared_identity(&entries[3].version)) << 3);
                    if mask == 0 {
                        return None;
                    }
                    let entry = &entries[mask.trailing_zeros() as usize];
                    #[cfg(debug_assertions)]
                    self.verify_entry(state, entry, target);
                    store.load_guarded_data(entry.slot as usize)
                })
                .flatten();
            count_event(
                target,
                if value.is_some() {
                    Event::Hit
                } else {
                    Event::Miss
                },
            );
            return value;
        }
        if let Entries::Poly(entries) = &state.entries {
            // Compare all own guards and load while the authority is borrowed
            // once. No Weak traffic, repeated RefCell borrow or ID probe.
            let own = target
                .with_store(|store| {
                    entries
                        .iter()
                        .find(|entry| {
                            entry.flags & (7 | OWN) == DATA | OWN
                                && store.matches_version(&entry.version)
                        })
                        .and_then(|entry| {
                            #[cfg(debug_assertions)]
                            self.verify_entry(state, entry, target);
                            store.load_guarded_data(entry.slot as usize)
                        })
                })
                .flatten();
            if own.is_some() {
                count_event(target, Event::Hit);
                return own;
            }
        }
        self.data_value_slow(state, target)
    }
    #[cold]
    #[inline(never)]
    fn data_value_slow(&self, state: &State, target: NamedTarget<'_>) -> Option<Value> {
        let Some(entry) = state.entries.find(target, &self.first) else {
            count_event(target, Event::Miss);
            return None;
        };
        #[cfg(debug_assertions)]
        self.verify_entry(state, entry, target);
        if let Some(constant) = entry.chain.as_ref().and_then(|c| c.constant.as_ref()) {
            return constant.data();
        }
        let probe = entry.probe();
        if matches!(probe, Probe::Accessor { .. }) {
            return None;
        }
        count_event(target, Event::Hit);
        if probe == Probe::Missing {
            return Some(Value::Undefined);
        }
        let load = |t: NamedTarget<'_>| match probe {
            Probe::Data { slot, .. } => t.load_data(slot),
            Probe::Virtual(hook) => t.virtual_value(hook),
            _ => None,
        };
        if let Some(chain) = &entry.chain {
            chain.holder.as_ref()?.upgrade()?.with_named_target(load)?
        } else {
            load(target)
        }
    }
    #[inline(always)]
    pub(super) fn get_value(&self, value: &Value, _env: Option<&CallEnv>) -> Option<Value> {
        if let Value::Object(object) = value {
            return self.data_value(NamedTarget::Object(object));
        }
        self.get_non_object(value)
    }
    // Retain the same guards/hits for every family without multiplying their
    // receiver-specific borrows through the ordinary-object dispatch loop.
    #[inline(never)]
    fn get_non_object(&self, value: &Value) -> Option<Value> {
        value.with_named_target(|t| self.data_value(t))?
    }
    #[inline(always)]
    pub(super) fn get(&self, object: &ObjectRef) -> Option<Value> {
        self.data_value(NamedTarget::Object(object))
    }
    /// A numeric consumer can read the guarded slot without constructing or
    /// cloning an owning Value. Value-type writes still check the live slot.
    #[inline(always)]
    pub(super) fn get_number(&self, object: &ObjectRef) -> Option<f64> {
        if let Some(entry) = self.hot_mono()
            && entry.flags & (7 | OWN) == DATA | OWN
        {
            let value = object.with_named(|store| {
                if !store.matches_version(&entry.version) {
                    return None;
                }
                store.load_guarded_number(entry.slot as usize)
            });
            #[cfg(debug_assertions)]
            if value.is_some() {
                self.verify_hot(entry, NamedTarget::Object(object));
            }
            count_event(
                NamedTarget::Object(object),
                if value.is_some() {
                    Event::Hit
                } else {
                    Event::Miss
                },
            );
            return value;
        }
        self.get_number_other(object)
    }
    #[cold]
    #[inline(never)]
    fn get_number_other(&self, object: &ObjectRef) -> Option<f64> {
        let state = self.state.borrow();
        let value = object.with_named(|store| {
            let entry = match &state.entries {
                Entries::MonoFirst | Entries::Mono(_)
                    if state.entries.mono(&self.first).is_some_and(|e| {
                        e.flags & (7 | OWN) == DATA | OWN && store.matches_version(&e.version)
                    }) =>
                {
                    state.entries.mono(&self.first).expect("mono guard")
                }
                Entries::SharedOwnPoly(ways) => {
                    let mask = u32::from(store.matches_shared_identity(&ways[0].version))
                        | (u32::from(store.matches_shared_identity(&ways[1].version)) << 1)
                        | (u32::from(store.matches_shared_identity(&ways[2].version)) << 2)
                        | (u32::from(store.matches_shared_identity(&ways[3].version)) << 3);
                    if mask == 0 {
                        return None;
                    }
                    &ways[mask.trailing_zeros() as usize]
                }
                Entries::Poly(ways) => ways.iter().find(|e| {
                    e.flags & (7 | OWN) == DATA | OWN && store.matches_version(&e.version)
                })?,
                _ => return None,
            };
            let number = store.load_guarded_number(entry.slot as usize)?;
            #[cfg(debug_assertions)]
            self.verify_entry(&state, entry, NamedTarget::Object(object));
            Some(number)
        });
        count_event(
            NamedTarget::Object(object),
            if value.is_some() {
                Event::Hit
            } else {
                Event::Miss
            },
        );
        value
    }
    pub(super) fn accessor(&self, value: &Value) -> Option<Property> {
        value.with_named_target(|t| match self.cached(t, true) {
            Some(ReadOutcome::Accessor(p)) => Some(p),
            _ => None,
        })?
    }
    #[inline(always)]
    pub(super) fn read_value(
        &self,
        value: &Value,
        key: &KeyLease,
        _env: Option<&CallEnv>,
    ) -> Option<Value> {
        self.get_value(value, _env)
            .or_else(|| self.read_key_slow(value, key))
    }
    #[cold]
    #[inline(never)]
    fn read_key_slow(&self, value: &Value, key: &KeyLease) -> Option<Value> {
        value.with_named_target(|t| {
            self.read_target(t, key.as_str(), Some(key))
                .and_then(ReadOutcome::value)
        })?
    }
    #[inline(always)]
    pub(super) fn read_string(&self, value: &Value, name: &str, _env: &CallEnv) -> Option<Value> {
        value.with_named_target(|t| {
            self.data_value(t)
                .or_else(|| self.read_string_slow(t, name))
        })?
    }
    #[cold]
    #[inline(never)]
    fn read_string_slow(&self, target: NamedTarget<'_>, name: &str) -> Option<Value> {
        self.read_target(target, name, None)
            .and_then(ReadOutcome::value)
    }
    pub(super) fn read_outcome(&self, value: &Value, name: &str) -> Option<ReadOutcome> {
        value.with_named_target(|t| self.read_target(t, name, None))?
    }
    /// Keep a prototype own-data hit independent of the miss handler frame.
    #[inline(always)]
    pub(super) fn read_object(&self, object: &ObjectRef, name: &str) -> Option<Value> {
        self.get(object)
            .or_else(|| self.read_object_slow(object, name))
    }
    #[cold]
    #[inline(never)]
    fn read_object_slow(&self, object: &ObjectRef, name: &str) -> Option<Value> {
        self.read_target(NamedTarget::Object(object), name, None)
            .and_then(ReadOutcome::value)
    }
    #[cfg(test)]
    pub(super) fn clear(&self) {
        let mut state = self.state.borrow_mut();
        self.first_active.set(false);
        state.entries = Entries::Empty;
        state.key = None;
        state.add = None;
    }
}
/// One pure chain walk on a miss. No cell construction/registration, getters,
/// coercion, lazy installation, storage allocation or interning is permitted.
#[cold]
#[inline(never)]
fn resolve(target: NamedTarget<'_>, name: &str, key: Option<&KeyLease>) -> Option<Resolution> {
    count_event(target, Event::Probe);
    if key.is_none() {
        count_event(target, Event::Hash)
    }
    let probe = target.probe(name, key);
    if probe == Probe::Exotic {
        return None;
    }
    if probe != Probe::Missing {
        return Some(Resolution {
            probe,
            holder: None,
            inherited: false,
            first_holder: false,
        });
    }
    let Some(mut proto) = target.prototype() else {
        return Some(Resolution {
            probe: Probe::Missing,
            holder: None,
            inherited: false,
            first_holder: false,
        });
    };
    for depth in 0..64 {
        let value = proto.to_value();
        let (probe, holder, next) = value.with_named_target(|t| {
            count_event(t, Event::Probe);
            (t.probe(name, key), t.downgrade(), t.prototype())
        })?;
        if probe == Probe::Exotic {
            return None;
        }
        if probe != Probe::Missing {
            return Some(Resolution {
                probe,
                holder: Some(holder),
                inherited: true,
                first_holder: depth == 0,
            });
        }
        let Some(next) = next else {
            return Some(Resolution {
                probe: Probe::Missing,
                holder: None,
                inherited: true,
                first_holder: false,
            });
        };
        proto = next;
    }
    None
}
#[derive(Clone, Copy)]
enum Event {
    Hit,
    Miss,
    Probe,
    Hash,
}
#[inline(always)]
fn count_event(target: NamedTarget<'_>, event: Event) {
    #[cfg(feature = "perf-counters")]
    crate::diagnostics::update(|c| {
        let kind = target.kind().unwrap_or(9);
        let count = match (kind, event) {
            (0, Event::Hit) => &mut c.ic_object_hits,
            (0, Event::Miss) => &mut c.ic_object_misses,
            (0, Event::Probe) => &mut c.ic_object_probes,
            (0, Event::Hash) => &mut c.ic_object_hashes,
            (1, Event::Hit) => &mut c.ic_typed_array_hits,
            (1, Event::Miss) => &mut c.ic_typed_array_misses,
            (1, Event::Probe) => &mut c.ic_typed_array_probes,
            (1, Event::Hash) => &mut c.ic_typed_array_hashes,
            (2, Event::Hit) => &mut c.ic_array_prototype_hits,
            (2, Event::Miss) => &mut c.ic_array_prototype_misses,
            (2, Event::Probe) => &mut c.ic_array_prototype_probes,
            (2, Event::Hash) => &mut c.ic_array_prototype_hashes,
            (3, Event::Hit) => &mut c.ic_string_wrapper_hits,
            (3, Event::Miss) => &mut c.ic_string_wrapper_misses,
            (3, Event::Probe) => &mut c.ic_string_wrapper_probes,
            (3, Event::Hash) => &mut c.ic_string_wrapper_hashes,
            (4, Event::Hit) => &mut c.ic_array_hits,
            (4, Event::Miss) => &mut c.ic_array_misses,
            (4, Event::Probe) => &mut c.ic_array_probes,
            (4, Event::Hash) => &mut c.ic_array_hashes,
            (5 | 6, Event::Hit) => &mut c.ic_function_hits,
            (5 | 6, Event::Miss) => &mut c.ic_function_misses,
            (5 | 6, Event::Probe) => &mut c.ic_function_probes,
            (5 | 6, Event::Hash) => &mut c.ic_function_hashes,
            (7, Event::Hit) => &mut c.ic_map_hits,
            (7, Event::Miss) => &mut c.ic_map_misses,
            (7, Event::Probe) => &mut c.ic_map_probes,
            (7, Event::Hash) => &mut c.ic_map_hashes,
            (8, Event::Hit) => &mut c.ic_set_hits,
            (8, Event::Miss) => &mut c.ic_set_misses,
            (8, Event::Probe) => &mut c.ic_set_probes,
            (8, Event::Hash) => &mut c.ic_set_hashes,
            (_, Event::Hit) => &mut c.ic_exotic_hits,
            (_, Event::Miss) => &mut c.ic_exotic_misses,
            (_, Event::Probe) => &mut c.ic_exotic_probes,
            (_, Event::Hash) => &mut c.ic_exotic_hashes,
        };
        *count += 1;
    });
    #[cfg(not(feature = "perf-counters"))]
    let _ = (target, event);
}
