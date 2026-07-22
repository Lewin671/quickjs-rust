//! `Iterator.from(O)` (27.1.3.1) and the `%WrapForValidIteratorPrototype%`
//! wrapper it returns for foreign iterators.
//!
//! `Iterator.from` accepts a string or an object. When the obtained iterator
//! already inherits `%Iterator.prototype%` it is returned directly; otherwise
//! it is wrapped so the helper methods become available. The wrapper's
//! `next`/`return` forward to the underlying iterator record.

use std::collections::HashMap;

use crate::CallEnv;
use crate::{
    NativeFunction, ObjectRef, Property, PropertyKey, RuntimeError, Value, call_function,
    object_prototype, property_value, property_value_key, symbol,
};

const WRAP_ITERATOR: &str = "\0wrap_for_valid_iterator";
const WRAP_NEXT: &str = "\0wrap_for_valid_iterator_next";

/// `Iterator.from(O)`.
pub(super) fn native_iterator_from(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let object = argument_values.first().cloned().unwrap_or(Value::Undefined);

    // GetIteratorFlattenable(O, iterate-string-primitives): a string is iterated
    // through its own `Symbol.iterator`; a primitive other than a string is
    // rejected; an object is resolved through `Symbol.iterator` or, when the
    // method is null/undefined, treated as the iterator object itself.
    let iterator = iterator_flattenable(object, env)?;

    // If the iterator already inherits %Iterator.prototype%, return it as-is.
    if iterator_inherits_iterator_prototype(&iterator, env)? {
        return Ok(iterator);
    }

    // Otherwise wrap it so the helpers apply.
    let next = property_value(iterator.clone(), "next", env)?;
    let wrapper = ObjectRef::with_prototype(HashMap::new(), wrap_prototype(env));
    wrapper.define_non_enumerable(WRAP_ITERATOR.to_owned(), iterator);
    wrapper.define_non_enumerable(WRAP_NEXT.to_owned(), next);
    Ok(Value::Object(wrapper))
}

/// GetIteratorFlattenable for `Iterator.from`: a string primitive is iterated;
/// any other primitive is a TypeError; objects go through GetIterator(sync).
fn iterator_flattenable(value: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    match &value {
        Value::String(_) => crate::bytecode::sync_iterator_for_value(value, env),
        Value::Object(object) if symbol::is_symbol_primitive(object) => Err(RuntimeError {
            thrown: None,
            message: "TypeError: Iterator.from called on a non-iterable primitive".to_owned(),
        }),
        Value::Object(_)
        | Value::Array(_)
        | Value::Function(_)
        | Value::Map(_)
        | Value::Set(_)
        | Value::Proxy(_) => {
            let Some(iterator_symbol) = symbol::iterator_symbol(env) else {
                return Err(RuntimeError {
                    thrown: None,
                    message: "iterator symbol is unavailable".to_owned(),
                });
            };
            let method =
                property_value_key(value.clone(), &PropertyKey::Symbol(iterator_symbol), env)?;
            if matches!(method, Value::Undefined | Value::Null) {
                return Ok(value);
            }
            if !matches!(method, Value::Function(_)) {
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
            Ok(iterator)
        }
        _ => Err(RuntimeError {
            thrown: None,
            message: "TypeError: Iterator.from called on a non-iterable primitive".to_owned(),
        }),
    }
}

fn is_object_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_)
            | Value::Array(_)
            | Value::Function(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Proxy(_)
    )
}

/// `OrdinaryHasInstance(%Iterator%, iterator)`: walks the iterator's prototype
/// chain looking for `%Iterator.prototype%`.
fn iterator_inherits_iterator_prototype(
    iterator: &Value,
    env: &mut CallEnv,
) -> Result<bool, RuntimeError> {
    let Some(target) = super::iterator_prototype(env) else {
        return Ok(false);
    };
    let mut current =
        crate::object::native_object_get_prototype_of(std::slice::from_ref(iterator), env)?;
    loop {
        if matches!(&current, Value::Object(object) if object.ptr_eq(&target)) {
            return Ok(true);
        }
        if matches!(&current, Value::Null) {
            return Ok(false);
        }
        current =
            crate::object::native_object_get_prototype_of(std::slice::from_ref(&current), env)?;
    }
}

/// `%WrapForValidIteratorPrototype%`, installed eagerly under
/// [`super::WRAP_PROTOTYPE_BINDING`]. It inherits `%Iterator.prototype%` and
/// forwards `next`/`return` to the wrapped iterator record.
pub(super) fn build_wrap_prototype(env: &CallEnv, iterator_prototype: &ObjectRef) -> ObjectRef {
    let _ = env;
    let prototype = ObjectRef::with_prototype(HashMap::new(), Some(iterator_prototype.clone()));
    prototype.define_non_enumerable(
        "next".to_owned(),
        Value::Function(crate::Function::new_native(
            Some("next"),
            0,
            NativeFunction::WrapForValidIteratorPrototypeNext,
            false,
        )),
    );
    prototype.define_non_enumerable(
        "return".to_owned(),
        Value::Function(crate::Function::new_native(
            Some("return"),
            0,
            NativeFunction::WrapForValidIteratorPrototypeReturn,
            false,
        )),
    );
    prototype
}

fn wrap_prototype(env: &CallEnv) -> Option<ObjectRef> {
    match env.get(super::WRAP_PROTOTYPE_BINDING) {
        Some(Value::Object(prototype)) => Some(prototype.clone()),
        _ => None,
    }
}

/// `%WrapForValidIteratorPrototype%.next`: forwards to the wrapped iterator's
/// `next` method with no arguments.
pub(super) fn native_wrap_next(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let Value::Object(wrapper) = &this_value else {
        return Err(not_a_wrapper());
    };
    let Some(iterator) = wrapper.own_property(WRAP_ITERATOR).map(|p| p.value) else {
        return Err(not_a_wrapper());
    };
    let Some(next) = wrapper.own_property(WRAP_NEXT).map(|p| p.value) else {
        return Err(not_a_wrapper());
    };
    call_function(next, iterator, Vec::new(), env, false)
}

/// `%WrapForValidIteratorPrototype%.return`: calls the wrapped iterator's
/// `return` method if present, otherwise returns a done result object.
pub(super) fn native_wrap_return(
    this_value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let Value::Object(wrapper) = &this_value else {
        return Err(not_a_wrapper());
    };
    let Some(iterator) = wrapper.own_property(WRAP_ITERATOR).map(|p| p.value) else {
        return Err(not_a_wrapper());
    };
    let return_method = property_value(iterator.clone(), "return", env)?;
    if matches!(return_method, Value::Null | Value::Undefined) {
        return Ok(done_result(env));
    }
    if !matches!(return_method, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: iterator return is not a function".to_owned(),
        });
    }
    call_function(return_method, iterator, Vec::new(), env, false)
}

fn done_result(env: &CallEnv) -> Value {
    let object = ObjectRef::with_prototype(HashMap::new(), object_prototype(env));
    object.define_property("value".to_owned(), Property::enumerable(Value::Undefined));
    object.define_property(
        "done".to_owned(),
        Property::enumerable(Value::Boolean(true)),
    );
    Value::Object(object)
}

fn not_a_wrapper() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: method called on a non-wrapper object".to_owned(),
    }
}
