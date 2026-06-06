mod install;
mod methods;
mod target;

pub(crate) use install::install_reflect;
pub(crate) use methods::{
    native_reflect_apply, native_reflect_construct, native_reflect_define_property,
    native_reflect_delete_property, native_reflect_get, native_reflect_get_own_property_descriptor,
    native_reflect_get_prototype_of, native_reflect_has, native_reflect_is_extensible,
    native_reflect_own_keys, native_reflect_prevent_extensions, native_reflect_set,
    native_reflect_set_prototype_of, ordinary_set,
};
