use std::rc::Rc;

use crate::{
    Property, Value,
    function::{LinkedGlobalStore, Upvalue},
};

use super::{ObjectRef, OwnDataPropertyWrite};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum LinkedDefinition {
    NotLinked,
    ValueOnly,
    DescriptorChanged,
    Detached,
}

impl ObjectRef {
    pub(super) fn global_data_cell(&self, key: &str) -> Option<Upvalue> {
        if !self.0.has_global_data_links.get() {
            return None;
        }
        self.0
            .cold_if_present()?
            .global_data_links
            .borrow()
            .get(key)
            .cloned()
    }

    pub(crate) fn has_global_data_link(&self, key: &str) -> bool {
        self.global_data_cell(key)
            .is_some_and(|cell| cell.is_linked_global())
    }

    pub(crate) fn global_data_link_matches(&self, key: &str, expected: &Upvalue) -> bool {
        self.global_data_cell(key)
            .is_some_and(|cell| cell.ptr_eq(expected) && cell.is_linked_global())
    }

    /// Installs the one-to-one storage link for a fixed realm's ordinary,
    /// non-configurable own data property. The caller supplies the realm's
    /// canonical binding cell; value equality guards against accidentally
    /// linking a stale or unrelated cell.
    pub(crate) fn link_realm_global_data_property(&self, key: &str, cell: &Upvalue) -> bool {
        if self.0.module_namespace_exotic.get()
            || self.0.typed_array_exotic.get()
            || self.0.symbol_brand.get() != super::SymbolBrand::None
        {
            return false;
        }
        if let Some(existing) = self.global_data_cell(key) {
            return existing.ptr_eq(cell) && existing.is_linked_global();
        }

        let cell_value = cell.get();
        let writable = {
            let mut properties = self.0.properties.borrow_mut();
            let Some(property) = properties.get_mut(key) else {
                return false;
            };
            if property.is_accessor()
                || property.configurable
                || !property.enumerable
                || !property.value.same_value(&cell_value)
            {
                return false;
            }
            property.value = Value::Undefined;
            property.writable
        };
        if !cell.try_link_global_data(writable) {
            // Restore the descriptor payload when another owner already linked
            // or detached this cell. No partially installed object link remains.
            if let Some(property) = self.0.properties.borrow_mut().get_mut(key) {
                property.value = cell_value;
            }
            return false;
        }
        self.0
            .cold()
            .global_data_links
            .borrow_mut()
            .insert(Rc::from(key), cell.clone());
        self.0.has_global_data_links.set(true);
        // Existing exact named caches must miss once at installation. Later
        // linked value writes deliberately leave this revision unchanged.
        self.bump_property_revision();
        true
    }

    pub(super) fn hydrate_global_data_property(&self, key: &str, property: &mut Property) {
        if let Some(cell) = self.global_data_cell(key)
            && cell.is_linked_global()
        {
            property.value = cell.get();
        }
    }

    pub(super) fn write_linked_global_data_property(
        &self,
        key: &str,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        let cell = self.global_data_cell(key)?;
        Some(match cell.try_store_linked_global(value.clone()) {
            LinkedGlobalStore::Written => OwnDataPropertyWrite::Written,
            LinkedGlobalStore::ReadOnly => OwnDataPropertyWrite::ReadOnly,
            LinkedGlobalStore::NotLinked => OwnDataPropertyWrite::NeedsSlowPath,
        })
    }

    pub(super) fn linked_global_data_number(&self, key: &str) -> Option<Option<f64>> {
        let cell = self.global_data_cell(key)?;
        if !cell.is_linked_global_writable() {
            return Some(None);
        }
        Some(cell.with_value(|value| match value {
            Value::Number(value) => Some(*value),
            _ => None,
        }))
    }

    pub(super) fn append_linked_global_data_string(
        &self,
        key: &str,
        suffix: &str,
    ) -> Option<Option<Value>> {
        let cell = self.global_data_cell(key)?;
        Some(
            cell.with_value_mut(|value| {
                let Value::String(string) = value else {
                    return None;
                };
                Rc::make_mut(string).push_str(suffix);
                Some(Value::String(string.clone()))
            })
            .flatten(),
        )
    }

    /// Keeps a linked cell authoritative across a complete data descriptor
    /// replacement. Unsupported low-level reconfiguration detaches
    /// monotonically so a stale slot can only fall back, never silently write.
    pub(super) fn synchronize_linked_definition(
        &self,
        key: &str,
        previous: &Property,
        replacement: &mut Property,
    ) -> LinkedDefinition {
        let Some(cell) = self.global_data_cell(key) else {
            return LinkedDefinition::NotLinked;
        };
        if replacement.is_accessor() || replacement.configurable {
            self.detach_global_data_link(key, &cell);
            return LinkedDefinition::Detached;
        }

        cell.set(replacement.value.clone());
        if !cell.set_linked_global_writable(replacement.writable) {
            self.detach_global_data_link(key, &cell);
            return LinkedDefinition::Detached;
        }
        replacement.value = Value::Undefined;
        if previous.accessor != replacement.accessor
            || previous.enumerable != replacement.enumerable
            || previous.writable != replacement.writable
            || previous.configurable != replacement.configurable
            || previous.get != replacement.get
            || previous.set != replacement.set
        {
            LinkedDefinition::DescriptorChanged
        } else {
            LinkedDefinition::ValueOnly
        }
    }

    pub(super) fn freeze_global_data_links(&self) {
        if !self.0.has_global_data_links.get() {
            return;
        }
        let Some(cold) = self.0.cold_if_present() else {
            return;
        };
        let cells = cold
            .global_data_links
            .borrow()
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for cell in cells {
            cell.set_linked_global_writable(false);
        }
    }

    pub(super) fn detach_global_data_link(&self, key: &str, cell: &Upvalue) {
        cell.detach_linked_global();
        let Some(cold) = self.0.cold_if_present() else {
            self.0.has_global_data_links.set(false);
            return;
        };
        let mut links = cold.global_data_links.borrow_mut();
        links.remove(key);
        self.0.has_global_data_links.set(!links.is_empty());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn linked_global(value: Value) -> (ObjectRef, Upvalue) {
        let object = ObjectRef::new(Default::default());
        object.define_property(
            "binding".to_owned(),
            Property::data(value.clone(), true, true, false),
        );
        let cell = Upvalue::new(value);
        assert!(object.link_realm_global_data_property("binding", &cell));
        (object, cell)
    }

    #[test]
    fn linked_value_is_visible_in_both_directions_without_revision_bumps() {
        let (object, cell) = linked_global(Value::Number(1.0));
        let installed_revision = object.property_revision();

        assert!(matches!(
            object.write_existing_own_data_property("binding", &Value::Number(2.0)),
            OwnDataPropertyWrite::Written
        ));
        assert_eq!(cell.get(), Value::Number(2.0));
        assert_eq!(object.property_revision(), installed_revision);

        assert_eq!(
            cell.try_store_linked_global(Value::Number(3.0)),
            LinkedGlobalStore::Written
        );
        assert_eq!(object.get("binding"), Some(Value::Number(3.0)));
        assert_eq!(object.property_revision(), installed_revision);
    }

    #[test]
    fn freezing_a_linked_property_preserves_value_and_blocks_writes() {
        let (object, cell) = linked_global(Value::Number(1.0));
        object.freeze();

        assert_eq!(cell.get(), Value::Number(1.0));
        assert_eq!(object.get("binding"), Some(Value::Number(1.0)));
        assert_eq!(
            cell.try_store_linked_global(Value::Number(2.0)),
            LinkedGlobalStore::ReadOnly
        );
        let descriptor = object.own_property("binding").unwrap();
        assert!(!descriptor.writable);
        assert!(!descriptor.configurable);
        assert_eq!(descriptor.value, Value::Number(1.0));
    }

    #[test]
    fn alternating_links_and_value_only_definitions_keep_revision_stable() {
        let object = ObjectRef::new(Default::default());
        object.define_property(
            "left".to_owned(),
            Property::data(Value::Number(1.0), true, true, false),
        );
        object.define_property(
            "right".to_owned(),
            Property::data(Value::Number(2.0), true, true, false),
        );
        let left = Upvalue::new(Value::Number(1.0));
        let right = Upvalue::new(Value::Number(2.0));
        assert!(object.link_realm_global_data_property("left", &left));
        assert!(object.link_realm_global_data_property("right", &right));
        let installed_revision = object.property_revision();

        for value in 0..10 {
            assert_eq!(
                left.try_store_linked_global(Value::Number(value as f64)),
                LinkedGlobalStore::Written
            );
            assert_eq!(
                right.try_store_linked_global(Value::Number((value + 1) as f64)),
                LinkedGlobalStore::Written
            );
        }
        assert_eq!(object.property_revision(), installed_revision);

        object.define_property(
            "left".to_owned(),
            Property::data(Value::Number(42.0), true, true, false),
        );
        assert_eq!(left.get(), Value::Number(42.0));
        assert_eq!(object.property_revision(), installed_revision);
    }

    #[test]
    fn incompatible_internal_definition_detaches_without_disabling_the_cell() {
        let (object, cell) = linked_global(Value::Number(1.0));

        // Public descriptor validation rejects this replacement because the
        // linked property is non-configurable. The raw internal API still
        // fails closed if a future caller bypasses that validation.
        object.define_property(
            "binding".to_owned(),
            Property::accessor(None, None, true, false),
        );

        assert!(cell.is_detached_global());
        assert!(!object.has_global_data_link("binding"));
        assert!(object.own_property("binding").unwrap().is_accessor());
        assert_eq!(
            cell.try_store_linked_global(Value::Number(2.0)),
            LinkedGlobalStore::NotLinked
        );
        cell.set(Value::Number(3.0));
        assert_eq!(cell.get(), Value::Number(3.0));
    }
}
