mod constructor;
mod descriptor;
mod enumeration;
mod install;
mod integrity;
mod prototype;

pub(crate) use constructor::{
    native_object, native_object_assign, native_object_create, native_object_is,
};
pub(crate) use descriptor::{
    native_object_define_properties, native_object_define_property,
    native_object_get_own_property_descriptor, native_object_get_own_property_descriptors,
};
pub(crate) use enumeration::{
    native_object_entries, native_object_get_own_property_names, native_object_has_own,
    native_object_keys, native_object_values,
};
pub(crate) use install::install_object;
pub(crate) use integrity::{
    native_object_freeze, native_object_is_extensible, native_object_is_frozen,
    native_object_is_sealed, native_object_prevent_extensions, native_object_seal,
};
pub(crate) use prototype::{
    native_object_get_prototype_of, native_object_prototype_has_own_property,
    native_object_prototype_is_prototype_of, native_object_prototype_property_is_enumerable,
    native_object_prototype_to_string, native_object_prototype_value_of,
};
