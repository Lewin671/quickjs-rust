//! Non-reusable structural validity: an allocation identity plus edit epoch.
//! Weak identity pins the allocation address without retaining keys/domain.
use super::*;

#[derive(Clone, Debug)]
pub(crate) struct LayoutVersion {
    layout: Weak<Layout>,
    epoch: u64,
}
impl PartialEq for LayoutVersion {
    fn eq(&self, other: &Self) -> bool {
        self.epoch == other.epoch && self.layout.ptr_eq(&other.layout)
    }
}
impl Eq for LayoutVersion {}
impl NamedStore {
    pub(crate) fn version(&self) -> LayoutVersion {
        LayoutVersion {
            layout: Rc::downgrade(&self.layout),
            epoch: self.epoch,
        }
    }
}
impl Clone for Layout {
    fn clone(&self) -> Self {
        self.copied(self.dictionary)
    }
}
