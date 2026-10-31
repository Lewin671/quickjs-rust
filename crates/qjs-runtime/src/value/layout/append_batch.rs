//! Observer-free append segments fill slots in order before final publication.
//! Admission proves a string-only default-data chain; no JS may run in apply.
use super::*;
pub(crate) struct AppendBatchRecipe {
    before: LayoutVersion,
    after: Rc<Layout>,
    start: usize,
    width: usize,
    indices: usize,
}
impl AppendBatchRecipe {
    pub(crate) fn apply(
        &self,
        store: &mut NamedStore,
        values: impl ExactSizeIterator<Item = Value>,
    ) -> Option<usize> {
        if values.len() != self.width
            || store.len() != self.start
            || !store.matches_version(&self.before)
        {
            return None;
        }
        store.invalidate_dependencies();
        store.values.extend(values);
        store.layout = self.after.clone();
        store.set_epoch(0);
        for slot in self.start..store.len() {
            store.verify_slot(slot);
        }
        Some(self.indices)
    }
}
impl NamedStore {
    pub(crate) fn append_batch_recipe(
        &self,
        before: LayoutVersion,
        start: usize,
        width: usize,
    ) -> Option<AppendBatchRecipe> {
        if width == 0
            || start.checked_add(width)? != self.len()
            || !before.is_shared()
            || self.layout.dictionary
        {
            return None;
        }
        let mut prefix = &self.layout;
        while prefix.len > start {
            if !matches!(
                prefix.incoming,
                Some(Transition::Append {
                    attrs: Attributes(7),
                    ..
                })
            ) {
                return None;
            }
            prefix = prefix.parent.as_ref()?;
        }
        if prefix.len != start || !before.matches_identity(prefix) {
            return None;
        }
        let keys = self.layout.keys();
        if keys.iter().any(|key| key.symbol().is_some()) {
            return None;
        }
        let added = &keys[start..];
        if added.iter().any(|key| {
            matches!(
                key.as_str(),
                "globalThis" | "__quickjsRustDynamicFunctionRealm"
            )
        }) {
            return None;
        }
        Some(AppendBatchRecipe {
            before,
            after: self.layout.clone(),
            start,
            width,
            indices: added
                .iter()
                .filter(|key| key.array_index().is_some())
                .count(),
        })
    }
}
