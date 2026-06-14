//! `Iterator.zip` construction and advancement.

use std::collections::HashMap;

use crate::{
    ArrayRef, CallEnv, ObjectRef, PropertyKey, RuntimeError, Value, call_function, property_value,
    property_value_key, symbol,
};

use super::protocol::{iterator_close, iterator_close_on_throw, iterator_step, iterator_value};

const HELPER_DONE: &str = "\0iterator_helper_done";
const HELPER_EXECUTING: &str = "\0iterator_helper_executing";
const HELPER_KIND: &str = "\0iterator_helper_kind";
const HELPER_STARTED: &str = "\0iterator_helper_started";
const ZIP_COUNT: &str = "\0iterator_zip_count";
const ZIP_MODE: &str = "\0iterator_zip_mode";
const ZIP_ITERATOR_PREFIX: &str = "\0iterator_zip_iterator_";
const ZIP_NEXT_PREFIX: &str = "\0iterator_zip_next_";
const ZIP_OPEN_PREFIX: &str = "\0iterator_zip_open_";
const ZIP_PADDING_PREFIX: &str = "\0iterator_zip_padding_";

#[derive(Clone, Copy, PartialEq, Eq)]
enum ZipMode {
    Shortest,
    Longest,
    Strict,
}

impl ZipMode {
    fn tag(self) -> &'static str {
        match self {
            Self::Shortest => "shortest",
            Self::Longest => "longest",
            Self::Strict => "strict",
        }
    }

    fn from_tag(tag: &str) -> Option<Self> {
        Some(match tag {
            "shortest" => Self::Shortest,
            "longest" => Self::Longest,
            "strict" => Self::Strict,
            _ => return None,
        })
    }
}

#[derive(Clone)]
struct IteratorRecord {
    iterator: Value,
    next: Value,
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
            Err(error) => return Err(close_iterators(records.iter(), error, env)),
        };
        let Some(result) = next else {
            break;
        };
        let item = match iterator_value(result, env) {
            Ok(item) => item,
            Err(error) => return Err(close_iterators(records.iter(), error, env)),
        };
        let record = match get_iterator_flattenable_record(item, env) {
            Ok(record) => record,
            Err(error) => {
                let mut to_close = Vec::with_capacity(records.len() + 1);
                to_close.push(input_record.clone());
                to_close.extend(records.iter().cloned());
                return Err(close_iterators(to_close.iter(), error, env));
            }
        };
        records.push(record);
    }

    let padding = if mode == ZipMode::Longest {
        match zip_padding(padding_option, records.len(), env) {
            Ok(padding) => padding,
            Err(error) => return Err(close_iterators(records.iter(), error, env)),
        }
    } else {
        Vec::new()
    };

    let helper = ObjectRef::with_prototype(HashMap::new(), super::iterator_helper_prototype(env));
    helper.define_non_enumerable(HELPER_KIND.to_owned(), Value::String("zip".to_owned()));
    helper.define_non_enumerable(HELPER_DONE.to_owned(), Value::Boolean(false));
    helper.define_non_enumerable(HELPER_EXECUTING.to_owned(), Value::Boolean(false));
    helper.define_non_enumerable(HELPER_STARTED.to_owned(), Value::Boolean(false));
    helper.define_non_enumerable(ZIP_COUNT.to_owned(), Value::Number(records.len() as f64));
    helper.define_non_enumerable(ZIP_MODE.to_owned(), Value::String(mode.tag().to_owned()));
    for (index, record) in records.into_iter().enumerate() {
        helper.define_non_enumerable(format!("{ZIP_ITERATOR_PREFIX}{index}"), record.iterator);
        helper.define_non_enumerable(format!("{ZIP_NEXT_PREFIX}{index}"), record.next);
        helper.define_non_enumerable(format!("{ZIP_OPEN_PREFIX}{index}"), Value::Boolean(true));
    }
    for (index, value) in padding.into_iter().enumerate() {
        helper.define_non_enumerable(format!("{ZIP_PADDING_PREFIX}{index}"), value);
    }
    Ok(Value::Object(helper))
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
) -> Result<IteratorRecord, RuntimeError> {
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
    Ok(IteratorRecord { iterator, next })
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

fn helper_slot(helper: &ObjectRef, key: &str) -> Option<Value> {
    helper.own_property(key).map(|property| property.value)
}

fn number_slot(helper: &ObjectRef, key: &str) -> usize {
    match helper_slot(helper, key) {
        Some(Value::Number(n)) if n.is_finite() && n >= 0.0 => n as usize,
        _ => 0,
    }
}

fn zip_mode(helper: &ObjectRef) -> ZipMode {
    match helper_slot(helper, ZIP_MODE) {
        Some(Value::String(tag)) => ZipMode::from_tag(&tag).unwrap_or(ZipMode::Shortest),
        _ => ZipMode::Shortest,
    }
}

fn zip_record(helper: &ObjectRef, index: usize) -> Option<IteratorRecord> {
    let iterator = helper_slot(helper, &format!("{ZIP_ITERATOR_PREFIX}{index}"))?;
    let next = helper_slot(helper, &format!("{ZIP_NEXT_PREFIX}{index}"))?;
    Some(IteratorRecord { iterator, next })
}

fn zip_is_open(helper: &ObjectRef, index: usize) -> bool {
    matches!(
        helper_slot(helper, &format!("{ZIP_OPEN_PREFIX}{index}")),
        Some(Value::Boolean(true))
    )
}

fn zip_set_open(helper: &ObjectRef, index: usize, open: bool) {
    helper.define_non_enumerable(format!("{ZIP_OPEN_PREFIX}{index}"), Value::Boolean(open));
}

fn zip_padding_value(helper: &ObjectRef, index: usize) -> Value {
    helper_slot(helper, &format!("{ZIP_PADDING_PREFIX}{index}")).unwrap_or(Value::Undefined)
}

pub(super) fn close_open_zip_iterators(
    helper: &ObjectRef,
    except: Option<usize>,
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let count = number_slot(helper, ZIP_COUNT);
    let mut completion = None;
    for index in (0..count).rev() {
        if Some(index) == except || !zip_is_open(helper, index) {
            continue;
        }
        zip_set_open(helper, index, false);
        if let Some(record) = zip_record(helper, index) {
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
    helper: &ObjectRef,
    except: Option<usize>,
    error: RuntimeError,
    env: &mut CallEnv,
) -> RuntimeError {
    let count = number_slot(helper, ZIP_COUNT);
    let mut completion = error;
    for index in (0..count).rev() {
        if Some(index) == except || !zip_is_open(helper, index) {
            continue;
        }
        zip_set_open(helper, index, false);
        if let Some(record) = zip_record(helper, index) {
            completion = iterator_close_on_throw(&record.iterator, completion, env);
        }
    }
    completion
}

pub(super) fn advance_zip(
    helper: &ObjectRef,
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    let count = number_slot(helper, ZIP_COUNT);
    if count == 0 {
        return Ok(None);
    }
    let mode = zip_mode(helper);
    let mut values = Vec::with_capacity(count);
    let mut produced_value = false;

    for index in 0..count {
        if !zip_is_open(helper, index) {
            debug_assert!(mode == ZipMode::Longest);
            values.push(zip_padding_value(helper, index));
            continue;
        }
        let Some(record) = zip_record(helper, index) else {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: invalid Iterator.zip helper state".to_owned(),
            });
        };
        match iterator_step(&record.iterator, &record.next, env) {
            Ok(Some(result)) => match iterator_value(result, env) {
                Ok(value) => {
                    produced_value = true;
                    values.push(value);
                }
                Err(error) => {
                    zip_set_open(helper, index, false);
                    return Err(close_open_zip_iterators_on_throw(helper, None, error, env));
                }
            },
            Ok(None) => {
                zip_set_open(helper, index, false);
                match mode {
                    ZipMode::Shortest => {
                        close_open_zip_iterators(helper, Some(index), env)?;
                        return Ok(None);
                    }
                    ZipMode::Longest => values.push(zip_padding_value(helper, index)),
                    ZipMode::Strict => {
                        if index != 0 {
                            let error = RuntimeError {
                                thrown: None,
                                message: "TypeError: Iterator.zip strict mode length mismatch"
                                    .to_owned(),
                            };
                            return Err(close_open_zip_iterators_on_throw(
                                helper,
                                Some(index),
                                error,
                                env,
                            ));
                        }
                        for next_index in 1..count {
                            if !zip_is_open(helper, next_index) {
                                continue;
                            }
                            let Some(next_record) = zip_record(helper, next_index) else {
                                return Err(RuntimeError {
                                    thrown: None,
                                    message: "TypeError: invalid Iterator.zip helper state"
                                        .to_owned(),
                                });
                            };
                            match iterator_step(&next_record.iterator, &next_record.next, env) {
                                Ok(Some(_)) => {
                                    let error = RuntimeError {
                                        thrown: None,
                                        message:
                                            "TypeError: Iterator.zip strict mode length mismatch"
                                                .to_owned(),
                                    };
                                    return Err(close_open_zip_iterators_on_throw(
                                        helper, None, error, env,
                                    ));
                                }
                                Ok(None) => zip_set_open(helper, next_index, false),
                                Err(error) => {
                                    zip_set_open(helper, next_index, false);
                                    return Err(close_open_zip_iterators_on_throw(
                                        helper, None, error, env,
                                    ));
                                }
                            }
                        }
                        return Ok(None);
                    }
                }
            }
            Err(error) => {
                zip_set_open(helper, index, false);
                return Err(close_open_zip_iterators_on_throw(helper, None, error, env));
            }
        }
    }

    if mode == ZipMode::Longest && !produced_value {
        return Ok(None);
    }
    Ok(Some(Value::Array(ArrayRef::new(values))))
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
