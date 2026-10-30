//! The intrinsic Array prototype is an ObjectRef with array internal methods.
//! Its indices remain in NamedStore; length/index algorithms run before named
//! storage. No borrow crosses length coercion or another observable operation.
use super::{PropertyDescriptor, descriptor_record::resolve_property_definition};
use crate::{CallEnv, ObjectRef, RuntimeError, Value};

pub(crate) fn define_array_prototype_property(
    object: &ObjectRef,
    key: &str,
    mut descriptor: PropertyDescriptor,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    if key == "length" {
        let new_len = descriptor
            .value
            .clone()
            .map(|value| super::descriptor::array_length_from_array_set_length_value(value, env))
            .transpose()?;
        if let Some(length) = new_len {
            descriptor.value = Some(Value::Number(length as f64));
        }
        let old = object
            .own_property("length")
            .expect("intrinsic array length");
        let old_len = match old.value {
            Value::Number(n) => n as usize,
            _ => unreachable!("array length is numeric"),
        };
        let Some(mut next) = resolve_property_definition(Some(old), descriptor) else {
            return Ok(false);
        };
        if let Some(length) = new_len
            && length < old_len
        {
            let writable = next.writable;
            next.writable = true;
            object.define_property("length".into(), next.clone());
            let mut indices: Vec<_> = object
                .own_property_names()
                .into_iter()
                .filter_map(|key| crate::array_index_property_key(&key))
                .filter(|&index| index >= length)
                .collect();
            indices.sort_unstable_by(|a, b| b.cmp(a));
            for index in indices {
                if !object.delete_own_property(&index.to_string()) {
                    next.value = Value::Number((index as f64) + 1.0);
                    next.writable = writable;
                    object.define_property("length".into(), next);
                    return Ok(false);
                }
            }
            next.writable = writable;
        }
        object.define_property("length".into(), next);
        return Ok(true);
    }
    let length = object
        .own_property("length")
        .expect("intrinsic array length");
    let old_len = match length.value {
        Value::Number(n) => n as usize,
        _ => unreachable!("array length is numeric"),
    };
    let index = crate::array_index_property_key(key);
    if index.is_some_and(|index| index >= old_len && !length.writable) {
        return Ok(false);
    }
    let existing = object.own_property(key);
    if existing.is_none() && !object.is_extensible() {
        return Ok(false);
    }
    let Some(property) = resolve_property_definition(existing, descriptor) else {
        return Ok(false);
    };
    object.define_property(key.to_owned(), property);
    if let Some(index) = index
        && index >= old_len
    {
        let mut length = length;
        length.value = Value::Number((index + 1) as f64);
        object.define_property("length".into(), length);
    }
    Ok(true)
}

pub(crate) fn set_array_prototype_length(
    object: &ObjectRef,
    value: Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    if !object
        .own_property("length")
        .expect("intrinsic array length")
        .writable
    {
        return Ok(false);
    }
    define_array_prototype_property(object, "length", PropertyDescriptor::data_value(value), env)
}
