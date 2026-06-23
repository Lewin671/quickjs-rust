mod arguments;
mod call;
mod captures;
mod env;
mod install;
mod local_names;
mod native_kind;
mod prototype;
mod strict;
mod upvalue;
mod value;

/// Realm key under which the single shared %ThrowTypeError% intrinsic is
/// stashed. The `%`-delimited name cannot be spelled by a source identifier, so
/// it is invisible to user code while letting the arguments-object builder reuse
/// the exact function object installed on `Function.prototype`.
pub(crate) const THROW_TYPE_ERROR_INTRINSIC: &str = "%ThrowTypeError%";

/// Realm key for the shared `%Array.prototype.values%` intrinsic, reused so an
/// arguments object's `[Symbol.iterator]` is the *same* function object as
/// `Array.prototype.values` / `Array.prototype[Symbol.iterator]`.
pub(crate) const ARRAY_PROTO_VALUES_INTRINSIC: &str = "%ArrayProto_values%";

pub(crate) use arguments::{native_mapped_argument_get, native_mapped_argument_set};
pub(crate) use call::{
    call_function, construct_function, ensure_constructor, initialize_instance_fields,
};
#[allow(unused_imports)]
pub(crate) use env::{CallEnv, ModuleImports, Realm};
pub(crate) use install::install_function;
pub(crate) use local_names::{
    collect_function_local_names, is_internal_binding_name, parameter_argument_binding_name,
    parameter_binding_name, rest_parameter_argument_binding_name, rest_parameter_binding_name,
};
pub(crate) use native_kind::NativeFunction;
pub(crate) use prototype::{
    ASYNC_GENERATOR_FUNCTION_REALM_PROTOTYPE, apply_dense_native_fast_path, function_call_this,
    native_async_function_constructor, native_async_generator_function_constructor,
    native_function, native_function_prototype_apply, native_function_prototype_bind,
    native_function_prototype_call, native_function_prototype_has_instance,
    native_function_prototype_to_string, native_generator_function, native_throw_type_error,
};
pub(crate) use strict::is_strict_function_body;
pub(crate) use upvalue::Upvalue;
pub(crate) use value::{
    CompiledUserFunction, Function, InstanceElementInitializer, InstanceFieldInitializer,
    InstancePrivateElement, PrivateFieldInit,
};
