//! `new C(...)` on a built-in constructor, with `new.target` the constructor
//! itself, whose result can be built in one step. The general construct path
//! reads `C.prototype`, allocates an ordinary receiver and calls the native
//! with it; for these constructors `prototype` is non-writable and
//! non-configurable and the arguments below cannot run user code, so none of
//! those steps is observable.

use crate::{CallEnv, Function, NativeFunction, RuntimeError, Value};

/// The constructed object, or `None` for any other constructor or argument
/// shape, which takes the general path. The caller guarantees `new.target`
/// is `function`.
pub(crate) fn construct_intrinsic_directly(
    function: &Function,
    argument_values: &[Value],
    env: &CallEnv,
) -> Option<Result<Value, RuntimeError>> {
    if function.bound.is_some() {
        return None;
    }
    match function.native? {
        NativeFunction::RegExp => {
            crate::regexp::construct_regexp_from_strings(function, argument_values, env)
        }
        NativeFunction::String => {
            crate::string::construct_string_wrapper(function, argument_values, env).map(Ok)
        }
        _ => None,
    }
}
