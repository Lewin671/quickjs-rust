use std::collections::HashMap;

use crate::CallEnv;
use crate::{
    Function, NativeFunction, ObjectRef, Property, PropertyKey, Prototype, RuntimeError, Value,
    array_buffer, construct_function, ensure_constructor, property_value, property_value_key,
    symbol, to_number_with_env,
};

mod construct;
mod element;
mod iteration;
mod ordering;

pub(crate) use construct::{native_typed_array, native_typed_array_from, native_typed_array_of};
pub(crate) use element::{
    IndexedDefine, IndexedDelete, IndexedRead, IndexedWrite, define_indexed_element_value,
    define_indexed_property_descriptor, delete_indexed_element, get_view_element,
    indexed_element_value, read_view_elements, set_indexed_element, set_view_elements,
};
pub(crate) use iteration::*;
pub(crate) use ordering::*;

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
/// Internal slot marking views whose length tracks a resizable ArrayBuffer.
pub(crate) const TYPED_ARRAY_LENGTH_TRACKING_PROPERTY: &str = "\0TypedArrayLengthTracking";

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
    env: &mut CallEnv,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    // %TypedArray% intrinsic: the shared [[Prototype]] of every concrete
    // constructor, and the holder of %TypedArray.prototype%.
    let typed_array_prototype =
        ObjectRef::with_prototype(HashMap::new(), Some(object_prototype.clone()));
    install_typed_array_prototype_accessors(env, &typed_array_prototype);
    install_typed_array_prototype_methods(env, &typed_array_prototype);

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
    install_typed_array_static_methods(&typed_array_intrinsic);

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

/// ES specifies `%TypedArray%.prototype.toString` as the same function object
/// as `%Array.prototype.toString%`. Array is installed after TypedArray in this
/// runtime, so the alias is patched once both prototypes exist.
pub(crate) fn alias_array_prototype_to_string(env: &mut CallEnv) {
    let array_constructor = match env.get("Array") {
        Some(Value::Function(function)) => function,
        _ => return,
    };
    let array_prototype = match property_value(Value::Function(array_constructor), "prototype", env)
    {
        Ok(Value::Object(object)) => object,
        _ => return,
    };
    let array_to_string = match array_prototype.own_property("toString") {
        Some(property) => property.value,
        None => return,
    };

    let typed_array_constructor = match env.get("Uint8Array") {
        Some(Value::Function(function)) => function,
        _ => return,
    };
    let typed_array_intrinsic = match typed_array_constructor.internal_prototype_slot() {
        Some(Some(Prototype::Function(function))) => function,
        _ => return,
    };
    let typed_array_prototype = match typed_array_intrinsic.properties.borrow().get("prototype") {
        Some(Property {
            value: Value::Object(object),
            ..
        }) => object.clone(),
        _ => return,
    };
    typed_array_prototype.define_non_enumerable("toString".to_owned(), array_to_string);
}

/// Installs the `%TypedArray%.from` and `%TypedArray%.of` static methods on the
/// shared intrinsic; concrete constructors inherit them through the function
/// prototype chain.
fn install_typed_array_static_methods(intrinsic: &Function) {
    for (name, length, native) in [
        ("from", 1, NativeFunction::TypedArrayFrom),
        ("of", 0, NativeFunction::TypedArrayOf),
    ] {
        intrinsic.properties.borrow_mut().insert(
            name.to_owned(),
            Property::non_enumerable(Value::Function(Function::new_native(
                Some(name),
                length,
                native,
                false,
            ))),
        );
    }
}

/// Installs `buffer`/`byteLength`/`byteOffset`/`length` accessors and the
/// `Symbol.toStringTag` accessor on `%TypedArray.prototype%`.
fn install_typed_array_prototype_accessors(env: &CallEnv, prototype: &ObjectRef) {
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

/// Installs the shared `%TypedArray.prototype%` methods (ES2023 23.2.3),
/// brand-checked through their receiver.
fn install_typed_array_prototype_methods(env: &CallEnv, prototype: &ObjectRef) {
    for (name, length, native) in [
        ("at", 1, NativeFunction::TypedArrayPrototypeAt),
        ("indexOf", 1, NativeFunction::TypedArrayPrototypeIndexOf),
        (
            "lastIndexOf",
            1,
            NativeFunction::TypedArrayPrototypeLastIndexOf,
        ),
        ("includes", 1, NativeFunction::TypedArrayPrototypeIncludes),
        ("join", 1, NativeFunction::TypedArrayPrototypeJoin),
        ("keys", 0, NativeFunction::TypedArrayPrototypeKeys),
        ("values", 0, NativeFunction::TypedArrayPrototypeValues),
        ("entries", 0, NativeFunction::TypedArrayPrototypeEntries),
        ("forEach", 1, NativeFunction::TypedArrayPrototypeForEach),
        ("map", 1, NativeFunction::TypedArrayPrototypeMap),
        ("filter", 1, NativeFunction::TypedArrayPrototypeFilter),
        ("reduce", 1, NativeFunction::TypedArrayPrototypeReduce),
        (
            "reduceRight",
            1,
            NativeFunction::TypedArrayPrototypeReduceRight,
        ),
        ("some", 1, NativeFunction::TypedArrayPrototypeSome),
        ("every", 1, NativeFunction::TypedArrayPrototypeEvery),
        ("find", 1, NativeFunction::TypedArrayPrototypeFind),
        ("findIndex", 1, NativeFunction::TypedArrayPrototypeFindIndex),
        ("findLast", 1, NativeFunction::TypedArrayPrototypeFindLast),
        (
            "findLastIndex",
            1,
            NativeFunction::TypedArrayPrototypeFindLastIndex,
        ),
        ("slice", 2, NativeFunction::TypedArrayPrototypeSlice),
        ("subarray", 2, NativeFunction::TypedArrayPrototypeSubarray),
        ("toString", 0, NativeFunction::TypedArrayPrototypeToString),
        (
            "toLocaleString",
            0,
            NativeFunction::TypedArrayPrototypeToLocaleString,
        ),
        ("set", 1, NativeFunction::TypedArrayPrototypeSet),
        ("fill", 1, NativeFunction::TypedArrayPrototypeFill),
        (
            "copyWithin",
            2,
            NativeFunction::TypedArrayPrototypeCopyWithin,
        ),
        ("reverse", 0, NativeFunction::TypedArrayPrototypeReverse),
        ("sort", 1, NativeFunction::TypedArrayPrototypeSort),
        (
            "toReversed",
            0,
            NativeFunction::TypedArrayPrototypeToReversed,
        ),
        ("toSorted", 1, NativeFunction::TypedArrayPrototypeToSorted),
        ("with", 2, NativeFunction::TypedArrayPrototypeWith),
    ] {
        define_prototype_method(prototype, name, length, native);
    }
    // %TypedArray.prototype%[Symbol.iterator] is the same function object as
    // `values`.
    symbol::define_well_known_iterator_alias(env, prototype, "values");
}

fn define_prototype_method(
    prototype: &ObjectRef,
    name: &str,
    length: usize,
    native: NativeFunction,
) {
    prototype.define_non_enumerable(
        name.to_owned(),
        Value::Function(Function::new_native(Some(name), length, native, false)),
    );
}

fn install_typed_array_constructor(
    env: &mut CallEnv,
    global_this: &Value,
    typed_array_prototype: ObjectRef,
    typed_array_intrinsic: &Function,
    name: &str,
    native: NativeFunction,
) {
    // Each concrete prototype inherits from %TypedArray.prototype%.
    let prototype = ObjectRef::with_prototype(HashMap::new(), Some(typed_array_prototype));
    let bytes = bytes_per_element(native) as f64;
    if native == NativeFunction::Uint8Array {
        install_uint8_array_prototype_methods(&prototype);
    }

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
    env.insert_realm(name.to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable(name.to_owned(), value);
    }
}

fn install_uint8_array_prototype_methods(prototype: &ObjectRef) {
    define_prototype_method(
        prototype,
        "setFromHex",
        1,
        NativeFunction::Uint8ArrayPrototypeSetFromHex,
    );
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
    if typed_array_is_out_of_bounds(&object) {
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
        _ => Err(typed_array_receiver_error()),
    }
}

pub(crate) fn typed_array_receiver_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: TypedArray method called on incompatible receiver".to_owned(),
    }
}

/// Brand-checks `value` as a typed array and validates its buffer is attached,
/// throwing `TypeError` otherwise. Returns the receiver and its current length.
pub(crate) fn validate_typed_array(value: &Value) -> Result<(ObjectRef, usize), RuntimeError> {
    let object = typed_array_receiver(value)?;
    if typed_array_buffer_detached(&object) {
        return Err(array_buffer::detached_error());
    }
    if typed_array_is_out_of_bounds(&object) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: TypedArray is out of bounds".to_owned(),
        });
    }
    Ok((object.clone(), typed_array_length(&object)))
}

/// Brand-checks `value` as a writable typed array, rejecting immutable backing
/// buffers before argument coercion can run.
pub(crate) fn validate_typed_array_write(
    value: &Value,
) -> Result<(ObjectRef, usize), RuntimeError> {
    let (object, length) = validate_typed_array(value)?;
    if typed_array_buffer(&object).is_some_and(|buffer| array_buffer::is_immutable(&buffer)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: ArrayBuffer is immutable".to_owned(),
        });
    }
    Ok((object, length))
}

/// Brand-checks `value` as an attached typed array, but preserves the current
/// length computation for out-of-bounds resizable-buffer views.
pub(crate) fn validate_typed_array_length(
    value: &Value,
) -> Result<(ObjectRef, usize), RuntimeError> {
    let object = typed_array_receiver(value)?;
    if typed_array_buffer_detached(&object) {
        return Err(array_buffer::detached_error());
    }
    Ok((object.clone(), typed_array_length(&object)))
}

/// Builds a fresh typed array of `native`'s kind backed by a new buffer, with
/// the given already-coerced element values, materializing index reads. Used by
/// the methods that return a new typed array (`map`, `filter`, `slice`, …).
pub(crate) fn create_typed_array_of_kind(
    native: NativeFunction,
    values: Vec<Value>,
    env: &CallEnv,
) -> ObjectRef {
    construct::create_with_values(native, values, env)
}

/// Implements TypedArraySpeciesCreate for prototype methods that allocate a
/// result through the receiver's constructor / @@species hook.
pub(crate) fn typed_array_species_create(
    exemplar: &ObjectRef,
    length: usize,
    env: &mut CallEnv,
) -> Result<(Value, ObjectRef), RuntimeError> {
    let default_constructor = env
        .get(typed_array_name(typed_array_kind(exemplar)))
        .unwrap_or(Value::Undefined);
    let constructor = typed_array_species_constructor(exemplar, default_constructor, env)?;
    ensure_constructor(&constructor).map_err(|_| RuntimeError {
        thrown: None,
        message: "TypeError: TypedArray species is not a constructor".to_owned(),
    })?;
    let result = construct_function(
        constructor.clone(),
        constructor,
        vec![Value::Number(length as f64)],
        env,
    )?;
    let (object, actual_length) = validate_typed_array(&result)?;
    if actual_length < length {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: typed array species result is too short".to_owned(),
        });
    }
    Ok((result, object))
}

fn typed_array_species_constructor(
    exemplar: &ObjectRef,
    default_constructor: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let constructor = property_value(Value::Object(exemplar.clone()), "constructor", env)?;
    if matches!(constructor, Value::Undefined) {
        return Ok(default_constructor);
    }
    if !is_object_like(&constructor) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: TypedArray constructor property is not an object".to_owned(),
        });
    }
    let species = match symbol::species_symbol(env) {
        Some(symbol) => property_value_key(constructor, &PropertyKey::Symbol(symbol), env)?,
        None => Value::Undefined,
    };
    if matches!(species, Value::Undefined | Value::Null) {
        return Ok(default_constructor);
    }
    Ok(species)
}

fn is_object_like(value: &Value) -> bool {
    if matches!(value, Value::Object(object) if symbol::is_symbol_primitive(object)) {
        return false;
    }
    matches!(
        value,
        Value::Object(_) | Value::Function(_) | Value::Array(_) | Value::Map(_) | Value::Set(_)
    ) || matches!(value, Value::Proxy(_))
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
    let fixed_length = match object.own_property(TYPED_ARRAY_LENGTH_PROPERTY) {
        Some(Property {
            value: Value::Number(length),
            ..
        }) => length as usize,
        _ => return 0,
    };
    let Some(buffer) = typed_array_buffer(object) else {
        return fixed_length;
    };
    if array_buffer::is_detached(&buffer) {
        return 0;
    }
    let buffer_byte_length = array_buffer::buffer_bytes(&buffer).len();
    let offset = typed_array_byte_offset(object);
    let element = bytes_per_element(typed_array_kind(object));
    if typed_array_is_length_tracking(object) {
        if offset > buffer_byte_length {
            return 0;
        }
        return (buffer_byte_length - offset) / element;
    }
    let Some(byte_length) = fixed_length.checked_mul(element) else {
        return 0;
    };
    if offset
        .checked_add(byte_length)
        .is_none_or(|end| end > buffer_byte_length)
    {
        0
    } else {
        fixed_length
    }
}

pub(crate) fn typed_array_is_length_tracking(object: &ObjectRef) -> bool {
    matches!(
        object.own_property(TYPED_ARRAY_LENGTH_TRACKING_PROPERTY),
        Some(Property {
            value: Value::Boolean(true),
            ..
        })
    )
}

pub(crate) fn typed_array_is_out_of_bounds(object: &ObjectRef) -> bool {
    let Some(buffer) = typed_array_buffer(object) else {
        return false;
    };
    if array_buffer::is_detached(&buffer) {
        return false;
    }
    if typed_array_is_length_tracking(object) {
        typed_array_byte_offset(object) > array_buffer::buffer_bytes(&buffer).len()
    } else {
        let fixed_length = match object.own_property(TYPED_ARRAY_LENGTH_PROPERTY) {
            Some(Property {
                value: Value::Number(length),
                ..
            }) => length as usize,
            _ => 0,
        };
        let element = bytes_per_element(typed_array_kind(object));
        let byte_length = fixed_length.checked_mul(element);
        byte_length.is_none_or(|byte_length| {
            typed_array_byte_offset(object)
                .checked_add(byte_length)
                .is_none_or(|end| end > array_buffer::buffer_bytes(&buffer).len())
        })
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

/// Integer-indexed exotic `[[OwnPropertyKeys]]` string-keyed portion. The
/// indexed keys are derived from the current effective view length so resizable
/// buffers can grow or shrink after the view was created; ordinary non-index
/// string properties keep the object's stored insertion order after those
/// indices.
pub(crate) fn typed_array_own_property_names(object: &ObjectRef) -> Vec<String> {
    typed_array_own_property_strings(object, false)
}

/// Enumerable string-keyed portion used by `Object.keys`/entries/values.
pub(crate) fn typed_array_own_property_keys(object: &ObjectRef) -> Vec<String> {
    typed_array_own_property_strings(object, true)
}

pub(crate) fn typed_array_own_property_descriptor(
    object: &ObjectRef,
    key: &str,
) -> Option<Property> {
    match indexed_element_value(object, key) {
        IndexedRead::Present(value) => Some(Property::data(*value, true, true, true)),
        IndexedRead::Missing => None,
        IndexedRead::NotIndexed => object.own_property(key),
    }
}

fn typed_array_own_property_strings(object: &ObjectRef, enumerable_only: bool) -> Vec<String> {
    let mut keys: Vec<String> = (0..typed_array_length(object))
        .map(|index| index.to_string())
        .collect();
    let stored = if enumerable_only {
        object.own_property_keys()
    } else {
        object.own_property_names()
    };
    keys.extend(
        stored
            .into_iter()
            .filter(|key| array_index_property_key(key).is_none()),
    );
    keys
}

fn array_index_property_key(key: &str) -> Option<u32> {
    key.parse::<u32>()
        .ok()
        .filter(|index| *index < u32::MAX && index.to_string() == key)
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
    env: &mut CallEnv,
) -> Result<usize, RuntimeError> {
    let number = to_number_with_env(value, env)?;
    let integer = if number.is_nan() { 0.0 } else { number.trunc() };
    if integer < 0.0 || !integer.is_finite() {
        return Err(RuntimeError {
            thrown: None,
            message: "RangeError: invalid typed array length".to_owned(),
        });
    }
    let length = integer as usize;
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
mod ordering_tests;
#[cfg(test)]
mod tests;
