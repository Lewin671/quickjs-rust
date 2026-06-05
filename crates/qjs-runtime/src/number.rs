mod formatting;
mod install;
mod parsing;
mod prototype;
mod statics;

pub(crate) use formatting::number_to_js_string;
pub(super) use install::{inherited_number_prototype_property, install_number, is_number_object};
pub(super) use parsing::{native_parse_float, native_parse_int};
pub(super) use prototype::{
    native_number, native_number_prototype_to_fixed, native_number_prototype_to_string,
    native_number_prototype_value_of,
};
pub(super) use statics::{
    native_number_is_finite, native_number_is_integer, native_number_is_nan,
    native_number_is_safe_integer,
};

pub(crate) const NUMBER_DATA_PROPERTY: &str = "\0NumberData";
