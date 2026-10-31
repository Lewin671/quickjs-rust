//! Store ICs reuse live slots or a string-only old-layout to new-layout append.
//! Set-add additionally guards the transitive prototype validity generation.
use super::*;
use crate::value::layout::AppendRecipe;
#[derive(Debug)]
pub(super) struct AddEntry {
    recipe: AppendRecipe,
    validity: Option<StructuralDependency>,
}
impl CacheData {
    /// Snapshot only when this site can use the old-layout append proof.
    pub(in crate::bytecode) fn before_write_version(
        &self,
        object: &ObjectRef,
    ) -> Option<LayoutVersion> {
        if matches!(self.state.borrow().entries, Entries::Mega) {
            return None;
        }
        object.with_named(|store| store.domain().ic_policy().enabled.then(|| store.version()))
    }

    #[cold]
    #[inline(never)]
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
            self.state.borrow_mut().add = Some(Box::new(AddEntry { recipe, validity }));
        }
    }
    /// Caller excludes the realm global; the entry proves own writable data.
    #[inline(always)]
    pub(in crate::bytecode) fn write_number(&self, object: &ObjectRef, number: f64) -> bool {
        if let Some(entry) = self.hot_mono() {
            return object.with_named_mut(|store| {
                entry.flags & (7 | OWN | WRITABLE | WRITE_FORBIDDEN) == DATA | OWN | WRITABLE
                    && store.matches_own_version(&entry.version)
                    && store.overwrite_guarded_number(entry.slot as usize, number)
            });
        }
        self.write_number_other(object, number)
    }
    #[cold]
    #[inline(never)]
    fn write_number_other(&self, object: &ObjectRef, number: f64) -> bool {
        let state = self.state.borrow();
        object.with_named_mut(|store| {
            let entry = match &state.entries {
                Entries::MonoFirst | Entries::Mono(_) => {
                    state.entries.mono(&self.first).expect("mono")
                }
                Entries::Poly(entries) | Entries::SharedOwnPoly(entries) => {
                    let Some(entry) = entries
                        .iter()
                        .find(|entry| store.matches_own_version(&entry.version))
                    else {
                        return false;
                    };
                    entry
                }
                _ => return false,
            };
            entry.flags & (7 | OWN | WRITABLE | WRITE_FORBIDDEN) == DATA | OWN | WRITABLE
                && store.matches_own_version(&entry.version)
                && store.overwrite_guarded_number(entry.slot as usize, number)
        })
    }
    #[inline(always)]
    pub(in crate::bytecode) fn write_value(
        &self,
        value: &Value,
        new: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        value.with_named_target(|t| {
            if let Some(entry) = self.hot_mono()
                && entry.flags & (7 | OWN | WRITABLE | WRITE_FORBIDDEN) == DATA | OWN | WRITABLE
                && let Some(result) = t.write_matched_data(&entry.version, entry.slot, new)
            {
                return Some(result);
            }
            self.write_target(t, new, true)
        })?
    }
    #[cold]
    #[inline(never)]
    fn write_target(
        &self,
        target: NamedTarget<'_>,
        value: &Value,
        allow_add: bool,
    ) -> Option<OwnDataPropertyWrite> {
        let state = self.state.borrow();
        if let NamedTarget::Object(object) = target {
            let mut appended = None;
            let result = object.with_named_mut(|store| {
                let entry = match &state.entries {
                    Entries::MonoFirst | Entries::Mono(_) => state.entries.mono(&self.first),
                    Entries::Poly(es) | Entries::SharedOwnPoly(es) => {
                        es.iter().find(|e| store.matches_own_version(&e.version))
                    }
                    _ => None,
                };
                if let Some(e) = entry
                    && e.flags & (7 | OWN | WRITE_FORBIDDEN) == DATA | OWN
                    && store.matches_own_version(&e.version)
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
        let entry = state.entries.find(target, &self.first)?;
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
    #[inline(always)]
    pub(in crate::bytecode) fn write_hot(
        &self,
        object: &ObjectRef,
        value: &Value,
    ) -> Option<OwnDataPropertyWrite> {
        // Typed-loop hoisting requires writes to preserve property structure.
        if let Some(entry) = self.hot_mono()
            && entry.flags & (7 | OWN | WRITABLE | WRITE_FORBIDDEN) == DATA | OWN | WRITABLE
            && let Some(result) =
                NamedTarget::Object(object).write_matched_data(&entry.version, entry.slot, value)
        {
            return Some(result);
        }
        self.write_target(NamedTarget::Object(object), value, false)
    }
}
