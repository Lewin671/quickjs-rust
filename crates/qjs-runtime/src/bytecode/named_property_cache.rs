//! Per-site shape ICs. Entries contain weak structural guards and live slots,
//! never a receiver or property value. Inherited proofs subscribe every chain
//! node and explicitly guard the current receiver's first prototype edge.
use crate::property::atom::KeyLease;
use crate::value::layout::{LayoutVersion, StructuralDependency};
use crate::value::named::{NamedTarget, WeakEdge, WeakTarget};
use crate::value::{OwnDataPropertyRead, OwnDataPropertyWrite};
use crate::{CallEnv, ObjectRef, Prototype, Value};
use std::{cell::RefCell, rc::Rc};

#[cfg(test)]
mod tests;

#[derive(Clone, Debug)]
pub(super) struct NamedPropertyCache(Rc<RefCell<State>>);
#[derive(Debug)]
struct State {
    entries: Entries,
    local_slot: Option<usize>,
    enabled: bool,
    #[cfg(debug_assertions)]
    verify: bool,
    #[cfg(debug_assertions)]
    name: Option<String>,
}
#[derive(Debug, Default)]
enum Entries {
    #[default]
    Empty,
    Mono(Entry),
    Poly(Box<[Option<Entry>; 4]>),
    Mega,
}
#[derive(Debug)]
struct Entry {
    version: LayoutVersion,
    slot: u32,
    kind: u8,
    chain: Option<Box<Chain>>,
}
#[derive(Debug)]
struct Chain {
    first: WeakEdge,
    holder: WeakTarget,
    holder_version: LayoutVersion,
    holder_kind: u8,
    validity: StructuralDependency,
}
#[cfg(target_pointer_width = "64")]
const _: () = assert!(std::mem::size_of::<Entry>() == 32);

impl Default for NamedPropertyCache {
    fn default() -> Self {
        Self(Rc::new(RefCell::new(State {
            entries: Entries::Empty,
            local_slot: None,
            enabled: std::env::var_os("QJS_DISABLE_IC").is_none(),
            #[cfg(debug_assertions)]
            verify: std::env::var_os("QJS_VERIFY_IC").is_some(),
            #[cfg(debug_assertions)]
            name: None,
        })))
    }
}
impl Entry {
    #[inline(always)]
    fn read(&self, target: NamedTarget<'_>, env: Option<&CallEnv>) -> Option<Value> {
        if let Some(chain) = &self.chain {
            if target.kind()? != self.kind
                || !target.matches_version(&self.version)
                || !chain.validity.is_valid()
                || !chain.first.matches(&target.prototype(env)?)
            {
                return None;
            }
            chain.holder.upgrade()?.with_named_target(|holder| {
                holder.load(chain.holder_kind, &chain.holder_version, self.slot)
            })?
        } else {
            target.load(self.kind, &self.version, self.slot)
        }
    }
    fn same_guard(&self, other: &Self) -> bool {
        if self.kind != other.kind || self.version != other.version {
            return false;
        }
        match (&self.chain, &other.chain) {
            (None, None) => true,
            (Some(a), Some(b)) => a.first.same(&b.first),
            _ => false,
        }
    }
    fn stale(&self) -> bool {
        self.chain.as_ref().is_some_and(|c| !c.validity.is_valid())
    }
}
impl Entries {
    fn each(&self, mut f: impl FnMut(&Entry) -> Option<Value>) -> Option<Value> {
        match self {
            Self::Mono(e) => f(e),
            Self::Poly(entries) => entries.iter().flatten().find_map(f),
            _ => None,
        }
    }
    fn record(&mut self, entry: Entry) {
        match self {
            Self::Empty => *self = Self::Mono(entry),
            Self::Mono(old) if old.same_guard(&entry) || old.stale() => *old = entry,
            Self::Mono(_) => {
                let Self::Mono(old) = std::mem::take(self) else {
                    unreachable!()
                };
                *self = Self::Poly(Box::new([Some(old), Some(entry), None, None]));
            }
            Self::Poly(entries) => {
                if let Some(slot) = entries.iter_mut().find(|e| {
                    e.as_ref()
                        .is_some_and(|e| e.same_guard(&entry) || e.stale())
                }) {
                    *slot = Some(entry);
                } else if let Some(slot) = entries.iter_mut().find(|e| e.is_none()) {
                    *slot = Some(entry);
                } else {
                    *self = Self::Mega;
                }
            }
            Self::Mega => {}
        }
    }
}
impl NamedPropertyCache {
    pub(super) fn for_local(slot: usize) -> Self {
        let cache = Self::default();
        cache.0.borrow_mut().local_slot = Some(slot);
        cache
    }
    pub(super) fn local_slot(&self) -> Option<usize> {
        self.0.borrow().local_slot
    }
    #[inline(always)]
    fn get_target(&self, target: NamedTarget<'_>, env: Option<&CallEnv>) -> Option<Value> {
        let state = self.0.borrow();
        if !state.enabled {
            return None;
        }
        let answer = state.entries.each(|entry| entry.read(target, env));
        count_event(
            target,
            if answer.is_some() {
                Event::Hit
            } else {
                Event::Miss
            },
        );
        let value = answer?;
        #[cfg(debug_assertions)]
        if state.verify {
            let key = target.with_store(|s| {
                s.domain()
                    .intern(state.name.as_deref().expect("verified name"))
            })?;
            let generic = resolve(target, &key, env, false)?.0;
            assert!(
                value.same_value(&generic),
                "IC result differs from uncached data lookup"
            );
        }
        Some(value)
    }
    #[inline(always)]
    pub(super) fn get_value(&self, receiver: &Value, env: Option<&CallEnv>) -> Option<Value> {
        receiver.with_named_target(|target| self.get_target(target, env))?
    }
    pub(super) fn get(&self, receiver: &ObjectRef) -> Option<Value> {
        self.get_target(NamedTarget::Object(receiver), None)
    }
    pub(super) fn read_value(
        &self,
        receiver: &Value,
        key: &KeyLease,
        env: Option<&CallEnv>,
    ) -> Option<Value> {
        receiver.with_named_target(|target| self.read_target(target, key, env))?
    }
    pub(super) fn read_string(&self, receiver: &Value, name: &str, env: &CallEnv) -> Option<Value> {
        if let Some(value) = self.get_value(receiver, Some(env)) {
            return Some(value);
        }
        receiver.with_named_target(|t| count_event(t, Event::Hash));
        let key = env
            .realm()
            .atoms()
            .find(name)
            .unwrap_or_else(|| env.realm().atoms().intern(name));
        self.read_value(receiver, &key, Some(env))
    }
    pub(super) fn read_object(&self, receiver: &ObjectRef, name: &str) -> Option<Value> {
        if let Some(value) = self.get(receiver) {
            return Some(value);
        }
        count_event(NamedTarget::Object(receiver), Event::Hash);
        let key = receiver.domain().intern(name);
        self.read_target(NamedTarget::Object(receiver), &key, None)
    }
    fn read_target(
        &self,
        target: NamedTarget<'_>,
        key: &KeyLease,
        env: Option<&CallEnv>,
    ) -> Option<Value> {
        if let Some(value) = self.get_target(target, env) {
            return Some(value);
        }
        let enabled = {
            let s = self.0.borrow();
            s.enabled && !matches!(s.entries, Entries::Mega)
        };
        let (value, entry) = resolve(target, key, env, enabled)?;
        if let Some(entry) = entry {
            self.record(entry, key.as_str());
        }
        Some(value)
    }
    fn record(&self, entry: Entry, name: &str) {
        let mut state = self.0.borrow_mut();
        if !state.enabled {
            return;
        }
        #[cfg(debug_assertions)]
        if state.verify {
            state.name = Some(name.to_owned());
        }
        #[cfg(not(debug_assertions))]
        let _ = name;
        state.entries.record(entry);
    }
    pub(super) fn update(&self, receiver: &ObjectRef, key: &str, _value: &Value) {
        let key = receiver.domain().intern(key);
        if let Some((_, Some(entry))) = resolve(NamedTarget::Object(receiver), &key, None, true) {
            self.record(entry, key.as_str());
        }
    }
    pub(super) fn record_data_slot(&self, receiver: &ObjectRef, key: &KeyLease, slot: usize) {
        let target = NamedTarget::Object(receiver);
        if let (Some(kind), Some(version)) = (target.kind(), target.version()) {
            self.record(
                Entry {
                    version,
                    slot: slot as u32,
                    kind,
                    chain: None,
                },
                key.as_str(),
            );
        }
    }
    pub(super) fn write_value(
        &self,
        receiver: &Value,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        receiver.with_named_target(|target| self.write_target(target, value))?
    }
    fn write_target(&self, target: NamedTarget<'_>, value: &Value) -> Option<OwnDataPropertyWrite> {
        let state = self.0.borrow();
        if !state.enabled {
            return None;
        }
        let write = |e: &Entry| {
            if e.chain.is_none() {
                target.write(e.kind, &e.version, e.slot, value)
            } else {
                None
            }
        };
        match &state.entries {
            Entries::Mono(e) => write(e),
            Entries::Poly(entries) => entries.iter().flatten().find_map(write),
            _ => None,
        }
    }
    pub(super) fn write(
        &self,
        receiver: &ObjectRef,
        _key: &str,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        self.write_target(NamedTarget::Object(receiver), value)
    }
    pub(super) fn write_hot(
        &self,
        receiver: &ObjectRef,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        self.write_target(NamedTarget::Object(receiver), value)
    }
    #[cfg(test)]
    pub(super) fn clear(&self) {
        self.0.borrow_mut().entries = Entries::Empty;
    }
}

/// Pure data lookup, releasing the store before advancing the prototype chain.
/// No getter, coercion or Proxy trap can be replayed by debug verification.
fn resolve(
    target: NamedTarget<'_>,
    key: &KeyLease,
    env: Option<&CallEnv>,
    cache: bool,
) -> Option<(Value, Option<Entry>)> {
    count_event(target, Event::Probe);
    let (read, slot) = target.read(key);
    match read {
        OwnDataPropertyRead::Data(value) => {
            let entry = if cache {
                slot.and_then(|(version, slot)| {
                    Some(Entry {
                        version,
                        slot,
                        kind: target.kind()?,
                        chain: None,
                    })
                })
            } else {
                None
            };
            return Some((value, entry));
        }
        OwnDataPropertyRead::NeedsSlowPath => return None,
        OwnDataPropertyRead::Missing => {}
    }
    let version = target.version()?;
    let kind = target.kind()?;
    let Some(first_proto) = target.prototype(env) else {
        return Some((Value::Undefined, None));
    };
    let first = WeakEdge::new(&first_proto)?;
    let mut current_proto = first_proto;
    let validity = StructuralDependency::new();
    for _ in 0..16 {
        let current = current_proto.to_value();
        let (read, slot, holder, holder_kind, next, subscribed) =
            current.with_named_target(|t| {
                count_event(t, Event::Probe);
                let (read, slot) = t.read(key);
                (
                    read,
                    slot,
                    t.downgrade(),
                    t.kind(),
                    t.prototype(env),
                    !cache || t.subscribe(&validity),
                )
            })?;
        if !subscribed {
            return None;
        }
        match read {
            OwnDataPropertyRead::Data(value) => {
                let entry = if cache {
                    slot.map(|(holder_version, slot)| Entry {
                        version,
                        slot,
                        kind,
                        chain: Some(Box::new(Chain {
                            first,
                            holder,
                            holder_version,
                            holder_kind: holder_kind.expect("admitted holder"),
                            validity,
                        })),
                    })
                } else {
                    None
                };
                return Some((value, entry));
            }
            OwnDataPropertyRead::NeedsSlowPath => return None,
            OwnDataPropertyRead::Missing => {}
        }
        let next = match &current_proto {
            Prototype::Array(a) => a.effective_prototype_slot(),
            _ => next,
        };
        let Some(next) = next else {
            return Some((Value::Undefined, None));
        };
        current_proto = next;
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
