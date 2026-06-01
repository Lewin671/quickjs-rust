use super::Value;

#[derive(Clone, Debug)]
pub(crate) struct Property {
    pub(crate) value: Value,
    pub(crate) get: Option<Value>,
    pub(crate) set: Option<Value>,
    pub(crate) enumerable: bool,
    pub(crate) writable: bool,
    pub(crate) configurable: bool,
}

impl Property {
    pub(crate) fn data(value: Value, enumerable: bool, writable: bool, configurable: bool) -> Self {
        Self {
            value,
            get: None,
            set: None,
            enumerable,
            writable,
            configurable,
        }
    }

    pub(crate) fn accessor(
        get: Option<Value>,
        set: Option<Value>,
        enumerable: bool,
        configurable: bool,
    ) -> Self {
        Self {
            value: Value::Undefined,
            get,
            set,
            enumerable,
            writable: false,
            configurable,
        }
    }

    pub(crate) fn enumerable(value: Value) -> Self {
        Self {
            value,
            get: None,
            set: None,
            enumerable: true,
            writable: true,
            configurable: true,
        }
    }

    pub(crate) fn non_enumerable(value: Value) -> Self {
        Self {
            value,
            get: None,
            set: None,
            enumerable: false,
            writable: true,
            configurable: true,
        }
    }

    pub(crate) fn is_accessor(&self) -> bool {
        self.get.is_some() || self.set.is_some()
    }

    pub(crate) fn make_non_configurable(&mut self) {
        self.configurable = false;
    }

    pub(crate) fn make_non_writable(&mut self) {
        self.writable = false;
    }

    pub(crate) fn freeze_data(&mut self) {
        self.make_non_configurable();
        self.make_non_writable();
    }
}
