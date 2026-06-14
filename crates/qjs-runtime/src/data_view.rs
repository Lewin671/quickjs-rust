use std::collections::HashMap;

use num_bigint::BigInt;
use num_traits::ToPrimitive;

use crate::CallEnv;
use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, array_buffer,
    function_prototype, symbol, to_number_with_env,
};

/// Internal slot referencing the viewed `ArrayBuffer` object.
pub(crate) const DATA_VIEW_BUFFER_PROPERTY: &str = "\0DataViewBuffer";
/// Internal slot holding the byte length of the view.
pub(crate) const DATA_VIEW_BYTE_LENGTH_PROPERTY: &str = "\0DataViewByteLength";
/// Internal slot holding the byte offset of the view into its buffer.
pub(crate) const DATA_VIEW_BYTE_OFFSET_PROPERTY: &str = "\0DataViewByteOffset";

/// The element types accessible through `DataView` get/set methods.
#[derive(Clone, Copy)]
enum ElementType {
    Int8,
    Uint8,
    Int16,
    Uint16,
    Int32,
    Uint32,
    Float32,
    Float64,
    BigInt64,
    BigUint64,
}

impl ElementType {
    fn size(self) -> usize {
        match self {
            ElementType::Int8 | ElementType::Uint8 => 1,
            ElementType::Int16 | ElementType::Uint16 => 2,
            ElementType::Int32 | ElementType::Uint32 | ElementType::Float32 => 4,
            ElementType::Float64 | ElementType::BigInt64 | ElementType::BigUint64 => 8,
        }
    }

    fn is_big_int(self) -> bool {
        matches!(self, ElementType::BigInt64 | ElementType::BigUint64)
    }
}

pub(crate) fn install_data_view(
    env: &mut CallEnv,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));

    let constructor = Function::new_native(Some("DataView"), 1, NativeFunction::DataView, true);
    prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(constructor.clone()),
    );

    // Accessors: buffer / byteLength / byteOffset (no setters, configurable).
    for (name, native) in [
        ("buffer", NativeFunction::DataViewPrototypeBuffer),
        ("byteLength", NativeFunction::DataViewPrototypeByteLength),
        ("byteOffset", NativeFunction::DataViewPrototypeByteOffset),
    ] {
        prototype.define_property(
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

    // get*/set* methods. The getters take 1 declared argument (byteOffset);
    // the setters take 2 (byteOffset, value).
    for (name, length, native) in [
        ("getInt8", 1, NativeFunction::DataViewPrototypeGetInt8),
        ("getUint8", 1, NativeFunction::DataViewPrototypeGetUint8),
        ("getInt16", 1, NativeFunction::DataViewPrototypeGetInt16),
        ("getUint16", 1, NativeFunction::DataViewPrototypeGetUint16),
        ("getInt32", 1, NativeFunction::DataViewPrototypeGetInt32),
        ("getUint32", 1, NativeFunction::DataViewPrototypeGetUint32),
        ("getFloat32", 1, NativeFunction::DataViewPrototypeGetFloat32),
        ("getFloat64", 1, NativeFunction::DataViewPrototypeGetFloat64),
        (
            "getBigInt64",
            1,
            NativeFunction::DataViewPrototypeGetBigInt64,
        ),
        (
            "getBigUint64",
            1,
            NativeFunction::DataViewPrototypeGetBigUint64,
        ),
        ("setInt8", 2, NativeFunction::DataViewPrototypeSetInt8),
        ("setUint8", 2, NativeFunction::DataViewPrototypeSetUint8),
        ("setInt16", 2, NativeFunction::DataViewPrototypeSetInt16),
        ("setUint16", 2, NativeFunction::DataViewPrototypeSetUint16),
        ("setInt32", 2, NativeFunction::DataViewPrototypeSetInt32),
        ("setUint32", 2, NativeFunction::DataViewPrototypeSetUint32),
        ("setFloat32", 2, NativeFunction::DataViewPrototypeSetFloat32),
        ("setFloat64", 2, NativeFunction::DataViewPrototypeSetFloat64),
        (
            "setBigInt64",
            2,
            NativeFunction::DataViewPrototypeSetBigInt64,
        ),
        (
            "setBigUint64",
            2,
            NativeFunction::DataViewPrototypeSetBigUint64,
        ),
    ] {
        prototype.define_non_enumerable(
            name.to_owned(),
            Value::Function(Function::new_native(Some(name), length, native, false)),
        );
    }

    // %DataView.prototype%[Symbol.toStringTag] is a plain data property
    // { value: "DataView", writable: false, enumerable: false, configurable: true }.
    symbol::define_well_known_to_string_tag(env, &prototype, "DataView");

    constructor.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(prototype)),
    );

    let value = Value::Function(constructor);
    env.insert_realm("DataView".to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("DataView".to_owned(), value);
    }
}

/// `new DataView(buffer [, byteOffset [, byteLength]])` (ES2023 25.3.2.1).
pub(crate) fn native_data_view(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !is_construct {
        return Err(type_error("Constructor DataView requires 'new'"));
    }

    // Step 2: buffer must be an ArrayBuffer or SharedArrayBuffer (brand check).
    let buffer = match argument_values.first() {
        Some(Value::Object(object))
            if array_buffer::is_array_buffer_or_shared_array_buffer_object(object) =>
        {
            object.clone()
        }
        _ => return Err(type_error("DataView buffer must be an ArrayBuffer")),
    };

    // Step 4: offset = ToIndex(byteOffset).
    let offset = to_index(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;

    // Step 6: re-check detach after the (user-observable) ToIndex coercion.
    if data_block_detached(&buffer) {
        return Err(array_buffer::detached_error());
    }

    // Step 7: bufferByteLength = ArrayBufferByteLength.
    let buffer_byte_length = array_buffer::buffer_byte_length(&buffer);

    // Step 8: a present offset beyond the buffer is a RangeError.
    if offset > buffer_byte_length {
        return Err(range_error(
            "byteOffset is outside the bounds of the buffer",
        ));
    }

    // Steps 9-12: resolve the view byte length.
    let length_arg = argument_values.get(2).cloned().unwrap_or(Value::Undefined);
    let view_byte_length = if matches!(length_arg, Value::Undefined) {
        buffer_byte_length - offset
    } else {
        let length = to_index(length_arg, env)?;
        // ToIndex of byteLength can run user code; re-check detach (step 13).
        if data_block_detached(&buffer) {
            return Err(array_buffer::detached_error());
        }
        if offset
            .checked_add(length)
            .is_none_or(|end| end > array_buffer::buffer_byte_length(&buffer))
        {
            return Err(range_error("byteOffset + byteLength exceeds the buffer"));
        }
        length
    };

    // OrdinaryCreateFromConstructor: the VM hands us the `new.target`-derived
    // object as `this_value` for `new`; fall back to %DataView.prototype% if it
    // is not an object (Reflect.construct edge cases are handled by the VM).
    let object = match this_value {
        Value::Object(object) => object,
        _ => ObjectRef::with_prototype(HashMap::new(), function_prototype(function)),
    };

    object.define_property(
        DATA_VIEW_BUFFER_PROPERTY.to_owned(),
        Property::non_enumerable(Value::Object(buffer)),
    );
    object.define_property(
        DATA_VIEW_BYTE_LENGTH_PROPERTY.to_owned(),
        Property::non_enumerable(Value::Number(view_byte_length as f64)),
    );
    object.define_property(
        DATA_VIEW_BYTE_OFFSET_PROPERTY.to_owned(),
        Property::non_enumerable(Value::Number(offset as f64)),
    );

    Ok(Value::Object(object))
}

// --- prototype accessors -----------------------------------------------------

/// `get DataView.prototype.buffer`.
pub(crate) fn native_data_view_prototype_buffer(this_value: Value) -> Result<Value, RuntimeError> {
    let object = data_view_receiver(&this_value)?;
    match object.own_property(DATA_VIEW_BUFFER_PROPERTY) {
        Some(Property { value, .. }) => Ok(value),
        None => Ok(Value::Undefined),
    }
}

/// `get DataView.prototype.byteLength`.
pub(crate) fn native_data_view_prototype_byte_length(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = data_view_receiver(&this_value)?;
    if data_view_buffer_detached(&object) {
        return Err(array_buffer::detached_error());
    }
    Ok(Value::Number(data_view_byte_length(&object) as f64))
}

/// `get DataView.prototype.byteOffset`.
pub(crate) fn native_data_view_prototype_byte_offset(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = data_view_receiver(&this_value)?;
    if data_view_buffer_detached(&object) {
        return Err(array_buffer::detached_error());
    }
    Ok(Value::Number(data_view_byte_offset(&object) as f64))
}

// --- get/set dispatch --------------------------------------------------------

pub(crate) fn native_data_view_prototype_get(
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let element = element_type_for(native);
    get_view_value(
        this_value,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        element,
        env,
    )
}

pub(crate) fn native_data_view_prototype_set(
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let element = element_type_for(native);
    set_view_value(
        this_value,
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        argument_values.get(2).cloned().unwrap_or(Value::Undefined),
        element,
        env,
    )
}

/// GetViewValue (ES2023 25.3.1.1).
fn get_view_value(
    this_value: Value,
    request_index: Value,
    is_little_endian: Value,
    element: ElementType,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    // Step 1-2: RequireInternalSlot.
    let object = data_view_receiver(&this_value)?;
    // Step 3: getIndex = ToIndex(requestIndex).
    let get_index = to_index(request_index, env)?;
    // Step 4: isLittleEndian = ToBoolean.
    let little_endian = crate::is_truthy(&is_little_endian);
    // Step 5-6: detached buffer.
    if data_view_buffer_detached(&object) {
        return Err(array_buffer::detached_error());
    }
    let view_offset = data_view_byte_offset(&object);
    let view_size = data_view_byte_length(&object);
    let element_size = element.size();
    // Step 11: bounds check.
    if get_index + element_size > view_size {
        return Err(range_error("offset is outside the bounds of the DataView"));
    }
    let buffer_index = get_index + view_offset;
    let Some(buffer) = data_view_buffer(&object) else {
        return Err(array_buffer::detached_error());
    };
    let bytes = array_buffer::buffer_bytes(&buffer);
    let slice = match bytes.get(buffer_index..buffer_index + element_size) {
        Some(slice) => slice,
        None => return Err(range_error("offset is outside the bounds of the DataView")),
    };
    Ok(decode(element, slice, little_endian))
}

/// SetViewValue (ES2023 25.3.1.2).
fn set_view_value(
    this_value: Value,
    request_index: Value,
    value: Value,
    is_little_endian: Value,
    element: ElementType,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    // Step 1-2: RequireInternalSlot.
    let object = data_view_receiver(&this_value)?;
    // Step 3: getIndex = ToIndex(requestIndex).
    let get_index = to_index(request_index, env)?;
    // Step 4-6: coerce the value BEFORE the detach / bounds checks (per spec the
    // numeric conversion observably runs first).
    let encoded = if element.is_big_int() {
        let big = crate::bigint::to_bigint(value, env)?;
        encode_big_int(element, &big)
    } else {
        let number = to_number_with_env(value, env)?;
        encode_number(element, number)
    };
    // Step 7: isLittleEndian = ToBoolean.
    let little_endian = crate::is_truthy(&is_little_endian);
    // Step 8-9: detached buffer.
    if data_view_buffer_detached(&object) {
        return Err(array_buffer::detached_error());
    }
    let view_offset = data_view_byte_offset(&object);
    let view_size = data_view_byte_length(&object);
    let element_size = element.size();
    // Step 14: bounds check.
    if get_index + element_size > view_size {
        return Err(range_error("offset is outside the bounds of the DataView"));
    }
    let buffer_index = get_index + view_offset;
    let Some(buffer) = data_view_buffer(&object) else {
        return Err(array_buffer::detached_error());
    };
    let mut bytes = array_buffer::buffer_bytes(&buffer);
    if buffer_index + element_size > bytes.len() {
        return Err(range_error("offset is outside the bounds of the DataView"));
    }
    let ordered = order_bytes(encoded, little_endian);
    bytes[buffer_index..buffer_index + element_size].copy_from_slice(&ordered);
    array_buffer::set_buffer_bytes(&buffer, bytes);
    Ok(Value::Undefined)
}

// --- byte encode/decode ------------------------------------------------------

/// Encodes a numeric (non-BigInt) element as big-endian bytes; endianness is
/// applied later by [`order_bytes`].
fn encode_number(element: ElementType, number: f64) -> Vec<u8> {
    match element {
        ElementType::Int8 => vec![to_int(number, 8) as u8],
        ElementType::Uint8 => vec![to_uint(number, 8) as u8],
        ElementType::Int16 => (to_int(number, 16) as i16).to_be_bytes().to_vec(),
        ElementType::Uint16 => (to_uint(number, 16) as u16).to_be_bytes().to_vec(),
        ElementType::Int32 => (to_int(number, 32) as i32).to_be_bytes().to_vec(),
        ElementType::Uint32 => (to_uint(number, 32) as u32).to_be_bytes().to_vec(),
        ElementType::Float32 => (number as f32).to_be_bytes().to_vec(),
        ElementType::Float64 => number.to_be_bytes().to_vec(),
        ElementType::BigInt64 | ElementType::BigUint64 => unreachable!("bigint handled separately"),
    }
}

fn encode_big_int(element: ElementType, value: &BigInt) -> Vec<u8> {
    // Take the low 64 bits (modulo 2^64), as ToBigInt64/ToBigUint64 require.
    let modulo = BigInt::from(1u128 << 64);
    let wrapped = ((value % &modulo) + &modulo) % &modulo;
    let low = wrapped.to_u64().unwrap_or(0);
    match element {
        ElementType::BigUint64 => low.to_be_bytes().to_vec(),
        ElementType::BigInt64 => (low as i64).to_be_bytes().to_vec(),
        _ => unreachable!("non-bigint handled separately"),
    }
}

/// Decodes big-endian-or-little-endian `slice` to the element's JS value.
fn decode(element: ElementType, slice: &[u8], little_endian: bool) -> Value {
    let ordered = order_bytes(slice.to_vec(), little_endian);
    match element {
        ElementType::Int8 => Value::Number(f64::from(ordered[0] as i8)),
        ElementType::Uint8 => Value::Number(f64::from(ordered[0])),
        ElementType::Int16 => {
            Value::Number(f64::from(i16::from_be_bytes([ordered[0], ordered[1]])))
        }
        ElementType::Uint16 => {
            Value::Number(f64::from(u16::from_be_bytes([ordered[0], ordered[1]])))
        }
        ElementType::Int32 => Value::Number(f64::from(i32::from_be_bytes([
            ordered[0], ordered[1], ordered[2], ordered[3],
        ]))),
        ElementType::Uint32 => Value::Number(f64::from(u32::from_be_bytes([
            ordered[0], ordered[1], ordered[2], ordered[3],
        ]))),
        ElementType::Float32 => Value::Number(f64::from(f32::from_be_bytes([
            ordered[0], ordered[1], ordered[2], ordered[3],
        ]))),
        ElementType::Float64 => {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&ordered);
            Value::Number(f64::from_be_bytes(buf))
        }
        ElementType::BigInt64 => {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&ordered);
            Value::BigInt(BigInt::from(i64::from_be_bytes(buf)))
        }
        ElementType::BigUint64 => {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&ordered);
            Value::BigInt(BigInt::from(u64::from_be_bytes(buf)))
        }
    }
}

/// `encode_*` produces big-endian bytes; if `little_endian` is set, reverse to
/// little-endian. `decode` reuses the same transform symmetrically.
fn order_bytes(mut bytes: Vec<u8>, little_endian: bool) -> Vec<u8> {
    if little_endian {
        bytes.reverse();
    }
    bytes
}

/// Truncates `number` (already ToNumber'd) to a signed integer of `bits` width
/// using modulo-2^bits wrapping (matching ToInt16/ToInt32 semantics).
fn to_int(number: f64, bits: u32) -> i64 {
    let modulo = 2f64.powi(bits as i32);
    let wrapped = modulo_integer(number, modulo);
    let sign = 2f64.powi(bits as i32 - 1);
    let signed = if wrapped >= sign {
        wrapped - modulo
    } else {
        wrapped
    };
    signed as i64
}

/// Truncates `number` to an unsigned integer of `bits` width (ToUint8/16/32).
fn to_uint(number: f64, bits: u32) -> u64 {
    let modulo = 2f64.powi(bits as i32);
    modulo_integer(number, modulo) as u64
}

fn modulo_integer(number: f64, modulo: f64) -> f64 {
    if !number.is_finite() || number == 0.0 {
        return 0.0;
    }
    let integer = number.trunc();
    ((integer % modulo) + modulo) % modulo
}

// --- internal-slot helpers ---------------------------------------------------

fn element_type_for(native: NativeFunction) -> ElementType {
    match native {
        NativeFunction::DataViewPrototypeGetInt8 | NativeFunction::DataViewPrototypeSetInt8 => {
            ElementType::Int8
        }
        NativeFunction::DataViewPrototypeGetUint8 | NativeFunction::DataViewPrototypeSetUint8 => {
            ElementType::Uint8
        }
        NativeFunction::DataViewPrototypeGetInt16 | NativeFunction::DataViewPrototypeSetInt16 => {
            ElementType::Int16
        }
        NativeFunction::DataViewPrototypeGetUint16 | NativeFunction::DataViewPrototypeSetUint16 => {
            ElementType::Uint16
        }
        NativeFunction::DataViewPrototypeGetInt32 | NativeFunction::DataViewPrototypeSetInt32 => {
            ElementType::Int32
        }
        NativeFunction::DataViewPrototypeGetUint32 | NativeFunction::DataViewPrototypeSetUint32 => {
            ElementType::Uint32
        }
        NativeFunction::DataViewPrototypeGetFloat32
        | NativeFunction::DataViewPrototypeSetFloat32 => ElementType::Float32,
        NativeFunction::DataViewPrototypeGetFloat64
        | NativeFunction::DataViewPrototypeSetFloat64 => ElementType::Float64,
        NativeFunction::DataViewPrototypeGetBigInt64
        | NativeFunction::DataViewPrototypeSetBigInt64 => ElementType::BigInt64,
        NativeFunction::DataViewPrototypeGetBigUint64
        | NativeFunction::DataViewPrototypeSetBigUint64 => ElementType::BigUint64,
        _ => unreachable!("data view get/set native expected"),
    }
}

/// Whether `object` carries the `DataView` brand.
pub(crate) fn is_data_view_object(object: &ObjectRef) -> bool {
    object.has_own_property(DATA_VIEW_BUFFER_PROPERTY)
}

fn data_view_receiver(value: &Value) -> Result<ObjectRef, RuntimeError> {
    match value {
        Value::Object(object) if is_data_view_object(object) => Ok(object.clone()),
        _ => Err(type_error(
            "DataView method called on incompatible receiver",
        )),
    }
}

fn data_view_buffer(object: &ObjectRef) -> Option<ObjectRef> {
    match object.own_property(DATA_VIEW_BUFFER_PROPERTY) {
        Some(Property {
            value: Value::Object(buffer),
            ..
        }) => Some(buffer),
        _ => None,
    }
}

fn data_view_buffer_detached(object: &ObjectRef) -> bool {
    data_view_buffer(object).is_some_and(|buffer| data_block_detached(&buffer))
}

fn data_block_detached(buffer: &ObjectRef) -> bool {
    array_buffer::is_array_buffer_object(buffer) && array_buffer::is_detached(buffer)
}

fn data_view_byte_length(object: &ObjectRef) -> usize {
    match object.own_property(DATA_VIEW_BYTE_LENGTH_PROPERTY) {
        Some(Property {
            value: Value::Number(length),
            ..
        }) => length as usize,
        _ => 0,
    }
}

fn data_view_byte_offset(object: &ObjectRef) -> usize {
    match object.own_property(DATA_VIEW_BYTE_OFFSET_PROPERTY) {
        Some(Property {
            value: Value::Number(offset),
            ..
        }) => offset as usize,
        _ => 0,
    }
}

// --- ToIndex / errors --------------------------------------------------------

/// ToIndex: a non-negative integer in `[0, 2^53 - 1]`, RangeError otherwise.
fn to_index(value: Value, env: &mut CallEnv) -> Result<usize, RuntimeError> {
    let number = to_number_with_env(value, env)?;
    let integer = if number.is_nan() { 0.0 } else { number.trunc() };
    if integer < 0.0 || !integer.is_finite() || integer > 9_007_199_254_740_991.0 {
        return Err(range_error("invalid index"));
    }
    Ok(integer as usize)
}

fn type_error(message: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("TypeError: {message}"),
    }
}

fn range_error(message: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("RangeError: {message}"),
    }
}

#[cfg(test)]
mod tests;
