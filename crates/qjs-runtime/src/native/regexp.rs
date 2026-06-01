use crate::{Function, NativeFunction, Value, regexp};

use super::NativeCallResult;

pub(super) fn call_regexp_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    is_construct: bool,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::RegExp => regexp::native_regexp(function, this_value, is_construct)?,
        _ => return Ok(None),
    };
    Ok(Some(value))
}
