//! Operation entries for core; bound once when a native is created.

use super::CallInfo;
use crate::{RuntimeError, Value, bigint, boolean, global, symbol};

pub(super) fn boolean(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    boolean::native_boolean(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
    )
}

pub(super) fn boolean_prototype_to_string(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    boolean::native_boolean_prototype_to_string(info.this_value.clone())
}

pub(super) fn boolean_prototype_value_of(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    boolean::native_boolean_prototype_value_of(info.this_value.clone())
}

pub(super) fn decode_uri(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_global_decode_uri(argument_values, env)
}

pub(super) fn decode_uri_component(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_global_decode_uri_component(argument_values, env)
}

pub(super) fn encode_uri(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_global_encode_uri(argument_values, env)
}

pub(super) fn encode_uri_component(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_global_encode_uri_component(argument_values, env)
}

pub(super) fn global_is_finite(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_global_is_finite(argument_values, env)
}

pub(super) fn global_is_na_n(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_global_is_nan(argument_values, env)
}

pub(super) fn test262_assert_same_value(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_test262_assert_same_value(argument_values)
}

pub(super) fn test262_verify_property(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_test262_verify_property(argument_values, env)
}

pub(super) fn test262_build_string(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_test262_build_string(argument_values, env)
}

pub(super) fn test262_to_numbers(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_test262_to_numbers(env.realm(), argument_values)
}

pub(super) fn test262_compare_array(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_test262_compare_array(argument_values, env)
}

pub(super) fn test262_assert_iterator_result(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_test262_assert_iterator_result(argument_values, env)
}

pub(super) fn test262_assert_packed_array(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_test262_assert_packed_array(argument_values, env)
}

pub(super) fn test262_assert_null_proto_mutable_object(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_test262_assert_null_proto_mutable_object(argument_values)
}

pub(super) fn test262_assert_native_fn(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::test262_host::native_assert_native_function(argument_values)
}

pub(super) fn test262_assert_reg_exp_source_loop(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::test262_host::native_assert_regexp_source_loop(argument_values)
}

pub(super) fn test262_assert_reg_exp_whitespace_loop(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::test262_host::native_assert_regexp_whitespace_loop(env)
}

pub(super) fn test262_assert_uri_loop(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::test262_host::native_assert_uri_loop(argument_values)
}

pub(super) fn test262_assert_string_substr_number_loop(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::test262_host::native_assert_string_substr_number_loop(env)
}

pub(super) fn test262_assert_array_buffer_slice_to_immutable_argument_coercion(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::test262_host::native_assert_array_buffer_slice_to_immutable_argument_coercion(env)
}

pub(super) fn test262_assert_line_comment_unicode_loop(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::test262_host::native_assert_line_comment_unicode_loop()
}

pub(super) fn print(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_global_print(argument_values, env)
}

pub(super) fn is_html_dda(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    Ok(crate::html_dda::native_is_html_dda())
}

pub(super) fn big_int(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    bigint::native_bigint(argument_values, info.is_construct, env)
}

pub(super) fn big_int_as_int_n(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    bigint::native_bigint_as_int_n(argument_values, env)
}

pub(super) fn big_int_as_uint_n(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    bigint::native_bigint_as_uint_n(argument_values, env)
}

pub(super) fn big_int_prototype_to_string(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    bigint::native_bigint_prototype_to_string(info.this_value.clone(), argument_values, env)
}

pub(super) fn big_int_prototype_to_locale_string(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    bigint::native_bigint_prototype_to_string(info.this_value.clone(), &[], env)
}

pub(super) fn big_int_prototype_value_of(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    bigint::native_bigint_prototype_value_of(info.this_value.clone())
}

pub(super) fn eval(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_global_eval(argument_values, env)
}

pub(super) fn eval_script(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_eval_script(argument_values, env)
}

pub(super) fn proxy(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::proxy::native_proxy(argument_values, info.is_construct)
}

pub(super) fn proxy_revocable(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::proxy::native_proxy_revocable(env.realm(), argument_values, env.realm().atoms())
}

pub(super) fn proxy_revoke(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::proxy::native_proxy_revoke(info.callee)
}

pub(super) fn escape(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_global_escape(argument_values, env)
}

pub(super) fn unescape(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    global::native_global_unescape(argument_values, env)
}

pub(super) fn symbol(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    symbol::native_symbol(info.callee, argument_values, info.is_construct, env)
}

pub(super) fn symbol_for(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    symbol::native_symbol_for(argument_values, env)
}

pub(super) fn symbol_key_for(
    env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    symbol::native_symbol_key_for(argument_values, env)
}

pub(super) fn symbol_prototype_description(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    symbol::native_symbol_prototype_description(info.this_value.clone())
}

pub(super) fn symbol_prototype_to_primitive(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    symbol::native_symbol_prototype_to_primitive(info.this_value.clone())
}

pub(super) fn symbol_prototype_to_string(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    symbol::native_symbol_prototype_to_string(info.this_value.clone())
}

pub(super) fn symbol_prototype_value_of(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    symbol::native_symbol_prototype_value_of(info.this_value.clone())
}

pub(super) fn function(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::function::native_function(
        info.callee,
        info.this_value.clone(),
        argument_values,
        info.is_construct,
        env,
    )
}

pub(super) fn function_prototype(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    Ok(Value::Undefined)
}

pub(super) fn generator_function(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::function::native_generator_function(info.callee, argument_values, env)
}

pub(super) fn async_function(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::function::native_async_function_constructor(info.callee, argument_values, env)
}

pub(super) fn async_generator_function(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::function::native_async_generator_function_constructor(info.callee, argument_values, env)
}

pub(super) fn function_prototype_apply(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::function::native_function_prototype_apply(info.this_value.clone(), argument_values, env)
}

pub(super) fn function_prototype_bind(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::function::native_function_prototype_bind(info.this_value.clone(), argument_values, env)
}

pub(super) fn function_prototype_call(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::function::native_function_prototype_call(info.this_value.clone(), argument_values, env)
}

pub(super) fn function_prototype_has_instance(
    env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::function::native_function_prototype_has_instance(
        info.this_value.clone(),
        argument_values,
        env,
    )
}

pub(super) fn iterator_prototype_iterator(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    Ok(info.this_value.clone())
}

pub(super) fn species_getter(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    Ok(info.this_value.clone())
}

pub(super) fn function_prototype_to_string(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::function::native_function_prototype_to_string(info.this_value.clone())
}

pub(super) fn mapped_argument_get(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::function::native_mapped_argument_get(info.callee)
}

pub(super) fn mapped_argument_set(
    _env: &mut crate::context::Context<'_>,
    info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::function::native_mapped_argument_set(info.callee, argument_values)
}

pub(super) fn throw_type_error(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    _argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::function::native_throw_type_error()
}

pub(super) fn realm_throw_type_error(
    _env: &mut crate::context::Context<'_>,
    _info: CallInfo<'_>,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    crate::function::native_realm_throw_type_error(argument_values)
}
