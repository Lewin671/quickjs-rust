//! Shared guards are allocation identities. Only private dictionaries carry an
//! epoch; zero is reserved for immutable shared layouts and is never recycled.
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
impl LayoutVersion {
    pub(super) fn shared(layout: &Rc<Layout>) -> Self {
        Self {
            layout: Rc::downgrade(layout),
            epoch: 0,
        }
    }
    pub(super) fn matches_layout(&self, layout: &Rc<Layout>, epoch: u64) -> bool {
        self.layout.as_ptr() == Rc::as_ptr(layout) && (self.epoch == 0 || self.epoch == epoch)
    }
    pub(crate) fn empty() -> Self {
        Self {
            layout: Weak::new(),
            epoch: 0,
        }
    }
    pub(crate) fn is_shared(&self) -> bool {
        self.epoch == 0
    }
    pub(super) fn matches_identity(&self, layout: &Rc<Layout>) -> bool {
        self.layout.as_ptr() == Rc::as_ptr(layout)
    }
    pub(crate) fn is_alive(&self) -> bool {
        self.layout.strong_count() > 0
    }
}
impl NamedStore {
    pub(crate) fn version(&self) -> LayoutVersion {
        LayoutVersion {
            layout: Rc::downgrade(&self.layout),
            epoch: if self.layout.dictionary {
                self.epoch.max(1)
            } else {
                0
            },
        }
    }
    /// The caller established an immutable shared-layout guard at admission.
    #[inline(always)]
    pub(crate) fn matches_shared_identity(&self, expected: &LayoutVersion) -> bool {
        expected.matches_identity(&self.layout)
    }
    #[inline(always)]
    pub(crate) fn matches_version(&self, expected: &LayoutVersion) -> bool {
        expected.matches_layout(&self.layout, self.epoch)
    }
}
impl Clone for Layout {
    fn clone(&self) -> Self {
        self.copied(self.dictionary)
    }
}
#[derive(Clone, Debug)]
pub(crate) struct StructuralDependency(Rc<std::cell::Cell<bool>>);
impl StructuralDependency {
    pub(super) fn new() -> Self {
        Self(Rc::new(std::cell::Cell::new(true)))
    }
    pub(crate) fn is_valid(&self) -> bool {
        self.0.get()
    }
    pub(super) fn invalidate(&self) {
        self.0.set(false)
    }
}
