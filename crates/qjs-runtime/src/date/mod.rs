mod accessors;
mod constructor;
mod format;
mod install;
mod iso;
mod mutation;
mod value;

pub(crate) use accessors::{
    native_date_prototype_get_time, native_date_prototype_get_timezone_offset,
    native_date_prototype_get_utc_date, native_date_prototype_get_utc_day,
    native_date_prototype_get_utc_full_year, native_date_prototype_get_utc_hours,
    native_date_prototype_get_utc_milliseconds, native_date_prototype_get_utc_minutes,
    native_date_prototype_get_utc_month, native_date_prototype_get_utc_seconds,
    native_date_prototype_value_of,
};
pub(crate) use constructor::{native_date, native_date_now, native_date_parse, native_date_utc};
pub(crate) use format::{
    native_date_prototype_to_date_string, native_date_prototype_to_iso_string,
    native_date_prototype_to_json, native_date_prototype_to_string,
    native_date_prototype_to_time_string, native_date_prototype_to_utc_string,
};
pub(crate) use install::install_date;
pub(crate) use mutation::{
    native_date_prototype_set_time, native_date_prototype_set_utc_date,
    native_date_prototype_set_utc_full_year, native_date_prototype_set_utc_hours,
    native_date_prototype_set_utc_milliseconds, native_date_prototype_set_utc_minutes,
    native_date_prototype_set_utc_month, native_date_prototype_set_utc_seconds,
};

const DATE_VALUE_PROPERTY: &str = "\0DateValue";
const MS_PER_DAY: f64 = 86_400_000.0;
const MS_PER_HOUR: f64 = 3_600_000.0;
const MS_PER_MINUTE: f64 = 60_000.0;
const MS_PER_SECOND: f64 = 1_000.0;
