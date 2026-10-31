//! One exclusive key lookup for ordinary data writes and dynamic additions.
//! No storage borrow crosses a callback; prototype exceptions decline before effects.
use super::*;
use crate::property::atom::KeyLease;

/// Outcome and cache metadata from the same exclusive lookup. No miss proof
/// survives this operation or crosses an observable callback.
pub(crate) struct OrdinaryDataWrite {
    pub(crate) result: OwnDataPropertyWrite,
    pub(crate) cache_slot: Option<usize>,
    pub(crate) created: bool,
}
impl ObjectRef {
    pub(crate) fn write_or_create_data_key(
        &self,
        key: &KeyLease,
        value: &Value,
        chain_proven: bool,
    ) -> OrdinaryDataWrite {
        self.write_or_create_data_key_impl(key, value, chain_proven, false)
    }
    pub(crate) fn write_or_create_computed_data_key(
        &self,
        key: &KeyLease,
        value: &Value,
    ) -> OrdinaryDataWrite {
        self.write_or_create_data_key_impl(key, value, false, true)
    }
    fn write_or_create_data_key_impl(
        &self,
        key: &KeyLease,
        value: &Value,
        chain_proven: bool,
        dynamic: bool,
    ) -> OrdinaryDataWrite {
        let decline = || OrdinaryDataWrite {
            result: OwnDataPropertyWrite::NeedsSlowPath,
            cache_slot: None,
            created: false,
        };
        if self.is_symbol_primitive()
            || self.is_typed_array_exotic()
            || self.is_module_namespace_exotic()
            || self.is_array_prototype_exotic()
        {
            return decline();
        }
        if self.0.flag(super::flags::STRING_INDICES)
            && self.virtual_index_property(key.as_str()).is_some()
        {
            return OrdinaryDataWrite {
                result: OwnDataPropertyWrite::ReadOnly,
                ..decline()
            };
        }
        let mut store = self.0.properties.borrow_mut();
        let normalized;
        let key = if key.belongs_to(store.domain()) {
            key
        } else {
            normalized = store
                .domain()
                .find(key.as_str())
                .unwrap_or_else(|| store.domain().intern(key.as_str()));
            &normalized
        };
        let append_slot = store.len();
        let lookup = store.lookup(key).expect("normalized receiver key");
        let lookup = match lookup.write_existing(value) {
            Ok((result, slot)) => {
                drop(store);
                if matches!(result, OwnDataPropertyWrite::Written)
                    && key.as_str() == "globalThis"
                    && matches!(value, Value::Object(global) if self.ptr_eq(global))
                {
                    self.capture_realm_intrinsic_identity();
                }
                return OrdinaryDataWrite {
                    cache_slot: matches!(result, OwnDataPropertyWrite::Written).then_some(slot),
                    result,
                    created: false,
                };
            }
            Err(lookup) => lookup,
        };
        if !chain_proven {
            let mut current = lookup.prototype();
            loop {
                match current {
                    None => break,
                    Some(Prototype::Object(parent)) => {
                        if parent.is_symbol_primitive()
                            || parent.is_typed_array_exotic()
                            || parent.is_module_namespace_exotic()
                            || parent.is_array_prototype_exotic()
                        {
                            return decline();
                        }
                        if let Some(p) = parent.own_property(key.as_str()) {
                            if p.is_accessor() || !p.writable {
                                return decline();
                            }
                            break;
                        }
                        current = parent.prototype_slot();
                    }
                    Some(_) => return decline(),
                }
            }
        }
        if !self.is_extensible() {
            return OrdinaryDataWrite {
                result: OwnDataPropertyWrite::ReadOnly,
                ..decline()
            };
        }
        let created = true;
        let slot = append_slot;
        if dynamic {
            lookup.define_dynamic(Property::enumerable(value.clone()));
        } else {
            lookup.define(Property::enumerable(value.clone()));
        }
        if created && key.array_index().is_some() {
            self.0
                .index_property_count
                .set(self.0.index_property_count.get() + 1);
        }
        drop(store);

        if key.as_str() == "globalThis"
            && matches!(value, Value::Object(global) if self.ptr_eq(global))
        {
            self.capture_realm_intrinsic_identity();
        }
        OrdinaryDataWrite {
            result: OwnDataPropertyWrite::Written,
            cache_slot: Some(slot),
            created,
        }
    }
}

impl ObjectRef {
    pub(crate) fn note_cached_append(&self, index: bool) {
        if index {
            self.0
                .index_property_count
                .set(self.0.index_property_count.get() + 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dynamic_new_branches_use_private_maps_but_repeated_and_static_paths_share() {
        let domain = crate::property::atom::AtomDomain::new();
        let _owner = super::super::super::layout::PropertyOwner::new(domain.clone());
        let mut live = Vec::new();
        for i in 0..9 {
            let object = ObjectRef::new(HashMap::new(), &domain);
            object.set(format!("k{i}"), Value::Null);
            live.push(object);
        }
        let object = ObjectRef::new(HashMap::new(), &domain);
        let key = domain.intern("new");
        assert!(matches!(
            object
                .write_or_create_computed_data_key(&key, &Value::Number(1.0))
                .result,
            OwnDataPropertyWrite::Written
        ));
        assert!(object.with_named(|store| store.is_dictionary()));
        let other = domain.intern("0");
        object.write_or_create_computed_data_key(&other, &Value::Null);
        object.write_or_create_computed_data_key(&key, &Value::Number(2.0));
        assert_eq!(object.own_property_names(), vec!["0", "new"]);
        assert_eq!(object.get("new"), Some(Value::Number(2.0)));
        let repeated = ObjectRef::new(HashMap::new(), &domain);
        let key = domain.intern("k0");
        repeated.write_or_create_computed_data_key(&key, &Value::Null);
        assert!(!repeated.with_named(|store| store.is_dictionary()));
        let named = ObjectRef::new(HashMap::new(), &domain);
        named.write_or_create_data_key(&domain.intern("stable"), &Value::Null, false);
        assert!(!named.with_named(|store| store.is_dictionary()));
    }
    #[test]
    fn computed_write_declines_accessors_and_inherited_readonly_before_effects() {
        let domain = crate::property::atom::AtomDomain::new();
        let parent = ObjectRef::new(HashMap::new(), &domain);
        parent.define_property("x".into(), Property::data(Value::Null, true, false, true));
        let object = ObjectRef::with_prototype(HashMap::new(), Some(parent), &domain);
        let key = domain.intern("x");
        assert!(matches!(
            object
                .write_or_create_computed_data_key(&key, &Value::Null)
                .result,
            OwnDataPropertyWrite::NeedsSlowPath
        ));
        assert!(!object.has_own_property("x"));
        object.define_property(
            "x".into(),
            Property::accessor(Some(Value::Undefined), None, true, true),
        );
        assert!(matches!(
            object
                .write_or_create_computed_data_key(&key, &Value::Null)
                .result,
            OwnDataPropertyWrite::NeedsSlowPath
        ));
        assert!(object.own_property("x").unwrap().is_accessor());
    }
}
