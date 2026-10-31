//! Cold resolution and bounded cache admission. No miss/update code is inlined
//! into typed, compact, wide or interpreter dispatch. All hooks stay outside borrows.
use super::*;
use crate::value::layout::NamedStore;
impl CacheData {
    #[cold]
    #[inline(never)]
    pub(super) fn resolve_read(
        &self,
        target: NamedTarget<'_>,
        name: &str,
        key: Option<&KeyLease>,
    ) -> Option<Resolution> {
        if let Some(key) = key {
            return resolve(target, name, Some(key));
        }
        let cached = self
            .state
            .borrow()
            .key
            .as_ref()
            .and_then(WeakKeyLease::upgrade)
            .filter(|key| target.owns_key(key));
        let key = if let Some(key) = cached {
            Some(key)
        } else {
            count_event(target, Event::Hash);
            let key = target.find_key(name);
            self.state.borrow_mut().key = key.as_ref().map(KeyLease::downgrade);
            key
        };
        resolve(target, name, key.as_ref())
    }
    #[cold]
    #[inline(never)]
    pub(super) fn read_target(
        &self,
        target: NamedTarget<'_>,
        name: &str,
        key: Option<&KeyLease>,
    ) -> Option<ReadOutcome> {
        if matches!(self.state.borrow().entries, Entries::Mega) {
            return self.resolve_read(target, name, key)?.fetch(target);
        }
        if let Some(value) = self.cached(target, false) {
            return Some(value);
        }
        if let Some(value) = self.cached(target, true) {
            return Some(value);
        }
        let resolution = self.resolve_read(target, name, key)?;
        self.record_resolution(target, name, &resolution);
        resolution.fetch(target)
    }
    #[cold]
    #[inline(never)]
    pub(super) fn record_resolution(
        &self,
        target: NamedTarget<'_>,
        name: &str,
        resolution: &Resolution,
    ) {
        if matches!(self.state.borrow().entries, Entries::Mega) {
            return;
        }
        let policy = target.policy();
        if !policy.enabled {
            return;
        }
        // Ordinary empty receivers defer roots until admission. Publish the
        // root once so inherited hits use the normal direct layout guard.
        // Array/function empty authorities already carry their final root.
        match target {
            NamedTarget::Object(o) => o.with_named_mut(NamedStore::materialize_layout),
            NamedTarget::Map(m) => m
                .object_ref()
                .with_named_mut(NamedStore::materialize_layout),
            NamedTarget::Set(s) => s
                .object_ref()
                .with_named_mut(NamedStore::materialize_layout),
            _ => {}
        }
        let Some(version) = target.version() else {
            return;
        };
        let mut flags = if resolution.inherited { 0 } else { OWN };
        if resolution.first_holder {
            flags |= FIRST_HOLDER;
        }
        let slot = match resolution.probe {
            Probe::Data { slot, writable } => {
                flags |= DATA | if writable { WRITABLE } else { 0 };
                slot
            }
            Probe::Accessor { slot } => {
                flags |= ACCESSOR;
                slot
            }
            Probe::Missing => {
                flags |= MISSING;
                0
            }
            Probe::Virtual(h) => {
                flags |= VIRTUAL;
                match h {
                    Virtual::ArrayLength => 0,
                    Virtual::FunctionName => 1,
                    Virtual::FunctionLength => 2,
                    Virtual::FunctionPrototype => 3,
                    Virtual::FunctionCaller => 4,
                    Virtual::FunctionArguments => 5,
                }
            }
            Probe::Exotic => return,
        };
        if name == "globalThis" || name == "__quickjsRustDynamicFunctionRealm" {
            flags |= WRITE_FORBIDDEN
        }
        if policy.verify {
            flags |= VERIFY
        }
        let chain = if resolution.inherited {
            let Some(validity) = target.chain_dependency() else {
                return;
            };
            Some(Box::new(Chain {
                holder: resolution.holder.clone(),
                constant: resolution
                    .holder
                    .as_ref()
                    .and_then(|h| Constant::admit(h, resolution.probe)),
                validity,
            }))
        } else {
            None
        };
        let mut state = self.state.borrow_mut();
        #[cfg(debug_assertions)]
        if policy.verify {
            state.name = Some(name.to_owned())
        }
        self.first_active.set(false);
        state.entries.record(
            Entry {
                version,
                slot,
                flags,
                chain,
            },
            &self.first,
        );
        self.first_active
            .set(matches!(state.entries, Entries::MonoFirst));
    }
    #[cold]
    #[inline(never)]
    pub(in crate::bytecode) fn update(&self, object: &ObjectRef, name: &str, _value: &Value) {
        let target = NamedTarget::Object(object);
        if let Some(r) = self.resolve_read(target, name, None) {
            self.record_resolution(target, name, &r)
        }
    }
}
