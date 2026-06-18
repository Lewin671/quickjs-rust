use crate::CallEnv;
use crate::{Function, NativeFunction, RuntimeError, Value, bigint, boolean, global, symbol};

pub(super) fn call_core_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    match native {
        NativeFunction::Boolean => {
            boolean::native_boolean(function, this_value, argument_values, is_construct)
        }
        NativeFunction::BooleanPrototypeToString => {
            boolean::native_boolean_prototype_to_string(this_value)
        }
        NativeFunction::BooleanPrototypeValueOf => {
            boolean::native_boolean_prototype_value_of(this_value)
        }
        NativeFunction::DecodeUri => global::native_global_decode_uri(argument_values, env),
        NativeFunction::DecodeUriComponent => {
            global::native_global_decode_uri_component(argument_values, env)
        }
        NativeFunction::EncodeUri => global::native_global_encode_uri(argument_values, env),
        NativeFunction::EncodeUriComponent => {
            global::native_global_encode_uri_component(argument_values, env)
        }
        NativeFunction::GlobalIsFinite => global::native_global_is_finite(argument_values, env),
        NativeFunction::GlobalIsNaN => global::native_global_is_nan(argument_values, env),
        NativeFunction::Test262AssertSameValue => {
            global::native_test262_assert_same_value(argument_values)
        }
        NativeFunction::Print => global::native_global_print(argument_values, env),
        NativeFunction::IsHtmlDda => Ok(crate::html_dda::native_is_html_dda()),
        NativeFunction::BigInt => bigint::native_bigint(argument_values, is_construct, env),
        NativeFunction::BigIntAsIntN => bigint::native_bigint_as_int_n(argument_values, env),
        NativeFunction::BigIntAsUintN => bigint::native_bigint_as_uint_n(argument_values, env),
        NativeFunction::BigIntPrototypeToString => {
            bigint::native_bigint_prototype_to_string(this_value, argument_values, env)
        }
        NativeFunction::BigIntPrototypeToLocaleString => {
            bigint::native_bigint_prototype_to_string(this_value, &[], env)
        }
        NativeFunction::BigIntPrototypeValueOf => {
            bigint::native_bigint_prototype_value_of(this_value)
        }
        NativeFunction::Eval => global::native_global_eval(argument_values, env),
        NativeFunction::EvalScript => global::native_eval_script(argument_values, env),
        NativeFunction::Proxy => crate::proxy::native_proxy(argument_values, is_construct),
        NativeFunction::ProxyRevocable => crate::proxy::native_proxy_revocable(argument_values),
        NativeFunction::ProxyRevoke => crate::proxy::native_proxy_revoke(function),
        NativeFunction::Escape => global::native_global_escape(argument_values, env),
        NativeFunction::Unescape => global::native_global_unescape(argument_values, env),
        NativeFunction::Symbol => {
            symbol::native_symbol(function, argument_values, is_construct, env)
        }
        NativeFunction::SymbolFor => symbol::native_symbol_for(argument_values, env),
        NativeFunction::SymbolKeyFor => symbol::native_symbol_key_for(argument_values, env),
        NativeFunction::SymbolPrototypeDescription => {
            symbol::native_symbol_prototype_description(this_value)
        }
        NativeFunction::SymbolPrototypeToPrimitive => {
            symbol::native_symbol_prototype_to_primitive(this_value)
        }
        NativeFunction::SymbolPrototypeToString => {
            symbol::native_symbol_prototype_to_string(this_value)
        }
        NativeFunction::SymbolPrototypeValueOf => {
            symbol::native_symbol_prototype_value_of(this_value)
        }
        NativeFunction::Function => crate::function::native_function(
            function,
            this_value,
            argument_values,
            is_construct,
            env,
        ),
        NativeFunction::GeneratorFunction => {
            crate::function::native_generator_function(function, argument_values, env)
        }
        NativeFunction::AsyncFunction => {
            crate::function::native_async_function_constructor(argument_values, env)
        }
        NativeFunction::AsyncGeneratorFunction => {
            crate::function::native_async_generator_function_constructor(argument_values, env)
        }
        NativeFunction::FunctionPrototypeApply => {
            crate::function::native_function_prototype_apply(this_value, argument_values, env)
        }
        NativeFunction::FunctionPrototypeBind => {
            crate::function::native_function_prototype_bind(this_value, argument_values)
        }
        NativeFunction::FunctionPrototypeCall => {
            crate::function::native_function_prototype_call(this_value, argument_values, env)
        }
        NativeFunction::FunctionPrototypeHasInstance => {
            crate::function::native_function_prototype_has_instance(
                this_value,
                argument_values,
                env,
            )
        }
        NativeFunction::IteratorPrototypeIterator => Ok(this_value),
        NativeFunction::SpeciesGetter => Ok(this_value),
        NativeFunction::FunctionPrototypeToString => {
            crate::function::native_function_prototype_to_string(this_value)
        }
        NativeFunction::MappedArgumentGet => {
            crate::function::native_mapped_argument_get(argument_values, env)
        }
        NativeFunction::MappedArgumentSet => {
            crate::function::native_mapped_argument_set(argument_values, env)
        }
        NativeFunction::ThrowTypeError => crate::function::native_throw_type_error(),
        _ => unreachable!("native function was not handled by its owning dispatcher"),
    }
}
