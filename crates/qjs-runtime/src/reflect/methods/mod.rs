mod descriptors;
mod integrity;
mod invocation;
mod keys;
mod property_access;
mod prototype;
mod set;

pub(crate) use descriptors::{
    native_reflect_define_property, native_reflect_delete_property,
    native_reflect_get_own_property_descriptor,
};
pub(crate) use integrity::{native_reflect_is_extensible, native_reflect_prevent_extensions};
pub(crate) use invocation::native_reflect_apply;
pub(crate) use keys::native_reflect_own_keys;
pub(crate) use property_access::{native_reflect_get, native_reflect_has};
pub(crate) use prototype::{native_reflect_get_prototype_of, native_reflect_set_prototype_of};
pub(crate) use set::native_reflect_set;
