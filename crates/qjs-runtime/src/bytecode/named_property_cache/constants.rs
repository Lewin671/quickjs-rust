//! Prototype-only constants. Chain cells protect values; RC payloads are weak
//! so a cache cannot keep its holder, method, accessor closure or realm alive.
use super::*;
use crate::value::{WeakJsString, layout::NamedStore};
use std::rc::Weak;

#[derive(Clone, Debug)]
pub(super) enum ConstantValue {
    Number(f64),
    Boolean(bool),
    Null,
    Undefined,
    String(WeakJsString),
    BigInt(Weak<num_bigint::BigInt>),
    Target(WeakTarget),
    Proxy(crate::proxy::ProxyWeakRef),
}
impl ConstantValue {
    fn new(value: &Value) -> Self {
        match value {
            Value::Number(n) => Self::Number(*n),
            Value::Boolean(b) => Self::Boolean(*b),
            Value::Null => Self::Null,
            Value::Undefined => Self::Undefined,
            Value::String(s) => Self::String(s.downgrade()),
            Value::BigInt(n) => Self::BigInt(Rc::downgrade(n)),
            Value::Proxy(p) => Self::Proxy(p.downgrade()),
            _ => Self::Target(
                value
                    .with_named_target(|t| t.downgrade())
                    .expect("named value"),
            ),
        }
    }
    #[inline]
    fn load(&self) -> Option<Value> {
        Some(match self {
            Self::Number(n) => Value::Number(*n),
            Self::Boolean(b) => Value::Boolean(*b),
            Self::Null => Value::Null,
            Self::Undefined => Value::Undefined,
            Self::String(s) => Value::String(s.upgrade()?),
            Self::BigInt(n) => Value::BigInt(n.upgrade()?),
            Self::Proxy(p) => Value::Proxy(p.upgrade()?),
            Self::Target(t) => return t.upgrade(),
        })
    }
}
#[derive(Clone, Debug)]
pub(super) enum Constant {
    Data(ConstantValue),
    Accessor {
        get: Option<ConstantValue>,
        set: Option<ConstantValue>,
        enumerable: bool,
        configurable: bool,
    },
}
impl Constant {
    pub(super) fn admit(holder: &WeakTarget, probe: Probe) -> Option<Self> {
        holder.upgrade()?.with_named_target(|target| {
            if !target.with_store(NamedStore::prototype_values_stable)? {
                return None;
            }
            Some(match fetch_probe(target, probe)? {
                ReadOutcome::Value(value) if matches!(probe, Probe::Data { .. }) => {
                    Self::Data(ConstantValue::new(&value))
                }
                ReadOutcome::Accessor(property) => Self::Accessor {
                    get: property.getter().map(ConstantValue::new),
                    set: property.setter().map(ConstantValue::new),
                    enumerable: property.enumerable,
                    configurable: property.configurable,
                },
                _ => return None,
            })
        })?
    }
    #[inline]
    pub(super) fn data(&self) -> Option<Value> {
        match self {
            Self::Data(value) => value.load(),
            _ => None,
        }
    }
    pub(super) fn fetch(&self) -> Option<ReadOutcome> {
        Some(match self {
            Self::Data(value) => ReadOutcome::Value(value.load()?),
            Self::Accessor {
                get,
                set,
                enumerable,
                configurable,
            } => {
                let load = |v: &Option<ConstantValue>| -> Option<Option<Value>> {
                    Some(match v {
                        Some(v) => Some(v.load()?),
                        None => None,
                    })
                };
                ReadOutcome::Accessor(Property::accessor(
                    load(get)?,
                    load(set)?,
                    *enumerable,
                    *configurable,
                ))
            }
        })
    }
}
