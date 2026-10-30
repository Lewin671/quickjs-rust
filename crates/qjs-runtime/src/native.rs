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

/// Immutable call metadata. The realm and service parameter identify the active
/// caller context; callee retains captures and cross-realm intrinsic metadata.
/// Only [[Construct]] supplies
/// new.target, even when an ordinary call happens inside a constructor.
#[derive(Clone, Copy)]
pub(crate) struct CallInfo<'a> {
    pub(crate) callee: &'a Function,
    pub(crate) this_value: &'a Value,
    pub(crate) new_target: Option<&'a Value>,
    pub(crate) realm: &'a crate::function::Realm,
}

pub(crate) fn bind(native: NativeFunction) -> NativeEntry {
    entry::bind(native)
}

pub(crate) fn call_native_function(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let new_target = is_construct.then(|| {
        env.get(crate::NEW_TARGET_BINDING)
            .unwrap_or_else(|| Value::Function(function.clone()))
    });
    let realm = env.realm_rc();
    let info = CallInfo {
        callee: function,
        this_value: &this_value,
        new_target: new_target.as_ref(),
        realm: &realm,
    };
    let entry = function
        .native_entry
        .expect("native installation binds an entry");
    let _boundary = NativeBoundary::enter(info.realm)?;
    entry(env, info, argument_values)
}

/// The caller's detached operand/register owner or host slice stays outside
/// this boundary. Only the depth cell is borrowed, never an Executor/RefCell.
struct NativeBoundary<'a>(&'a std::cell::Cell<usize>);

impl<'a> NativeBoundary<'a> {
    // Debug Rust frames are much larger than optimized frames. Keep the
    // synchronous boundary below their stack budget; S3d removes recursive JS
    // driver frames, but this stage still uses the existing executors.
    const MAX_DEPTH: usize = if cfg!(debug_assertions) { 32 } else { 256 };

    fn enter(realm: &'a crate::function::Realm) -> Result<Self, RuntimeError> {
        let depth = &realm.native_depth;
        if depth.get() >= Self::MAX_DEPTH {
            return Err(RuntimeError {
                thrown: None,
                message: "RangeError: native call depth limit exceeded".to_owned(),
            });
        }
        depth.set(depth.get() + 1);
        Ok(Self(depth))
    }
}

impl Drop for NativeBoundary<'_> {
    fn drop(&mut self) {
        self.0.set(self.0.get() - 1);
    }
}
