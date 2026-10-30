//! Store ICs reuse live slots or a string-only old-layout to new-layout append.
//! Set-add additionally guards the transitive prototype validity generation.
use super::*;
use crate::value::layout::AppendRecipe;
#[derive(Debug)]
pub(super) struct AddEntry {
    recipe: AppendRecipe,
    validity: Option<StructuralDependency>,
}
impl NamedPropertyCache {
    pub(in crate::bytecode) fn record_data_slot(
        &self,
        object: &ObjectRef,
        key: &KeyLease,
        slot: usize,
        before: Option<LayoutVersion>,
        created: bool,
    ) {
        let target = NamedTarget::Object(object);
        self.record_resolution(
            target,
            key.as_str(),
            &Resolution {
                probe: Probe::Data {
                    slot: slot as u32,
                    writable: true,
                },
                holder: None,
                inherited: false,
                first_holder: false,
            },
        );
        if !created || !target.policy().enabled || object.named_kind() != Some(0) {
            return;
        }
        let Some(before) = before else {
            return;
        };
        let validity = if target.prototype().is_some() {
            let Some(validity) = target.chain_dependency() else {
                return;
            };
            Some(validity)
        } else {
            None
        };
        if let Some(recipe) = object.with_named(|s| s.append_recipe(before, slot)) {
            self.0.borrow_mut().add = Some(Box::new(AddEntry { recipe, validity }));
        }
    }
    pub(in crate::bytecode) fn write_value(
        &self,
        value: &Value,
        new: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        value.with_named_target(|t| self.write_target(t, new, true))?
    }
    fn write_target(
        &self,
        target: NamedTarget<'_>,
        value: &Value,
        allow_add: bool,
    ) -> Option<OwnDataPropertyWrite> {
        let state = self.0.borrow();
        if let NamedTarget::Object(object) = target {
            let mut appended = None;
            let result = object.with_named_mut(|store| {
                let entry = match &state.entries {
                    Entries::Mono(e) => Some(e),
                    Entries::Poly(es) | Entries::SharedOwnPoly(es) => {
                        es.iter().find(|e| store.matches_version(&e.version))
                    }
                    _ => None,
                };
                if let Some(e) = entry
                    && e.flags & (7 | OWN | WRITE_FORBIDDEN) == DATA | OWN
                    && store.matches_version(&e.version)
                {
                    if e.flags & WRITABLE == 0 {
                        return Some(OwnDataPropertyWrite::ReadOnly);
                    }
                    return store
                        .overwrite_guarded_data(e.slot as usize, value.clone())
                        .then_some(OwnDataPropertyWrite::Written);
                }
                if !allow_add {
                    return None;
                }
                let add = state.add.as_ref()?;
                if object.named_kind() != Some(0)
                    || !object.is_extensible()
                    || !add
                        .validity
                        .as_ref()
                        .is_none_or(StructuralDependency::is_valid)
                {
                    return None;
                }
                if !add.recipe.apply(store, value.clone()) {
                    return None;
                }
                appended = Some(add.recipe.is_index());
                Some(OwnDataPropertyWrite::Written)
            });
            if let Some(index) = appended {
                object.note_cached_append(index);
            }
            return result;
        }
        let entry = state.entries.find(target)?;
        if entry.flags & (7 | OWN | WRITE_FORBIDDEN) != DATA | OWN {
            return None;
        }
        if entry.flags & WRITABLE == 0 {
            return Some(OwnDataPropertyWrite::ReadOnly);
        }
        target.write_matched_data(&entry.version, entry.slot, value)
    }
    pub(in crate::bytecode) fn write(
        &self,
        object: &ObjectRef,
        _name: &str,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        self.write_target(NamedTarget::Object(object), value, true)
    }
    pub(in crate::bytecode) fn write_hot(
        &self,
        object: &ObjectRef,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        // Typed-loop hoisting requires writes to preserve property structure.
        self.write_target(NamedTarget::Object(object), value, false)
    }
}
