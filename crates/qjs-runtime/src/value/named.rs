//! Pure named lookup outcomes shared by every IC consumer. Probing never
//! installs lazy properties, allocates payloads, interns keys, or runs JS.
use super::layout::{LayoutVersion, NamedStore, StructuralDependency};
use super::{ArrayWeakRef, ObjectWeakRef, OwnDataPropertyWrite};
use crate::property::atom::{IcPolicy, KeyLease};
use crate::{ArrayRef, Function, ObjectRef, Property, Prototype, Value};

#[derive(Clone, Copy)]
pub(crate) enum NamedTarget<'a> {
    Object(&'a ObjectRef),
    Array(&'a ArrayRef),
    Function(&'a Function),
    Map(&'a super::MapRef),
    Set(&'a super::SetRef),
}
#[derive(Clone, Debug)]
pub(crate) enum WeakTarget {
    Object(ObjectWeakRef),
    Array(ArrayWeakRef),
    Function(crate::function::FunctionWeakRef),
    Map(super::map::MapWeakRef),
    Set(super::set::SetWeakRef),
}
impl WeakTarget {
    pub(crate) fn upgrade(&self) -> Option<Value> {
        match self {
            Self::Object(w) => w.upgrade().map(Value::Object),
            Self::Array(w) => w.upgrade().map(Value::Array),
            Self::Function(w) => w.upgrade().map(Value::Function),
            Self::Map(w) => w.upgrade().map(Value::Map),
            Self::Set(w) => w.upgrade().map(Value::Set),
        }
    }
    pub(crate) fn identity(&self) -> (u8, usize) {
        match self {
            Self::Object(w) => (0, w.address()),
            Self::Array(w) => (4, w.address()),
            Self::Function(w) => (5, w.address()),
            Self::Map(w) => (7, w.address()),
            Self::Set(w) => (8, w.address()),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Virtual {
    ArrayLength,
    FunctionName,
    FunctionLength,
    FunctionPrototype,
    FunctionCaller,
    FunctionArguments,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Probe {
    Data { slot: u32, writable: bool },
    Accessor { slot: u32 },
    Virtual(Virtual),
    Missing,
    Exotic,
}
impl Value {
    pub(crate) fn with_named_target<R>(&self, f: impl FnOnce(NamedTarget<'_>) -> R) -> Option<R> {
        Some(match self {
            Self::Object(o) => f(NamedTarget::Object(o)),
            Self::Array(a) => f(NamedTarget::Array(a)),
            Self::Function(fun) => f(NamedTarget::Function(fun)),
            Self::Map(m) => f(NamedTarget::Map(m)),
            Self::Set(s) => f(NamedTarget::Set(s)),
            _ => return None,
        })
    }
}
impl<'a> NamedTarget<'a> {
    fn object(self) -> Option<&'a ObjectRef> {
        match self {
            Self::Object(o) => Some(o),
            Self::Map(m) => Some(m.object_ref()),
            Self::Set(s) => Some(s.object_ref()),
            _ => None,
        }
    }
    pub(crate) fn kind(self) -> Option<u8> {
        Some(match self {
            Self::Object(o) => return o.named_kind(),
            Self::Array(_) => 4,
            Self::Function(_) => 5,
            Self::Map(_) => 7,
            Self::Set(_) => 8,
        })
    }
    #[inline(always)]
    pub(crate) fn with_store<R>(self, f: impl FnOnce(&NamedStore) -> R) -> Option<R> {
        if let Some(o) = self.object() {
            return Some(o.with_named(f));
        }
        match self {
            Self::Array(a) => a.with_named(|s| s.map(f)),
            Self::Function(fun) => fun.with_named(|s| s.map(f)),
            _ => unreachable!(),
        }
    }
    pub(crate) fn policy(self) -> IcPolicy {
        if let Some(o) = self.object() {
            return o.with_named(|s| s.domain().ic_policy());
        }
        match self {
            Self::Array(a) => a.named_policy(),
            Self::Function(f) => f.property_domain().ic_policy(),
            _ => unreachable!(),
        }
    }
    pub(crate) fn version(self) -> Option<LayoutVersion> {
        match self {
            Self::Array(a) => Some(a.named_version()),
            Self::Function(f) => f.named_version(),
            _ => self.with_store(NamedStore::version),
        }
    }
    pub(crate) fn matches_version(self, v: &LayoutVersion) -> bool {
        match self {
            Self::Array(a) => a.named_matches(v),
            Self::Function(f) => f.named_matches(v),
            _ => self.with_store(|s| s.matches_version(v)).unwrap_or(false),
        }
    }
    pub(crate) fn chain_dependency(self) -> Option<StructuralDependency> {
        match self {
            Self::Array(a) => a.chain_dependency(),
            Self::Function(f) => f.chain_dependency(),
            _ => self.with_store(NamedStore::chain_dependency).flatten(),
        }
    }
    pub(crate) fn prototype(self) -> Option<Prototype> {
        match self {
            Self::Array(a) => a.prototype_slot(),
            Self::Function(f) => f.effective_internal_prototype(),
            _ => self.object().and_then(ObjectRef::prototype_slot),
        }
    }
    pub(crate) fn downgrade(self) -> WeakTarget {
        match self {
            Self::Object(o) => WeakTarget::Object(o.downgrade()),
            Self::Array(a) => WeakTarget::Array(a.downgrade()),
            Self::Function(f) => WeakTarget::Function(f.downgrade()),
            Self::Map(m) => WeakTarget::Map(m.downgrade()),
            Self::Set(s) => WeakTarget::Set(s.downgrade()),
        }
    }
    pub(crate) fn probe(self, name: &str, key: Option<&KeyLease>) -> Probe {
        if self.kind().is_none() {
            return Probe::Exotic;
        }
        match self {
            Self::Object(o)
                if (o.is_typed_array_exotic()
                    && crate::typed_array::canonical_numeric_index(name).is_some())
                    || ((o.is_array_prototype_exotic() || o.named_kind() == Some(3))
                        && (name == "length"
                            || crate::array_index_property_key(name).is_some())) =>
            {
                return Probe::Exotic;
            }
            Self::Array(_) if name == "length" => return Probe::Virtual(Virtual::ArrayLength),
            Self::Array(_) if crate::array_index_property_key(name).is_some() => {
                return Probe::Exotic;
            }
            Self::Function(f) => {
                if let Some(hook) = f.virtual_named_probe(name) {
                    return Probe::Virtual(hook);
                }
            }
            _ => {}
        }
        let probe = self
            .with_store(|s| {
                let normalized;
                let key = match key {
                    Some(k) if k.belongs_to(s.domain()) => k,
                    _ => {
                        let Some(k) = s.domain().find(name) else {
                            return Probe::Missing;
                        };
                        normalized = k;
                        &normalized
                    }
                };
                s.probe_slot(key)
            })
            .unwrap_or(Probe::Missing);
        if probe == Probe::Missing {
            if let Self::Function(f) = self {
                if f.has_compat_caller() {
                    if name == "caller" {
                        return Probe::Virtual(Virtual::FunctionCaller);
                    }
                    if name == "arguments" {
                        return Probe::Virtual(Virtual::FunctionArguments);
                    }
                }
            }
        }
        probe
    }
    /// Called only after shape or prototype-cell validation proves this kind.
    #[inline(always)]
    pub(crate) fn load_matched_data(self, v: &LayoutVersion, slot: u32) -> Option<Value> {
        self.with_store(|s| {
            s.matches_version(v)
                .then(|| s.load_guarded_data(slot as usize))
                .flatten()
        })?
    }
    pub(crate) fn write_matched_data(
        self,
        v: &LayoutVersion,
        slot: u32,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        let write = |s: &mut NamedStore| {
            if s.matches_version(v) && s.overwrite_guarded_data(slot as usize, value.clone()) {
                Some(OwnDataPropertyWrite::Written)
            } else {
                None
            }
        };
        if let Some(o) = self.object() {
            return o.with_named_mut(write);
        }
        match self {
            Self::Array(a) => a.with_named_mut(write),
            Self::Function(f) => f.with_named_mut(write),
            _ => None,
        }
    }
    pub(crate) fn load_data(self, slot: u32) -> Option<Value> {
        self.with_store(|s| s.load_guarded_data(slot as usize))?
    }
    pub(crate) fn load_accessor(self, slot: u32) -> Option<Property> {
        self.with_store(|s| s.accessor_at(slot as usize))?
    }
    pub(crate) fn virtual_value(self, hook: Virtual) -> Option<Value> {
        match (self, hook) {
            (Self::Array(a), Virtual::ArrayLength) => Some(Value::Number(a.len() as f64)),
            (Self::Function(f), _) => f.virtual_named_value(hook),
            _ => None,
        }
    }
}
