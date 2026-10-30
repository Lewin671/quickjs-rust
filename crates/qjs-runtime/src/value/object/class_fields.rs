//! Installation of observer-free constant public class-field segments.
//!
//! Admission completes before mutation. Only ordinary storage with
//! configurable existing descriptors is admitted; every other receiver keeps
//! the sequential DefineOwnProperty path. No JavaScript runs under the borrow.

use std::rc::Rc;

use crate::{Property, Value};

use super::ObjectRef;

use crate::value::layout::{AppendRecipe, NamedStore};
use std::cell::RefCell;
type Plan = RefCell<Option<AppendRecipe>>;

fn define_field(store: &mut NamedStore, key: &str, value: &Value, plan: &Plan) -> bool {
    if let Some(recipe) = plan.borrow().as_ref()
        && recipe.apply(store, value.clone())
    {
        return recipe.is_index();
    }
    let before = store.version();
    let slot = store.len();
    let lease = store
        .domain()
        .find(key)
        .unwrap_or_else(|| store.domain().intern(key));
    let lookup = store.lookup(&lease).expect("own domain");
    let added = lookup.descriptor().is_none();
    lookup.define(Property::enumerable(value.clone()));
    if added && store.domain().ic_policy().enabled {
        *plan.borrow_mut() = store.append_recipe(before, slot);
    }
    added && lease.array_index().is_some()
}
fn ordinary(object: &ObjectRef) -> bool {
    object.named_kind() == Some(0) && object.is_extensible()
}
fn special(key: &str) -> bool {
    key == "globalThis" || key == "__quickjsRustDynamicFunctionRealm"
}
impl ObjectRef {
    /// One initialized field is published only after its initializer completes.
    pub(crate) fn install_shared_class_field(&self, key: &str, value: &Value, plan: &Plan) -> bool {
        if !ordinary(self) || special(key) {
            return false;
        }
        let mut store = self.0.properties.borrow_mut();
        // A recipe guard also proves absence; only the cold path probes by key.
        if let Some(recipe) = plan.borrow().as_ref()
            && recipe.apply(&mut store, value.clone())
        {
            self.note_cached_append(recipe.is_index());
            return true;
        }
        if store.get_text(key).is_some() {
            return false;
        }
        self.note_cached_append(define_field(&mut store, key, value, plan));
        true
    }
    /// Only observer-free primitive segments may share a borrow/reservation.
    /// All definitions still publish their prefixes in source order.
    pub(crate) fn install_constant_class_fields<'a>(
        &self,
        fields: impl ExactSizeIterator<Item = (&'a Rc<str>, &'a Value, &'a Plan)> + Clone,
    ) -> bool {
        if !ordinary(self) || fields.clone().any(|(key, _, _)| special(key)) {
            return false;
        }
        let mut store = self.0.properties.borrow_mut();
        if store.descriptors().any(|p| !p.configurable) {
            return false;
        }
        store.reserve(fields.len());
        for (key, value, plan) in fields {
            self.note_cached_append(define_field(&mut store, key, value, plan));
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use std::{collections::HashMap, rc::Rc};

    use crate::{ObjectRef, Property, Value};
    use std::cell::RefCell;

    #[test]
    fn declines_without_mutation_for_descriptor_boundaries() {
        let domain = crate::property::atom::AtomDomain::new();
        let a: Rc<str> = "a".into();
        let b: Rc<str> = "b".into();
        let value = Value::Number(2.0);
        let plan_a = RefCell::default();
        let plan_b = RefCell::default();
        let fields = [(&a, &value, &plan_a), (&b, &value, &plan_b)];
        let object = ObjectRef::new(HashMap::new(), &domain);
        let mut fixed = Property::enumerable(Value::Number(1.0));
        fixed.configurable = false;
        object.define_property("b".into(), fixed);
        assert!(!object.install_constant_class_fields(fields.into_iter()));
        assert!(!object.has_own_property("a"));
        assert_eq!(object.get("b"), Some(Value::Number(1.0)));
    }

    #[test]
    fn duplicates_keep_slots_and_invalidate_cached_layouts() {
        let domain = crate::property::atom::AtomDomain::new();
        let object = ObjectRef::new(HashMap::new(), &domain);
        let key: Rc<str> = "0".into();
        let first = Value::Number(1.0);
        let last = Value::Number(2.0);
        let layout = object.layout_revision();
        let plan_a = RefCell::default();
        let plan_b = RefCell::default();
        assert!(object.install_constant_class_fields(
            [(&key, &first, &plan_a), (&key, &last, &plan_b)].into_iter()
        ));
        assert_eq!(object.get("0"), Some(last));
        assert_eq!(object.0.index_property_count.get(), 1);
        assert_ne!(object.layout_revision(), layout);
    }
}
