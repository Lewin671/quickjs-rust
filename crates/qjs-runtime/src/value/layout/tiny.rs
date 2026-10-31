//! Prototype-independent leased metadata for tiny property sequences.
//! Weak interning keeps live buffers reusable without retaining dead key leases.
use super::*;
impl Layout {
    pub(super) fn append_data(
        &self,
        key: &KeyLease,
        attrs: Attributes,
        proof: IdProbe,
    ) -> Rc<RefCell<storage::LayoutData>> {
        let owner = self.owner.upgrade();
        let signature = (self.len < 2).then(|| {
            let mut signature = [0; 2];
            for (i, entry) in self.entries().iter().enumerate() {
                signature[i] = (u64::from(entry.key.get()) << 16) | u64::from(entry.attrs.0);
            }
            signature[self.len] = (u64::from(key.id().get()) << 16) | u64::from(attrs.0);
            signature
        });
        if let (Some(owner), Some(signature)) = (&owner, signature)
            && let Some(data) = owner
                .tiny_data
                .borrow()
                .get(&signature)
                .and_then(Weak::upgrade)
        {
            return data;
        }
        let tail = self.len > 0 && self.data.borrow().entries.len() == self.len;
        let data = if tail {
            self.data.clone()
        } else {
            Rc::new(RefCell::new(self.data.borrow().prefix(self.len)))
        };
        {
            let mut fields = data.borrow_mut();
            let proof = if tail {
                proof
            } else {
                fields.probe(key.id(), false)
            };
            fields.append(key, attrs, proof);
        }
        if let (Some(owner), Some(signature)) = (owner, signature) {
            let mut cache = owner.tiny_data.borrow_mut();
            if cache.len() >= 64 && cache.len().is_power_of_two() {
                cache.retain(|_, data| data.strong_count() > 0);
            }
            cache.insert(signature, Rc::downgrade(&data));
        }
        data
    }
}
