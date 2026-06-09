use std::collections::HashMap;

use num_bigint::BigInt;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, function_prototype,
    property_value, symbol, to_length_with_env, to_number_with_env,
};

const MAX_TYPED_ARRAY_LENGTH: usize = 1_000_000;

pub(crate) fn install_typed_arrays(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    for (name, native) in [
        ("Uint8Array", NativeFunction::Uint8Array),
        ("Int8Array", NativeFunction::Int8Array),
        ("Uint8ClampedArray", NativeFunction::Uint8ClampedArray),
        ("Uint16Array", NativeFunction::Uint16Array),
        ("Int16Array", NativeFunction::Int16Array),
        ("Uint32Array", NativeFunction::Uint32Array),
        ("Int32Array", NativeFunction::Int32Array),
        ("Float32Array", NativeFunction::Float32Array),
        ("Float64Array", NativeFunction::Float64Array),
        ("BigInt64Array", NativeFunction::BigInt64Array),
        ("BigUint64Array", NativeFunction::BigUint64Array),
    ] {
        install_typed_array_constructor(env, global_this, object_prototype.clone(), name, native);
    }
}

fn install_typed_array_constructor(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
    name: &str,
    native: NativeFunction,
) {
    let prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    prototype.set_to_string_tag(name);
    symbol::define_well_known_to_string_tag(env, &prototype, name);

    let constructor = Function::new_native(Some(name), 3, native, true);
    constructor.properties.borrow_mut().insert(
        "BYTES_PER_ELEMENT".to_owned(),
        Property::fixed_non_enumerable(Value::Number(bytes_per_element(native) as f64)),
    );
    prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(constructor.clone()),
    );
    prototype.define_property(
        "BYTES_PER_ELEMENT".to_owned(),
        Property::fixed_non_enumerable(Value::Number(bytes_per_element(native) as f64)),
    );
    constructor.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(prototype)),
    );

    let value = Value::Function(constructor);
    env.insert(name.to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable(name.to_owned(), value);
    }
}

pub(crate) fn native_typed_array(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: format!(
                "TypeError: Constructor {} requires 'new'",
                typed_array_name(native)
            ),
        });
    }

    let object = match this_value {
        Value::Object(object) => object,
        _ => ObjectRef::with_prototype(HashMap::new(), function_prototype(function)),
    };
    let values = typed_array_values(native, argument_values.first().cloned(), env)?;
    define_typed_array_data(&object, native, values);
    Ok(Value::Object(object))
}

fn typed_array_values(
    native: NativeFunction,
    source: Option<Value>,
    env: &mut HashMap<String, Value>,
) -> Result<Vec<Value>, RuntimeError> {
    let Some(source) = source else {
        return Ok(Vec::new());
    };
    if matches!(source, Value::Undefined) {
        return Ok(Vec::new());
    }
    if let Value::Array(array) = source {
        let mut values = Vec::with_capacity(array.len());
        for index in 0..array.len() {
            let value = array.get(index).unwrap_or(Value::Undefined);
            values.push(coerce_element(native, value, env)?);
        }
        return Ok(values);
    }
    if matches!(source, Value::Object(_) | Value::Function(_)) {
        let length = to_typed_array_length(property_value(source.clone(), "length", env)?, env)?;
        let mut values = Vec::with_capacity(length);
        for index in 0..length {
            values.push(coerce_element(
                native,
                property_value(source.clone(), &index.to_string(), env)?,
                env,
            )?);
        }
        return Ok(values);
    }

    let length = to_typed_array_length(source, env)?;
    Ok(std::iter::repeat_n(Value::Number(0.0), length).collect())
}

fn define_typed_array_data(object: &ObjectRef, native: NativeFunction, values: Vec<Value>) {
    object.set_to_string_tag(typed_array_name(native));
    object.define_property(
        "length".to_owned(),
        Property::data(Value::Number(values.len() as f64), false, true, true),
    );
    for (index, value) in values.into_iter().enumerate() {
        object.define_property(index.to_string(), Property::enumerable(value));
    }
}

fn coerce_element(
    native: NativeFunction,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if matches!(
        native,
        NativeFunction::BigInt64Array | NativeFunction::BigUint64Array
    ) {
        return match value {
            Value::BigInt(value) => Ok(Value::BigInt(value)),
            Value::Number(value) => Ok(Value::BigInt(BigInt::from(value as i64))),
            Value::Undefined => Ok(Value::BigInt(BigInt::from(0))),
            _ => Err(RuntimeError {
                thrown: None,
                message: "TypeError: cannot convert value to BigInt typed array element".to_owned(),
            }),
        };
    }

    let number = to_number_with_env(value, env)?;
    let value = match native {
        NativeFunction::Uint8Array => modulo_integer(number, 256.0),
        NativeFunction::Int8Array => signed_integer(number, 8),
        NativeFunction::Uint8ClampedArray => clamp_uint8(number),
        NativeFunction::Uint16Array => modulo_integer(number, 65_536.0),
        NativeFunction::Int16Array => signed_integer(number, 16),
        NativeFunction::Uint32Array => modulo_integer(number, 4_294_967_296.0),
        NativeFunction::Int32Array => signed_integer(number, 32),
        NativeFunction::Float32Array | NativeFunction::Float64Array => number,
        _ => unreachable!("typed array native expected"),
    };
    Ok(Value::Number(value))
}

fn modulo_integer(number: f64, modulo: f64) -> f64 {
    if !number.is_finite() || number == 0.0 {
        return 0.0;
    }
    let integer = number.trunc();
    ((integer % modulo) + modulo) % modulo
}

fn signed_integer(number: f64, bits: u32) -> f64 {
    let modulo = 2_f64.powi(bits as i32);
    let value = modulo_integer(number, modulo);
    let sign = 2_f64.powi(bits as i32 - 1);
    if value >= sign { value - modulo } else { value }
}

fn clamp_uint8(number: f64) -> f64 {
    if number.is_nan() || number <= 0.0 {
        0.0
    } else if number >= 255.0 {
        255.0
    } else {
        number.round()
    }
}

fn bytes_per_element(native: NativeFunction) -> usize {
    match native {
        NativeFunction::Uint8Array
        | NativeFunction::Int8Array
        | NativeFunction::Uint8ClampedArray => 1,
        NativeFunction::Uint16Array | NativeFunction::Int16Array => 2,
        NativeFunction::Uint32Array | NativeFunction::Int32Array | NativeFunction::Float32Array => {
            4
        }
        NativeFunction::Float64Array
        | NativeFunction::BigInt64Array
        | NativeFunction::BigUint64Array => 8,
        _ => unreachable!("typed array native expected"),
    }
}

fn to_typed_array_length(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<usize, RuntimeError> {
    let length = to_length_with_env(value, env)?;
    if length > MAX_TYPED_ARRAY_LENGTH {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid typed array length".to_owned(),
        });
    }
    Ok(length)
}

fn typed_array_name(native: NativeFunction) -> &'static str {
    match native {
        NativeFunction::Uint8Array => "Uint8Array",
        NativeFunction::Int8Array => "Int8Array",
        NativeFunction::Uint8ClampedArray => "Uint8ClampedArray",
        NativeFunction::Uint16Array => "Uint16Array",
        NativeFunction::Int16Array => "Int16Array",
        NativeFunction::Uint32Array => "Uint32Array",
        NativeFunction::Int32Array => "Int32Array",
        NativeFunction::Float32Array => "Float32Array",
        NativeFunction::Float64Array => "Float64Array",
        NativeFunction::BigInt64Array => "BigInt64Array",
        NativeFunction::BigUint64Array => "BigUint64Array",
        _ => unreachable!("typed array native expected"),
    }
}

#[cfg(test)]
mod tests {
    use crate::{Value, eval};

    #[test]
    fn typed_array_constructors_create_array_like_objects() {
        assert_eq!(
            eval("let ta = new Uint8Array([1, 258]); ta.length + ':' + ta[0] + ':' + ta[1];"),
            Ok(Value::String("2:1:2".to_owned()))
        );
        assert_eq!(
            eval("let ta = new Float64Array(3); ta.length + ':' + ta[0] + ':' + ta[2];"),
            Ok(Value::String("3:0:0".to_owned()))
        );
    }

    #[test]
    fn typed_array_integer_constructor_surface() {
        assert_eq!(
            eval(
                "let ta = new Int8Array([127, 128, 255]); ta.length + ':' + ta[0] + ':' + ta[1] + ':' + ta[2];"
            ),
            Ok(Value::String("3:127:-128:-1".to_owned()))
        );
        assert_eq!(
            eval("Array.prototype.join.call(new Uint8ClampedArray([-1, 2.6, 300]));"),
            Ok(Value::String("0,3,255".to_owned()))
        );
        assert_eq!(
            eval(
                "Int16Array.BYTES_PER_ELEMENT + ':' + Int32Array.prototype.BYTES_PER_ELEMENT + ':' + Object.prototype.toString.call(new Uint8ClampedArray(0));"
            ),
            Ok(Value::String("2:4:[object Uint8ClampedArray]".to_owned()))
        );
    }

    #[test]
    fn concat_spreads_opted_in_typed_array_objects() {
        assert_eq!(
            eval(
                "let ta = new Uint16Array([7, 8]); let kept = [].concat(ta); ta[Symbol.isConcatSpreadable] = true; let spread = [].concat(ta); kept.length + ':' + (kept[0] === ta) + ':' + spread.join();"
            ),
            Ok(Value::String("1:true:7,8".to_owned()))
        );
        assert_eq!(
            eval(
                "let ta = new Uint8Array(1); Object.defineProperty(ta, 'length', { value: 4 }); ta[Symbol.isConcatSpreadable] = true; let out = [].concat(ta); out.length + ':' + out[0] + ':' + out.hasOwnProperty('3');"
            ),
            Ok(Value::String("4:0:false".to_owned()))
        );
    }
}
