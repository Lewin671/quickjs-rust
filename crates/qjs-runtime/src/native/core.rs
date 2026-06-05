use std::collections::HashMap;

use crate::{Function, NativeFunction, RuntimeError, Value, boolean, global, symbol};

pub(super) fn call_core_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
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
        NativeFunction::GlobalIsFinite => global::native_global_is_finite(argument_values),
        NativeFunction::GlobalIsNaN => global::native_global_is_nan(argument_values),
        NativeFunction::Eval => global::native_global_eval(argument_values, env),
        NativeFunction::DecodeUri => global::native_decode_uri(argument_values, env),
        NativeFunction::DecodeUriComponent => {
            global::native_decode_uri_component(argument_values, env)
        }
        NativeFunction::EncodeUri => global::native_encode_uri(argument_values, env),
        NativeFunction::EncodeUriComponent => {
            global::native_encode_uri_component(argument_values, env)
        }
        NativeFunction::Symbol => symbol::native_symbol(function),
        NativeFunction::Function => crate::function::native_function(
            function,
            this_value,
            argument_values,
            is_construct,
            env,
        ),
        NativeFunction::FunctionPrototypeApply => {
            crate::function::native_function_prototype_apply(this_value, argument_values, env)
        }
        NativeFunction::FunctionPrototypeBind => {
            crate::function::native_function_prototype_bind(this_value, argument_values)
        }
        NativeFunction::FunctionPrototypeCall => {
            crate::function::native_function_prototype_call(this_value, argument_values, env)
        }
        NativeFunction::FunctionPrototypeToString => {
            crate::function::native_function_prototype_to_string(this_value)
        }
        _ => unreachable!("native function was not handled by its owning dispatcher"),
    }
}
