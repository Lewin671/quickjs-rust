//! Code-owned string-only append plans retain metadata, never receiver values.
//! Each hit publishes one prefix under the receiver's exclusive store borrow.
use super::*;

pub(crate) struct AppendRecipe {
    before: LayoutVersion,
    after: Rc<Layout>,
    slot: usize,
    index: bool,
}
impl std::fmt::Debug for AppendRecipe {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppendRecipe")
            .field("slot", &self.slot)
            .finish()
    }
}
impl AppendRecipe {
    pub(crate) fn is_index(&self) -> bool {
        self.index
    }
    pub(crate) fn apply(&self, store: &mut NamedStore, value: Value) -> bool {
        if !store.matches_shared_identity(&self.before) || store.len() != self.slot {
            return false;
        }
        store.values.push(value);
        store.layout = self.after.clone();
        store.epoch = 0;
        store.verify_slot(self.slot);
        true
    }
}
impl NamedStore {
    pub(crate) fn append_recipe(&self, before: LayoutVersion, slot: usize) -> Option<AppendRecipe> {
        if !before.is_shared()
            || self.layout.dictionary
            || self.layout.mode.is_some()
            || slot + 1 != self.len()
            || !before.matches_identity(self.layout.parent.as_ref()?)
            || !matches!(
                self.layout.incoming,
                Some(Transition::Append {
                    attrs: Attributes(7),
                    ..
                })
            )
        {
            return None;
        }
        let keys = self.layout.keys();
        if keys.iter().any(|key| key.symbol().is_some()) {
            return None;
        }
        let name = keys[slot].as_str();
        if name == "globalThis" || name == "__quickjsRustDynamicFunctionRealm" {
            return None;
        }
        Some(AppendRecipe {
            before,
            after: self.layout.clone(),
            slot,
            index: keys[slot].array_index().is_some(),
        })
    }
}
