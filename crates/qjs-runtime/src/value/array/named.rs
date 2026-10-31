//! Pure array named probes use the shared seed until named storage exists.
//! Mutations materialize one NamedStore; no callback may cross a storage read.
use super::*;
pub(super) struct SeedRead<'a> {
    pub(super) cell: &'a Cell<Option<Rc<super::super::layout::NamedStorage>>>,
    pub(super) value: Option<Rc<super::super::layout::NamedStorage>>,
}
impl Drop for SeedRead<'_> {
    fn drop(&mut self) {
        self.cell.set(self.value.take());
    }
}
impl ArrayRef {
    #[inline(always)]
    pub(crate) fn with_named<R>(
        &self,
        read: impl FnOnce(Option<&super::super::layout::NamedStore>) -> R,
    ) -> R {
        if let Some(named) = self.0.materialized_named() {
            read(Some(&named.borrow()))
        } else {
            read(None)
        }
    }
    pub(crate) fn with_named_mut<R>(
        &self,
        write: impl FnOnce(&mut super::super::layout::NamedStore) -> R,
    ) -> R {
        write(&mut self.0.named_store_mut())
    }
    pub(crate) fn with_key_domain<R>(
        &self,
        read: impl FnOnce(&crate::property::atom::AtomDomain) -> R,
    ) -> R {
        if let Some(named) = self.0.materialized_named() {
            read(named.borrow().domain())
        } else {
            self.0.with_seed(|seed| read(seed.domain()))
        }
    }
    pub(crate) fn named_policy(&self) -> crate::property::atom::IcPolicy {
        self.with_key_domain(|domain| domain.ic_policy())
    }
    pub(crate) fn named_version(&self) -> super::super::layout::LayoutVersion {
        if let Some(named) = self.0.materialized_named() {
            named.borrow().version()
        } else {
            self.0
                .with_seed(|seed| seed.version().expect("array layout"))
        }
    }
    pub(crate) fn first_prototype_data(
        &self,
        v: &super::super::layout::LayoutVersion,
        slot: u32,
    ) -> Option<Value> {
        if let Some(named) = self.0.materialized_named() {
            named.borrow().first_prototype_data(v, slot)
        } else {
            self.0.with_seed(|seed| seed.first_prototype_data(v, slot))
        }
    }
    pub(crate) fn named_matches(&self, v: &super::super::layout::LayoutVersion) -> bool {
        if let Some(named) = self.0.materialized_named() {
            named.borrow().matches_version(v)
        } else {
            self.0.with_seed(|seed| seed.matches(v))
        }
    }
    pub(crate) fn chain_dependency(&self) -> Option<super::super::layout::StructuralDependency> {
        if let Some(named) = self.0.materialized_named() {
            named.borrow().chain_dependency()
        } else {
            self.0.with_seed(|seed| seed.chain_dependency())
        }
    }
    pub(crate) fn prototype_slot(&self) -> Option<Prototype> {
        if let Some(named) = self.0.materialized_named() {
            named.borrow().prototype()
        } else {
            self.0.with_seed(|seed| seed.prototype())
        }
    }
}
