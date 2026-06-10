//! Iterator-protocol and destructuring helper ops.

use std::collections::HashMap;

use crate::{
    ArrayRef, ObjectRef, PropertyKey, RuntimeError, Value, call_function, is_truthy, object,
    object_prototype, property_value, property_value_key, symbol,
};

use super::util::is_object_value;
use super::vm::Vm;

impl Vm<'_> {
    pub(super) fn get_iterator(&mut self) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let mut env = self.current_env();
        let result = iterator_for_value(value, &mut env);
        self.apply_env(env);
        if let Some(iterator) = self.handle_runtime_result(result)? {
            self.stack.push(iterator);
        }
        Ok(())
    }

    pub(super) fn iterator_step(&mut self, done_slot: usize) -> Result<(), RuntimeError> {
        let next = self.pop()?;
        let iterator = self.pop()?;
        // Pessimistically mark the iterator done: errors raised by the step
        // itself must not trigger a close on the abrupt path.
        self.store_local(done_slot, Value::Boolean(true))?;
        let mut env = self.current_env();
        let result = iterator_step_value(&iterator, &next, &mut env);
        self.apply_env(env);
        match self.handle_runtime_result(result)? {
            Some(Some(value)) => {
                self.store_local(done_slot, Value::Boolean(false))?;
                self.stack.push(value);
            }
            Some(None) => self.stack.push(Value::Undefined),
            None => {}
        }
        Ok(())
    }

    pub(super) fn iterator_rest(&mut self, done_slot: usize) -> Result<(), RuntimeError> {
        let next = self.pop()?;
        let iterator = self.pop()?;
        if matches!(self.load_local(done_slot)?, Value::Boolean(true)) {
            self.stack.push(Value::Array(ArrayRef::new(Vec::new())));
            return Ok(());
        }
        self.store_local(done_slot, Value::Boolean(true))?;
        let mut env = self.current_env();
        let result = iterator_rest_values(&iterator, &next, &mut env);
        self.apply_env(env);
        if let Some(values) = self.handle_runtime_result(result)? {
            self.stack.push(Value::Array(ArrayRef::new(values)));
        }
        Ok(())
    }

    pub(super) fn iterator_close(&mut self, swallow: bool) -> Result<(), RuntimeError> {
        let iterator = self.pop()?;
        let mut env = self.current_env();
        let result = close_iterator(&iterator, &mut env);
        self.apply_env(env);
        if swallow {
            return Ok(());
        }
        self.handle_runtime_result(result)?;
        Ok(())
    }

    pub(super) fn object_rest_excluding(
        &mut self,
        excluded: &[String],
    ) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let mut env = self.current_env();
        let result = object::enumerable_property_entries(value, &mut env);
        self.apply_env(env);
        let Some(entries) = self.handle_runtime_result(result)? else {
            return Ok(());
        };
        let rest = ObjectRef::with_prototype(HashMap::new(), object_prototype(&self.globals));
        for (key, value) in entries {
            if !excluded.iter().any(|name| name == &key) {
                rest.set(key, value);
            }
        }
        self.stack.push(Value::Object(rest));
        Ok(())
    }

    pub(super) fn require_object_coercible(&mut self) -> Result<(), RuntimeError> {
        if matches!(self.stack.last(), Some(Value::Undefined | Value::Null)) {
            let result: Result<(), RuntimeError> = Err(RuntimeError {
                thrown: None,
                message: "TypeError: cannot destructure undefined or null".to_owned(),
            });
            self.handle_runtime_result(result)?;
        }
        Ok(())
    }
}

fn iterator_for_value(
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    if matches!(value, Value::Undefined | Value::Null) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: value is not iterable".to_owned(),
        });
    }
    let Some(iterator_symbol) = symbol::iterator_symbol(env) else {
        return Err(RuntimeError {
            thrown: None,
            message: "iterator symbol is unavailable".to_owned(),
        });
    };
    let method = property_value_key(value.clone(), &PropertyKey::Symbol(iterator_symbol), env)?;
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

fn iterator_step_value(
    iterator: &Value,
    next: &Value,
    env: &mut HashMap<String, Value>,
) -> Result<Option<Value>, RuntimeError> {
    let result = call_function(next.clone(), iterator.clone(), Vec::new(), env, false)?;
    if !is_object_value(&result) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: iterator result is not an object".to_owned(),
        });
    }
    if is_truthy(&property_value(result.clone(), "done", env)?) {
        return Ok(None);
    }
    Ok(Some(property_value(result, "value", env)?))
}

fn iterator_rest_values(
    iterator: &Value,
    next: &Value,
    env: &mut HashMap<String, Value>,
) -> Result<Vec<Value>, RuntimeError> {
    let mut values = Vec::new();
    while let Some(value) = iterator_step_value(iterator, next, env)? {
        values.push(value);
    }
    Ok(values)
}

fn close_iterator(iterator: &Value, env: &mut HashMap<String, Value>) -> Result<(), RuntimeError> {
    let return_method = property_value(iterator.clone(), "return", env)?;
    if matches!(return_method, Value::Null | Value::Undefined) {
        return Ok(());
    }
    let result = call_function(return_method, iterator.clone(), Vec::new(), env, false)?;
    if is_object_value(&result) {
        return Ok(());
    }
    Err(RuntimeError {
        thrown: None,
        message: "TypeError: iterator return result must be an object".to_owned(),
    })
}
