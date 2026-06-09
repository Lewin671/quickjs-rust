use std::collections::HashMap;

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
        ("Uint16Array", NativeFunction::Uint16Array),
        ("Uint32Array", NativeFunction::Uint32Array),
        ("Float32Array", NativeFunction::Float32Array),
        ("Float64Array", NativeFunction::Float64Array),
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
    prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(constructor.clone()),
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
    let number = to_number_with_env(value, env)?;
    let value = match native {
        NativeFunction::Uint8Array => modulo_integer(number, 256.0),
        NativeFunction::Uint16Array => modulo_integer(number, 65_536.0),
        NativeFunction::Uint32Array => modulo_integer(number, 4_294_967_296.0),
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
        NativeFunction::Uint16Array => "Uint16Array",
        NativeFunction::Uint32Array => "Uint32Array",
        NativeFunction::Float32Array => "Float32Array",
        NativeFunction::Float64Array => "Float64Array",
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
