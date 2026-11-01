//! Immutable creation metadata shared by roots, functions and retained jobs.
//! Module routes point to canonical cells. No execution binding values or
//! activation-local names are captured here.
use super::Realm;
use crate::{
    ObjectRef, Value, function::DynamicBindings, module::ModuleHostRef, private::PrivateEnvironment,
};
use std::rc::Rc;
/// Structurally immutable module-import routing shared by function objects and
/// call frames. Module setup uses copy-on-write so environment clones preserve
/// their previous routing table while ordinary scripts and nested functions
/// retain only one pointer-sized empty map.
pub(crate) type ModuleImports = Rc<ModuleImportMap>;
pub(crate) type ModuleImportMap =
    crate::value::name_hash::NameMap<String, (DynamicBindings, String)>;

#[derive(Clone)]
pub(crate) struct ModuleContext {
    pub(crate) imports: ModuleImports,
    pub(crate) live: Option<DynamicBindings>,
}

#[derive(Clone)]
pub(crate) enum ScriptOrModule {
    Script(Option<ModuleHostRef>),
    Module(Option<ModuleHostRef>),
}
impl ScriptOrModule {
    pub(crate) fn host(&self) -> Option<&ModuleHostRef> {
        match self {
            Self::Script(host) | Self::Module(host) => host.as_ref(),
        }
    }
    pub(crate) fn set_host(&mut self, host: ModuleHostRef) {
        match self {
            Self::Script(selected) | Self::Module(selected) => *selected = Some(host),
        }
    }
    pub(crate) fn select_module(&mut self) {
        *self = Self::Module(self.host().cloned());
    }
}

#[derive(Clone, Default)]
pub(crate) enum GlobalSelection {
    #[default]
    Default,
    Override(ObjectRef),
    Disabled,
}
impl GlobalSelection {
    pub(crate) fn from_control(value: Value) -> Self {
        match value {
            Value::Object(object) => Self::Override(object),
            _ => Self::Disabled,
        }
    }
    pub(crate) fn explicit_value(&self) -> Option<Value> {
        match self {
            Self::Default => None,
            Self::Override(object) => Some(Value::Object(object.clone())),
            Self::Disabled => Some(Value::Undefined),
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct IntrinsicOverrides {
    pub(crate) object: Option<ObjectRef>,
    pub(crate) array: Option<ObjectRef>,
    pub(crate) function: Option<crate::Prototype>,
    pub(crate) primitive: [Option<ObjectRef>; 5],
    pub(crate) errors: [Option<ObjectRef>; 9],
}

impl IntrinsicOverrides {
    /// Retain semantic identities for the explicit synthetic-realm protocol.
    /// This reads only known constructor descriptors, never a namespace copy
    /// and never a user getter. Canonical Object/Array identity slots win.
    pub(crate) fn for_marked_global(global: &ObjectRef) -> Self {
        let mut result = Self::default();
        for kind in [
            super::PrimitiveIntrinsic::Boolean,
            super::PrimitiveIntrinsic::Number,
            super::PrimitiveIntrinsic::String,
            super::PrimitiveIntrinsic::BigInt,
            super::PrimitiveIntrinsic::Symbol,
        ] {
            result.primitive[kind.index()] = global
                .own_property(kind.name())
                .and_then(|property| super::constructor_object_prototype(property.value));
        }
        for kind in super::ErrorKind::ALL {
            result.errors[kind.index()] = global
                .own_property(kind.name())
                .and_then(|property| super::constructor_object_prototype(property.value));
        }
        result.function =
            global
                .own_property("Function")
                .and_then(|property| match property.value {
                    Value::Function(function) => super::constructor_function_prototype(&function),
                    _ => None,
                });
        if let Some((object, array)) = global.realm_intrinsic_prototype_identity() {
            result.object = Some(object);
            result.array = Some(array);
        } else {
            result.object = global
                .own_property("Object")
                .and_then(|property| super::constructor_object_prototype(property.value));
            result.array = global
                .own_property("Array")
                .and_then(|property| super::constructor_object_prototype(property.value));
        }
        result
    }
}

#[derive(Clone)]
pub(crate) struct CreationContext {
    pub(crate) realm: Realm,
    pub(crate) module: Option<Rc<ModuleContext>>,
    pub(crate) referrer: ScriptOrModule,
    pub(crate) private: Option<PrivateEnvironment>,
    pub(crate) global: GlobalSelection,
    pub(crate) intrinsics: IntrinsicOverrides,
    // The existing Function/arguments APIs retain an imports handle even for
    // scripts. Keep one empty metadata handle per root, shared by its views.
    pub(crate) empty_imports: ModuleImports,
}
impl CreationContext {
    pub(crate) fn root(realm: Realm) -> Rc<Self> {
        if let Some(creation) = realm.cached_root_creation() {
            return creation;
        }
        let creation = Rc::new(Self {
            realm,
            module: None,
            referrer: ScriptOrModule::Script(None),
            private: None,
            global: GlobalSelection::Default,
            intrinsics: IntrinsicOverrides::default(),
            empty_imports: ModuleImports::default(),
        });
        creation.realm.cache_root_creation(Rc::downgrade(&creation));
        creation
    }
    pub(crate) fn imports(&self) -> &ModuleImports {
        self.module
            .as_ref()
            .map_or(&self.empty_imports, |module| &module.imports)
    }
    pub(crate) fn module_mut(&mut self) -> &mut ModuleContext {
        Rc::make_mut(self.module.get_or_insert_with(|| {
            Rc::new(ModuleContext {
                imports: self.empty_imports.clone(),
                live: None,
            })
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    #[test]
    fn root_metadata_is_shared_weakly_and_mutation_cannot_change_the_default() {
        let realm = crate::function::new_realm(HashMap::new());
        let first = CreationContext::root(realm.clone());
        let second = CreationContext::root(realm.clone());
        assert!(Rc::ptr_eq(&first, &second));
        drop(second);
        let old = Rc::downgrade(&first);
        let mut selected = first;
        Rc::make_mut(&mut selected).global = GlobalSelection::Disabled;
        assert!(old.upgrade().is_none());
        let default = CreationContext::root(realm);
        assert!(matches!(default.global, GlobalSelection::Default));
        assert!(!Rc::ptr_eq(&selected, &default));
        let weak = Rc::downgrade(&default);
        drop(default);
        assert!(weak.upgrade().is_none());
    }
}
