use std::collections::HashMap;

use crate::{Function, NativeFunction, ObjectRef, Property, Value, symbol};

pub(crate) fn install_date(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let date_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));

    let date_function = Function::new_native(Some("Date"), 7, NativeFunction::Date, true);
    date_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(date_function.clone()),
    );
    define_date_prototype_function(
        &date_prototype,
        "getTime",
        0,
        NativeFunction::DatePrototypeGetTime,
    );
    define_date_prototype_function(
        &date_prototype,
        "getTimezoneOffset",
        0,
        NativeFunction::DatePrototypeGetTimezoneOffset,
    );
    define_date_prototype_function(
        &date_prototype,
        "getYear",
        0,
        NativeFunction::DatePrototypeGetYear,
    );
    define_date_prototype_function(
        &date_prototype,
        "getFullYear",
        0,
        NativeFunction::DatePrototypeGetUtcFullYear,
    );
    define_date_prototype_function(
        &date_prototype,
        "setYear",
        1,
        NativeFunction::DatePrototypeSetYear,
    );
    define_date_prototype_function(
        &date_prototype,
        "getUTCDate",
        0,
        NativeFunction::DatePrototypeGetUtcDate,
    );
    define_date_prototype_function(
        &date_prototype,
        "getUTCDay",
        0,
        NativeFunction::DatePrototypeGetUtcDay,
    );
    define_date_prototype_function(
        &date_prototype,
        "getUTCFullYear",
        0,
        NativeFunction::DatePrototypeGetUtcFullYear,
    );
    define_date_prototype_function(
        &date_prototype,
        "getUTCHours",
        0,
        NativeFunction::DatePrototypeGetUtcHours,
    );
    define_date_prototype_function(
        &date_prototype,
        "getUTCMilliseconds",
        0,
        NativeFunction::DatePrototypeGetUtcMilliseconds,
    );
    define_date_prototype_function(
        &date_prototype,
        "getUTCMinutes",
        0,
        NativeFunction::DatePrototypeGetUtcMinutes,
    );
    define_date_prototype_function(
        &date_prototype,
        "getUTCMonth",
        0,
        NativeFunction::DatePrototypeGetUtcMonth,
    );
    define_date_prototype_function(
        &date_prototype,
        "getUTCSeconds",
        0,
        NativeFunction::DatePrototypeGetUtcSeconds,
    );
    define_date_prototype_function(
        &date_prototype,
        "setTime",
        1,
        NativeFunction::DatePrototypeSetTime,
    );
    define_date_prototype_function(
        &date_prototype,
        "setUTCDate",
        1,
        NativeFunction::DatePrototypeSetUtcDate,
    );
    define_date_prototype_function(
        &date_prototype,
        "setUTCFullYear",
        3,
        NativeFunction::DatePrototypeSetUtcFullYear,
    );
    define_date_prototype_function(
        &date_prototype,
        "setUTCHours",
        4,
        NativeFunction::DatePrototypeSetUtcHours,
    );
    define_date_prototype_function(
        &date_prototype,
        "setUTCMilliseconds",
        1,
        NativeFunction::DatePrototypeSetUtcMilliseconds,
    );
    define_date_prototype_function(
        &date_prototype,
        "setUTCMinutes",
        3,
        NativeFunction::DatePrototypeSetUtcMinutes,
    );
    define_date_prototype_function(
        &date_prototype,
        "setUTCMonth",
        2,
        NativeFunction::DatePrototypeSetUtcMonth,
    );
    define_date_prototype_function(
        &date_prototype,
        "setUTCSeconds",
        2,
        NativeFunction::DatePrototypeSetUtcSeconds,
    );
    define_date_prototype_function(
        &date_prototype,
        "toDateString",
        0,
        NativeFunction::DatePrototypeToDateString,
    );
    define_date_prototype_function(
        &date_prototype,
        "toISOString",
        0,
        NativeFunction::DatePrototypeToISOString,
    );
    define_date_prototype_function(
        &date_prototype,
        "toJSON",
        1,
        NativeFunction::DatePrototypeToJson,
    );
    if let Some(to_primitive) = symbol::to_primitive_symbol(env) {
        date_prototype.define_symbol_property(
            to_primitive,
            Property::data(
                Value::Function(Function::new_native(
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
    define_date_prototype_function(
        &date_prototype,
        "toString",
        0,
        NativeFunction::DatePrototypeToString,
    );
    define_date_prototype_function(
        &date_prototype,
        "toTimeString",
        0,
        NativeFunction::DatePrototypeToTimeString,
    );
    let to_utc_string = Value::Function(Function::new_native(
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

    date_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(date_prototype)),
    );
    define_date_function(&date_function, "now", 0, NativeFunction::DateNow);
    define_date_function(&date_function, "parse", 1, NativeFunction::DateParse);
    define_date_function(&date_function, "UTC", 7, NativeFunction::DateUtc);

    let date_value = Value::Function(date_function);
    env.insert("Date".to_owned(), date_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Date".to_owned(), date_value);
    }
}

fn define_date_function(function: &Function, key: &str, length: usize, native: NativeFunction) {
    function.properties.borrow_mut().insert(
        key.to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
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
        Value::Function(Function::new_native(Some(key), length, native, false)),
    );
}
