//! Named-property fast paths for the compact executor's `GetPropNamed` and
//! `SetPropNamed` operations.
//!
//! These mirror the interpreter's own cached handlers -- `try_cached_get_string`
//! and `set_named_prop` -- without a `Vm`: they read and write the same
//! per-site [`NamedPropertyCache`] state and fall back to the same general
//! [[Get]]/[[Set]] implementations, so a body executed on the compact tier
//! cannot observe a property result the ordinary interpreter would not have
//! produced. The slow fallbacks run in an empty realm frame exactly like the
//! tier's binary-coercion path: user hooks carry their own cells and global
//! effects go through the realm.

use std::rc::Rc;

use crate::bytecode::named_property_cache::{CacheProbe, NamedPropertyCache};
use crate::function::CallEnv;
use crate::value::{OwnDataPropertyRead, OwnDataPropertyWrite};
use crate::{PropertyKey, RuntimeError, Value};

/// Reads the statically named property `key` of `object`, consulting `cache`
/// before the general path, exactly as `Op::GetPropNamed` does.
#[cold]
#[inline(never)]
pub(super) fn get_prop_named(
    object: Value,
    key: &Rc<str>,
    cache: &NamedPropertyCache,
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    if let Value::Object(object_ref) = &object
        && !crate::symbol::is_symbol_primitive(object_ref)
        && !crate::typed_array::is_typed_array_object(object_ref)
        && !object_ref.is_module_namespace_exotic()
    {
        let probe = cache.probe(object_ref);
        if let CacheProbe::Own(value) = probe {
            return Ok(value);
        }
        match object_ref.own_data_property_read(key) {
            OwnDataPropertyRead::Data(value) => {
                cache.update(object_ref, key, &value);
                return Ok(value);
            }
            // The receiver has no own property of this name; the candidate the
            // probe retained is the same prototype answer the interpreter uses
            // once that receiver miss is proven.
            OwnDataPropertyRead::Missing => {
                if let CacheProbe::PrototypeCandidate { holder, slot } = probe
                    && let Some(value) = holder.own_data_slot_value(slot)
                {
                    return Ok(value);
                }
                let mut call_env = env.empty_frame();
                let value =
                    crate::bytecode::vm_props::get_property(object.clone(), key, &mut call_env)?;
                cache.update_from_prototype(object_ref, key);
                return Ok(value);
            }
            OwnDataPropertyRead::NeedsSlowPath => {}
        }
    }
    cache.clear();
    let mut call_env = env.empty_frame();
    crate::bytecode::vm_props::get_property(object, key, &mut call_env)
}

/// Writes `value` to the statically named property `key` of `object`, honoring
/// `is_strict`, and returns the assigned value to leave in the object's
/// register, exactly as `Op::SetPropNamed` does.
#[cold]
#[inline(never)]
pub(super) fn set_prop_named(
    object: Value,
    key: &Rc<str>,
    cache: Option<&NamedPropertyCache>,
    is_strict: bool,
    value: Value,
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    let updates_global_binding = is_global_object(env, &object);
    if !updates_global_binding
        && let Value::Object(object_ref) = &object
        && !crate::symbol::is_symbol_primitive(object_ref)
    {
        let cached = cache.and_then(|cache| cache.write(object_ref, key, &value));
        let cached_hit = cached.is_some();
        let write =
            cached.unwrap_or_else(|| object_ref.write_existing_own_data_property(key, &value));
        match write {
            OwnDataPropertyWrite::Written => {
                if !cached_hit && let Some(cache) = cache {
                    cache.record_write(object_ref, key);
                }
                return Ok(value);
            }
            OwnDataPropertyWrite::ReadOnly => {
                if is_strict {
                    return Err(read_only_set_error());
                }
                return Ok(value);
            }
            OwnDataPropertyWrite::NeedsSlowPath => {
                if try_create_ordinary_own_data_property(object_ref, Rc::clone(key), &value) {
                    if let Some(cache) = cache {
                        cache.record_write(object_ref, key);
                    }
                    return Ok(value);
                }
            }
        }
    }
    // The general fallback mirrors `Vm::set_property_value`: the named key is
    // always a string, so a Symbol-primitive receiver reaches the ordinary
    // primitive-property semantics below rather than the symbol-key check.
    let mut call_env = env.empty_frame();
    let wrote_data = crate::bytecode::vm_set::set_property_key(
        object,
        PropertyKey::String(key.to_string()),
        value.clone(),
        &mut call_env,
    )?;
    if !wrote_data && is_strict {
        return Err(read_only_set_error());
    }
    if updates_global_binding && wrote_data {
        env.insert_realm(key.to_string(), value.clone());
    }
    Ok(value)
}

/// Whether `object` is the realm's global object, matching
/// `Vm::is_global_object` without a VM-local cache.
fn is_global_object(env: &CallEnv, object: &Value) -> bool {
    let Value::Object(object_ref) = object else {
        return false;
    };
    match env.global_this() {
        Some(Value::Object(global_this)) => object_ref.ptr_eq(&global_this),
        _ => false,
    }
}

/// Creates a missing ordinary own string data property without cloning the
/// call environment when the complete [[Set]] result is already known.
/// Mirrors `Vm::try_create_ordinary_own_data_property`.
fn try_create_ordinary_own_data_property(
    object: &crate::ObjectRef,
    key: Rc<str>,
    value: &Value,
) -> bool {
    if crate::symbol::is_symbol_primitive(object)
        || crate::typed_array::is_typed_array_object(object)
        || object.is_module_namespace_exotic()
        || !object.is_extensible()
        || !matches!(
            object.own_data_property_read(&key),
            OwnDataPropertyRead::Missing
        )
    {
        return false;
    }
    let mut current = object.prototype_slot();
    loop {
        match current {
            Some(crate::Prototype::Object(prototype)) => {
                if crate::symbol::is_symbol_primitive(&prototype)
                    || crate::typed_array::is_typed_array_object(&prototype)
                    || prototype.is_module_namespace_exotic()
                {
                    return false;
                }
                if let Some(property) = prototype.own_property(&key) {
                    if property.is_accessor() || !property.writable {
                        return false;
                    }
                    if !object.create_absent_own_data_property(key.clone(), value.clone()) {
                        object.set_shared_key(key, value.clone());
                    }
                    return true;
                }
                current = prototype.prototype_slot();
            }
            Some(
                crate::Prototype::Array(_)
                | crate::Prototype::Function(_)
                | crate::Prototype::Proxy(_),
            ) => {
                return false;
            }
            None => {
                if !object.create_absent_own_data_property(key.clone(), value.clone()) {
                    object.set_shared_key(key, value.clone());
                }
                return true;
            }
        }
    }
}

#[cold]
fn read_only_set_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: cannot set property".to_owned(),
    }
}

/// Reads `object[key]` with a computed key, exactly as `Vm::get_prop` does:
/// the `null`/`undefined` diagnostics, the dense-array and typed-array index
/// fast paths, then the general [[Get]] after `ToPropertyKey` in an empty
/// realm frame.
#[inline(never)]
pub(super) fn get_prop_computed(
    object: Value,
    key_value: Value,
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    if matches!(object, Value::Null | Value::Undefined) {
        let object_name = if matches!(object, Value::Null) {
            "null"
        } else {
            "undefined"
        };
        let key_name = match &key_value {
            Value::String(key) => Some(key.to_string()),
            Value::Number(number) => Some(number.to_string()),
            _ => None,
        };
        let message = match key_name {
            Some(key) => {
                format!("TypeError: Cannot read properties of {object_name} (reading '{key}')")
            }
            None => format!("TypeError: cannot convert {object_name} to object"),
        };
        return Err(RuntimeError {
            thrown: None,
            message,
        });
    }
    if let Value::Number(number) = &key_value
        && let Some(index) = crate::bytecode::vm_props::array_index_from_number(*number)
    {
        if let Value::Array(elements) = &object
            && let Some(value) = elements.direct_dense_index_value(index)
        {
            return Ok(value);
        }
        if let Value::Object(object) = &object
            && crate::typed_array::is_typed_array_object(object)
        {
            return Ok(crate::typed_array::integer_indexed_value(object, index));
        }
    }
    // A string key on an ordinary object needs no owned `PropertyKey`: the
    // borrowed chain walk is the same one `Vm::try_direct_get_string` uses,
    // and `ToPropertyKey` on a string is side-effect free, so trying it first
    // is observably identical to the general path below.
    if let (Value::String(name), Value::Object(object_ref)) = (&key_value, &object)
        && !crate::symbol::is_symbol_primitive(object_ref)
        && !crate::typed_array::is_typed_array_object(object_ref)
        && !object_ref.is_module_namespace_exotic()
    {
        use crate::bytecode::vm_props::{DirectPropertyRead, ordinary_chain_data_value};
        match ordinary_chain_data_value(object_ref, name.as_str()) {
            Ok(DirectPropertyRead::Data(value)) => return Ok(value),
            Ok(DirectPropertyRead::Missing) => return Ok(Value::Undefined),
            Ok(DirectPropertyRead::NeedsSlowPath) | Err(_) => {}
        }
    }
    let mut call_env = env.empty_frame();
    let key = match crate::property::try_to_property_key_without_coercion(key_value) {
        Ok(key) => key,
        Err(value) => crate::to_property_key_value(value, &mut call_env)?,
    };
    crate::bytecode::vm_props::get_property_key(object, &key, &mut call_env)
}
