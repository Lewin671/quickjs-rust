use std::collections::HashMap;

use crate::{Function, NativeFunction, ObjectRef, Property, Value};

const STRING_PROTOTYPE_METHODS: &[(&str, usize, NativeFunction)] = &[
    ("at", 1, NativeFunction::StringPrototypeAt),
    ("charAt", 1, NativeFunction::StringPrototypeCharAt),
    ("charCodeAt", 1, NativeFunction::StringPrototypeCharCodeAt),
    ("codePointAt", 1, NativeFunction::StringPrototypeCodePointAt),
    ("concat", 1, NativeFunction::StringPrototypeConcat),
    ("endsWith", 1, NativeFunction::StringPrototypeEndsWith),
    ("includes", 1, NativeFunction::StringPrototypeIncludes),
    ("indexOf", 1, NativeFunction::StringPrototypeIndexOf),
    ("lastIndexOf", 1, NativeFunction::StringPrototypeLastIndexOf),
    ("padEnd", 1, NativeFunction::StringPrototypePadEnd),
    ("padStart", 1, NativeFunction::StringPrototypePadStart),
    ("repeat", 1, NativeFunction::StringPrototypeRepeat),
    ("slice", 2, NativeFunction::StringPrototypeSlice),
    ("split", 2, NativeFunction::StringPrototypeSplit),
    ("startsWith", 1, NativeFunction::StringPrototypeStartsWith),
    ("substring", 2, NativeFunction::StringPrototypeSubstring),
    ("toLowerCase", 0, NativeFunction::StringPrototypeToLowerCase),
    ("trim", 0, NativeFunction::StringPrototypeTrim),
    ("trimEnd", 0, NativeFunction::StringPrototypeTrimEnd),
    ("trimStart", 0, NativeFunction::StringPrototypeTrimStart),
    ("toString", 0, NativeFunction::StringPrototypeToString),
    ("toUpperCase", 0, NativeFunction::StringPrototypeToUpperCase),
    ("valueOf", 0, NativeFunction::StringPrototypeValueOf),
];

pub(crate) fn install_string(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let string_prototype =
        ObjectRef::with_prototype(HashMap::new(), Some(object_prototype.clone()));
    let string_function = Function::new_native(Some("String"), 1, NativeFunction::String, true);
    string_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(string_function.clone()),
    );
    for (name, length, native) in STRING_PROTOTYPE_METHODS {
        define_object_method(&string_prototype, name, *length, *native);
    }
    string_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(string_prototype)),
    );
    define_function_property(
        &string_function,
        "fromCharCode",
        1,
        NativeFunction::StringFromCharCode,
    );
    let string_value = Value::Function(string_function);
    env.insert("String".to_owned(), string_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("String".to_owned(), string_value);
    }
}

fn define_object_method(object: &ObjectRef, key: &str, length: usize, native: NativeFunction) {
    object.define_non_enumerable(
        key.to_owned(),
        Value::Function(Function::new_native(Some(key), length, native, false)),
    );
}

fn define_function_property(function: &Function, key: &str, length: usize, native: NativeFunction) {
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
