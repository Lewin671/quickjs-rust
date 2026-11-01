//! Function named authority. Lazy defaults publish into the same store before
//! any structural edit, preserving their original order without a side index.
use super::*;
use crate::value::layout::NamedStore;

impl Function {
    pub(crate) fn has_compat_caller(&self) -> bool {
        !self.is_strict
            && self.constructable
            && self.bound.is_none()
            && !self.is_generator
            && !self.is_async
    }
    pub(crate) fn virtual_named_probe(&self, key: &str) -> Option<crate::value::named::Virtual> {
        use crate::value::named::Virtual;
        if key == "length" && self.auxiliary.implicit_length_property.get() {
            Some(Virtual::FunctionLength)
        } else if key == "name" && self.auxiliary.implicit_name_property.get() {
            Some(Virtual::FunctionName)
        } else if key == "prototype" && self.auxiliary.lazy_default_prototype.get() {
            Some(Virtual::FunctionPrototype)
        } else {
            None
        }
    }
    pub(crate) fn virtual_named_value(&self, hook: crate::value::named::Virtual) -> Option<Value> {
        use crate::value::named::Virtual;
        Some(match hook {
            Virtual::FunctionLength => Value::Number(self.params.length() as f64),
            Virtual::FunctionName => Value::String(self.name.clone().unwrap_or_default().into()),
            Virtual::FunctionPrototype => {
                self.ensure_default_prototype();
                self.own_property("prototype")?.value
            }
            Virtual::FunctionCaller => Value::Undefined,
            Virtual::FunctionArguments => Value::Null,
            _ => return None,
        })
    }
    pub(crate) fn named_shape_flavor(&self) -> u16 {
        0x100
            | u16::from(self.auxiliary.implicit_name_property.get())
            | (u16::from(self.auxiliary.implicit_length_property.get()) << 1)
            | (u16::from(self.auxiliary.lazy_default_prototype.get()) << 2)
            | (u16::from(
                !self.is_strict
                    && self.constructable
                    && self.bound.is_none()
                    && !self.is_generator
                    && !self.is_async,
            ) << 3)
    }
    pub(super) fn initialize_named(&self) {
        if self.realm().is_none() {
            return;
        }
        let prototype = self.creation.as_ref().and_then(|creation| {
            creation
                .intrinsics
                .function
                .clone()
                .or_else(|| creation.realm.function_prototype())
        });
        self.initialize_named_with_prototype(prototype);
    }

    /// Initial authority for an unpublished function, without mutation checks.
    pub(super) fn initialize_named_with_prototype(&self, prototype: Option<Prototype>) {
        *self.named.borrow_mut() = crate::value::layout::NamedStorage::new(
            self.property_domain(),
            prototype,
            self.named_shape_flavor(),
        );
    }
    fn refresh_named_flavor(&self) {
        self.named
            .borrow_mut()
            .set_flavor(self.named_shape_flavor());
    }
    pub(crate) fn named_version(&self) -> Option<crate::value::layout::LayoutVersion> {
        self.named.borrow().version()
    }
    pub(crate) fn first_prototype_data(
        &self,
        v: &crate::value::layout::LayoutVersion,
        slot: u32,
    ) -> Option<Value> {
        self.named.borrow().first_prototype_data(v, slot)
    }
    pub(crate) fn named_matches(&self, v: &crate::value::layout::LayoutVersion) -> bool {
        self.named.borrow().matches(v)
    }
    pub(crate) fn chain_dependency(&self) -> Option<crate::value::layout::StructuralDependency> {
        self.named.borrow().chain_dependency()
    }

    #[inline(always)]
    pub(crate) fn with_named<R>(&self, read: impl FnOnce(Option<&NamedStore>) -> R) -> R {
        let named = self.named.borrow();
        read(named.store())
    }

    pub(crate) fn with_named_mut<R>(&self, edit: impl FnOnce(&mut NamedStore) -> R) -> R {
        edit(self.named.borrow_mut().store_mut())
    }

    fn materialize_default_data_properties(&self) {
        let length = self.auxiliary.implicit_length_property.replace(false);
        let name = self.auxiliary.implicit_name_property.replace(false);
        let prototype = self.auxiliary.lazy_default_prototype.get();
        self.refresh_named_flavor();
        if !length && !name && !prototype {
            return;
        }
        self.with_named_mut(|store| {
            if length {
                store.insert_text("length", self.default_length_property());
            }
            if name {
                store.insert_text("name", self.default_name_property());
            }
            // Reserve the original slot, without creating the prototype object.
            // The hook replaces this value before any observable read of it.
            if prototype && store.get_text("prototype").is_none() {
                store.insert_text(
                    "prototype",
                    Property::data(Value::Undefined, false, true, false),
                );
            }
        });
    }

    pub(super) fn mark_lazy_default_prototype(&self) {
        self.auxiliary.lazy_default_prototype.set(true);
        self.refresh_named_flavor();
    }

    fn ensure_default_prototype(&self) {
        if !self.auxiliary.lazy_default_prototype.get() {
            return;
        }
        self.materialize_default_data_properties();
        self.auxiliary.lazy_default_prototype.set(false);
        self.refresh_named_flavor();
        let prototype = ObjectRef::with_prototype(
            HashMap::new(),
            self.creation.as_ref().and_then(|creation| {
                creation
                    .intrinsics
                    .object
                    .clone()
                    .or_else(|| creation.realm.object_prototype())
            }),
            self.property_domain(),
        );
        prototype.define_non_enumerable("constructor".to_owned(), Value::Function(self.clone()));
        self.with_named_mut(|store| {
            store.insert_text(
                "prototype",
                Property::data(Value::Object(prototype), false, true, false),
            );
        });
    }

    pub(crate) fn is_extensible(&self) -> bool {
        self.auxiliary.extensible.get()
    }
    pub(crate) fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }
    pub(crate) fn address(&self) -> usize {
        Rc::as_ptr(&self.0) as usize
    }

    pub(crate) fn prevent_extensions(&self) {
        self.ensure_default_prototype();
        self.auxiliary.extensible.set(false);
    }
    pub(crate) fn seal(&self) {
        self.materialize_default_data_properties();
        self.ensure_default_prototype();
        self.prevent_extensions();
        self.auxiliary.sealed.set(true);
        self.with_named_mut(|store| store.transform_descriptors(false));
    }
    pub(crate) fn is_sealed(&self) -> bool {
        self.ensure_default_prototype();
        !self.is_extensible()
            && self.auxiliary.sealed.get()
            && self
                .with_named(|store| store.is_none_or(|s| s.descriptors().all(|p| !p.configurable)))
    }
    pub(crate) fn freeze(&self) {
        self.seal();
        self.auxiliary.frozen.set(true);
        self.with_named_mut(|store| store.transform_descriptors(true));
    }
    pub(crate) fn is_frozen(&self) -> bool {
        self.ensure_default_prototype();
        !self.is_extensible()
            && self.auxiliary.frozen.get()
            && self.with_named(|store| {
                store.is_none_or(|s| s.descriptors().all(|p| !p.configurable && !p.writable))
            })
    }

    pub(crate) fn set_property(&self, key: String, value: Value) {
        if let Some(mut property) = self.own_property(&key) {
            if property.writable {
                property.value = value;
                self.define_property(key, property);
            }
        } else if self.is_extensible() {
            self.define_property(key, Property::enumerable(value));
        }
    }
    pub(crate) fn define_property(&self, key: String, property: Property) {
        self.materialize_default_data_properties();
        if key == "prototype" {
            self.ensure_default_prototype();
        }
        self.with_named_mut(|store| {
            store.insert_text(&key, property);
        });
        self.refresh_dynamic_function_realm_override(&key);
    }
    fn refresh_dynamic_function_realm_override(&self, key: &str) {
        if key == DYNAMIC_FUNCTION_REALM_GLOBAL {
            self.has_dynamic_function_realm_override
                .set(self.with_named(|s| s.and_then(|s| s.get_text(key)).is_some()));
        }
    }
    pub(crate) fn own_property(&self, key: &str) -> Option<Property> {
        if key == "length" && self.auxiliary.implicit_length_property.get() {
            return Some(self.default_length_property());
        }
        if key == "name" && self.auxiliary.implicit_name_property.get() {
            return Some(self.default_name_property());
        }
        if key == "prototype" {
            self.ensure_default_prototype();
        }
        self.with_named(|s| s.and_then(|s| s.get_text(key)))
    }
    pub(crate) fn own_property_keys(&self) -> Vec<String> {
        self.ordered_property_names(|p| p.enumerable)
    }
    pub(crate) fn own_property_names(&self) -> Vec<String> {
        self.ordered_property_names(|_| true)
    }
    fn ordered_property_names(&self, include: impl Fn(&Property) -> bool) -> Vec<String> {
        self.materialize_default_data_properties();
        self.ensure_default_prototype();
        self.with_named(|s| {
            s.map_or_else(Vec::new, |s| {
                s.keys()
                    .into_iter()
                    .filter(|k| {
                        k.symbol().is_none()
                            && include(&s.get(k).expect("own domain").expect("own key"))
                    })
                    .map(|k| k.as_str().to_owned())
                    .collect()
            })
        })
    }
    pub(crate) fn delete_own_property(&self, key: &str) -> bool {
        if self.own_property(key).is_some_and(|p| !p.configurable) {
            return false;
        }
        self.remove_own_property_unchecked(key);
        true
    }
    pub(crate) fn remove_own_property_unchecked(&self, key: &str) {
        self.materialize_default_data_properties();
        if key == "prototype" {
            self.ensure_default_prototype();
        }
        self.with_named_mut(|s| {
            if let Some(key) = s.domain().find(key) {
                s.remove(&key).expect("own domain");
            }
        });
        if key == DYNAMIC_FUNCTION_REALM_GLOBAL {
            self.has_dynamic_function_realm_override.set(false);
        }
    }
    pub(crate) fn define_symbol_property(&self, symbol: ObjectRef, property: Property) {
        self.materialize_default_data_properties();
        self.with_named_mut(|s| {
            let key = s.domain().intern_symbol(&symbol);
            s.define(&key, property).expect("own domain");
        });
    }
    pub(crate) fn has_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        self.own_symbol_property(symbol).is_some()
    }
    pub(crate) fn own_symbol_property(&self, symbol: &ObjectRef) -> Option<Property> {
        self.with_named(|s| {
            let s = s?;
            let key = s.domain().find_symbol(symbol)?;
            s.get(&key).expect("own domain")
        })
    }
    pub(crate) fn delete_own_symbol_property(&self, symbol: &ObjectRef) -> bool {
        if self
            .own_symbol_property(symbol)
            .is_some_and(|p| !p.configurable)
        {
            return false;
        }
        self.with_named_mut(|s| {
            if let Some(key) = s.domain().find_symbol(symbol) {
                s.remove(&key).expect("own domain");
            }
        });
        true
    }
    pub(crate) fn own_property_symbols(&self) -> Vec<ObjectRef> {
        self.with_named(|s| {
            s.map_or_else(Vec::new, |s| {
                s.keys()
                    .into_iter()
                    .filter_map(|k| k.symbol().cloned())
                    .collect()
            })
        })
    }
}

impl Function {}
