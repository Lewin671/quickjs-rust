use std::collections::HashMap;

use crate::{Function, NativeFunction, RuntimeError, Value, boolean, global};

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
        _ => unreachable!("native function was not handled by its owning dispatcher"),
    }
}
