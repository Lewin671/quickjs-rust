mod call;
mod install;
mod local_names;
mod native_kind;
mod prototype;
mod strict;
mod value;

pub(crate) use call::{call_function, construct_function, ensure_constructor};
pub(crate) use call::{native_mapped_argument_get, native_mapped_argument_set};
pub(crate) use install::install_function;
pub(crate) use local_names::collect_function_local_names;
pub(crate) use native_kind::NativeFunction;
pub(crate) use prototype::{
    function_call_this, native_function, native_function_prototype_apply,
    native_function_prototype_bind, native_function_prototype_call,
    native_function_prototype_has_instance, native_function_prototype_to_string,
    native_throw_type_error,
};
pub(crate) use strict::is_strict_function_body;
pub(crate) use value::Function;
