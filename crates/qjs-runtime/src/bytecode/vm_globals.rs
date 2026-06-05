use crate::{GLOBAL_THIS_BINDING, Property, Value};

use super::vm::Vm;

impl Vm<'_> {
    pub(super) fn initialize_global_var_properties(&self) {
        if !self.sync_var_to_global_object {
            return;
        }
        let Some(Value::Object(global_object)) = self.globals.get(GLOBAL_THIS_BINDING) else {
            return;
        };
        for (slot, local) in self.bytecode.locals.iter().enumerate() {
            if !local.hoisted
                || local.name.starts_with('\0')
                || global_object.has_own_property(&local.name)
            {
                continue;
            }
            let value = self
                .locals
                .get(slot)
                .and_then(|value| value.clone())
                .unwrap_or(Value::Undefined);
            global_object
                .define_property(local.name.clone(), Property::data(value, true, true, false));
        }
    }
}
