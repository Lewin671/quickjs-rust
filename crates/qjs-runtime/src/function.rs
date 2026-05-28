mod call;
mod install;
mod native_kind;
mod prototype;
mod value;

pub(crate) use call::call_function;
pub(crate) use install::install_function;
pub(crate) use native_kind::NativeFunction;
pub(crate) use prototype::{
    native_function, native_function_prototype_apply, native_function_prototype_bind,
    native_function_prototype_call,
};
pub(crate) use value::Function;
