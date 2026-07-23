use super::{ArrayBufferKind, ObjectRef};

impl ObjectRef {
    pub(crate) fn internal_bytes(&self) -> Option<Vec<u8>> {
        self.0
            .cold_if_present()
            .and_then(|cold| cold.internal_bytes.borrow().clone())
    }

    pub(crate) fn with_internal_bytes<T>(&self, f: impl FnOnce(Option<&[u8]>) -> T) -> T {
        let Some(cold) = self.0.cold_if_present() else {
            return f(None);
        };
        let bytes = cold.internal_bytes.borrow();
        f(bytes.as_deref())
    }

    pub(crate) fn set_internal_bytes(&self, bytes: Vec<u8>) {
        *self.0.cold().internal_bytes.borrow_mut() = Some(bytes);
    }

    pub(crate) fn clear_internal_bytes(&self) {
        if let Some(cold) = self.0.cold_if_present() {
            *cold.internal_bytes.borrow_mut() = None;
        }
    }

    pub(crate) fn mark_array_buffer_immutable(&self) {
        self.0.cold().array_buffer_immutable.set(true);
    }

    pub(crate) fn install_array_buffer_state(&self, max_byte_length: Option<usize>) {
        let cold = self.0.cold();
        cold.array_buffer_kind.set(ArrayBufferKind::Ordinary);
        cold.array_buffer_detached.set(false);
        cold.array_buffer_max_byte_length.set(max_byte_length);
    }

    pub(crate) fn install_shared_array_buffer_state(&self, max_byte_length: Option<usize>) {
        let cold = self.0.cold();
        cold.array_buffer_kind.set(ArrayBufferKind::Shared);
        cold.array_buffer_detached.set(false);
        cold.array_buffer_max_byte_length.set(max_byte_length);
    }

    pub(crate) fn is_array_buffer_object(&self) -> bool {
        self.0
            .cold_if_present()
            .is_some_and(|cold| cold.array_buffer_kind.get() == ArrayBufferKind::Ordinary)
    }

    pub(crate) fn is_shared_array_buffer_object(&self) -> bool {
        self.0
            .cold_if_present()
            .is_some_and(|cold| cold.array_buffer_kind.get() == ArrayBufferKind::Shared)
    }

    pub(crate) fn mark_array_buffer_detached(&self) {
        if let Some(cold) = self.0.cold_if_present()
            && cold.array_buffer_kind.get() == ArrayBufferKind::Ordinary
        {
            cold.array_buffer_detached.set(true);
        }
    }

    pub(crate) fn is_array_buffer_detached(&self) -> bool {
        self.0.cold_if_present().is_some_and(|cold| {
            cold.array_buffer_kind.get() == ArrayBufferKind::Ordinary
                && cold.array_buffer_detached.get()
        })
    }

    pub(crate) fn array_buffer_max_byte_length(&self) -> Option<usize> {
        self.0
            .cold_if_present()
            .and_then(|cold| cold.array_buffer_max_byte_length.get())
    }

    pub(crate) fn set_array_buffer_max_byte_length(&self, max_byte_length: usize) {
        self.0
            .cold()
            .array_buffer_max_byte_length
            .set(Some(max_byte_length));
    }

    pub(crate) fn is_array_buffer_immutable(&self) -> bool {
        self.0
            .cold_if_present()
            .is_some_and(|cold| cold.array_buffer_immutable.get())
    }

    pub(crate) fn with_internal_bytes_mut<T>(
        &self,
        f: impl FnOnce(&mut Vec<u8>) -> T,
    ) -> Option<T> {
        self.0
            .cold_if_present()?
            .internal_bytes
            .borrow_mut()
            .as_mut()
            .map(f)
    }

    /// Tries to lease the internal byte backing without panicking when another
    /// runtime subsystem already holds a borrow. Dense TypedArray loops keep
    /// several independent ArrayBuffer backings borrowed together, so a
    /// fallible borrow is required to fail closed on re-entrant test harnesses
    /// and embedding-side borrows.
    pub(crate) fn try_borrow_internal_bytes_mut(&self) -> Option<std::cell::RefMut<'_, Vec<u8>>> {
        let cold = self.0.cold_if_present()?;
        let bytes = cold.internal_bytes.try_borrow_mut().ok()?;
        std::cell::RefMut::filter_map(bytes, Option::as_mut).ok()
    }

    /// The cross-thread `SharedArrayBuffer` backing for this object, if one was
    /// installed (agents harness only).
    #[cfg(feature = "agents")]
    pub(crate) fn shared_backing(&self) -> Option<crate::array_buffer::SharedBackingRef> {
        self.0
            .cold_if_present()
            .and_then(|cold| cold.shared_backing.borrow().clone())
    }

    /// Installs the cross-thread `SharedArrayBuffer` backing for this object.
    #[cfg(feature = "agents")]
    pub(crate) fn set_shared_backing(&self, backing: crate::array_buffer::SharedBackingRef) {
        *self.0.cold().shared_backing.borrow_mut() = Some(backing);
    }
}
