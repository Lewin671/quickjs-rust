use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, function_prototype,
    property_value, symbol, to_number_with_env,
};

const ARRAY_BUFFER_DATA_PROPERTY: &str = "\0ArrayBufferData";
const MAX_ARRAY_BUFFER_LENGTH: usize = 1_000_000;

pub(crate) fn install_array_buffer(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let array_buffer_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    array_buffer_prototype.set_to_string_tag("ArrayBuffer");
    symbol::define_well_known_to_string_tag(env, &array_buffer_prototype, "ArrayBuffer");
    let array_buffer_function =
        Function::new_native(Some("ArrayBuffer"), 1, NativeFunction::ArrayBuffer, true);
    array_buffer_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(array_buffer_function.clone()),
    );
    array_buffer_prototype.define_property(
        "byteLength".to_owned(),
        Property::accessor(
            Some(Value::Function(Function::new_native(
                Some("get byteLength"),
                0,
                NativeFunction::ArrayBufferPrototypeByteLength,
                false,
            ))),
            None,
            false,
            true,
        ),
    );
    array_buffer_prototype.define_non_enumerable(
        "slice".to_owned(),
        Value::Function(Function::new_native(
            Some("slice"),
            2,
            NativeFunction::ArrayBufferPrototypeSlice,
            false,
        )),
    );
    array_buffer_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(array_buffer_prototype)),
    );

    let array_buffer_value = Value::Function(array_buffer_function);
    env.insert("ArrayBuffer".to_owned(), array_buffer_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("ArrayBuffer".to_owned(), array_buffer_value);
    }
}

pub(crate) fn native_array_buffer(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Constructor ArrayBuffer requires 'new'".to_owned(),
        });
    }
    let length = to_index(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let object = match this_value {
        Value::Object(object) => object,
        _ => ObjectRef::with_prototype(HashMap::new(), function_prototype(function)),
    };
    define_array_buffer_data(&object, vec![0; length]);
    Ok(Value::Object(object))
}

pub(crate) fn native_array_buffer_prototype_byte_length(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let bytes = array_buffer_data(&this_value)?;
    Ok(Value::Number(bytes.len() as f64))
}

pub(crate) fn native_array_buffer_prototype_slice(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let bytes = array_buffer_data(&this_value)?;
    validate_species_constructor(this_value.clone(), env)?;
    let length = bytes.len();
    let start = slice_index(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        length,
        0,
        env,
    )?;
    let end = slice_index(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        length,
        length,
        env,
    )?;
    let new_length = end.saturating_sub(start);
    let constructor = env.get("ArrayBuffer").cloned().unwrap_or(Value::Undefined);
    let prototype = crate::constructor_prototype(&constructor, env);
    let object = ObjectRef::with_prototype(HashMap::new(), prototype);
    define_array_buffer_data(&object, bytes[start..start + new_length].to_vec());
    Ok(Value::Object(object))
}

fn validate_species_constructor(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    let constructor = property_value(value, "constructor", env)?;
    if matches!(constructor, Value::Undefined) || is_object_value(&constructor) {
        return Ok(());
    }
    Err(RuntimeError {
        thrown: None,
        message: "TypeError: ArrayBuffer species constructor must be an object".to_owned(),
    })
}

fn is_object_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(object) if !symbol::is_symbol_primitive(object)
    ) || matches!(
        value,
        Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_)
    )
}

fn define_array_buffer_data(object: &ObjectRef, bytes: Vec<u8>) {
    object.define_non_enumerable(
        ARRAY_BUFFER_DATA_PROPERTY.to_owned(),
        Value::String(bytes_to_string(bytes)),
    );
    object.set_to_string_tag("ArrayBuffer");
}

fn array_buffer_data(value: &Value) -> Result<Vec<u8>, RuntimeError> {
    let Value::Object(object) = value else {
        return Err(array_buffer_receiver_error());
    };
    match object.own_property(ARRAY_BUFFER_DATA_PROPERTY) {
        Some(Property {
            value: Value::String(data),
            ..
        }) => Ok(string_to_bytes(&data)),
        _ => Err(array_buffer_receiver_error()),
    }
}

fn array_buffer_receiver_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: ArrayBuffer method called on incompatible receiver".to_owned(),
    }
}

fn to_index(value: Value, env: &mut HashMap<String, Value>) -> Result<usize, RuntimeError> {
    let number = to_number_with_env(value, env)?;
    let integer = if number.is_nan() { 0.0 } else { number.trunc() };
    if integer < 0.0 || !integer.is_finite() || integer > MAX_ARRAY_BUFFER_LENGTH as f64 {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid ArrayBuffer length".to_owned(),
        });
    }
    Ok(integer as usize)
}

fn slice_index(
    value: Value,
    length: usize,
    default: usize,
    env: &mut HashMap<String, Value>,
) -> Result<usize, RuntimeError> {
    if matches!(value, Value::Undefined) {
        return Ok(default);
    }
    let number = to_number_with_env(value, env)?;
    let integer = if number.is_nan() { 0.0 } else { number.trunc() };
    if integer < 0.0 {
        Ok((length as f64 + integer).max(0.0) as usize)
    } else {
        Ok(integer.min(length as f64) as usize)
    }
}

fn bytes_to_string(bytes: Vec<u8>) -> String {
    bytes.into_iter().map(char::from).collect()
}

fn string_to_bytes(value: &str) -> Vec<u8> {
    value.chars().map(|character| character as u8).collect()
}

#[cfg(test)]
mod tests {
    use crate::{Value, eval};

    #[test]
    fn array_buffer_constructor_and_byte_length() {
        assert_eq!(
            eval("let buffer = new ArrayBuffer(8); buffer.byteLength;"),
            Ok(Value::Number(8.0))
        );
        assert_eq!(
            eval("Object.prototype.toString.call(new ArrayBuffer(0));"),
            Ok(Value::String("[object ArrayBuffer]".to_owned()))
        );
    }

    #[test]
    fn array_buffer_slice_resolves_bounds() {
        assert_eq!(
            eval("new ArrayBuffer(8).slice(2, 6).byteLength;"),
            Ok(Value::Number(4.0))
        );
        assert_eq!(
            eval("new ArrayBuffer(8).slice(-5, -1).byteLength;"),
            Ok(Value::Number(4.0))
        );
        assert_eq!(
            eval("new ArrayBuffer(8).slice(9, 1).byteLength;"),
            Ok(Value::Number(0.0))
        );
    }

    #[test]
    fn array_buffer_slice_rejects_non_object_constructor() {
        assert!(
            eval("let buffer = new ArrayBuffer(8); buffer.constructor = true; buffer.slice();")
                .is_err()
        );
    }
}
