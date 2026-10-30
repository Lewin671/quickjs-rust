//! Constructor root links are cold metadata with weak prototype identities.
use super::*;
impl Function {
    pub(crate) fn new_instance_receiver(
        &self,
        prototype: Option<Prototype>,
        domain: &crate::property::atom::AtomDomain,
    ) -> ObjectRef {
        self.auxiliary.with_cold_mut(|cold| {
            ObjectRef::with_constructor_layout(&mut cold.receiver_layout, domain, prototype)
        })
    }
}

impl Function {
    /// Direct construction admits an ordinary object or function prototype.
    pub(crate) fn direct_instance_receiver(
        &self,
        domain: &crate::property::atom::AtomDomain,
    ) -> Option<ObjectRef> {
        self.linked_instance_receiver(domain, true)
    }
    /// Wide construction retains its ordinary-object prototype admission.
    pub(crate) fn wide_instance_receiver(
        &self,
        domain: &crate::property::atom::AtomDomain,
    ) -> Option<ObjectRef> {
        self.linked_instance_receiver(domain, false)
    }
    /// Own constructor data only. The caller retains full Get for lazy misses/accessors.
    #[inline(always)]
    pub(crate) fn constructor_prototype_data(&self) -> Option<Value> {
        let policy = self.property_domain().ic_policy();
        if policy.enabled
            && let Some(value) = self.auxiliary.with_cold(|cold| {
                let (version, slot) = cold?.constructor_prototype_slot.as_ref()?;
                self.with_named(|store| {
                    let store = store?;
                    store
                        .matches_version(version)
                        .then(|| store.load_guarded_data(*slot))
                        .flatten()
                })
            })
        {
            #[cfg(debug_assertions)]
            if policy.verify {
                let generic = self.own_property("prototype").expect("guarded own data");
                assert!(!generic.is_accessor() && generic.value.same_value(&value));
            }
            crate::diagnostics::count!(property_constructor_prototype_hits);
            return Some(value);
        }
        self.constructor_prototype_data_slow(policy.enabled)
    }
    #[cold]
    #[inline(never)]
    fn constructor_prototype_data_slow(&self, enabled: bool) -> Option<Value> {
        crate::diagnostics::count!(property_constructor_prototype_misses);
        let property = self.own_property("prototype")?;
        if property.is_accessor() {
            return None;
        }
        if enabled {
            let link = self.with_named(|store| {
                let store = store?;
                let key = store.domain().find("prototype")?;
                Some((store.version(), store.data_slot(&key).ok()??))
            });
            self.auxiliary
                .with_cold_mut(|cold| cold.constructor_prototype_slot = link);
        }
        Some(property.value)
    }
    fn linked_instance_receiver(
        &self,
        domain: &crate::property::atom::AtomDomain,
        allow_function: bool,
    ) -> Option<ObjectRef> {
        let policy = self.property_domain().ic_policy();
        if policy.enabled
            && let Some(receiver) = self.auxiliary.with_cold_mut(|cold| {
                let (version, slot) = cold.constructor_prototype_slot.as_ref()?;
                let value = self.with_named(|store| {
                    let store = store?;
                    store
                        .matches_version(version)
                        .then(|| store.load_guarded_data(*slot))
                        .flatten()
                })?;
                #[cfg(debug_assertions)]
                if policy.verify {
                    let generic = self
                        .own_property("prototype")
                        .expect("guarded own prototype");
                    assert!(!generic.is_accessor() && generic.value.same_value(&value));
                }
                let prototype = admitted_prototype(value, allow_function)?;
                crate::diagnostics::count!(property_constructor_prototype_hits);
                Some(ObjectRef::with_constructor_layout(
                    &mut cold.receiver_layout,
                    domain,
                    Some(prototype),
                ))
            })
        {
            return Some(receiver);
        }
        crate::diagnostics::count!(property_constructor_prototype_misses);
        // Lazy materialization/accessor decisions happen outside the cold borrow.
        let property = self.own_property("prototype")?;
        if property.is_accessor() {
            return None;
        }
        let prototype = admitted_prototype(property.value, allow_function)?;
        let slot = if policy.enabled {
            self.with_named(|store| {
                let store = store?;
                let key = store.domain().find("prototype")?;
                Some((store.version(), store.data_slot(&key).ok()??))
            })
        } else {
            None
        };
        self.auxiliary.with_cold_mut(|cold| {
            cold.constructor_prototype_slot = slot;
            Some(ObjectRef::with_constructor_layout(
                &mut cold.receiver_layout,
                domain,
                Some(prototype),
            ))
        })
    }
}
fn admitted_prototype(value: Value, allow_function: bool) -> Option<Prototype> {
    match value {
        Value::Object(object) if !object.is_symbol_primitive() => Some(Prototype::Object(object)),
        Value::Function(function) if allow_function => Some(Prototype::Function(function)),
        _ => None,
    }
}
