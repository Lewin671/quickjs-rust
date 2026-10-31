//! Code-owned string-only append plans retain metadata, never receiver values.
//! Each hit publishes one prefix under the receiver's exclusive store borrow.
use super::*;

#[derive(Default)]
pub(crate) struct FieldPlans {
    pub(crate) single: Option<AppendRecipe>,
    pub(crate) segment: Option<super::append_batch::AppendBatchRecipe>,
}

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
        if !store.matches_version(&self.before) || store.len() != self.slot {
            return false;
        }
        store.invalidate_dependencies();
        store.values.push(value);
        store.layout = self.after.clone();
        store.set_epoch(0);
        store.verify_slot(self.slot);
        true
    }
}
impl NamedStore {
    pub(crate) fn append_recipe(&self, before: LayoutVersion, slot: usize) -> Option<AppendRecipe> {
        if !before.is_shared()
            || self.layout.dictionary
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
