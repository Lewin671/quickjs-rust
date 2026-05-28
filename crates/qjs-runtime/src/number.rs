use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, function_prototype,
    inherited_object_prototype_property, to_int32, to_number,
};

const NUMBER_DATA_PROPERTY: &str = "\0NumberData";

pub(super) fn install_number(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let number_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    let number_function = Function::new_native(Some("Number"), 1, NativeFunction::Number, true);
    number_prototype.define_non_enumerable(NUMBER_DATA_PROPERTY.to_owned(), Value::Number(0.0));
    number_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(number_function.clone()),
    );
    number_prototype.define_non_enumerable(
        "toString".to_owned(),
        Value::Function(Function::new_native(
            Some("toString"),
            1,
            NativeFunction::NumberPrototypeToString,
            false,
        )),
    );
    number_prototype.define_non_enumerable(
        "valueOf".to_owned(),
        Value::Function(Function::new_native(
            Some("valueOf"),
            0,
            NativeFunction::NumberPrototypeValueOf,
            false,
        )),
    );
    number_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(number_prototype)),
    );
    define_number_constant(&number_function, "EPSILON", f64::EPSILON);
    define_number_constant(
        &number_function,
        "MAX_SAFE_INTEGER",
        9_007_199_254_740_991.0,
    );
    define_number_constant(&number_function, "MAX_VALUE", f64::MAX);
    define_number_constant(
        &number_function,
        "MIN_SAFE_INTEGER",
        -9_007_199_254_740_991.0,
    );
    define_number_constant(&number_function, "MIN_VALUE", f64::MIN_POSITIVE);
    define_number_constant(&number_function, "NaN", f64::NAN);
    define_number_constant(&number_function, "NEGATIVE_INFINITY", f64::NEG_INFINITY);
    define_number_constant(&number_function, "POSITIVE_INFINITY", f64::INFINITY);
    define_function_property(
        &number_function,
        "isFinite",
        1,
        NativeFunction::NumberIsFinite,
    );
    define_function_property(
        &number_function,
        "isInteger",
        1,
        NativeFunction::NumberIsInteger,
    );
    define_function_property(&number_function, "isNaN", 1, NativeFunction::NumberIsNaN);
    define_function_property(
        &number_function,
        "isSafeInteger",
        1,
        NativeFunction::NumberIsSafeInteger,
    );
    let parse_float_value = Value::Function(Function::new_native(
        Some("parseFloat"),
        1,
        NativeFunction::ParseFloat,
        false,
    ));
    let parse_int_value = Value::Function(Function::new_native(
        Some("parseInt"),
        2,
        NativeFunction::ParseInt,
        false,
    ));
    number_function.properties.borrow_mut().insert(
        "parseFloat".to_owned(),
        Property::non_enumerable(parse_float_value.clone()),
    );
    number_function.properties.borrow_mut().insert(
        "parseInt".to_owned(),
        Property::non_enumerable(parse_int_value.clone()),
    );
    let number_value = Value::Function(number_function);
    env.insert("Number".to_owned(), number_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Number".to_owned(), number_value);
    }

    env.insert("parseFloat".to_owned(), parse_float_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("parseFloat".to_owned(), parse_float_value);
    }

    env.insert("parseInt".to_owned(), parse_int_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("parseInt".to_owned(), parse_int_value);
    }
}

fn define_number_constant(function: &Function, key: &str, value: f64) {
    function.properties.borrow_mut().insert(
        key.to_owned(),
        Property::data(Value::Number(value), false, false, false),
    );
}

fn define_function_property(function: &Function, key: &str, length: usize, native: NativeFunction) {
    function.properties.borrow_mut().insert(
        key.to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some(key),
            length,
            native,
            false,
        ))),
    );
}

pub(super) fn number_prototype(env: &HashMap<String, Value>) -> Option<ObjectRef> {
    let Some(Value::Function(number_function)) = env.get("Number") else {
        return None;
    };
    function_prototype(number_function)
}

pub(super) fn native_number(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    let number = match argument_values.first() {
        Some(value) => to_number(value.clone())?,
        None => 0.0,
    };
    if !is_construct {
        return Ok(Value::Number(number));
    }

    let object = match this_value {
        Value::Object(object) => object,
        _ => ObjectRef::with_prototype(HashMap::new(), function_prototype(function)),
    };
    object.define_non_enumerable(NUMBER_DATA_PROPERTY.to_owned(), Value::Number(number));
    Ok(Value::Object(object))
}

pub(super) fn native_number_prototype_to_string(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let number = this_number_value(this_value)?;
    let radix =
        number_to_string_radix(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    Ok(Value::String(number_to_radix_string(number, radix)?))
}

pub(super) fn native_number_prototype_value_of(this_value: Value) -> Result<Value, RuntimeError> {
    Ok(Value::Number(this_number_value(this_value)?))
}

fn this_number_value(value: Value) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(value) => Ok(value),
        Value::Object(object) => match object.own_property(NUMBER_DATA_PROPERTY) {
            Some(Property {
                value: Value::Number(value),
                ..
            }) => Ok(value),
            _ => Err(RuntimeError {
                message: "Number.prototype method called on non-number object".to_owned(),
            }),
        },
        _ => Err(RuntimeError {
            message: "Number.prototype method called on non-number".to_owned(),
        }),
    }
}

fn number_to_string_radix(value: Value) -> Result<u32, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(10);
    }
    let radix = to_int32(value)?;
    if !(2..=36).contains(&radix) {
        return Err(RuntimeError {
            message: "radix must be between 2 and 36".to_owned(),
        });
    }
    Ok(radix as u32)
}

fn number_to_radix_string(number: f64, radix: u32) -> Result<String, RuntimeError> {
    if radix == 10 || !number.is_finite() {
        return Ok(number_to_js_string(number));
    }
    if number.fract() != 0.0 {
        return Err(RuntimeError {
            message: "non-decimal number formatting supports integers only".to_owned(),
        });
    }

    let sign = if number < 0.0 { "-" } else { "" };
    let mut integer = number.abs() as u128;
    if integer == 0 {
        return Ok("0".to_owned());
    }

    const DIGITS: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
    let mut output = Vec::new();
    while integer > 0 {
        let digit = (integer % u128::from(radix)) as usize;
        output.push(DIGITS[digit] as char);
        integer /= u128::from(radix);
    }
    output.reverse();
    Ok(format!("{sign}{}", output.into_iter().collect::<String>()))
}

pub(super) fn native_number_is_finite(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Number(number)) if number.is_finite()
    )))
}

pub(super) fn native_number_is_integer(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Number(number)) if number.is_finite() && number.fract() == 0.0
    )))
}

pub(super) fn native_number_is_nan(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    Ok(Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Number(number)) if number.is_nan()
    )))
}

pub(super) fn native_number_is_safe_integer(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    const MAX_SAFE_INTEGER: f64 = 9_007_199_254_740_991.0;
    Ok(Value::Boolean(matches!(
        argument_values.first(),
        Some(Value::Number(number))
            if number.is_finite() && number.fract() == 0.0 && number.abs() <= MAX_SAFE_INTEGER
    )))
}

pub(super) fn inherited_number_prototype_property(
    env: &HashMap<String, Value>,
    key: &str,
) -> Option<Value> {
    number_prototype(env)
        .and_then(|prototype| prototype.get(key))
        .or_else(|| inherited_object_prototype_property(env, key))
}

pub(super) fn is_number_object(object: &ObjectRef) -> bool {
    matches!(
        object.own_property(NUMBER_DATA_PROPERTY),
        Some(Property {
            value: Value::Number(_),
            ..
        })
    )
}

pub(crate) fn number_to_js_string(number: f64) -> String {
    if number.is_nan() {
        "NaN".to_owned()
    } else if number == f64::INFINITY {
        "Infinity".to_owned()
    } else if number == f64::NEG_INFINITY {
        "-Infinity".to_owned()
    } else if number == 0.0 {
        "0".to_owned()
    } else if number.fract() == 0.0 {
        format!("{number:.0}")
    } else {
        number.to_string()
    }
}
