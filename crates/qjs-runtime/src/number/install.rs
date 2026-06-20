use std::collections::HashMap;

use crate::{Function, NativeFunction, ObjectRef, Property, Value};

use super::NUMBER_DATA_PROPERTY;
use crate::CallEnv;

pub(crate) fn install_number(env: &mut CallEnv, global_this: &Value, object_prototype: ObjectRef) {
    let number_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    let number_function = Function::new_native(Some("Number"), 1, NativeFunction::Number, true);
    number_prototype.define_non_enumerable(NUMBER_DATA_PROPERTY.to_owned(), Value::Number(0.0));
    number_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(number_function.clone()),
    );
    number_prototype.define_non_enumerable(
        "toExponential".to_owned(),
        Value::Function(Function::new_native(
            Some("toExponential"),
            1,
            NativeFunction::NumberPrototypeToExponential,
            false,
        )),
    );
    number_prototype.define_non_enumerable(
        "toFixed".to_owned(),
        Value::Function(Function::new_native(
            Some("toFixed"),
            1,
            NativeFunction::NumberPrototypeToFixed,
            false,
        )),
    );
    number_prototype.define_non_enumerable(
        "toPrecision".to_owned(),
        Value::Function(Function::new_native(
            Some("toPrecision"),
            1,
            NativeFunction::NumberPrototypeToPrecision,
            false,
        )),
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
        "toLocaleString".to_owned(),
        Value::Function(Function::new_native(
            Some("toLocaleString"),
            0,
            NativeFunction::NumberPrototypeToLocaleString,
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
        Property::fixed_non_enumerable(Value::Object(number_prototype)),
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
    define_number_constant(&number_function, "MIN_VALUE", f64::from_bits(1));
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
    env.insert_realm("Number".to_owned(), number_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("Number".to_owned(), number_value);
    }

    env.insert_realm("parseFloat".to_owned(), parse_float_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("parseFloat".to_owned(), parse_float_value);
    }

    env.insert_realm("parseInt".to_owned(), parse_int_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("parseInt".to_owned(), parse_int_value);
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

pub(crate) fn is_number_object(object: &ObjectRef) -> bool {
    matches!(
        object.own_property(NUMBER_DATA_PROPERTY),
        Some(Property {
            value: Value::Number(_),
            ..
        })
    )
}
