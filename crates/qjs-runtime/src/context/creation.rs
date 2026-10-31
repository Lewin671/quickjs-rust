//! Immutable creation metadata shared by roots, functions and retained jobs.
//! Module routes point to canonical cells. No execution binding values or
//! activation-local names are captured here.
use crate::{
    ObjectRef, Value,
    function::{DynamicBindings, ModuleImports, Realm},
    module::ModuleHostRef,
    private::PrivateEnvironment,
};
use std::rc::Rc;

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

#[derive(Clone)]
pub(crate) struct CreationContext {
    pub(crate) realm: Realm,
    pub(crate) module: Option<Rc<ModuleContext>>,
    pub(crate) referrer: ScriptOrModule,
    pub(crate) private: Option<PrivateEnvironment>,
    pub(crate) global: GlobalSelection,
    // The existing Function/arguments APIs retain an imports handle even for
    // scripts. Keep one empty metadata handle per root, shared by its views.
    pub(crate) empty_imports: ModuleImports,
}
impl CreationContext {
    pub(crate) fn root(realm: Realm) -> Rc<Self> {
        Rc::new(Self {
            realm,
            module: None,
            referrer: ScriptOrModule::Script(None),
            private: None,
            global: GlobalSelection::Default,
            empty_imports: ModuleImports::default(),
        })
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
