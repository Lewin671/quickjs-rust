//! Constructor root links are cold metadata with weak prototype identities.
use super::*;
impl Function {
    pub(crate) fn new_instance_receiver(
        &self,
        prototype: Option<Prototype>,
        domain: &crate::property::atom::AtomDomain,
    ) -> ObjectRef {
        self.auxiliary.with_cold_mut(|cold| {
            ObjectRef::with_constructor_layout(&mut cold.receiver_layout, domain, prototype)
        })
    }
}
