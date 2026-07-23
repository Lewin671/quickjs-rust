use super::ObjectRef;

impl ObjectRef {
    fn mark_typed_array_exotic(&self) {
        self.0.typed_array_exotic.set(true);
    }

    pub(crate) fn install_typed_array_slots(&self, slots: crate::typed_array::TypedArraySlots) {
        self.mark_typed_array_exotic();
        let installed = self.0.cold().typed_array_slots.set(slots);
        debug_assert!(
            installed.is_ok(),
            "TypedArray internal slots must only be installed once"
        );
    }

    pub(crate) fn typed_array_slots(&self) -> Option<&crate::typed_array::TypedArraySlots> {
        self.0
            .cold_if_present()
            .and_then(|cold| cold.typed_array_slots.get())
    }

    pub(crate) fn is_typed_array_exotic(&self) -> bool {
        self.0.typed_array_exotic.get()
    }
}
