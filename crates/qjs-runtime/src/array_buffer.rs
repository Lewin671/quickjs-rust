use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, function_prototype,
    property_value, symbol, to_number_with_env,
};

/// Internal slot holding the backing bytes of an `ArrayBuffer`, encoded as a
/// Latin-1 string (one `char` per byte). Absent on detached buffers.
pub(crate) const ARRAY_BUFFER_DATA_PROPERTY: &str = "\0ArrayBufferData";
/// Internal marker set on a detached `ArrayBuffer`. Once set, the data slot is
/// cleared and every accessor that reaches the buffer observes a detached
/// state.
pub(crate) const ARRAY_BUFFER_DETACHED_PROPERTY: &str = "\0ArrayBufferDetached";
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
    array_buffer_function.properties.borrow_mut().insert(
        "isView".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("isView"),
            1,
            NativeFunction::ArrayBufferIsView,
            false,
        ))),
    );
    symbol::define_species_accessor(env, &array_buffer_function);

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
    // `maxByteLength` (resizable/growable buffers) is not supported yet. The
    // options object is still consumed per spec so a poisoned getter throws, and
    // a present `maxByteLength` is rejected cleanly rather than silently ignored.
    reject_resizable_options(argument_values.get(1).cloned(), env)?;
    let object = match this_value {
        Value::Object(object) => object,
        _ => ObjectRef::with_prototype(HashMap::new(), function_prototype(function)),
    };
    define_array_buffer_data(&object, vec![0; length]);
    Ok(Value::Object(object))
}

pub(crate) fn native_array_buffer_is_view(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let is_view = matches!(
        argument_values.first(),
        Some(Value::Object(object)) if crate::typed_array::is_typed_array_object(object)
    );
    Ok(Value::Boolean(is_view))
}

pub(crate) fn native_array_buffer_prototype_byte_length(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = array_buffer_object(&this_value)?;
    // A detached buffer reports a byte length of 0 rather than throwing.
    if is_detached(&object) {
        return Ok(Value::Number(0.0));
    }
    Ok(Value::Number(array_buffer_bytes(&object).len() as f64))
}

pub(crate) fn native_array_buffer_prototype_slice(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let object = array_buffer_object(&this_value)?;
    if is_detached(&object) {
        return Err(detached_error());
    }
    let length = array_buffer_bytes(&object).len();
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
    // SpeciesConstructor(O, %ArrayBuffer%): a non-undefined, non-object
    // `constructor` throws; the species slot itself is left at the default so
    // the result is always a plain ArrayBuffer for now.
    validate_species_constructor(this_value.clone(), env)?;
    let constructor = env.get("ArrayBuffer").cloned().unwrap_or(Value::Undefined);
    let prototype = crate::constructor_prototype(&constructor, env);
    let result = ObjectRef::with_prototype(HashMap::new(), prototype);
    // Re-read the source after the user-observable index coercion above, which
    // could have detached it.
    if is_detached(&object) {
        return Err(detached_error());
    }
    let bytes = array_buffer_bytes(&object);
    let slice = bytes
        .get(start..start + new_length)
        .map(<[u8]>::to_vec)
        .unwrap_or_default();
    define_array_buffer_data(&result, slice);
    Ok(Value::Object(result))
}

fn reject_resizable_options(
    options: Option<Value>,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    let Some(options) = options else {
        return Ok(());
    };
    if matches!(options, Value::Undefined) {
        return Ok(());
    }
    if !is_object_value(&options) {
        return Ok(());
    }
    let max = property_value(options, "maxByteLength", env)?;
    if matches!(max, Value::Undefined) {
        return Ok(());
    }
    Err(RuntimeError {
        thrown: None,
        message: "TypeError: resizable ArrayBuffer (maxByteLength) is not supported".to_owned(),
    })
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
    object.define_property(
        ARRAY_BUFFER_DATA_PROPERTY.to_owned(),
        Property::non_enumerable(Value::String(bytes_to_string(bytes))),
    );
    object.set_to_string_tag("ArrayBuffer");
}

/// Whether `object` carries the `ArrayBuffer` brand (data slot or detached
/// marker), used for brand checks and `ArrayBuffer.isView` consumers.
pub(crate) fn is_array_buffer_object(object: &ObjectRef) -> bool {
    object.has_own_property(ARRAY_BUFFER_DATA_PROPERTY)
        || object.has_own_property(ARRAY_BUFFER_DETACHED_PROPERTY)
}

/// The `ArrayBuffer` receiver as an object, after a brand check.
pub(crate) fn array_buffer_object(value: &Value) -> Result<ObjectRef, RuntimeError> {
    match value {
        Value::Object(object) if is_array_buffer_object(object) => Ok(object.clone()),
        _ => Err(array_buffer_receiver_error()),
    }
}

/// Whether `object` is a detached `ArrayBuffer`.
pub(crate) fn is_detached(object: &ObjectRef) -> bool {
    object.has_own_property(ARRAY_BUFFER_DETACHED_PROPERTY)
        || !object.has_own_property(ARRAY_BUFFER_DATA_PROPERTY)
}

/// The backing bytes of a (non-detached) `ArrayBuffer`.
pub(crate) fn array_buffer_bytes(object: &ObjectRef) -> Vec<u8> {
    match object.own_property(ARRAY_BUFFER_DATA_PROPERTY) {
        Some(Property {
            value: Value::String(data),
            ..
        }) => string_to_bytes(&data),
        _ => Vec::new(),
    }
}

pub(crate) fn detached_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: ArrayBuffer is detached".to_owned(),
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

    #[test]
    fn array_buffer_is_view_reports_typed_arrays() {
        assert_eq!(
            eval("ArrayBuffer.isView(new Uint8Array(4));"),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval("ArrayBuffer.isView(new ArrayBuffer(4));"),
            Ok(Value::Boolean(false))
        );
        assert_eq!(eval("ArrayBuffer.isView({});"), Ok(Value::Boolean(false)));
        assert_eq!(eval("ArrayBuffer.isView();"), Ok(Value::Boolean(false)));
    }

    #[test]
    fn array_buffer_species_is_self() {
        assert_eq!(
            eval("ArrayBuffer[Symbol.species] === ArrayBuffer;"),
            Ok(Value::Boolean(true))
        );
    }

    #[test]
    fn array_buffer_max_byte_length_is_rejected() {
        assert!(eval("new ArrayBuffer(8, { maxByteLength: 16 });").is_err());
        // A plain or undefined options argument must still succeed.
        assert_eq!(
            eval("new ArrayBuffer(8, {}).byteLength;"),
            Ok(Value::Number(8.0))
        );
        assert_eq!(
            eval("new ArrayBuffer(8, undefined).byteLength;"),
            Ok(Value::Number(8.0))
        );
    }

    #[test]
    fn array_buffer_byte_length_brand_check() {
        assert!(
            eval(
                "Object.getOwnPropertyDescriptor(ArrayBuffer.prototype, 'byteLength').get.call({});"
            )
            .is_err()
        );
    }
}
