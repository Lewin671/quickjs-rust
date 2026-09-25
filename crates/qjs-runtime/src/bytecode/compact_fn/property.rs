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

/// The hit path of `get_prop_named`, inlined into the driver's arms: an
/// ordinary object the site's hot cache entry answers (`probe_hot`). Paying
/// `get_prop_named`'s whole frame for that was a third of a read's cost.
#[inline(always)]
pub(super) fn get_prop_named_hot(object: &Value, cache: &NamedPropertyCache) -> Option<Value> {
    let Value::Object(object_ref) = object else {
        return None;
    };
    if crate::symbol::is_symbol_primitive(object_ref)
        || crate::typed_array::is_typed_array_object(object_ref)
        || object_ref.is_module_namespace_exotic()
    {
        return None;
    }
    cache.probe_hot(object_ref)
}

/// Reads the statically named property `key` of `object`, consulting `cache`
/// before the general path, exactly as `Op::GetPropNamed` does.
#[cold]
#[inline(never)]
pub(super) fn get_prop_named(
    object: &Value,
    key: &Rc<str>,
    cache: &NamedPropertyCache,
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    if let Value::Object(object_ref) = object
        && !crate::symbol::is_symbol_primitive(object_ref)
        && !crate::typed_array::is_typed_array_object(object_ref)
        && !object_ref.is_module_namespace_exotic()
    {
        if let Some(value) = cache.probe_hot(object_ref) {
            return Ok(value);
        }
        let probe = cache.probe(object_ref);
        if let CacheProbe::Own(value) = probe {
            return Ok(value);
        }
        if let CacheProbe::PrototypeCandidate { holder, slot } = &probe
            && cache.receiver_miss_proven(object_ref)
            && let Some(value) = holder.prototype_data_slot_value(*slot)
        {
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
                    && let Some(value) = holder.prototype_data_slot_value(slot)
                {
                    cache.remember_receiver_miss(object_ref);
                    return Ok(value);
                }
                // An inherited getter the interpreter would call directly.
                if let Some(result) = crate::bytecode::vm_props::direct_leaf_getter(
                    object,
                    key,
                    env,
                    env.module_host(),
                    #[cfg(feature = "agents")]
                    env.agent_context(),
                ) {
                    return result;
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
    if let Some(value) = prototype_receiver_named_value(object, key, cache, env) {
        return Ok(value);
    }
    cache.clear();
    let mut call_env = env.empty_frame();
    crate::bytecode::vm_props::get_property(object.clone(), key, &mut call_env)
}

/// A named read on a primitive string or number, or an array: `length`, an
/// own index or (for an array) an own named property, then a data property
/// on the realm's live `%String.prototype%`, `%Number.prototype%` or
/// `%Array.prototype%`, as
/// `Vm::try_direct_get_string` answers it. The prototype's answer is the
/// site's cached entry for that prototype object, revalidated by its
/// revision like any receiver's, so `text.charCodeAt` and `list.push` read
/// their method without walking the chain. `None` leaves the read to the
/// general [[Get]].
#[inline(never)]
fn prototype_receiver_named_value(
    object: &Value,
    key: &Rc<str>,
    cache: &NamedPropertyCache,
    env: &CallEnv,
) -> Option<Value> {
    let prototype = match object {
        // A function's own data property -- `String.fromCharCode`,
        // `Array.isArray` -- read where it is.
        Value::Function(function) => {
            return function
                .own_property(key)
                .filter(|property| !property.is_accessor())
                .map(|property| property.value);
        }
        Value::String(text) => {
            if &**key == "length" {
                return Some(Value::Number(
                    crate::string::js_string_code_unit_len(text) as f64
                ));
            }
            if let Some(value) = crate::string::string_property(text, key) {
                return Some(value);
            }
            if env.has_dynamic_function_realm_global() {
                return None;
            }
            env.realm().string_prototype()?
        }
        Value::Number(_) => {
            if env.has_dynamic_function_realm_global() {
                return None;
            }
            env.realm().number_prototype()?
        }
        Value::Array(array) => {
            if &**key == "length" {
                return Some(Value::Number(array.len() as f64));
            }
            if crate::array_index_property_key(key).is_some() || array.property(key).is_some() {
                return None;
            }
            match array.effective_prototype_slot(env)? {
                crate::Prototype::Object(prototype) => prototype,
                _ => return None,
            }
        }
        _ => return None,
    };
    if let CacheProbe::Own(value) = cache.probe(&prototype) {
        return Some(value);
    }
    use crate::bytecode::vm_props::{DirectPropertyRead, ordinary_chain_data_value};
    match prototype.own_data_property_read(key) {
        OwnDataPropertyRead::Data(value) => {
            cache.update(&prototype, key, &value);
            Some(value)
        }
        OwnDataPropertyRead::Missing => match ordinary_chain_data_value(&prototype, key) {
            Ok(DirectPropertyRead::Data(value)) => Some(value),
            Ok(DirectPropertyRead::Missing) => Some(Value::Undefined),
            Ok(DirectPropertyRead::NeedsSlowPath) | Err(_) => None,
        },
        OwnDataPropertyRead::NeedsSlowPath => None,
    }
}

/// The hit path of `set_prop_named`, inlined into the driver's arms: an
/// existing property in a slot a constructor's instances share, on an
/// ordinary object that is not the realm's global object (whose writes also
/// update the realm binding).
#[inline(always)]
pub(super) fn set_prop_named_hot(
    object: &Value,
    cache: Option<&NamedPropertyCache>,
    value: &Value,
    env: &CallEnv,
) -> bool {
    let (Value::Object(object_ref), Some(cache)) = (object, cache) else {
        return false;
    };
    !crate::symbol::is_symbol_primitive(object_ref)
        && !env.is_realm_global_object(object_ref)
        && matches!(
            cache.write_hot(object_ref, value),
            Some(OwnDataPropertyWrite::Written)
        )
}

/// Writes `value` to the statically named property `key` of `object`, honoring
/// `is_strict`, and returns the assigned value to leave in the object's
/// register, exactly as `Op::SetPropNamed` does.
#[cold]
#[inline(never)]
pub(super) fn set_prop_named(
    object: &Value,
    key: &Rc<str>,
    cache: Option<&NamedPropertyCache>,
    is_strict: bool,
    value: Value,
    env: &CallEnv,
    creation: Option<&super::creation_cache::CreationCache>,
) -> Result<Value, RuntimeError> {
    let updates_global_binding = is_global_object(env, object);
    if !updates_global_binding
        && let Value::Object(object_ref) = object
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
                // A constructor's `this.x = x` creates the property on every
                // instance; the site's proof skips the prototype walk.
                if creation.is_some_and(|creation| creation.try_create(object_ref, key, &value)) {
                    return Ok(value);
                }
                if try_create_ordinary_own_data_property(object_ref, Rc::clone(key), &value) {
                    if let Some(creation) = creation {
                        creation.record(object_ref, key);
                    }
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
        object.clone(),
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
    matches!(object, Value::Object(object_ref) if env.is_realm_global_object(object_ref))
}

/// Creates a missing ordinary own string data property without cloning the
/// call environment when the complete [[Set]] result is already known.
/// Mirrors `Vm::try_create_ordinary_own_data_property`.
pub(in crate::bytecode) fn try_create_ordinary_own_data_property(
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

/// `registers[obj][registers[key]] = registers[value]` when the store is
/// plain, leaving the value in `obj`'s register as `Op::SetProp` leaves it on
/// the stack: a dense index of an array whose prototype chain is the realm's
/// ordinary one, or a string-keyed own data property of an ordinary object
/// other than the global object, overwritten or created. These are the
/// interpreter's own `set_prop` fast paths and give its result. Returns
/// `false`, with every register untouched, for any other store, which the
/// caller hands to the interpreter.
#[inline(never)]
pub(super) fn try_plain_set_prop(
    registers: &mut [Value],
    obj: u16,
    key: u16,
    value: u16,
    env: &CallEnv,
) -> bool {
    let (obj, key_register, value_register) = (obj as usize, key as usize, value as usize);
    let (object, key, value) = (
        &registers[obj],
        &registers[key_register],
        &registers[value_register],
    );
    let stored = match object {
        Value::Array(elements) => {
            let index = match key {
                Value::Number(number) => {
                    crate::bytecode::vm_props::array_index_from_number(*number)
                }
                Value::String(key) => crate::bytecode::vm_props::array_index_from_string(key),
                _ => None,
            };
            match index {
                Some(index)
                    if elements.dense_index_store_eligible(index)
                        && array_access_is_plain(elements, env) =>
                {
                    elements.set(index, value.clone());
                    true
                }
                _ => false,
            }
        }
        Value::Object(object_ref) => match key {
            Value::String(name) if !is_global_object(env, object) => {
                match object_ref.write_existing_own_data_property(name.as_str(), value) {
                    OwnDataPropertyWrite::Written => true,
                    OwnDataPropertyWrite::ReadOnly => false,
                    OwnDataPropertyWrite::NeedsSlowPath => try_create_ordinary_own_data_property(
                        object_ref,
                        Rc::from(name.as_str()),
                        value,
                    ),
                }
            }
            _ => false,
        },
        _ => false,
    };
    if stored {
        let value = std::mem::replace(&mut registers[value_register], Value::Undefined);
        registers[key_register] = Value::Undefined;
        registers[obj] = value;
    }
    stored
}

/// `registers[obj][index] = registers[value]` for a constant index when the
/// store is plain -- a dense index of an array with the realm's ordinary
/// prototype chain, or an in-range element of a typed array given a
/// primitive -- leaving the value in `obj`'s register as `Op::SetPropIndex`
/// leaves it on the stack. `false`, with the registers untouched, otherwise.
#[inline(never)]
pub(super) fn try_plain_set_index(
    registers: &mut [Value],
    obj: u16,
    value: u16,
    index: usize,
    env: &CallEnv,
) -> bool {
    let (obj, value_register) = (obj as usize, value as usize);
    let stored = match (&registers[obj], &registers[value_register]) {
        (Value::Array(elements), value) => {
            elements.dense_index_store_eligible(index) && array_access_is_plain(elements, env) && {
                elements.set(index, value.clone());
                true
            }
        }
        (Value::Object(object), value) if crate::typed_array::is_typed_array_object(object) => {
            crate::typed_array::try_set_integer_indexed_primitive_element(object, index, value)
                == Some(true)
        }
        _ => false,
    };
    if stored {
        registers[obj] = std::mem::replace(&mut registers[value_register], Value::Undefined);
    }
    stored
}

/// Whether an element access on `array` meets no index accessor or exotic
/// object on its prototype chain: `Vm::array_uses_realm_prototype` and
/// `Vm::array_prototype_chain_has_index_hazard` without the VM's caches.
pub(in crate::bytecode) fn array_access_is_plain(array: &crate::ArrayRef, env: &CallEnv) -> bool {
    let Some(array_prototype) = crate::property::array_prototype(env) else {
        return false;
    };
    (array.uses_default_prototype() || array.uses_prototype_object(&array_prototype))
        && !crate::bytecode::vm_props::prototype_chain_has_index_hazard(Some(
            crate::Prototype::Object(array_prototype),
        ))
}

/// An object literal of statically known data properties from the values in
/// `registers`, which it takes, as `Vm::new_object_data_literal` builds it:
/// the realm's `Object.prototype`, and each non-constructor function value's
/// home object set to the literal.
#[inline(never)]
pub(super) fn object_data_literal(
    shape: &Rc<crate::value::ObjectLiteralShape>,
    registers: &mut [Value],
    env: &CallEnv,
) -> Value {
    let values: Vec<Value> = registers
        .iter_mut()
        .map(|register| std::mem::replace(register, Value::Undefined))
        .collect();
    let home_functions: Vec<crate::Function> = values
        .iter()
        .filter_map(|value| match value {
            Value::Function(function) if !function.constructable => Some(function.clone()),
            _ => None,
        })
        .collect();
    let prototype = crate::object_prototype(env);
    let object = if let [first, second] = values.as_slice()
        && shape.unique_len() == 2
    {
        crate::ObjectRef::with_literal_pair(
            Rc::clone(shape),
            [first.clone(), second.clone()],
            prototype,
        )
    } else {
        crate::ObjectRef::with_literal_properties(Rc::clone(shape), values, prototype)
    };
    for function in home_functions {
        function.set_home_object(Value::Object(object.clone()));
    }
    Value::Object(object)
}

#[cold]
fn read_only_set_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: cannot set property".to_owned(),
    }
}

/// `object[key]` for a receiver in a register the read consumes: an array
/// element at a number key answers in place, where moving the array into
/// the general computed read cost it a full round of checks; any other
/// receiver is moved out, exactly as before.
#[inline(never)]
pub(super) fn get_prop_element(
    object: &mut Value,
    key: Value,
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    if let (Value::Array(elements), Value::Number(number)) = (&*object, &key)
        && let Some(index) = crate::bytecode::vm_props::array_index_from_number(*number)
        && let Some(value) = elements.plain_dense_index_value(index)
    {
        return Ok(value);
    }
    get_prop_computed(std::mem::replace(object, Value::Undefined), key, env)
}

/// `get_prop_element` of a receiver that stays where it is.
#[inline(never)]
pub(super) fn get_prop_element_of(
    object: &Value,
    key: Value,
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    if let (Value::Array(elements), Value::Number(number)) = (object, &key)
        && let Some(index) = crate::bytecode::vm_props::array_index_from_number(*number)
        && let Some(value) = elements.plain_dense_index_value(index)
    {
        return Ok(value);
    }
    get_prop_computed(
        crate::bytecode::vm_bindings::clone_local_value(object),
        key,
        env,
    )
}

/// `object[index]` for a constant array index, reading the receiver where
/// it is: a present element of a dense array answers without the receiver
/// being cloned into the general computed read and dropped again, which was
/// half the cost of `v[0]` in vector arithmetic.
#[inline(never)]
pub(super) fn get_prop_index(
    object: &Value,
    index: u16,
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    if let Value::Array(elements) = object
        && let Some(value) = elements.plain_dense_index_value(usize::from(index))
    {
        return Ok(value);
    }
    get_prop_computed(
        crate::bytecode::vm_bindings::clone_local_value(object),
        Value::Number(f64::from(index)),
        env,
    )
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
        if let Value::Array(elements) = &object {
            if let Some(value) = elements.direct_dense_index_value(index) {
                return Ok(value);
            }
            // A hole, or an index past the end, of an array whose prototype
            // chain holds no indexed property reads `undefined`: a bucket
            // table made by `new Array(n)` reads its empty buckets this way,
            // which otherwise formatted the index as a key and walked the chain.
            if elements.index_is_absent(index) && array_access_is_plain(elements, env) {
                return Ok(Value::Undefined);
            }
        }
        if let Value::Object(object) = &object
            && crate::typed_array::is_typed_array_object(object)
        {
            return Ok(crate::typed_array::integer_indexed_value(object, index));
        }
        // A string's own integer-keyed properties are exactly its code units,
        // as `Vm::get_prop` answers them: `table[i]` in an encoder loop
        // otherwise built a property key and a realm frame per read.
        if let Value::String(text) = &object
            && let Some(code_unit) = crate::string::js_string_code_unit_at(text, index)
        {
            return Ok(Value::String(crate::string::js_string_from_code_unit(
                code_unit,
            )));
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

/// Reads a global name from a slot-only environment, exactly as
/// `Vm::load_global` resolves it for a function frame that carries no frame
/// bindings, deoptimized bindings, module imports, sloppy fallbacks or
/// immutable function name: the realm binding, then an own property of
/// `globalThis` (invoking its getter), then the ReferenceError.
#[inline(never)]
pub(in crate::bytecode) fn load_global(name: &str, env: &CallEnv) -> Result<Value, RuntimeError> {
    if let Some(value) = env.get(name) {
        if value.is_uninitialized_lexical_marker() {
            return Err(undefined_identifier(name));
        }
        return Ok(value);
    }
    if let Some(Value::Object(global_this)) = env.global_this()
        && global_this.has_own_property(name)
    {
        let mut call_env = env.empty_frame();
        return crate::property_value(Value::Object(global_this), name, &mut call_env);
    }
    Err(undefined_identifier(name))
}

/// A `LoadGlobal` site's memo of the realm cell its name resolved to: valid
/// while the frame reads the realm directly and the table still maps every
/// name to the cell it did (its generation). The table is held, not its
/// address, so a freed table's address cannot be mistaken for it.
#[derive(Clone, Default)]
pub(in crate::bytecode) struct GlobalReadSite(
    std::cell::RefCell<
        Option<(
            crate::function::DynamicBindings,
            u64,
            crate::function::Upvalue,
        )>,
    >,
);

/// [`load_global`] through a site's memo. Hashes `name` only on a miss: a
/// first read, a remapped realm table, or a frame with its own bindings.
#[inline]
pub(in crate::bytecode) fn load_global_cached(
    name: &str,
    site: &GlobalReadSite,
    env: &CallEnv,
) -> Result<Value, RuntimeError> {
    let Some(realm) = env.realm_read_layer() else {
        return load_global(name, env);
    };
    if let Some((bindings, generation, cell)) = &*site.0.borrow()
        && bindings.ptr_eq(realm)
        && *generation == realm.generation()
    {
        let value = cell.get();
        if !value.is_uninitialized_lexical_marker() {
            return Ok(value);
        }
    }
    let value = load_global(name, env)?;
    if name != crate::NEW_TARGET_BINDING
        && let Some(cell) = realm.cell(name)
    {
        *site.0.borrow_mut() = Some((realm.clone(), realm.generation(), cell));
    }
    Ok(value)
}

#[cold]
fn undefined_identifier(name: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("ReferenceError: undefined identifier `{name}`"),
    }
}

/// A sloppy assignment to a global variable, the store
/// `Op::StoreLocalOrGlobalSloppy` performs for a name the function neither
/// declares nor receives: the realm binding and the `globalThis` data
/// property that mirrors it are both written, or both created for a name
/// not yet bound, as the interpreter's store does. Returns `false`, having
/// changed nothing, for anything else -- a lexical or immutable binding, a
/// module binding, an accessor or read-only property, a global property
/// without a realm binding, or a mirror that disagrees with its binding --
/// which the caller leaves to the interpreter.
#[inline(never)]
pub(super) fn try_store_global_var(name: &str, value: &Value, env: &CallEnv) -> bool {
    match plain_global_var(name, env) {
        Some(GlobalVar::Absent(global_this)) => {
            // The first assignment of an undeclared name creates it, as the
            // interpreter's store does: a global data property and the realm
            // binding that mirrors it.
            global_this.set(name.to_owned(), value.clone());
            env.insert_realm(name.to_owned(), value.clone());
            true
        }
        Some(GlobalVar::Bound(global_this, cell)) => {
            if !matches!(
                global_this.write_existing_own_data_property(name, value),
                OwnDataPropertyWrite::Written
            ) {
                return false;
            }
            env.replace_existing_realm_with_cell(name, value.clone(), &cell)
        }
        None => false,
    }
}

/// `name = name + right` for the global variable `name`, whose value `left`
/// was read into a register: two numbers are summed and stored; otherwise the
/// concatenation extends the string in place,
/// as the interpreter's compound assignment does, instead of copying the
/// whole accumulator. The binding's two mirrors -- the realm cell and the
/// `globalThis` property -- are released first, so the register holds the
/// only reference unless JavaScript holds another. Returns `false`, having
/// changed nothing, unless `name` is a plain global variable holding exactly
/// `left`, a string, and `right` needs no `ToPrimitive`.
#[inline(never)]
pub(super) fn try_append_global_var(
    name: &str,
    left: &mut Value,
    right: &mut Value,
    env: &CallEnv,
) -> bool {
    // A numeric accumulator (`total += n`) takes the same exit; its sum is
    // stored like any other plain global assignment.
    if let (Value::Number(sum), Value::Number(addend)) = (&*left, &*right) {
        let value = Value::Number(sum + addend);
        if !try_store_global_var(name, &value, env) {
            return false;
        }
        *left = Value::Undefined;
        *right = Value::Undefined;
        return true;
    }
    let Value::String(current) = &*left else {
        return false;
    };
    if !crate::bytecode::vm_string_append::is_appendable(right) {
        return false;
    }
    let Some(GlobalVar::Bound(global_this, cell)) = plain_global_var(name, env) else {
        return false;
    };
    if !cell.with_value(
        |value| matches!(value, Value::String(bound) if crate::JsString::ptr_eq(bound, current)),
    ) {
        return false;
    }
    cell.set(Value::Undefined);
    if !matches!(
        global_this.write_existing_own_data_property(name, &Value::Undefined),
        OwnDataPropertyWrite::Written
    ) {
        cell.set(left.clone());
        return false;
    }
    let left = std::mem::replace(left, Value::Undefined);
    let right = std::mem::replace(right, Value::Undefined);
    let Ok(result) = crate::bytecode::vm_string_append::concat_primitives(left, right) else {
        unreachable!("a string and a plain primitive concatenate");
    };
    global_this.write_existing_own_data_property(name, &result);
    env.replace_existing_realm_with_cell(name, result, &cell)
}

/// A global variable a sloppy function may assign on this tier.
enum GlobalVar {
    /// Not bound yet, on an extensible global object: an assignment creates it.
    Absent(crate::ObjectRef),
    /// A writable global data property with the realm binding that mirrors
    /// it, the two in sync.
    Bound(crate::ObjectRef, crate::function::Upvalue),
}

/// The global variable `name` resolves to from a function that neither
/// declares nor receives it, when a store to it is the plain one; `None` for
/// a lexical or immutable binding, a module binding, an accessor or
/// read-only property, a global property without a realm binding, or a
/// mirror that disagrees with its binding -- which the interpreter handles.
fn plain_global_var(name: &str, env: &CallEnv) -> Option<GlobalVar> {
    if env.is_global_lexical_binding(name)
        || env.is_immutable_lexical_binding(name)
        || env.is_immutable_function_name(name)
        || env.has_module_import(name)
        || env.module_live_binding_cell(name).is_some()
    {
        return None;
    }
    let Some(Value::Object(global_this)) = env.global_this() else {
        return None;
    };
    let Some(cell) = env.realm_binding_cell(name) else {
        if global_this.own_property(name).is_some() || !global_this.is_extensible() {
            return None;
        }
        return Some(GlobalVar::Absent(global_this));
    };
    let property = global_this.own_property(name)?;
    let current = cell.get();
    // Identity is enough to prove the mirror in sync, and a string
    // accumulator compared by content would cost its whole length per store.
    let in_sync = match (&property.value, &current) {
        (Value::String(left), Value::String(right)) => crate::JsString::ptr_eq(left, right),
        (left, right) => left.same_value(right),
    };
    if property.is_accessor() || !property.writable || !in_sync {
        return None;
    }
    Some(GlobalVar::Bound(global_this, cell))
}

/// A class field initializer that only reads a named property of a binding
/// it captured -- `color = Material.defaultColor` -- answered without a call
/// frame: its body is exactly that read and a return, and the captured
/// binding is initialized. `None` leaves the thunk to the ordinary call.
pub(in crate::bytecode) fn field_initializer_member_read(
    thunk: &crate::Function,
    env: &CallEnv,
) -> Option<Result<Value, RuntimeError>> {
    let bytecode = thunk.bytecode.as_ref()?;
    let [
        crate::bytecode::ir::Op::GetPropNamed { key, cache },
        crate::bytecode::ir::Op::Return,
    ] = bytecode.code.as_slice()
    else {
        return None;
    };
    let slot = cache.local_slot()?;
    if thunk.upvalues.len() != bytecode.received_upvalue_slots().len() {
        return None;
    }
    let index = bytecode.readonly_received_upvalue_index(slot)?;
    let receiver = thunk.upvalues.get(index)?.get();
    if receiver.is_uninitialized_lexical_marker() {
        return None;
    }
    Some(get_prop_named(&receiver, key, cache, env))
}
