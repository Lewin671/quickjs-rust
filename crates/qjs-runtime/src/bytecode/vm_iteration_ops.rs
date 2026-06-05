use crate::{
    ArrayRef, RuntimeError, Value, call_function, is_truthy, property_value,
    string::{string_code_units, string_from_code_unit},
};

use super::vm::Vm;

impl Vm<'_> {
    pub(super) fn for_of_values(&mut self) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let values = match value {
            Value::Array(_) => value,
            Value::String(value) => string_values(&value),
            Value::Object(_) | Value::Function(_) => {
                if self.has_next_method(value.clone())? {
                    self.iterator_values(value)?
                } else if self.has_array_like_length(value.clone())? {
                    value
                } else {
                    return Err(not_iterable_error());
                }
            }
            Value::Null | Value::Undefined => return Err(not_iterable_error()),
            Value::Number(_) | Value::Boolean(_) => return Err(not_iterable_error()),
        };
        self.stack.push(values);
        Ok(())
    }

    fn has_array_like_length(&mut self, value: Value) -> Result<bool, RuntimeError> {
        let mut env = self.current_env();
        let length = property_value(value, "length", &mut env)?;
        self.apply_env(env);
        Ok(!matches!(length, Value::Undefined))
    }

    fn has_next_method(&mut self, value: Value) -> Result<bool, RuntimeError> {
        let mut env = self.current_env();
        let next = property_value(value, "next", &mut env)?;
        self.apply_env(env);
        Ok(matches!(next, Value::Function(_)))
    }

    fn iterator_values(&mut self, iterator: Value) -> Result<Value, RuntimeError> {
        let mut values = Vec::new();
        loop {
            let next = {
                let mut env = self.current_env();
                let next = property_value(iterator.clone(), "next", &mut env)?;
                self.apply_env(env);
                next
            };
            let mut env = self.current_env();
            let result = call_function(next, iterator.clone(), Vec::new(), &mut env, false)?;
            self.apply_env(env);
            let mut env = self.current_env();
            let done = property_value(result.clone(), "done", &mut env)?;
            let value = property_value(result, "value", &mut env)?;
            self.apply_env(env);
            if is_truthy(&done) {
                break;
            }
            values.push(value);
        }
        Ok(Value::Array(ArrayRef::new(values)))
    }
}

fn string_values(value: &str) -> Value {
    let units = string_code_units(value);
    let mut units = units.into_iter().peekable();
    let mut values = Vec::new();
    while let Some(first) = units.next() {
        let value = if (0xD800..=0xDBFF).contains(&first) {
            if let Some(second @ 0xDC00..=0xDFFF) = units.peek().copied() {
                units.next();
                String::from_utf16(&[first, second])
                    .unwrap_or_else(|_| string_from_code_unit(first))
            } else {
                string_from_code_unit(first)
            }
        } else {
            string_from_code_unit(first)
        };
        values.push(Value::String(value));
    }
    Value::Array(ArrayRef::new(values))
}

fn not_iterable_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: value is not iterable".to_owned(),
    }
}
