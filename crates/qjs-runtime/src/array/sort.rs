use std::{cmp::Ordering, collections::HashMap};

use crate::{Function, RuntimeError, Value, call_function, to_js_string, to_number};

pub(crate) fn native_array_prototype_sort(
    this_value: Value,
    argument_values: &[Value],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let Value::Array(array) = this_value.clone() else {
        return Err(RuntimeError {
            message: "Array.prototype.sort called on non-array".to_owned(),
        });
    };

    let comparator = match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => None,
        Value::Function(function) => Some(function),
        _ => {
            return Err(RuntimeError {
                message: "Array.prototype.sort comparator must be callable".to_owned(),
            });
        }
    };

    let mut defined = Vec::new();
    let mut undefined_count = 0;
    for value in array.to_vec() {
        if matches!(value, Value::Undefined) {
            undefined_count += 1;
        } else {
            defined.push(value);
        }
    }

    insertion_sort(&mut defined, comparator.as_ref(), env)?;
    defined.extend(std::iter::repeat_n(Value::Undefined, undefined_count));
    array.replace_with(defined);
    Ok(this_value)
}

fn insertion_sort(
    values: &mut [Value],
    comparator: Option<&Function>,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    for index in 1..values.len() {
        let mut candidate = index;
        while candidate > 0
            && compare_values(&values[candidate], &values[candidate - 1], comparator, env)?
                == Ordering::Less
        {
            values.swap(candidate, candidate - 1);
            candidate -= 1;
        }
    }
    Ok(())
}

fn compare_values(
    left: &Value,
    right: &Value,
    comparator: Option<&Function>,
    env: &mut HashMap<String, Value>,
) -> Result<Ordering, RuntimeError> {
    if let Some(function) = comparator {
        let result = call_function(
            Value::Function(function.clone()),
            Value::Undefined,
            vec![left.clone(), right.clone()],
            env,
            false,
        )?;
        let order = to_number(result)?;
        if order.is_nan() || order == 0.0 {
            Ok(Ordering::Equal)
        } else if order < 0.0 {
            Ok(Ordering::Less)
        } else {
            Ok(Ordering::Greater)
        }
    } else {
        Ok(to_js_string(left.clone())?.cmp(&to_js_string(right.clone())?))
    }
}
