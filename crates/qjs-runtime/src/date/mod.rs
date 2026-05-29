mod install;
mod iso;
mod value;

pub(crate) use install::install_date;
pub(crate) use value::{
    native_date, native_date_now, native_date_parse, native_date_prototype_get_time,
    native_date_prototype_to_iso_string, native_date_prototype_value_of, native_date_utc,
};

const DATE_VALUE_PROPERTY: &str = "\0DateValue";
const MS_PER_DAY: f64 = 86_400_000.0;
const MS_PER_HOUR: f64 = 3_600_000.0;
const MS_PER_MINUTE: f64 = 60_000.0;
const MS_PER_SECOND: f64 = 1_000.0;
