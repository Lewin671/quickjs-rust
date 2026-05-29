use std::collections::HashMap;

use crate::{Function, NativeFunction, ObjectRef, Property, Value};

use super::value::define_date_value;

pub(crate) fn install_date(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let date_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    define_date_value(&date_prototype, f64::NAN);

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
        "toISOString",
        0,
        NativeFunction::DatePrototypeToISOString,
    );
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
