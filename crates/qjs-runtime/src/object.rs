mod constructor;
mod descriptor;
mod descriptor_record;
mod enumeration;
mod from_entries;
mod group_by;
mod install;
mod integrity;
mod prototype;

pub(crate) use constructor::{
    boxed_primitive, native_object, native_object_assign, native_object_create, native_object_is,
};
pub(crate) use descriptor::{
    define_property_descriptor_on_value_key, define_property_on_value_key,
    native_object_define_properties, native_object_define_property,
    native_object_get_own_property_descriptor, native_object_get_own_property_descriptors,
};
pub(crate) use descriptor_record::to_property_descriptor_record;
pub(crate) use enumeration::{
    native_object_entries, native_object_get_own_property_names,
    native_object_get_own_property_symbols, native_object_has_own, native_object_keys,
    native_object_values,
};
pub(crate) use from_entries::native_object_from_entries;
pub(crate) use group_by::native_object_group_by;
pub(crate) use install::install_object;
pub(crate) use integrity::{
    native_object_freeze, native_object_is_extensible, native_object_is_frozen,
    native_object_is_sealed, native_object_prevent_extensions, native_object_seal,
};
pub(crate) use prototype::{
    native_object_get_prototype_of, native_object_prototype_has_own_property,
    native_object_prototype_is_prototype_of, native_object_prototype_property_is_enumerable,
    native_object_prototype_to_locale_string, native_object_prototype_to_string,
    native_object_prototype_value_of, native_object_set_prototype_of,
};
