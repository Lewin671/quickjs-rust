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

/// A chain cache owns the cell; layouts retain only weak subscriptions.
/// Invalidating a shared layout is conservative: it also retires dependencies
/// of other holders with that layout, without retaining any holder or key.
#[derive(Clone, Debug)]
pub(crate) struct StructuralDependency(Rc<std::cell::Cell<bool>>);
impl StructuralDependency {
    pub(crate) fn new() -> Self {
        Self(Rc::new(std::cell::Cell::new(true)))
    }
    pub(crate) fn is_valid(&self) -> bool {
        self.0.get()
    }
}
impl NamedStore {
    #[inline(always)]
    pub(crate) fn matches_version(&self, expected: &LayoutVersion) -> bool {
        expected.layout.as_ptr() == Rc::as_ptr(&self.layout) && expected.epoch == self.epoch
    }
    pub(crate) fn subscribe(&self, dependency: &StructuralDependency) {
        let mut subscribers = self.layout.subscribers.borrow_mut();
        subscribers.retain(|weak| weak.upgrade().is_some_and(|cell| cell.get()));
        // Bound a shared node's cold registration work and retained weak cells.
        // Overflow invalidates existing proofs; generic lookup can rebuild them.
        if subscribers.len() == 256 {
            for cell in subscribers.drain(..).filter_map(|w| w.upgrade()) {
                cell.set(false);
            }
        }
        subscribers.push(Rc::downgrade(&dependency.0));
    }
    pub(crate) fn invalidate_dependencies(&self) {
        if self.layout.subscribers.borrow().is_empty() {
            return;
        }
        for cell in self
            .layout
            .subscribers
            .borrow_mut()
            .drain(..)
            .filter_map(|w| w.upgrade())
        {
            cell.set(false);
        }
    }
}
