use std::collections::HashMap;

use crate::{
    Function, NativeFunction, RuntimeError, Value, array, boolean, global, math, number, object,
    string,
};

pub(crate) fn call_native_function(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: Vec<Value>,
    is_construct: bool,
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match native {
        NativeFunction::Array => array::native_array(&argument_values),
        NativeFunction::ArrayIsArray => array::native_array_is_array(&argument_values),
        NativeFunction::ArrayPrototypeAt => {
            array::native_array_prototype_at(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypeConcat => {
            array::native_array_prototype_concat(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypeCopyWithin => {
            array::native_array_prototype_copy_within(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypeFill => {
            array::native_array_prototype_fill(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypeIncludes => {
            array::native_array_prototype_includes(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypeIndexOf => {
            array::native_array_prototype_index_of(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypeLastIndexOf => {
            array::native_array_prototype_last_index_of(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypeJoin => {
            array::native_array_prototype_join(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypePop => array::native_array_prototype_pop(this_value),
        NativeFunction::ArrayPrototypePush => {
            array::native_array_prototype_push(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypeReverse => array::native_array_prototype_reverse(this_value),
        NativeFunction::ArrayPrototypeShift => array::native_array_prototype_shift(this_value),
        NativeFunction::ArrayPrototypeSlice => {
            array::native_array_prototype_slice(this_value, &argument_values)
        }
        NativeFunction::ArrayPrototypeToString => {
            array::native_array_prototype_to_string(this_value)
        }
        NativeFunction::ArrayPrototypeUnshift => {
            array::native_array_prototype_unshift(this_value, &argument_values)
        }
        NativeFunction::Boolean => {
            boolean::native_boolean(function, this_value, &argument_values, is_construct)
        }
        NativeFunction::BooleanPrototypeToString => {
            boolean::native_boolean_prototype_to_string(this_value)
        }
        NativeFunction::BooleanPrototypeValueOf => {
            boolean::native_boolean_prototype_value_of(this_value)
        }
        NativeFunction::MathAbs => math::native_math_unary(&argument_values, f64::abs),
        NativeFunction::MathAcos => math::native_math_unary(&argument_values, f64::acos),
        NativeFunction::MathAcosh => math::native_math_unary(&argument_values, f64::acosh),
        NativeFunction::MathAsin => math::native_math_unary(&argument_values, f64::asin),
        NativeFunction::MathAsinh => math::native_math_unary(&argument_values, f64::asinh),
        NativeFunction::MathAtan => math::native_math_unary(&argument_values, f64::atan),
        NativeFunction::MathAtan2 => math::native_math_atan2(&argument_values),
        NativeFunction::MathAtanh => math::native_math_unary(&argument_values, f64::atanh),
        NativeFunction::MathCbrt => math::native_math_unary(&argument_values, f64::cbrt),
        NativeFunction::MathCeil => math::native_math_unary(&argument_values, f64::ceil),
        NativeFunction::MathClz32 => math::native_math_clz32(&argument_values),
        NativeFunction::MathCos => math::native_math_unary(&argument_values, f64::cos),
        NativeFunction::MathCosh => math::native_math_unary(&argument_values, f64::cosh),
        NativeFunction::MathExp => math::native_math_unary(&argument_values, f64::exp),
        NativeFunction::MathExpm1 => math::native_math_unary(&argument_values, f64::exp_m1),
        NativeFunction::MathFloor => math::native_math_unary(&argument_values, f64::floor),
        NativeFunction::MathFround => math::native_math_fround(&argument_values),
        NativeFunction::MathHypot => math::native_math_hypot(&argument_values),
        NativeFunction::MathImul => math::native_math_imul(&argument_values),
        NativeFunction::MathLog => math::native_math_unary(&argument_values, f64::ln),
        NativeFunction::MathLog1p => math::native_math_unary(&argument_values, f64::ln_1p),
        NativeFunction::MathLog10 => math::native_math_unary(&argument_values, f64::log10),
        NativeFunction::MathLog2 => math::native_math_unary(&argument_values, f64::log2),
        NativeFunction::MathMax => math::native_math_max(&argument_values),
        NativeFunction::MathMin => math::native_math_min(&argument_values),
        NativeFunction::MathPow => math::native_math_pow(&argument_values),
        NativeFunction::MathRound => math::native_math_round(&argument_values),
        NativeFunction::MathSign => math::native_math_sign(&argument_values),
        NativeFunction::MathSin => math::native_math_unary(&argument_values, f64::sin),
        NativeFunction::MathSinh => math::native_math_unary(&argument_values, f64::sinh),
        NativeFunction::MathSqrt => math::native_math_unary(&argument_values, f64::sqrt),
        NativeFunction::MathTan => math::native_math_unary(&argument_values, f64::tan),
        NativeFunction::MathTanh => math::native_math_unary(&argument_values, f64::tanh),
        NativeFunction::MathTrunc => math::native_math_unary(&argument_values, f64::trunc),
        NativeFunction::GlobalIsFinite => global::native_global_is_finite(&argument_values),
        NativeFunction::GlobalIsNaN => global::native_global_is_nan(&argument_values),
        NativeFunction::Number => {
            number::native_number(function, this_value, &argument_values, is_construct)
        }
        NativeFunction::NumberIsFinite => number::native_number_is_finite(&argument_values),
        NativeFunction::NumberIsInteger => number::native_number_is_integer(&argument_values),
        NativeFunction::NumberIsNaN => number::native_number_is_nan(&argument_values),
        NativeFunction::NumberIsSafeInteger => {
            number::native_number_is_safe_integer(&argument_values)
        }
        NativeFunction::NumberPrototypeToString => {
            number::native_number_prototype_to_string(this_value, &argument_values)
        }
        NativeFunction::NumberPrototypeValueOf => {
            number::native_number_prototype_value_of(this_value)
        }
        NativeFunction::ParseFloat => number::native_parse_float(&argument_values),
        NativeFunction::ParseInt => number::native_parse_int(&argument_values),
        NativeFunction::Object => {
            object::native_object(function, this_value, &argument_values, is_construct)
        }
        NativeFunction::ObjectAssign => object::native_object_assign(&argument_values),
        NativeFunction::ObjectCreate => object::native_object_create(&argument_values),
        NativeFunction::ObjectDefineProperties => {
            object::native_object_define_properties(&argument_values)
        }
        NativeFunction::ObjectDefineProperty => {
            object::native_object_define_property(&argument_values)
        }
        NativeFunction::ObjectGetOwnPropertyDescriptor => {
            object::native_object_get_own_property_descriptor(&argument_values, env)
        }
        NativeFunction::ObjectGetPrototypeOf => {
            object::native_object_get_prototype_of(&argument_values, env)
        }
        NativeFunction::ObjectGetOwnPropertyNames => {
            object::native_object_get_own_property_names(&argument_values)
        }
        NativeFunction::ObjectHasOwn => object::native_object_has_own(&argument_values),
        NativeFunction::ObjectKeys => object::native_object_keys(&argument_values),
        NativeFunction::ObjectPrototypeHasOwnProperty => {
            object::native_object_prototype_has_own_property(this_value, &argument_values)
        }
        NativeFunction::ObjectPrototypeIsPrototypeOf => {
            object::native_object_prototype_is_prototype_of(this_value, &argument_values, env)
        }
        NativeFunction::ObjectPrototypePropertyIsEnumerable => {
            object::native_object_prototype_property_is_enumerable(this_value, &argument_values)
        }
        NativeFunction::ObjectPrototypeToString => {
            object::native_object_prototype_to_string(this_value)
        }
        NativeFunction::ObjectPrototypeValueOf => {
            object::native_object_prototype_value_of(this_value)
        }
        NativeFunction::String => string::native_string(&argument_values),
        NativeFunction::StringFromCharCode => {
            string::native_string_from_char_code(&argument_values)
        }
        NativeFunction::StringPrototypeAt => {
            string::native_string_prototype_at(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeCharAt => {
            string::native_string_prototype_char_at(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeCharCodeAt => {
            string::native_string_prototype_char_code_at(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeCodePointAt => {
            string::native_string_prototype_code_point_at(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeConcat => {
            string::native_string_prototype_concat(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeEndsWith => {
            string::native_string_prototype_ends_with(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeIncludes => {
            string::native_string_prototype_includes(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeIndexOf => {
            string::native_string_prototype_index_of(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeLastIndexOf => {
            string::native_string_prototype_last_index_of(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypePadEnd => string::native_string_prototype_pad(
            this_value,
            &argument_values,
            env,
            string::StringPadKind::End,
        ),
        NativeFunction::StringPrototypePadStart => string::native_string_prototype_pad(
            this_value,
            &argument_values,
            env,
            string::StringPadKind::Start,
        ),
        NativeFunction::StringPrototypeRepeat => {
            string::native_string_prototype_repeat(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeSlice => {
            string::native_string_prototype_slice(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeSplit => {
            string::native_string_prototype_split(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeStartsWith => {
            string::native_string_prototype_starts_with(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeSubstring => {
            string::native_string_prototype_substring(this_value, &argument_values, env)
        }
        NativeFunction::StringPrototypeToLowerCase => {
            string::native_string_prototype_to_lower_case(this_value, env)
        }
        NativeFunction::StringPrototypeTrim => {
            string::native_string_prototype_trim(this_value, env)
        }
        NativeFunction::StringPrototypeTrimEnd => {
            string::native_string_prototype_trim_end(this_value, env)
        }
        NativeFunction::StringPrototypeTrimStart => {
            string::native_string_prototype_trim_start(this_value, env)
        }
        NativeFunction::StringPrototypeToString | NativeFunction::StringPrototypeValueOf => {
            string::native_string_prototype_to_string(this_value, env)
        }
        NativeFunction::StringPrototypeToUpperCase => {
            string::native_string_prototype_to_upper_case(this_value, env)
        }
    }
}
