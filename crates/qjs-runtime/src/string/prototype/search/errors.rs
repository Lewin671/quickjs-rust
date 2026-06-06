use crate::RuntimeError;

pub(super) fn replace_all_regexp_flags_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: String.prototype.replaceAll RegExp flags are null or undefined"
            .to_owned(),
    }
}

pub(super) fn string_method_null_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "String.prototype method called on null or undefined".to_owned(),
    }
}
