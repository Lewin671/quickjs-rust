//! Runtime elision of compiled reference sites (`reference_plan`).
//!
//! B3b2 selects an identifier reference before its RHS. A global Name site
//! may skip that selection, and keep no saved reference, exactly when one of
//! these holds at its start:
//!
//! - its global HasBinding is already true without observable work: the name
//!   is a global lexical, a module import, or an own property of the ordinary
//!   global object. A bound global-environment reference and a late
//!   resolution then run the same steps at PutValue/GetValue (ES2025
//!   9.1.1.4.5 SetMutableBinding consults the declarative record, then the
//!   object record's HasProperty), whatever the RHS deletes or declares;
//! - the site reads its reference at its own start (compound/update/append):
//!   an unresolvable read throws there, before any RHS effect.
//!
//! Every other Name start still selects (and saves) its reference, so an
//! originally unresolvable name keeps its unresolvability even if the RHS
//! creates a property or a global lexical. Proofs are cached per plan and
//! hold structural facts only: realm/import identity, the global object's
//! layout version and the realm binding-table generation. Value writes never
//! invalidate them; a lexical or non-configurable property proof is permanent
//! because neither binding can be removed.
use super::reference_plan::{ReferencePlan, ReferenceTarget};
use crate::context::ActivationContext;
use std::rc::{Rc, Weak};

/// Per-site proof bits.
const BOUND: u8 = 1;
const IMMUTABLE: u8 = 1 << 1;

#[derive(Clone, Debug)]
pub(super) struct AdmissionCache {
    realm: Weak<crate::context::RealmState>,
    /// `None` for a creation without module imports.
    imports: Option<Weak<crate::context::ModuleImportMap>>,
    generation: u64,
    layout: crate::value::layout::LayoutVersion,
    sites: Box<[u8]>,
    /// Every checked site is permanently bound: no activation needs events.
    skip: bool,
}

impl AdmissionCache {
    fn same_identity(&self, env: &ActivationContext) -> bool {
        if Weak::as_ptr(&self.realm) != Rc::as_ptr(env.realm()) {
            return false;
        }
        let imports = env.creation.imports();
        match &self.imports {
            None => imports.is_empty(),
            Some(cached) => Weak::as_ptr(cached) == Rc::as_ptr(imports),
        }
    }

    fn same_structure(&self, env: &ActivationContext) -> bool {
        let realm = env.realm();
        self.generation == realm.binding_cells().generation()
            && realm
                .global_object()
                .is_some_and(|global| global.layout_matches(&self.layout))
    }
}

impl ReferencePlan {
    /// The activation's binding chain is static between this body and the
    /// global environment, so a Name site can only resolve globally.
    fn simple_context(&self, env: &ActivationContext) -> bool {
        env.scope_cursor().is_none()
            && (!self.sloppy_eval || self.global_code || env.has_global_binding_root())
    }

    /// Whether this activation needs the VM's dynamic reference resolver for
    /// some site. A body admitted elsewhere executes checked starts itself.
    pub(super) fn needs_dynamic_resolution(&self, env: &ActivationContext) -> bool {
        self.dynamic || !self.sites.is_empty() && !self.simple_context(env)
    }

    /// Whether the whole activation may run without any reference event.
    pub(super) fn can_skip(&self, env: &ActivationContext) -> bool {
        if self.needs_dynamic_resolution(env) {
            crate::diagnostics::count!(reference_admission_dynamic);
            return false;
        }
        if !self.has_checked {
            return true;
        }
        if !env.can_cache_realm_globals() {
            let skip = self
                .sites
                .iter()
                .filter(|site| site.checked)
                .all(|site| uncached_immutable(&site.name, env));
            if !skip {
                crate::diagnostics::count!(reference_admission_checkpoints);
            }
            return skip;
        }
        {
            let cache = self.admission.borrow();
            if let Some(cache) = cache.as_ref()
                && cache.same_identity(env)
                && (cache.skip || cache.same_structure(env))
            {
                crate::diagnostics::count!(reference_admission_cache_hits);
                if !cache.skip {
                    crate::diagnostics::count!(reference_admission_checkpoints);
                }
                return cache.skip;
            }
        }
        let skip = self.prove(env).skip;
        if !skip {
            crate::diagnostics::count!(reference_admission_checkpoints);
        }
        skip
    }

    /// Whether selection at `index`'s start may be elided now (see module
    /// header). False sends the start through full ordered resolution.
    pub(super) fn start_elidable(&self, index: usize, env: &ActivationContext) -> bool {
        let site = &self.sites[index];
        match site.target {
            ReferenceTarget::With | ReferenceTarget::EvalVariable | ReferenceTarget::Parameter => {
                false
            }
            _ if !self.simple_context(env) => false,
            ReferenceTarget::Received => true,
            ReferenceTarget::Name => !site.checked || self.name_bound(index, env),
        }
    }

    /// Whether every start of `event` may be elided now.
    #[inline]
    pub(super) fn event_starts_elidable(&self, event: usize, env: &ActivationContext) -> bool {
        // Every checked site permanently bound: so is each start here.
        if let Some(cache) = self.admission.borrow().as_ref()
            && cache.skip
            && cache.same_identity(env)
            && self.simple_context(env)
        {
            return true;
        }
        self.each_start_elidable(event, env)
    }

    #[inline(never)]
    fn each_start_elidable(&self, event: usize, env: &ActivationContext) -> bool {
        let elidable = self.events[event]
            .starts
            .iter()
            .all(|&index| self.start_elidable(index, env));
        if !elidable {
            crate::diagnostics::count!(reference_checkpoint_selections);
        }
        elidable
    }

    /// Selects every start of `event` that cannot be elided, before any RHS
    /// effect, into the activation's saved references. Used by a tier that
    /// runs checkpoints without a VM frame; checked sites resolve globally.
    #[cold]
    #[inline(never)]
    pub(super) fn select_checked_starts(
        &self,
        event: usize,
        env: &mut crate::context::Context<'_>,
    ) -> Result<(), crate::RuntimeError> {
        for &index in &self.events[event].starts {
            if self.start_elidable(index, env.activation) {
                continue;
            }
            let site = &self.sites[index];
            debug_assert!(site.checked && self.simple_context(env.activation));
            let reference = crate::binding::GlobalReference::select(&site.name, site.strict, env)?;
            env.activation
                .saved_references
                .get_or_insert_with(Default::default)
                .push((
                    site.lifetime,
                    crate::binding::ResolvedReference::Global(reference),
                ));
        }
        Ok(())
    }

    /// Takes the reference a checkpoint saved for the site storing at
    /// `store_pc`, with that site's strictness.
    pub(super) fn take_saved_store(
        &self,
        store_pc: usize,
        env: &mut ActivationContext,
    ) -> Option<(crate::binding::ResolvedReference, bool)> {
        let event = self
            .events
            .binary_search_by_key(&store_pc, |event| event.pc)
            .ok()?;
        let site = &self.sites[self.events[event].store?];
        let saved = env.saved_references.as_mut()?;
        let position = saved
            .iter()
            .position(|(lifetime, _)| *lifetime == site.lifetime)?;
        let (_, reference) = saved.swap_remove(position);
        if saved.is_empty() {
            env.saved_references = None;
        }
        Some((reference, site.strict))
    }

    /// A loop accelerator runs `[start, end]` without user code; a site bound
    /// at entry therefore stays bound for every iteration it runs.
    pub(super) fn can_skip_range(&self, start: usize, end: usize, env: &ActivationContext) -> bool {
        let first = self.sites.partition_point(|site| site.start < start);
        (first..self.sites.len())
            .take_while(|&index| self.sites[index].start <= end)
            .all(|index| self.start_elidable(index, env))
            && self
                .lifetimes
                .iter()
                .filter(|life| life.temporary.is_some())
                .all(|life| life.start > end || life.end <= start)
    }

    fn name_bound(&self, index: usize, env: &ActivationContext) -> bool {
        if !env.can_cache_realm_globals() {
            return uncached_immutable(&self.sites[index].name, env);
        }
        {
            let cache = self.admission.borrow();
            if let Some(cache) = cache.as_ref()
                && cache.same_identity(env)
            {
                let proof = cache.sites[index];
                if proof & IMMUTABLE != 0 {
                    crate::diagnostics::count!(reference_admission_cache_hits);
                    return true;
                }
                if cache.same_structure(env) {
                    crate::diagnostics::count!(reference_admission_cache_hits);
                    return proof & BOUND != 0;
                }
            }
        }
        self.prove(env).sites[index] & BOUND != 0
    }

    /// Recomputes every site's proof by name. Lookups may materialize lazy
    /// global properties, so the layout is captured after all of them.
    #[cold]
    #[inline(never)]
    fn prove(&self, env: &ActivationContext) -> std::cell::Ref<'_, AdmissionCache> {
        crate::diagnostics::count!(reference_admission_cache_misses);
        let realm = env.realm();
        let global = realm.global_object();
        let sites: Box<[u8]> = self
            .sites
            .iter()
            .map(|site| match site.target {
                ReferenceTarget::Received => BOUND | IMMUTABLE,
                ReferenceTarget::Name
                    if env.has_module_import(&site.name)
                        || env.is_global_lexical_binding(&site.name) =>
                {
                    BOUND | IMMUTABLE
                }
                ReferenceTarget::Name => global
                    .and_then(|global| global.own_property(&site.name))
                    .map_or(0, |property| {
                        if property.configurable {
                            BOUND
                        } else {
                            BOUND | IMMUTABLE
                        }
                    }),
                _ => 0,
            })
            .collect();
        let skip = self
            .sites
            .iter()
            .zip(sites.iter())
            .all(|(site, proof)| !site.checked || proof & IMMUTABLE != 0);
        let imports = env.creation.imports();
        *self.admission.borrow_mut() = Some(AdmissionCache {
            realm: Rc::downgrade(realm),
            imports: (!imports.is_empty()).then(|| Rc::downgrade(imports)),
            generation: realm.binding_cells().generation(),
            layout: global.map_or_else(
                crate::value::layout::LayoutVersion::empty,
                crate::ObjectRef::layout_revision,
            ),
            sites,
            skip,
        });
        std::cell::Ref::map(self.admission.borrow(), |cache| {
            cache.as_ref().expect("admission proof just stored")
        })
    }
}

/// A selected non-realm global object: only a non-configurable own property
/// is a proof that holds for the whole activation without a cache.
fn uncached_immutable(name: &str, env: &ActivationContext) -> bool {
    env.has_module_import(name)
        || matches!(env.selected_global_this(), Some(crate::Value::Object(object))
            if object.own_property(name).is_some_and(|property| !property.configurable))
}
