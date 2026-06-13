use crate::CallEnv;
use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, PropertyKey, RuntimeError, Value,
    construct_function, ensure_constructor, property_value, property_value_key, symbol,
    to_number_with_env,
};

/// Internal slot holding the backing bytes of an `ArrayBuffer`, encoded as a
/// Latin-1 string (one `char` per byte). Absent on detached buffers.
pub(crate) const ARRAY_BUFFER_DATA_PROPERTY: &str = "\0ArrayBufferData";
/// Internal marker set on a detached `ArrayBuffer`. Once set, the data slot is
/// cleared and every accessor that reaches the buffer observes a detached
/// state.
pub(crate) const ARRAY_BUFFER_DETACHED_PROPERTY: &str = "\0ArrayBufferDetached";
/// Internal slot holding the maximum byte length for resizable ArrayBuffers.
pub(crate) const ARRAY_BUFFER_MAX_BYTE_LENGTH_PROPERTY: &str = "\0ArrayBufferMaxByteLength";
const SHARED_ARRAY_BUFFER_DATA_PROPERTY: &str = "\0SharedArrayBufferData";
const MAX_ARRAY_BUFFER_LENGTH: usize = 1_000_000;

pub(crate) fn install_array_buffer(
    env: &mut CallEnv,
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
    for (name, native) in [
        (
            "maxByteLength",
            NativeFunction::ArrayBufferPrototypeMaxByteLength,
        ),
        ("resizable", NativeFunction::ArrayBufferPrototypeResizable),
    ] {
        array_buffer_prototype.define_property(
            name.to_owned(),
            Property::accessor(
                Some(Value::Function(Function::new_native(
                    Some(&format!("get {name}")),
                    0,
                    native,
                    false,
                ))),
                None,
                false,
                true,
            ),
        );
    }
    array_buffer_prototype.define_non_enumerable(
        "resize".to_owned(),
        Value::Function(Function::new_native(
            Some("resize"),
            1,
            NativeFunction::ArrayBufferPrototypeResize,
            false,
        )),
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
    env.insert_realm("ArrayBuffer".to_owned(), array_buffer_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("ArrayBuffer".to_owned(), array_buffer_value);
    }
}

pub(crate) fn install_shared_array_buffer(
    env: &mut CallEnv,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    prototype.set_to_string_tag("SharedArrayBuffer");
    symbol::define_well_known_to_string_tag(env, &prototype, "SharedArrayBuffer");
    let function = Function::new_native(
        Some("SharedArrayBuffer"),
        1,
        NativeFunction::SharedArrayBuffer,
        true,
    );
    prototype.define_non_enumerable("constructor".to_owned(), Value::Function(function.clone()));
    prototype.define_property(
        "byteLength".to_owned(),
        Property::accessor(
            Some(Value::Function(Function::new_native(
                Some("get byteLength"),
                0,
                NativeFunction::SharedArrayBufferPrototypeByteLength,
                false,
            ))),
            None,
            false,
            true,
        ),
    );
    function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(prototype)),
    );
    let value = Value::Function(function);
    env.insert_realm("SharedArrayBuffer".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("SharedArrayBuffer".to_owned(), value);
    }
}

pub(crate) fn native_array_buffer(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
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
    let max_byte_length = resizable_max_byte_length(argument_values.get(1).cloned(), env)?;
    if max_byte_length.is_some_and(|max| max < length) {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: maxByteLength is smaller than ArrayBuffer length".to_owned(),
        });
    }
    let object = match this_value {
        Value::Object(object) => object,
        _ => ObjectRef::with_prototype_slot(
            HashMap::new(),
            crate::native_construct_prototype_slot(function, env)?,
        ),
    };
    define_array_buffer_data(&object, vec![0; length]);
    if let Some(max) = max_byte_length {
        define_array_buffer_max_byte_length(&object, max);
    }
    Ok(Value::Object(object))
}

pub(crate) fn native_shared_array_buffer(
    function: &Function,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Constructor SharedArrayBuffer requires 'new'".to_owned(),
        });
    }
    let length = to_index(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let object = ObjectRef::with_prototype_slot(
        HashMap::new(),
        crate::native_construct_prototype_slot(function, env)?,
    );
    define_shared_array_buffer_data(&object, vec![0; length]);
    Ok(Value::Object(object))
}

pub(crate) fn native_shared_array_buffer_prototype_byte_length(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = shared_array_buffer_object(&this_value)?;
    Ok(Value::Number(
        shared_array_buffer_bytes(&object).len() as f64
    ))
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

pub(crate) fn native_array_buffer_prototype_max_byte_length(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = array_buffer_object(&this_value)?;
    if is_detached(&object) {
        return Ok(Value::Number(0.0));
    }
    Ok(Value::Number(
        array_buffer_max_byte_length(&object).unwrap_or_else(|| array_buffer_bytes(&object).len())
            as f64,
    ))
}

pub(crate) fn native_array_buffer_prototype_resizable(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = array_buffer_object(&this_value)?;
    Ok(Value::Boolean(
        !is_detached(&object) && is_resizable(&object),
    ))
}

pub(crate) fn native_array_buffer_prototype_resize(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = array_buffer_object(&this_value)?;
    if is_detached(&object) {
        return Err(detached_error());
    }
    let Some(max_byte_length) = array_buffer_max_byte_length(&object) else {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: ArrayBuffer is not resizable".to_owned(),
        });
    };
    let new_length = to_index(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    if new_length > max_byte_length {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: ArrayBuffer resize length exceeds maxByteLength".to_owned(),
        });
    }
    let mut bytes = array_buffer_bytes(&object);
    bytes.resize(new_length, 0);
    set_array_buffer_bytes(&object, bytes);
    Ok(Value::Undefined)
}

pub(crate) fn native_array_buffer_prototype_slice(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
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
    let constructor = species_constructor(this_value.clone(), env)?;
    let result_value = construct_function(
        constructor.clone(),
        constructor,
        vec![Value::Number(new_length as f64)],
        env,
    )?;
    let result = array_buffer_object(&result_value)?;
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

fn resizable_max_byte_length(
    options: Option<Value>,
    env: &mut CallEnv,
) -> Result<Option<usize>, RuntimeError> {
    let Some(options) = options else {
        return Ok(None);
    };
    if matches!(options, Value::Undefined) {
        return Ok(None);
    }
    if !is_object_value(&options) {
        return Ok(None);
    }
    let max = property_value(options, "maxByteLength", env)?;
    if matches!(max, Value::Undefined) {
        return Ok(None);
    }
    Ok(Some(to_index(max, env)?))
}

fn species_constructor(value: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    let default = env.get("ArrayBuffer").unwrap_or(Value::Undefined);
    let constructor = property_value(value, "constructor", env)?;
    if matches!(constructor, Value::Undefined) {
        return Ok(default);
    }
    if !is_object_value(&constructor) {
        return Err(array_buffer_species_constructor_error());
    }
    let species = match symbol::species_symbol(env) {
        Some(symbol) => property_value_key(constructor, &PropertyKey::Symbol(symbol), env)?,
        None => return Ok(default),
    };
    if matches!(species, Value::Undefined | Value::Null) {
        return Ok(default);
    }
    ensure_constructor(&species).map_err(|_| array_buffer_species_constructor_error())?;
    Ok(species)
}

fn array_buffer_species_constructor_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: ArrayBuffer species constructor must be an object".to_owned(),
    }
}

fn is_object_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(object) if !symbol::is_symbol_primitive(object)
    ) || matches!(
        value,
        Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) | Value::Proxy(_)
    )
}

fn define_array_buffer_data(object: &ObjectRef, bytes: Vec<u8>) {
    object.set_internal_bytes(bytes);
    object.define_property(
        ARRAY_BUFFER_DATA_PROPERTY.to_owned(),
        Property::non_enumerable(Value::String(String::new())),
    );
    object.set_to_string_tag("ArrayBuffer");
}

fn define_array_buffer_max_byte_length(object: &ObjectRef, max_byte_length: usize) {
    object.define_property(
        ARRAY_BUFFER_MAX_BYTE_LENGTH_PROPERTY.to_owned(),
        Property::non_enumerable(Value::Number(max_byte_length as f64)),
    );
}

fn define_shared_array_buffer_data(object: &ObjectRef, bytes: Vec<u8>) {
    object.set_internal_bytes(bytes);
    object.define_property(
        SHARED_ARRAY_BUFFER_DATA_PROPERTY.to_owned(),
        Property::non_enumerable(Value::String(String::new())),
    );
    object.set_to_string_tag("SharedArrayBuffer");
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

fn shared_array_buffer_object(value: &Value) -> Result<ObjectRef, RuntimeError> {
    match value {
        Value::Object(object) if object.has_own_property(SHARED_ARRAY_BUFFER_DATA_PROPERTY) => {
            Ok(object.clone())
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: SharedArrayBuffer method called on incompatible receiver"
                .to_owned(),
        }),
    }
}

/// Whether `object` is a detached `ArrayBuffer`.
pub(crate) fn is_detached(object: &ObjectRef) -> bool {
    object.has_own_property(ARRAY_BUFFER_DETACHED_PROPERTY)
        || !object.has_own_property(ARRAY_BUFFER_DATA_PROPERTY)
}

pub(crate) fn is_resizable(object: &ObjectRef) -> bool {
    object.has_own_property(ARRAY_BUFFER_MAX_BYTE_LENGTH_PROPERTY)
}

pub(crate) fn array_buffer_max_byte_length(object: &ObjectRef) -> Option<usize> {
    match object.own_property(ARRAY_BUFFER_MAX_BYTE_LENGTH_PROPERTY) {
        Some(Property {
            value: Value::Number(length),
            ..
        }) => Some(length as usize),
        _ => None,
    }
}

/// The backing bytes of a (non-detached) `ArrayBuffer`.
pub(crate) fn array_buffer_bytes(object: &ObjectRef) -> Vec<u8> {
    if let Some(bytes) = object.internal_bytes() {
        return bytes;
    }
    match object.own_property(ARRAY_BUFFER_DATA_PROPERTY) {
        Some(Property {
            value: Value::String(data),
            ..
        }) => string_to_bytes(&data),
        _ => Vec::new(),
    }
}

fn shared_array_buffer_bytes(object: &ObjectRef) -> Vec<u8> {
    object.internal_bytes().unwrap_or_default()
}

/// Replaces the backing bytes of an `ArrayBuffer` (used by typed-array writes).
pub(crate) fn set_array_buffer_bytes(object: &ObjectRef, bytes: Vec<u8>) {
    object.set_internal_bytes(bytes);
    object.define_property(
        ARRAY_BUFFER_DATA_PROPERTY.to_owned(),
        Property::non_enumerable(Value::String(String::new())),
    );
}

pub(crate) fn mutate_array_buffer_bytes<T>(
    object: &ObjectRef,
    f: impl FnOnce(&mut Vec<u8>) -> T,
) -> Option<T> {
    object.with_internal_bytes_mut(f)
}

/// Builds a fresh, zero-initialized `ArrayBuffer` inheriting from
/// `%ArrayBuffer.prototype%`. Used when a TypedArray allocates its own buffer.
pub(crate) fn new_array_buffer(env: &CallEnv, length: usize) -> ObjectRef {
    let constructor = env.get("ArrayBuffer").unwrap_or(Value::Undefined);
    let prototype = crate::constructor_prototype(&constructor, env);
    let object = ObjectRef::with_prototype(HashMap::new(), prototype);
    define_array_buffer_data(&object, vec![0; length]);
    object
}

/// DetachArrayBuffer: clears the backing data and marks `object` detached.
/// Idempotent; a non-ArrayBuffer argument is ignored. Used by the Test262 host
/// `$262.detachArrayBuffer` hook.
pub(crate) fn detach(object: &ObjectRef) {
    object.clear_internal_bytes();
    object.delete_own_property(ARRAY_BUFFER_DATA_PROPERTY);
    object.define_property(
        ARRAY_BUFFER_DETACHED_PROPERTY.to_owned(),
        Property::non_enumerable(Value::Boolean(true)),
    );
}

/// Host hook backing `$262.detachArrayBuffer(buffer)`: detaches `buffer` when
/// it is an `ArrayBuffer`, returning `null` (the spec's DetachArrayBuffer
/// result) in all cases.
pub(crate) fn native_detach_array_buffer(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    if let Some(Value::Object(object)) = argument_values.first() {
        if is_array_buffer_object(object) {
            detach(object);
        }
    }
    Ok(Value::Null)
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

fn to_index(value: Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
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
    env: &mut CallEnv,
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
    fn array_buffer_resize_and_resizable_accessors() {
        assert_eq!(
            eval(
                "let b = new ArrayBuffer(8, { maxByteLength: 16 }); \
                 b.resizable + ':' + b.byteLength + ':' + b.maxByteLength;"
            ),
            Ok(Value::String("true:8:16".to_owned()))
        );
        assert_eq!(
            eval(
                "let b = new ArrayBuffer(8, { maxByteLength: 16 }); \
                 b.resize(12); b.byteLength + ':' + new Uint8Array(b)[10];"
            ),
            Ok(Value::String("12:0".to_owned()))
        );
        assert!(eval("new ArrayBuffer(8, { maxByteLength: 4 });").is_err());
        assert!(eval("let b = new ArrayBuffer(8); b.resize(4);").is_err());
        // A plain or undefined options argument must still succeed.
        assert_eq!(
            eval("let b = new ArrayBuffer(8, {}); b.resizable + ':' + b.maxByteLength;"),
            Ok(Value::String("false:8".to_owned()))
        );
        assert_eq!(
            eval("new ArrayBuffer(8, undefined).byteLength;"),
            Ok(Value::Number(8.0))
        );
    }

    #[test]
    fn detach_array_buffer_host_hook_marks_detached() {
        // The Test262 host hook detaches the buffer: byteLength reads 0 and the
        // typed-array view observes a detached buffer (methods throw).
        assert_eq!(
            eval("let b = new ArrayBuffer(8); __quickjsRustDetachArrayBuffer(b); b.byteLength;"),
            Ok(Value::Number(0.0))
        );
        assert!(
            eval(
                "let b = new ArrayBuffer(8); let a = new Uint8Array(b); \
                 __quickjsRustDetachArrayBuffer(b); a.fill(1);"
            )
            .is_err()
        );
        // A non-ArrayBuffer argument is ignored and the hook returns null.
        assert_eq!(eval("__quickjsRustDetachArrayBuffer({});"), Ok(Value::Null));
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
