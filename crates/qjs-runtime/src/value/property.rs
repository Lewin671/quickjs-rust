use super::Value;

#[derive(Clone, Debug)]
pub(crate) struct Property {
    pub(crate) value: Value,
    pub(crate) enumerable: bool,
    pub(crate) writable: bool,
    pub(crate) configurable: bool,
}

impl Property {
    pub(crate) fn data(value: Value, enumerable: bool, writable: bool, configurable: bool) -> Self {
        Self {
            value,
            enumerable,
            writable,
            configurable,
        }
    }

    pub(crate) fn enumerable(value: Value) -> Self {
        Self {
            value,
            enumerable: true,
            writable: true,
            configurable: true,
        }
    }

    pub(crate) fn non_enumerable(value: Value) -> Self {
        Self {
            value,
            enumerable: false,
            writable: true,
            configurable: true,
        }
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
