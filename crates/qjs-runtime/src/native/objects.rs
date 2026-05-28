use std::collections::HashMap;

use crate::{Function, NativeFunction, Value, object};

use super::NativeCallResult;

pub(super) fn call_object_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut HashMap<String, Value>,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::Object => {
            object::native_object(function, this_value, argument_values, is_construct)?
        }
        NativeFunction::ObjectAssign => object::native_object_assign(argument_values)?,
        NativeFunction::ObjectCreate => object::native_object_create(argument_values)?,
        NativeFunction::ObjectDefineProperties => {
            object::native_object_define_properties(argument_values)?
        }
        NativeFunction::ObjectDefineProperty => {
            object::native_object_define_property(argument_values)?
        }
        NativeFunction::ObjectGetOwnPropertyDescriptor => {
            object::native_object_get_own_property_descriptor(argument_values, env)?
        }
        NativeFunction::ObjectGetPrototypeOf => {
            object::native_object_get_prototype_of(argument_values, env)?
        }
        NativeFunction::ObjectGetOwnPropertyNames => {
            object::native_object_get_own_property_names(argument_values)?
        }
        NativeFunction::ObjectHasOwn => object::native_object_has_own(argument_values)?,
        NativeFunction::ObjectIs => object::native_object_is(argument_values)?,
        NativeFunction::ObjectEntries => object::native_object_entries(argument_values)?,
        NativeFunction::ObjectKeys => object::native_object_keys(argument_values)?,
        NativeFunction::ObjectValues => object::native_object_values(argument_values)?,
        NativeFunction::ObjectPrototypeHasOwnProperty => {
            object::native_object_prototype_has_own_property(this_value, argument_values)?
        }
        NativeFunction::ObjectPrototypeIsPrototypeOf => {
            object::native_object_prototype_is_prototype_of(this_value, argument_values, env)?
        }
        NativeFunction::ObjectPrototypePropertyIsEnumerable => {
            object::native_object_prototype_property_is_enumerable(this_value, argument_values)?
        }
        NativeFunction::ObjectPrototypeToString => {
            object::native_object_prototype_to_string(this_value)?
        }
        NativeFunction::ObjectPrototypeValueOf => {
            object::native_object_prototype_value_of(this_value)?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
