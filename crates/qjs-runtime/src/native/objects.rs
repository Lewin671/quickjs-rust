use crate::{Function, NativeFunction, Value, object};

use super::NativeCallResult;
use crate::CallEnv;

pub(super) fn call_object_native(
    function: &Function,
    native: NativeFunction,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> NativeCallResult {
    let value = match native {
        NativeFunction::Object => {
            object::native_object(function, this_value, argument_values, is_construct, env)?
        }
        NativeFunction::ObjectAssign => object::native_object_assign(argument_values, env)?,
        NativeFunction::ObjectCreate => object::native_object_create(argument_values, env)?,
        NativeFunction::ObjectDefineProperties => {
            object::native_object_define_properties(argument_values, env)?
        }
        NativeFunction::ObjectDefineProperty => {
            object::native_object_define_property(argument_values, env)?
        }
        NativeFunction::ObjectGetOwnPropertyDescriptor => {
            object::native_object_get_own_property_descriptor(argument_values, env)?
        }
        NativeFunction::ObjectGetOwnPropertyDescriptors => {
            object::native_object_get_own_property_descriptors(argument_values, env)?
        }
        NativeFunction::ObjectGetPrototypeOf => {
            object::native_object_get_prototype_of(argument_values, env)?
        }
        NativeFunction::ObjectGetOwnPropertyNames => {
            object::native_object_get_own_property_names(argument_values)?
        }
        NativeFunction::ObjectGetOwnPropertySymbols => {
            object::native_object_get_own_property_symbols(argument_values)?
        }
        NativeFunction::ObjectFromEntries => {
            object::native_object_from_entries(argument_values, env)?
        }
        NativeFunction::ObjectGroupBy => object::native_object_group_by(argument_values, env)?,
        NativeFunction::ObjectFreeze => object::native_object_freeze(argument_values)?,
        NativeFunction::ObjectHasOwn => object::native_object_has_own(argument_values, env)?,
        NativeFunction::ObjectIs => object::native_object_is(argument_values)?,
        NativeFunction::ObjectIsExtensible => object::native_object_is_extensible(argument_values)?,
        NativeFunction::ObjectIsFrozen => object::native_object_is_frozen(argument_values)?,
        NativeFunction::ObjectIsSealed => object::native_object_is_sealed(argument_values)?,
        NativeFunction::ObjectPreventExtensions => {
            object::native_object_prevent_extensions(argument_values)?
        }
        NativeFunction::ObjectSeal => object::native_object_seal(argument_values)?,
        NativeFunction::ObjectSetPrototypeOf => {
            object::native_object_set_prototype_of(argument_values, env)?
        }
        NativeFunction::ObjectEntries => object::native_object_entries(argument_values, env)?,
        NativeFunction::ObjectKeys => object::native_object_keys(argument_values)?,
        NativeFunction::ObjectValues => object::native_object_values(argument_values, env)?,
        NativeFunction::ObjectPrototypeHasOwnProperty => {
            object::native_object_prototype_has_own_property(this_value, argument_values, env)?
        }
        NativeFunction::ObjectPrototypeIsPrototypeOf => {
            object::native_object_prototype_is_prototype_of(this_value, argument_values, env)?
        }
        NativeFunction::ObjectPrototypePropertyIsEnumerable => {
            object::native_object_prototype_property_is_enumerable(
                this_value,
                argument_values,
                env,
            )?
        }
        NativeFunction::ObjectPrototypeToString => {
            object::native_object_prototype_to_string(this_value, env)?
        }
        NativeFunction::ObjectPrototypeToLocaleString => {
            object::native_object_prototype_to_locale_string(this_value, env)?
        }
        NativeFunction::ObjectPrototypeValueOf => {
            object::native_object_prototype_value_of(this_value, env)?
        }
        NativeFunction::ObjectPrototypeGetProto => {
            object::native_object_prototype_get_proto(this_value, env)?
        }
        NativeFunction::ObjectPrototypeSetProto => {
            object::native_object_prototype_set_proto(this_value, argument_values, env)?
        }
        _ => return Ok(None),
    };

    Ok(Some(value))
}
