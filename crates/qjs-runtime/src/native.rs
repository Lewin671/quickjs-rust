//! Native ABI and synchronous entry boundary. Installation binds one operation
//! entry; calls borrow immutable arguments and return owning values. Semantic
//! implementations stay in their builtin families. CallEnv is the service
//! authority until S3b replaces its parameter with Context.

#[cfg(feature = "agents")]
mod agent;
mod array_buffers;
mod arrays;
mod atomics;
mod core;
mod data_views;
mod date;
mod disposable_stacks;
mod errors;
mod finalization_registries;
mod json;
mod maps;
mod math;
mod numbers;
mod objects;
mod promises;
mod reflect;
mod regexp;
mod sets;
mod strings;
mod typed_arrays;
mod weak_maps;
mod weak_refs;
mod weak_sets;

mod entry;
#[cfg(test)]
mod tests;

use crate::{CallEnv, Function, NativeFunction, RuntimeError, Value};

/// A native borrows the invocation metadata and arguments only for this call.
pub(crate) type NativeEntry =
    fn(&mut CallEnv, CallInfo<'_>, &[Value]) -> Result<Value, RuntimeError>;

/// Immutable call metadata. CallEnv supplies the active caller context;
/// callee retains captures and cross-realm intrinsic metadata.
/// Call versus construct is recorded without looking up new.target. Semantic
/// constructor helpers read its actual value from CallEnv only when needed.
#[derive(Clone, Copy)]
pub(crate) struct CallInfo<'a> {
    pub(crate) callee: &'a Function,
    pub(crate) this_value: &'a Value,
    pub(crate) is_construct: bool,
}

pub(crate) fn bind(native: NativeFunction) -> NativeEntry {
    entry::bind(native)
}

#[inline(always)]
pub(crate) fn call_native_function(
    entry: NativeEntry,
    function: &Function,
    this_value: &Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let info = CallInfo {
        callee: function,
        this_value,
        is_construct,
    };
    entry(env, info, argument_values)
}
