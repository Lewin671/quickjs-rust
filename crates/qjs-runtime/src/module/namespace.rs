//! Module Namespace exotic objects (`import * as ns`).
//!
//! A module namespace object exposes the module's exported names as own
//! properties in sorted order, carries `Symbol.toStringTag` "Module", and is
//! sealed: new properties cannot be added, existing ones cannot be reassigned
//! or deleted. It is approximated here with an ordinary sealed object whose
//! data properties are non-writable and non-configurable; the property values
//! are a snapshot taken after the exporting module has finished evaluating.

use crate::{ObjectRef, Property, Value};

/// Builds a sealed namespace object from `bindings`, a list of
/// `(export_name, value)` pairs. Names are installed in sorted order.
pub(super) fn build_namespace(mut bindings: Vec<(String, Value)>) -> Value {
    bindings.sort_by(|(left, _), (right, _)| left.cmp(right));
    // Namespace objects have a null [[Prototype]].
    let namespace = ObjectRef::new(std::collections::HashMap::new());
    let _ = namespace.set_prototype(None);
    for (name, value) in bindings {
        // Non-writable + non-configurable so reassignment and deletion of an
        // existing export fail, matching the sealed namespace behavior.
        namespace.define_property(name, Property::data(value, true, false, false));
    }
    namespace.set_to_string_tag("Module");
    namespace.prevent_extensions();
    Value::Object(namespace)
}
