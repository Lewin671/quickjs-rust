//! Receiver-owned cold metadata, independent of shared descriptor layouts.
//! Empty objects acquire a prototype-sensitive guard only when one is issued.
use super::*;

#[derive(Default)]
pub(super) struct ColdStorage(OnceCell<Box<ColdData>>);
#[derive(Default)]
pub(super) struct ColdData {
    slots: Option<Box<AccessorSlots>>,
    pub(super) epoch: std::cell::Cell<u64>,
    pub(super) mode: OnceCell<Rc<PrototypeState>>,
    pub(super) guard: OnceCell<Rc<Layout>>,
}
impl ColdStorage {
    pub(super) fn get(&self) -> &ColdData {
        self.0.get_or_init(|| Box::new(ColdData::default()))
    }
    pub(super) fn epoch(&self) -> u64 {
        self.0.get().map_or(0, |cold| cold.epoch.get())
    }
    pub(super) fn mode(&self) -> Option<&Rc<PrototypeState>> {
        self.0.get()?.mode.get()
    }
    pub(super) fn guard(&self) -> Option<&Rc<Layout>> {
        self.0.get()?.guard.get()
    }
    pub(super) fn clear_guard(&mut self) {
        if let Some(cold) = self.0.get_mut() {
            cold.guard.take();
        }
    }
    pub(super) fn as_deref(&self) -> Option<&AccessorSlots> {
        self.0.get()?.slots.as_deref()
    }
    pub(super) fn as_mut(&mut self) -> Option<&mut Box<AccessorSlots>> {
        self.0.get_mut()?.slots.as_mut()
    }
    pub(super) fn as_ref(&self) -> Option<&AccessorSlots> {
        self.as_deref()
    }
    #[cfg(test)]
    pub(super) fn is_unallocated(&self) -> bool {
        self.0.get().is_none()
    }
    #[cfg(test)]
    pub(super) fn is_none(&self) -> bool {
        self.as_ref().is_none()
    }
    pub(super) fn clear(&mut self) {
        if let Some(cold) = self.0.get_mut() {
            cold.slots = None;
        }
    }
    pub(super) fn get_or_insert_with(
        &mut self,
        f: impl FnOnce() -> Box<AccessorSlots>,
    ) -> &mut Box<AccessorSlots> {
        self.get();
        self.0
            .get_mut()
            .expect("initialized cold data")
            .slots
            .get_or_insert_with(f)
    }
}
impl From<Option<Box<AccessorSlots>>> for ColdStorage {
    fn from(slots: Option<Box<AccessorSlots>>) -> Self {
        match slots {
            None => Self::default(),
            Some(slots) => Self(OnceCell::from(Box::new(ColdData {
                slots: Some(slots),
                ..ColdData::default()
            }))),
        }
    }
}
