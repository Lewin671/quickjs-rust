//! Borrowed receiver facade for shape ICs. Exotic keys are admitted only by
//! their semantic hooks; all admitted reads/writes use NamedStore live slots.
use super::layout::{LayoutVersion, NamedStore, StructuralDependency};
use super::{ArrayWeakRef, ObjectWeakRef, OwnDataPropertyRead, OwnDataPropertyWrite};
use crate::property::atom::KeyLease;
use crate::{ArrayRef, CallEnv, Function, ObjectRef, Prototype, Value};

#[derive(Clone, Copy)]
pub(crate) enum NamedTarget<'a> {
    Object(&'a ObjectRef),
    Map(&'a ObjectRef),
    Set(&'a ObjectRef),
    Array(&'a ArrayRef),
    Function(&'a Function),
}
#[derive(Clone, Debug)]
pub(crate) enum WeakTarget {
    Object(ObjectWeakRef),
    Array(ArrayWeakRef),
    Function(crate::function::FunctionWeakRef),
}
impl WeakTarget {
    pub(crate) fn upgrade(&self) -> Option<Value> {
        match self {
            Self::Object(w) => w.upgrade().map(Value::Object),
            Self::Array(w) => w.upgrade().map(Value::Array),
            Self::Function(w) => w.upgrade().map(Value::Function),
        }
    }
}
impl Value {
    pub(crate) fn with_named_target<R>(
        &self,
        read: impl FnOnce(NamedTarget<'_>) -> R,
    ) -> Option<R> {
        Some(match self {
            Self::Object(o) => read(NamedTarget::Object(o)),
            Self::Array(a) => read(NamedTarget::Array(a)),
            Self::Function(f) => read(NamedTarget::Function(f)),
            Self::Map(m) => read(NamedTarget::Map(m.object_ref())),
            Self::Set(s) => read(NamedTarget::Set(s.object_ref())),
            _ => return None,
        })
    }
}
impl NamedTarget<'_> {
    /// Receiver kinds prevent hooks with different semantics sharing a proof.
    pub(crate) fn kind(self) -> Option<u8> {
        Some(match self {
            Self::Object(o) => return o.named_kind(),
            Self::Map(_) => 7,
            Self::Set(_) => 8,
            Self::Array(_) => 4,
            Self::Function(f) => f.named_kind(),
        })
    }
    pub(crate) fn with_store<R>(self, read: impl FnOnce(&NamedStore) -> R) -> Option<R> {
        match self {
            Self::Object(o) | Self::Map(o) | Self::Set(o) => Some(o.with_named(read)),
            Self::Array(a) => Some(a.with_named(read)),
            Self::Function(f) => f.with_named(|s| s.map(read)),
        }
    }
    pub(crate) fn version(self) -> Option<LayoutVersion> {
        self.with_store(NamedStore::version)
    }
    pub(crate) fn matches_version(self, version: &LayoutVersion) -> bool {
        self.with_store(|s| s.matches_version(version))
            .unwrap_or(false)
    }
    #[inline(always)]
    pub(crate) fn load(self, kind: u8, version: &LayoutVersion, slot: u32) -> Option<Value> {
        if self.kind()? != kind {
            return None;
        }
        self.with_store(|s| {
            s.matches_version(version)
                .then(|| s.load_data(slot as usize))
                .flatten()
        })?
    }
    pub(crate) fn write(
        self,
        kind: u8,
        version: &LayoutVersion,
        slot: u32,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        if self.kind()? != kind {
            return None;
        }
        // Array length/index, typed numeric indices, String virtual indices,
        // namespaces and primitive symbols are never admitted by read().
        let replace = |s: &mut NamedStore| {
            if s.slot_name_is(slot as usize, "globalThis") {
                return None;
            }
            if !s.matches_version(version) {
                return None;
            }
            s.replace_data(slot as usize, value.clone()).map(|written| {
                if written {
                    OwnDataPropertyWrite::Written
                } else {
                    OwnDataPropertyWrite::ReadOnly
                }
            })
        };
        match self {
            Self::Object(o) | Self::Map(o) | Self::Set(o) => o.with_named_mut(replace),
            Self::Array(a) => a.with_named_mut(replace),
            Self::Function(f) => f.with_named_mut(|s| {
                if s.slot_name_is(slot as usize, "__quickjsRustDynamicFunctionRealm") {
                    return None;
                }
                replace(s)
            }),
        }
    }
    pub(crate) fn read(
        self,
        key: &KeyLease,
    ) -> (OwnDataPropertyRead, Option<(LayoutVersion, u32)>) {
        let name = key.as_str();
        let slow = || (OwnDataPropertyRead::NeedsSlowPath, None);
        if self.kind().is_none() {
            return slow();
        }
        match self {
            Self::Object(o)
                if (o.is_typed_array_exotic()
                    && crate::typed_array::canonical_numeric_index(name).is_some())
                    || ((o.is_array_prototype_exotic() || o.named_kind() == Some(3))
                        && (name == "length"
                            || crate::array_index_property_key(name).is_some())) =>
            {
                return slow();
            }
            Self::Array(_)
                if name == "length" || crate::array_index_property_key(name).is_some() =>
            {
                return slow();
            }
            Self::Function(f) => {
                if name == "caller" || name == "arguments" {
                    return slow();
                }
                if let Some(read) = f.implicit_named_read(name) {
                    return (read, None);
                }
            }
            _ => {}
        }
        self.with_store(|s| {
            let normalized;
            let key = if key.belongs_to(s.domain()) {
                key
            } else {
                let Some(k) = s.domain().find(name) else {
                    return (OwnDataPropertyRead::Missing, None);
                };
                normalized = k;
                &normalized
            };
            let (read, slot) = s.read_data_slot(key);
            (read, slot.map(|slot| (s.version(), slot as u32)))
        })
        .unwrap_or((OwnDataPropertyRead::Missing, None))
    }
    pub(crate) fn prototype(self, env: Option<&CallEnv>) -> Option<Prototype> {
        match self {
            Self::Object(o) | Self::Map(o) | Self::Set(o) => o.prototype_slot(),
            Self::Array(a) => a
                .prototype_slot_override()
                .unwrap_or_else(|| env.and_then(crate::array_prototype).map(Prototype::Object)),
            Self::Function(f) => match env {
                Some(env) => crate::value_prototype_slot(Value::Function(f.clone()), env),
                None => f.effective_internal_prototype(),
            },
        }
    }
    pub(crate) fn downgrade(self) -> WeakTarget {
        match self {
            Self::Object(o) | Self::Map(o) | Self::Set(o) => WeakTarget::Object(o.downgrade()),
            Self::Array(a) => WeakTarget::Array(a.downgrade()),
            Self::Function(f) => WeakTarget::Function(f.downgrade()),
        }
    }
    pub(crate) fn subscribe(self, dependency: &StructuralDependency) -> bool {
        self.with_store(|s| s.subscribe(dependency)).is_some()
    }
}

#[derive(Clone, Debug)]
pub(crate) enum WeakEdge {
    Object(ObjectWeakRef),
    Array(super::object::ArrayPrototypeWeakRef),
    Function(crate::function::FunctionWeakRef),
}
impl WeakEdge {
    pub(crate) fn new(p: &Prototype) -> Option<Self> {
        Some(match p {
            Prototype::Object(o) => Self::Object(o.downgrade()),
            Prototype::Array(a) => Self::Array(a.downgrade()),
            Prototype::Function(f) => Self::Function(f.downgrade()),
            Prototype::Proxy(_) => return None,
        })
    }
    pub(crate) fn matches(&self, p: &Prototype) -> bool {
        match (self, p) {
            (Self::Object(w), Prototype::Object(o)) => w.ptr_eq(o),
            (Self::Array(w), Prototype::Array(a)) => w.ptr_eq(a),
            (Self::Function(w), Prototype::Function(f)) => w.ptr_eq(f),
            _ => false,
        }
    }
    pub(crate) fn same(&self, other: &Self) -> bool {
        // Weak handles keep allocation identities from being reused.
        match (self, other) {
            (Self::Object(a), Self::Object(b)) => a.same(b),
            (Self::Array(a), Self::Array(b)) => a.same(b),
            (Self::Function(a), Self::Function(b)) => a.same(b),
            _ => false,
        }
    }
}
