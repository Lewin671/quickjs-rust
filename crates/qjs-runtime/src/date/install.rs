use crate::CallEnv;
use std::collections::HashMap;

use crate::{Function, NativeFunction, ObjectRef, Property, Value, symbol};

pub(crate) fn install_date(env: &mut CallEnv, global_this: &Value, object_prototype: ObjectRef) {
    let date_prototype =
        ObjectRef::with_prototype(env.realm().atoms(), HashMap::new(), Some(object_prototype));

    let date_function = Function::new_native(
        env.realm().atoms(),
        Some("Date"),
        7,
        NativeFunction::Date,
        true,
    );
    date_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(date_function.clone()),
    );
    date_prototype.define_native_methods(&[
        ("getTime", 0, NativeFunction::DatePrototypeGetTime),
        (
            "getTimezoneOffset",
            0,
            NativeFunction::DatePrototypeGetTimezoneOffset,
        ),
        ("getYear", 0, NativeFunction::DatePrototypeGetYear),
        (
            "getFullYear",
            0,
            NativeFunction::DatePrototypeGetUtcFullYear,
        ),
        ("getMonth", 0, NativeFunction::DatePrototypeGetUtcMonth),
        ("getDate", 0, NativeFunction::DatePrototypeGetUtcDate),
        ("getDay", 0, NativeFunction::DatePrototypeGetUtcDay),
        ("getHours", 0, NativeFunction::DatePrototypeGetUtcHours),
        ("getMinutes", 0, NativeFunction::DatePrototypeGetUtcMinutes),
        ("getSeconds", 0, NativeFunction::DatePrototypeGetUtcSeconds),
        (
            "getMilliseconds",
            0,
            NativeFunction::DatePrototypeGetUtcMilliseconds,
        ),
        ("setYear", 1, NativeFunction::DatePrototypeSetYear),
        ("getUTCDate", 0, NativeFunction::DatePrototypeGetUtcDate),
        ("getUTCDay", 0, NativeFunction::DatePrototypeGetUtcDay),
        (
            "getUTCFullYear",
            0,
            NativeFunction::DatePrototypeGetUtcFullYear,
        ),
        ("getUTCHours", 0, NativeFunction::DatePrototypeGetUtcHours),
        (
            "getUTCMilliseconds",
            0,
            NativeFunction::DatePrototypeGetUtcMilliseconds,
        ),
        (
            "getUTCMinutes",
            0,
            NativeFunction::DatePrototypeGetUtcMinutes,
        ),
        ("getUTCMonth", 0, NativeFunction::DatePrototypeGetUtcMonth),
        (
            "getUTCSeconds",
            0,
            NativeFunction::DatePrototypeGetUtcSeconds,
        ),
        ("setTime", 1, NativeFunction::DatePrototypeSetTime),
        (
            "setFullYear",
            3,
            NativeFunction::DatePrototypeSetUtcFullYear,
        ),
        ("setMonth", 2, NativeFunction::DatePrototypeSetUtcMonth),
        ("setDate", 1, NativeFunction::DatePrototypeSetUtcDate),
        ("setHours", 4, NativeFunction::DatePrototypeSetUtcHours),
        ("setMinutes", 3, NativeFunction::DatePrototypeSetUtcMinutes),
        ("setSeconds", 2, NativeFunction::DatePrototypeSetUtcSeconds),
        (
            "setMilliseconds",
            1,
            NativeFunction::DatePrototypeSetUtcMilliseconds,
        ),
        ("setUTCDate", 1, NativeFunction::DatePrototypeSetUtcDate),
        (
            "setUTCFullYear",
            3,
            NativeFunction::DatePrototypeSetUtcFullYear,
        ),
        ("setUTCHours", 4, NativeFunction::DatePrototypeSetUtcHours),
        (
            "setUTCMilliseconds",
            1,
            NativeFunction::DatePrototypeSetUtcMilliseconds,
        ),
        (
            "setUTCMinutes",
            3,
            NativeFunction::DatePrototypeSetUtcMinutes,
        ),
        ("setUTCMonth", 2, NativeFunction::DatePrototypeSetUtcMonth),
        (
            "setUTCSeconds",
            2,
            NativeFunction::DatePrototypeSetUtcSeconds,
        ),
        ("toDateString", 0, NativeFunction::DatePrototypeToDateString),
        ("toLocaleString", 0, NativeFunction::DatePrototypeToString),
        (
            "toLocaleDateString",
            0,
            NativeFunction::DatePrototypeToDateString,
        ),
        (
            "toLocaleTimeString",
            0,
            NativeFunction::DatePrototypeToTimeString,
        ),
        ("toISOString", 0, NativeFunction::DatePrototypeToISOString),
        ("toJSON", 1, NativeFunction::DatePrototypeToJson),
    ]);
    if let Some(to_primitive) = symbol::to_primitive_symbol(env) {
        date_prototype.define_symbol_property(
            to_primitive,
            Property::data(
                Value::Function(Function::new_native(
                    env.realm().atoms(),
                    Some("[Symbol.toPrimitive]"),
                    1,
                    NativeFunction::DatePrototypeToPrimitive,
                    false,
                )),
                false,
                false,
                true,
            ),
        );
    }
    date_prototype.define_native_methods(&[
        ("toString", 0, NativeFunction::DatePrototypeToString),
        ("toTimeString", 0, NativeFunction::DatePrototypeToTimeString),
    ]);
    let to_utc_string = Value::Function(Function::new_native(
        env.realm().atoms(),
        Some("toUTCString"),
        0,
        NativeFunction::DatePrototypeToUtcString,
        false,
    ));
    date_prototype.define_non_enumerable("toUTCString".to_owned(), to_utc_string.clone());
    date_prototype.define_non_enumerable("toGMTString".to_owned(), to_utc_string);
    define_date_prototype_function(
        &date_prototype,
        "valueOf",
        0,
        NativeFunction::DatePrototypeValueOf,
    );

    date_function.properties.borrow_mut().insert_in(
        env.realm().atoms(),
        "prototype",
        Property::fixed_non_enumerable(Value::Object(date_prototype)),
    );
    define_date_function(
        env.realm().atoms(),
        &date_function,
        "now",
        0,
        NativeFunction::DateNow,
    );
    define_date_function(
        env.realm().atoms(),
        &date_function,
        "parse",
        1,
        NativeFunction::DateParse,
    );
    define_date_function(
        env.realm().atoms(),
        &date_function,
        "UTC",
        7,
        NativeFunction::DateUtc,
    );

    let date_value = Value::Function(date_function);
    env.insert_realm("Date".to_owned(), date_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("Date".to_owned(), date_value);
    }
}

fn define_date_function(
    atoms: &crate::property::atom::AtomDomain,
    function: &Function,
    key: &str,
    length: usize,
    native: NativeFunction,
) {
    function.properties.borrow_mut().insert_in(
        atoms,
        key,
        Property::non_enumerable(Value::Function(Function::new_native(
            atoms,
            Some(key),
            length,
            native,
            false,
        ))),
    );
}

fn define_date_prototype_function(
    prototype: &ObjectRef,
    key: &str,
    length: usize,
    native: NativeFunction,
) {
    prototype.define_non_enumerable(
        key.to_owned(),
        Value::Function(Function::new_native(
            &prototype.store_domain(),
            Some(key),
            length,
            native,
            false,
        )),
    );
}
