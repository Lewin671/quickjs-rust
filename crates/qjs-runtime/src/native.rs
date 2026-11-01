//! Native ABI and synchronous entry boundary. Installation binds one operation
//! entry; calls borrow immutable arguments and return owning values. Semantic
//! implementations stay in their builtin families. Context borrows services and
//! caller state; NativeSite borrows call info and the distinct construct target.

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

use crate::{Function, NativeFunction, RuntimeError, Value};

/// A native borrows the invocation metadata and arguments only for this call.
pub(crate) type NativeEntry =
    fn(&mut crate::context::Context<'_>, CallInfo<'_>, &[Value]) -> Result<Value, RuntimeError>;

/// Immutable invocation metadata. Callee and receiver remain borrowed from their
/// S3a owners; NativeSite selects callee realm and target only when queried.
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
    info: CallInfo<'_>,
    argument_values: &[Value],
    target: Option<&Value>,
    direct_eval: Option<&crate::context::DirectEvalSite<'_>>,
    env: &mut crate::context::Context<'_>,
) -> Result<Value, RuntimeError> {
    let mut native = env.with_native(&info, target, direct_eval);
    entry(&mut native, info, argument_values).map_err(|error| {
        // The private wide-register TDZ sentinel has no callable realm.
        // Its installed rejecting entry belongs to the active activation.
        if info.callee.native_kind() == Some(NativeFunction::UninitializedLexical) {
            native.native = None;
        }
        crate::error::capture_error_origin(error, &native)
    })
}
