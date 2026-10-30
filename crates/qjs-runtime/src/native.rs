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
/// this boundary. Only short Cell operations occur here, never an Executor or
/// RefCell borrow. Rc realm ownership keeps this synchronous state on one thread.
struct NativeBoundary<'a>(&'a crate::function::Realm);

impl<'a> NativeBoundary<'a> {
    const MAX_DEPTH: usize = 256;
    const MAX_STACK_BYTES: usize = 1024 * 1024;

    fn enter(realm: &'a crate::function::Realm) -> Result<Self, RuntimeError> {
        // Observe addresses only: no pointer is retained or dereferenced. The
        // absolute distance works for either stack growth direction and accounts
        // for different Rust frame sizes across profiles/callback executors.
        let marker = 0_u8;
        let here = std::ptr::from_ref(&marker).addr();
        let depth = realm.native_depth.get();
        if depth >= Self::MAX_DEPTH
            || (depth != 0
                && realm.native_stack_anchor.get().abs_diff(here) >= Self::MAX_STACK_BYTES)
        {
            return Err(RuntimeError {
                thrown: None,
                message: "RangeError: native call stack limit exceeded".to_owned(),
            });
        }
        if depth == 0 {
            realm.native_stack_anchor.set(here);
        }
        realm.native_depth.set(depth + 1);
        Ok(Self(realm))
    }
}

impl Drop for NativeBoundary<'_> {
    fn drop(&mut self) {
        let depth = self.0.native_depth.get() - 1;
        self.0.native_depth.set(depth);
        if depth == 0 {
            self.0.native_stack_anchor.set(0);
        }
    }
}
