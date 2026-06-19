//! `Iterator.zip` construction and advancement.

use std::collections::HashMap;

use crate::{
    ArrayRef, CallEnv, NativeFunction, ObjectRef, Property, PropertyKey, RuntimeError, Value,
    call_function, property_value, property_value_key, symbol,
};

use super::protocol::{iterator_close, iterator_close_on_throw, iterator_step, iterator_value};

const HELPER_DONE: &str = "\0iterator_helper_done";
const HELPER_EXECUTING: &str = "\0iterator_helper_executing";
const HELPER_KIND: &str = "\0iterator_helper_kind";
const HELPER_STARTED: &str = "\0iterator_helper_started";

#[derive(Clone, Copy, PartialEq, Eq)]
enum ZipMode {
    Shortest,
    Longest,
    Strict,
}

impl ZipMode {
    fn from_tag(tag: &str) -> Option<Self> {
        Some(match tag {
            "shortest" => Self::Shortest,
            "longest" => Self::Longest,
            "strict" => Self::Strict,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ZipResultKind {
    Array,
    Keyed,
}

#[derive(Clone)]
struct IteratorRecord {
    iterator: Value,
    next: Value,
}

#[derive(Clone)]
enum ZipRecord {
    Iterator(Box<IteratorRecord>),
    Array { elements: ArrayRef, index: usize },
}

#[derive(Clone)]
struct ZipEntry {
    record: ZipRecord,
    open: bool,
}

pub(crate) struct ZipState {
    records: Vec<ZipEntry>,
    keys: Vec<PropertyKey>,
    padding: Vec<Value>,
    mode: ZipMode,
    result_kind: ZipResultKind,
}

/// `Iterator.zip(iterables, options)`.
pub(super) fn native_iterator_zip(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iterables = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !is_object_value(&iterables) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Iterator.zip iterables must be an object".to_owned(),
        });
    }
    let options = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let (mode, padding_option) = zip_options(options, env)?;

    let input_record = get_iterator(iterables, env)?;
    let mut records = Vec::new();
    loop {
        let next = match iterator_step(&input_record.iterator, &input_record.next, env) {
            Ok(next) => next,
            Err(error) => return Err(close_zip_records(records.iter(), error, env)),
        };
        let Some(result) = next else {
            break;
        };
        let item = match iterator_value(result, env) {
            Ok(item) => item,
            Err(error) => return Err(close_zip_records(records.iter(), error, env)),
        };
        let record = match get_iterator_flattenable_record(item, env) {
            Ok(record) => record,
            Err(error) => {
                let mut to_close = Vec::with_capacity(records.len() + 1);
                to_close.push(input_record.clone());
                to_close.extend(records.iter().filter_map(iterator_record_from_zip_record));
                return Err(close_iterators(to_close.iter(), error, env));
            }
        };
        records.push(record);
    }

    let padding = if mode == ZipMode::Longest {
        match zip_padding(padding_option, records.len(), env) {
            Ok(padding) => padding,
            Err(error) => return Err(close_zip_records(records.iter(), error, env)),
        }
    } else {
        Vec::new()
    };

    Ok(Value::Object(zip_helper(
        records,
        Vec::new(),
        padding,
        mode,
        ZipResultKind::Array,
        env,
    )))
}

/// `Iterator.zipKeyed(iterables, options)`.
pub(super) fn native_iterator_zip_keyed(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let iterables = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if !is_object_value(&iterables) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Iterator.zipKeyed iterables must be an object".to_owned(),
        });
    }
    let options = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    let (mode, padding_option) = zip_options(options, env)?;

    let all_keys = own_property_keys(iterables.clone(), env)?;
    let mut keys = Vec::new();
    let mut records = Vec::new();
    for key in all_keys {
        let descriptor = match own_property_descriptor_key_trap(iterables.clone(), &key, env) {
            Ok(descriptor) => descriptor,
            Err(error) => return Err(close_zip_records(records.iter(), error, env)),
        };
        if !descriptor.is_some_and(|descriptor| descriptor.enumerable) {
            continue;
        }
        let value = match property_value_key(iterables.clone(), &key, env) {
            Ok(value) => value,
            Err(error) => return Err(close_zip_records(records.iter(), error, env)),
        };
        if matches!(value, Value::Undefined) {
            continue;
        }
        let record = match get_iterator_flattenable_record(value, env) {
            Ok(record) => record,
            Err(error) => return Err(close_zip_records(records.iter(), error, env)),
        };
        keys.push(key);
        records.push(record);
    }

    let padding = if mode == ZipMode::Longest {
        match keyed_zip_padding(padding_option, &keys, env) {
            Ok(padding) => padding,
            Err(error) => return Err(close_zip_records(records.iter(), error, env)),
        }
    } else {
        Vec::new()
    };

    Ok(Value::Object(zip_helper(
        records,
        keys,
        padding,
        mode,
        ZipResultKind::Keyed,
        env,
    )))
}

fn zip_options(options: Value, env: &mut CallEnv) -> Result<(ZipMode, Value), RuntimeError> {
    if !matches!(options, Value::Undefined) && !is_object_value(&options) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Iterator.zip options must be undefined or an object".to_owned(),
        });
    }
    let options = if matches!(options, Value::Undefined) {
        None
    } else {
        Some(options)
    };
    let raw_mode = match &options {
        Some(options) => property_value(options.clone(), "mode", env)?,
        None => Value::Undefined,
    };
    let mode = match raw_mode {
        Value::Undefined => ZipMode::Shortest,
        Value::String(mode) => ZipMode::from_tag(&mode).ok_or_else(|| RuntimeError {
            thrown: None,
            message: "TypeError: invalid Iterator.zip mode".to_owned(),
        })?,
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: invalid Iterator.zip mode".to_owned(),
            });
        }
    };
    let padding = if mode == ZipMode::Longest {
        match &options {
            Some(options) => {
                let padding = property_value(options.clone(), "padding", env)?;
                if !matches!(padding, Value::Undefined) && !is_object_value(&padding) {
                    return Err(RuntimeError {
                        thrown: None,
                        message: "TypeError: Iterator.zip padding must be undefined or an object"
                            .to_owned(),
                    });
                }
                padding
            }
            None => Value::Undefined,
        }
    } else {
        Value::Undefined
    };
    Ok((mode, padding))
}

fn get_iterator(value: Value, env: &mut CallEnv) -> Result<IteratorRecord, RuntimeError> {
    let Some(iterator_symbol) = symbol::iterator_symbol(env) else {
        return Err(RuntimeError {
            thrown: None,
            message: "iterator symbol is unavailable".to_owned(),
        });
    };
    let method = property_value_key(value.clone(), &PropertyKey::Symbol(iterator_symbol), env)?;
    if matches!(method, Value::Undefined | Value::Null) || !is_callable_value(&method) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: value is not iterable".to_owned(),
        });
    }
    let iterator = call_function(method, value, Vec::new(), env, false)?;
    if !is_object_value(&iterator) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: iterator method must return an object".to_owned(),
        });
    }
    let next = property_value(iterator.clone(), "next", env)?;
    Ok(IteratorRecord { iterator, next })
}

fn get_iterator_flattenable_record(
    value: Value,
    env: &mut CallEnv,
) -> Result<ZipRecord, RuntimeError> {
    if !is_object_value(&value) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Iterator.zip item must be an object".to_owned(),
        });
    }
    let Some(iterator_symbol) = symbol::iterator_symbol(env) else {
        return Err(RuntimeError {
            thrown: None,
            message: "iterator symbol is unavailable".to_owned(),
        });
    };
    let method = property_value_key(value.clone(), &PropertyKey::Symbol(iterator_symbol), env)?;
    if matches!(method, Value::Function(ref function) if function.native_kind() == Some(NativeFunction::ArrayPrototypeValues))
        && let Value::Array(elements) = value
    {
        return Ok(ZipRecord::Array { elements, index: 0 });
    }
    let iterator = if matches!(method, Value::Undefined | Value::Null) {
        value
    } else {
        if !is_callable_value(&method) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: Iterator.zip item is not iterable".to_owned(),
            });
        }
        let iterator = call_function(method, value, Vec::new(), env, false)?;
        if !is_object_value(&iterator) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: iterator method must return an object".to_owned(),
            });
        }
        iterator
    };
    let next = property_value(iterator.clone(), "next", env)?;
    Ok(ZipRecord::Iterator(Box::new(IteratorRecord {
        iterator,
        next,
    })))
}

fn iterator_step_value(
    record: &IteratorRecord,
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    let Some(result) = iterator_step(&record.iterator, &record.next, env)? else {
        return Ok(None);
    };
    Ok(Some(iterator_value(result, env)?))
}

fn close_iterators<'a>(
    records: impl Iterator<Item = &'a IteratorRecord>,
    error: RuntimeError,
    env: &mut CallEnv,
) -> RuntimeError {
    let mut completion = error;
    let records = records.cloned().collect::<Vec<_>>();
    for record in records.iter().rev() {
        completion = iterator_close_on_throw(&record.iterator, completion, env);
    }
    completion
}

fn iterator_record_from_zip_record(record: &ZipRecord) -> Option<IteratorRecord> {
    match record {
        ZipRecord::Iterator(record) => Some(record.as_ref().clone()),
        ZipRecord::Array { .. } => None,
    }
}

fn close_zip_records<'a>(
    records: impl Iterator<Item = &'a ZipRecord>,
    error: RuntimeError,
    env: &mut CallEnv,
) -> RuntimeError {
    let mut completion = error;
    let records = records
        .filter_map(iterator_record_from_zip_record)
        .collect::<Vec<_>>();
    for record in records.iter().rev() {
        completion = iterator_close_on_throw(&record.iterator, completion, env);
    }
    completion
}

fn zip_padding(
    padding_option: Value,
    count: usize,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    if matches!(padding_option, Value::Undefined) {
        return Ok(vec![Value::Undefined; count]);
    }
    let record = get_iterator(padding_option, env)?;
    let mut padding = Vec::with_capacity(count);
    let mut using_iterator = true;
    for _ in 0..count {
        if using_iterator {
            match iterator_step_value(&record, env)? {
                Some(value) => padding.push(value),
                None => {
                    using_iterator = false;
                    padding.push(Value::Undefined);
                }
            }
        } else {
            padding.push(Value::Undefined);
        }
    }
    if using_iterator {
        iterator_close(&record.iterator, env)?;
    }
    Ok(padding)
}

fn keyed_zip_padding(
    padding_option: Value,
    keys: &[PropertyKey],
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    if matches!(padding_option, Value::Undefined) {
        return Ok(vec![Value::Undefined; keys.len()]);
    }
    keys.iter()
        .map(|key| property_value_key(padding_option.clone(), key, env))
        .collect()
}

fn zip_helper(
    records: Vec<ZipRecord>,
    keys: Vec<PropertyKey>,
    padding: Vec<Value>,
    mode: ZipMode,
    result_kind: ZipResultKind,
    env: &CallEnv,
) -> ObjectRef {
    let helper = ObjectRef::with_prototype(HashMap::new(), super::iterator_helper_prototype(env));
    helper.define_non_enumerable(
        HELPER_KIND.to_owned(),
        Value::String("zip".to_owned().into()),
    );
    helper.define_non_enumerable(HELPER_DONE.to_owned(), Value::Boolean(false));
    helper.define_non_enumerable(HELPER_EXECUTING.to_owned(), Value::Boolean(false));
    helper.define_non_enumerable(HELPER_STARTED.to_owned(), Value::Boolean(false));
    helper.set_iterator_zip_state(ZipState {
        records: records
            .into_iter()
            .map(|record| ZipEntry { record, open: true })
            .collect(),
        keys,
        padding,
        mode,
        result_kind,
    });
    helper
}

pub(super) fn close_open_zip_iterators(
    helper: &ObjectRef,
    except: Option<usize>,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let Some(result) = helper
        .with_iterator_zip_state_mut(|state| close_open_zip_iterators_state(state, except, env))
    else {
        return Err(invalid_zip_state());
    };
    result
}

fn close_open_zip_iterators_state(
    state: &mut ZipState,
    except: Option<usize>,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let mut completion = None;
    for (index, entry) in state.records.iter_mut().enumerate().rev() {
        if Some(index) == except || !entry.open {
            continue;
        }
        entry.open = false;
        if let ZipRecord::Iterator(record) = &entry.record {
            completion = match completion {
                Some(error) => Some(iterator_close_on_throw(&record.iterator, error, env)),
                None => iterator_close(&record.iterator, env).err(),
            };
        }
    }
    match completion {
        Some(error) => Err(error),
        None => Ok(()),
    }
}

fn close_open_zip_iterators_on_throw(
    state: &mut ZipState,
    except: Option<usize>,
    error: RuntimeError,
    env: &mut CallEnv,
) -> RuntimeError {
    let mut completion = error;
    for (index, entry) in state.records.iter_mut().enumerate().rev() {
        if Some(index) == except || !entry.open {
            continue;
        }
        entry.open = false;
        if let ZipRecord::Iterator(record) = &entry.record {
            completion = iterator_close_on_throw(&record.iterator, completion, env);
        }
    }
    completion
}

pub(super) fn advance_zip(
    helper: &ObjectRef,
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    let Some(result) = helper.with_iterator_zip_state_mut(|state| advance_zip_state(state, env))
    else {
        return Err(invalid_zip_state());
    };
    result
}

fn advance_zip_state(
    state: &mut ZipState,
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    let count = state.records.len();
    if count == 0 {
        return Ok(None);
    }
    let mode = state.mode;
    let mut values = Vec::with_capacity(count);
    let mut produced_value = false;

    for index in 0..count {
        if !state.records[index].open {
            debug_assert!(mode == ZipMode::Longest);
            values.push(zip_padding_value(state, index));
            continue;
        }
        match zip_record_step_value(&mut state.records[index].record, env) {
            Ok(Some(value)) => {
                produced_value = true;
                values.push(value);
            }
            Ok(None) => {
                state.records[index].open = false;
                match mode {
                    ZipMode::Shortest => {
                        close_open_zip_iterators_state(state, Some(index), env)?;
                        return Ok(None);
                    }
                    ZipMode::Longest => values.push(zip_padding_value(state, index)),
                    ZipMode::Strict => {
                        if index != 0 {
                            let error = RuntimeError {
                                thrown: None,
                                message: "TypeError: Iterator.zip strict mode length mismatch"
                                    .to_owned(),
                            };
                            return Err(close_open_zip_iterators_on_throw(
                                state,
                                Some(index),
                                error,
                                env,
                            ));
                        }
                        for next_index in 1..count {
                            if !state.records[next_index].open {
                                continue;
                            }
                            match zip_record_step_value(&mut state.records[next_index].record, env)
                            {
                                Ok(Some(_)) => {
                                    let error = RuntimeError {
                                        thrown: None,
                                        message:
                                            "TypeError: Iterator.zip strict mode length mismatch"
                                                .to_owned(),
                                    };
                                    return Err(close_open_zip_iterators_on_throw(
                                        state, None, error, env,
                                    ));
                                }
                                Ok(None) => state.records[next_index].open = false,
                                Err(error) => {
                                    state.records[next_index].open = false;
                                    return Err(close_open_zip_iterators_on_throw(
                                        state, None, error, env,
                                    ));
                                }
                            }
                        }
                        return Ok(None);
                    }
                }
            }
            Err(error) => {
                state.records[index].open = false;
                return Err(close_open_zip_iterators_on_throw(state, None, error, env));
            }
        }
    }

    if mode == ZipMode::Longest && !produced_value {
        return Ok(None);
    }
    Ok(Some(zip_result(state.result_kind, &state.keys, values)))
}

fn zip_record_step_value(
    record: &mut ZipRecord,
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    match record {
        ZipRecord::Iterator(record) => {
            let Some(result) = iterator_step(&record.iterator, &record.next, env)? else {
                return Ok(None);
            };
            Ok(Some(iterator_value(result, env)?))
        }
        ZipRecord::Array {
            elements,
            index: next_index,
        } => {
            let length = elements.len();
            if *next_index >= length {
                return Ok(None);
            }
            let index = *next_index;
            *next_index += 1;
            let value = elements.dense_index_value(index, env).map_or_else(
                || property_value(Value::Array(elements.clone()), &index.to_string(), env),
                Ok,
            )?;
            Ok(Some(value))
        }
    }
}

fn zip_padding_value(state: &ZipState, index: usize) -> Value {
    state
        .padding
        .get(index)
        .cloned()
        .unwrap_or(Value::Undefined)
}

fn zip_result(result_kind: ZipResultKind, keys: &[PropertyKey], values: Vec<Value>) -> Value {
    if result_kind == ZipResultKind::Array {
        return Value::Array(ArrayRef::new(values));
    }

    let result = ObjectRef::with_prototype(HashMap::new(), None);
    for (index, value) in values.into_iter().enumerate() {
        let Some(key) = keys.get(index).cloned() else {
            continue;
        };
        match key {
            PropertyKey::String(key) => result.define_property(key, Property::enumerable(value)),
            PropertyKey::Symbol(symbol) => {
                result.define_symbol_property(symbol, Property::enumerable(value));
            }
        }
    }
    Value::Object(result)
}

fn invalid_zip_state() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: invalid Iterator.zip helper state".to_owned(),
    }
}

fn own_property_keys(value: Value, env: &mut CallEnv) -> Result<Vec<PropertyKey>, RuntimeError> {
    if let Value::Proxy(proxy) = value.clone() {
        return crate::proxy::proxy_own_keys(proxy, env);
    }
    let names = match value.clone() {
        Value::Object(object) => object.own_property_names(),
        Value::Map(map) => map.object().own_property_names(),
        Value::Set(set) => set.object().own_property_names(),
        Value::Array(elements) => crate::array_own_property_names(&elements),
        Value::Function(function) => crate::function_own_property_names(&function),
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: Iterator.zipKeyed iterables must be an object".to_owned(),
            });
        }
    };
    let symbols = match value {
        Value::Object(object) => object.own_property_symbols(),
        Value::Map(map) => map.object().own_property_symbols(),
        Value::Set(set) => set.object().own_property_symbols(),
        Value::Array(elements) => elements.own_property_symbols(),
        Value::Function(function) => crate::function_own_property_symbols(&function),
        _ => Vec::new(),
    };
    Ok(names
        .into_iter()
        .map(PropertyKey::String)
        .chain(symbols.into_iter().map(PropertyKey::Symbol))
        .collect())
}

fn own_property_descriptor_key_trap(
    value: Value,
    key: &PropertyKey,
    env: &mut CallEnv,
) -> Result<Option<Property>, RuntimeError> {
    if let Value::Proxy(proxy) = value.clone() {
        crate::proxy::proxy_get_own_property_descriptor(proxy, key, env, |target, _env| {
            crate::object::own_property_descriptor_key(target, key)
        })
    } else {
        crate::object::own_property_descriptor_key(value, key)
    }
}

fn is_object_value(value: &Value) -> bool {
    match value {
        Value::Object(object) => !symbol::is_symbol_primitive(object),
        Value::Array(_) | Value::Function(_) | Value::Map(_) | Value::Set(_) | Value::Proxy(_) => {
            true
        }
        _ => false,
    }
}

fn is_callable_value(value: &Value) -> bool {
    match value {
        Value::Function(_) => true,
        Value::Proxy(proxy) => crate::proxy::proxy_is_callable(proxy),
        _ => false,
    }
}
