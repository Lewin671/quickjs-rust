use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, Prototype, RuntimeError, Value, array_buffer,
    symbol, to_length_with_env,
};

mod construct;
mod element;

pub(crate) use construct::native_typed_array;

const MAX_TYPED_ARRAY_LENGTH: usize = 1_000_000;

/// Internal slot naming the concrete TypedArray kind (e.g. `"Uint8Array"`).
/// Its presence is also the TypedArray brand used by `ArrayBuffer.isView` and
/// the prototype accessors.
pub(crate) const TYPED_ARRAY_KIND_PROPERTY: &str = "\0TypedArrayKind";
/// Internal slot referencing the backing `ArrayBuffer` object.
pub(crate) const TYPED_ARRAY_BUFFER_PROPERTY: &str = "\0TypedArrayBuffer";
/// Internal slot holding the byte offset of the view into its buffer.
pub(crate) const TYPED_ARRAY_BYTE_OFFSET_PROPERTY: &str = "\0TypedArrayByteOffset";
/// Internal slot holding the element count of the view.
pub(crate) const TYPED_ARRAY_LENGTH_PROPERTY: &str = "\0TypedArrayArrayLength";

/// Whether `object` carries the TypedArray brand.
pub(crate) fn is_typed_array_object(object: &ObjectRef) -> bool {
    object.has_own_property(TYPED_ARRAY_KIND_PROPERTY)
}

/// The eleven concrete TypedArray kinds, in installation order.
const TYPED_ARRAY_KINDS: [(&str, NativeFunction); 11] = [
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
];

pub(crate) fn install_typed_arrays(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    // %TypedArray% intrinsic: the shared [[Prototype]] of every concrete
    // constructor, and the holder of %TypedArray.prototype%.
    let typed_array_prototype =
        ObjectRef::with_prototype(HashMap::new(), Some(object_prototype.clone()));
    install_typed_array_prototype_accessors(env, &typed_array_prototype);

    let typed_array_intrinsic =
        Function::new_native(Some("TypedArray"), 0, NativeFunction::TypedArray, true);
    typed_array_intrinsic.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(typed_array_prototype.clone())),
    );
    typed_array_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(typed_array_intrinsic.clone()),
    );
    symbol::define_species_accessor(env, &typed_array_intrinsic);

    for (name, native) in TYPED_ARRAY_KINDS {
        install_typed_array_constructor(
            env,
            global_this,
            typed_array_prototype.clone(),
            &typed_array_intrinsic,
            name,
            native,
        );
    }
}

/// Installs `buffer`/`byteLength`/`byteOffset`/`length` accessors and the
/// `Symbol.toStringTag` accessor on `%TypedArray.prototype%`.
fn install_typed_array_prototype_accessors(env: &HashMap<String, Value>, prototype: &ObjectRef) {
    for (name, native) in [
        ("buffer", NativeFunction::TypedArrayPrototypeBuffer),
        ("byteLength", NativeFunction::TypedArrayPrototypeByteLength),
        ("byteOffset", NativeFunction::TypedArrayPrototypeByteOffset),
        ("length", NativeFunction::TypedArrayPrototypeLength),
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
    // %TypedArray.prototype%[Symbol.toStringTag] is a configurable accessor with
    // no setter that returns the constructor name for a branded receiver.
    if let Some(symbol) = symbol::to_string_tag_symbol(env) {
        prototype.define_symbol_property(
            symbol,
            Property::accessor(
                Some(Value::Function(Function::new_native(
                    Some("get [Symbol.toStringTag]"),
                    0,
                    NativeFunction::TypedArrayPrototypeToStringTag,
                    false,
                ))),
                None,
                false,
                true,
            ),
        );
    }
}

fn install_typed_array_constructor(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    typed_array_prototype: ObjectRef,
    typed_array_intrinsic: &Function,
    name: &str,
    native: NativeFunction,
) {
    // Each concrete prototype inherits from %TypedArray.prototype%.
    let prototype = ObjectRef::with_prototype(HashMap::new(), Some(typed_array_prototype));
    let bytes = bytes_per_element(native) as f64;

    let constructor = Function::new_native(Some(name), 3, native, true);
    // Concrete constructors inherit from %TypedArray% (a function prototype).
    let _ = constructor
        .set_internal_prototype_slot(Some(Prototype::Function(typed_array_intrinsic.clone())));
    constructor.properties.borrow_mut().insert(
        "BYTES_PER_ELEMENT".to_owned(),
        Property::fixed_non_enumerable(Value::Number(bytes)),
    );
    prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(constructor.clone()),
    );
    prototype.define_property(
        "BYTES_PER_ELEMENT".to_owned(),
        Property::fixed_non_enumerable(Value::Number(bytes)),
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

// --- Prototype accessors ----------------------------------------------------

/// `get %TypedArray.prototype%.buffer`: the backing buffer object.
pub(crate) fn native_typed_array_prototype_buffer(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = typed_array_receiver(&this_value)?;
    match object.own_property(TYPED_ARRAY_BUFFER_PROPERTY) {
        Some(Property { value, .. }) => Ok(value),
        None => Ok(Value::Undefined),
    }
}

/// `get %TypedArray.prototype%[Symbol.toStringTag]`: the kind name, or
/// `undefined` for a non-branded receiver (the accessor never throws).
pub(crate) fn native_typed_array_prototype_to_string_tag(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    match this_value {
        Value::Object(object) if is_typed_array_object(&object) => {
            match object.own_property(TYPED_ARRAY_KIND_PROPERTY) {
                Some(Property {
                    value: Value::String(name),
                    ..
                }) => Ok(Value::String(name)),
                _ => Ok(Value::Undefined),
            }
        }
        _ => Ok(Value::Undefined),
    }
}

/// `get %TypedArray.prototype%.byteLength`.
pub(crate) fn native_typed_array_prototype_byte_length(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = typed_array_receiver(&this_value)?;
    if typed_array_buffer_detached(&object) {
        return Ok(Value::Number(0.0));
    }
    let length = typed_array_length(&object);
    let element = bytes_per_element(typed_array_kind(&object)) as f64;
    Ok(Value::Number(length as f64 * element))
}

/// `get %TypedArray.prototype%.byteOffset`.
pub(crate) fn native_typed_array_prototype_byte_offset(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = typed_array_receiver(&this_value)?;
    if typed_array_buffer_detached(&object) {
        return Ok(Value::Number(0.0));
    }
    Ok(Value::Number(typed_array_byte_offset(&object) as f64))
}

/// `get %TypedArray.prototype%.length`.
pub(crate) fn native_typed_array_prototype_length(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let object = typed_array_receiver(&this_value)?;
    if typed_array_buffer_detached(&object) {
        return Ok(Value::Number(0.0));
    }
    Ok(Value::Number(typed_array_length(&object) as f64))
}

fn typed_array_receiver(value: &Value) -> Result<ObjectRef, RuntimeError> {
    match value {
        Value::Object(object) if is_typed_array_object(object) => Ok(object.clone()),
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: TypedArray method called on incompatible receiver".to_owned(),
        }),
    }
}

// --- Internal-slot helpers ---------------------------------------------------

pub(crate) fn typed_array_kind(object: &ObjectRef) -> NativeFunction {
    match object.own_property(TYPED_ARRAY_KIND_PROPERTY) {
        Some(Property {
            value: Value::String(name),
            ..
        }) => native_for_name(&name),
        _ => NativeFunction::Uint8Array,
    }
}

pub(crate) fn typed_array_length(object: &ObjectRef) -> usize {
    match object.own_property(TYPED_ARRAY_LENGTH_PROPERTY) {
        Some(Property {
            value: Value::Number(length),
            ..
        }) => length as usize,
        _ => 0,
    }
}

pub(crate) fn typed_array_byte_offset(object: &ObjectRef) -> usize {
    match object.own_property(TYPED_ARRAY_BYTE_OFFSET_PROPERTY) {
        Some(Property {
            value: Value::Number(offset),
            ..
        }) => offset as usize,
        _ => 0,
    }
}

pub(crate) fn typed_array_buffer(object: &ObjectRef) -> Option<ObjectRef> {
    match object.own_property(TYPED_ARRAY_BUFFER_PROPERTY) {
        Some(Property {
            value: Value::Object(buffer),
            ..
        }) => Some(buffer),
        _ => None,
    }
}

pub(crate) fn typed_array_buffer_detached(object: &ObjectRef) -> bool {
    typed_array_buffer(object).is_some_and(|buffer| array_buffer::is_detached(&buffer))
}

// --- Element coercion --------------------------------------------------------

pub(crate) use element::coerce_element;

pub(crate) fn modulo_integer(number: f64, modulo: f64) -> f64 {
    if !number.is_finite() || number == 0.0 {
        return 0.0;
    }
    let integer = number.trunc();
    ((integer % modulo) + modulo) % modulo
}

pub(crate) fn signed_integer(number: f64, bits: u32) -> f64 {
    let modulo = 2_f64.powi(bits as i32);
    let value = modulo_integer(number, modulo);
    let sign = 2_f64.powi(bits as i32 - 1);
    if value >= sign { value - modulo } else { value }
}

pub(crate) fn clamp_uint8(number: f64) -> f64 {
    if number.is_nan() || number <= 0.0 {
        0.0
    } else if number >= 255.0 {
        255.0
    } else {
        // Round half to even (per Uint8Clamped conversion).
        let floor = number.floor();
        let diff = number - floor;
        if diff < 0.5 {
            floor
        } else if diff > 0.5 {
            floor + 1.0
        } else if (floor as i64) % 2 == 0 {
            floor
        } else {
            floor + 1.0
        }
    }
}

pub(crate) fn is_big_int_kind(native: NativeFunction) -> bool {
    matches!(
        native,
        NativeFunction::BigInt64Array | NativeFunction::BigUint64Array
    )
}

pub(crate) fn bytes_per_element(native: NativeFunction) -> usize {
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

pub(crate) fn to_typed_array_length(
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

pub(crate) fn typed_array_name(native: NativeFunction) -> &'static str {
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

fn native_for_name(name: &str) -> NativeFunction {
    TYPED_ARRAY_KINDS
        .iter()
        .find(|(kind, _)| *kind == name)
        .map(|(_, native)| *native)
        .unwrap_or(NativeFunction::Uint8Array)
}

#[cfg(test)]
mod tests;
