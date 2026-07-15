mod arguments;
mod call;
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

pub(crate) const CROSS_REALM_TYPE_ERROR_PROTOTYPE: &str = "__quickjsRustRealmTypeErrorPrototype";

pub(crate) const CROSS_REALM_THROW_TYPE_ERROR_INTRINSIC: &str = "__quickjsRustRealmThrowTypeError";

pub(crate) use arguments::{native_mapped_argument_get, native_mapped_argument_set};
pub(crate) use call::{
    call_function, construct_function, ensure_constructor, initialize_instance_fields,
    is_direct_leaf_function, try_call_direct_leaf_function,
};
pub(crate) fn is_call_frame_binding(name: &str) -> bool {
    matches!(
        name,
        crate::GLOBAL_THIS_BINDING
            | crate::DIRECT_EVAL_STRICT_BINDING
            | crate::DIRECT_EVAL_ARGUMENTS_BINDING
            | crate::DIRECT_EVAL_FUNCTION_CONTEXT_BINDING
            | crate::FIELD_INITIALIZER_EVAL_BINDING
            | crate::HOME_OBJECT_BINDING
            | crate::NEW_TARGET_BINDING
            | crate::SUPER_CONSTRUCTOR_BINDING
            | crate::ACTIVE_CONSTRUCTOR_BINDING
            | "this"
            | "arguments"
    )
}
#[allow(unused_imports)]
pub(crate) use env::{CallEnv, DynamicBindings, ModuleImports, Realm, new_realm};
pub(crate) use install::install_function;
pub(crate) use local_names::{
    collect_function_local_names, parameter_argument_binding_name, parameter_binding_name,
    rest_parameter_argument_binding_name, rest_parameter_binding_name,
};
pub(crate) use native_kind::NativeFunction;
pub(crate) use prototype::{
    ASYNC_GENERATOR_FUNCTION_REALM_PROTOTYPE, GENERATOR_FUNCTION_REALM_PROTOTYPE,
    apply_dense_native_fast_path, function_call_this, native_async_function_constructor,
    native_async_generator_function_constructor, native_function, native_function_prototype_apply,
    native_function_prototype_bind, native_function_prototype_call,
    native_function_prototype_has_instance, native_function_prototype_to_string,
    native_generator_function, native_realm_throw_type_error, native_throw_type_error,
};
pub(crate) use strict::is_strict_function_body;
pub(crate) use upvalue::Upvalue;
pub(crate) use value::{
    CompiledUserFunction, Function, InstanceElementInitializer, InstanceFieldInitializer,
    InstancePrivateElement, PrivateFieldInit,
};
