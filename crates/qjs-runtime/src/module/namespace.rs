//! Module Namespace exotic objects (`import * as ns`).
//!
//! A module namespace object exposes the module's exported names as own
//! properties in sorted order, carries `Symbol.toStringTag` "Module", and is
//! sealed: new properties cannot be added, existing ones cannot be reassigned
//! or deleted. Export data descriptors read their value from the module's live
//! binding map when queried.

use crate::{CallEnv, ModuleNamespaceBindings, ObjectRef, Property, Value};

pub(super) fn empty_namespace(live_bindings: ModuleNamespaceBindings) -> ObjectRef {
    let namespace = ObjectRef::new(std::collections::HashMap::new());
    let _ = namespace.set_prototype(None);
    namespace.mark_module_namespace_exotic();
    namespace.set_module_namespace_bindings(live_bindings);
    namespace
}

pub(super) fn populate_namespace(
    namespace: &ObjectRef,
    bindings: &mut Vec<(String, Value)>,
    env: &CallEnv,
) {
    bindings.sort_by(|(left, _), (right, _)| left.cmp(right));
    for (name, value) in bindings.drain(..) {
        // Non-writable + non-configurable so reassignment and deletion of an
        // existing export fail, matching the sealed namespace behavior.
        namespace.define_property(name, Property::data(value, true, false, false));
    }
    // A module namespace carries an own `@@toStringTag` data property "Module"
    // (writable:false, enumerable:false, configurable:false) — unlike the
    // configurable form on ordinary prototypes. The internal slot below also
    // keeps `Object.prototype.toString` reporting "[object Module]".
    if let Some(symbol) = crate::symbol::to_string_tag_symbol(env) {
        namespace.define_symbol_property(
            symbol,
            Property::data(
                Value::String("Module".to_owned().into()),
                false,
                false,
                false,
            ),
        );
    }
    namespace.set_to_string_tag("Module");
    namespace.prevent_extensions();
}
