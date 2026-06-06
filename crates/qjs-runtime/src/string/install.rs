use std::collections::HashMap;

use crate::{Function, NativeFunction, ObjectRef, Property, Value};

const STRING_PROTOTYPE_METHODS: &[(&str, usize, NativeFunction)] = &[
    ("at", 1, NativeFunction::StringPrototypeAt),
    ("anchor", 1, NativeFunction::StringPrototypeAnchor),
    ("big", 0, NativeFunction::StringPrototypeBig),
    ("blink", 0, NativeFunction::StringPrototypeBlink),
    ("bold", 0, NativeFunction::StringPrototypeBold),
    ("charAt", 1, NativeFunction::StringPrototypeCharAt),
    ("charCodeAt", 1, NativeFunction::StringPrototypeCharCodeAt),
    ("codePointAt", 1, NativeFunction::StringPrototypeCodePointAt),
    ("concat", 1, NativeFunction::StringPrototypeConcat),
    ("endsWith", 1, NativeFunction::StringPrototypeEndsWith),
    ("fixed", 0, NativeFunction::StringPrototypeFixed),
    ("fontcolor", 1, NativeFunction::StringPrototypeFontcolor),
    ("fontsize", 1, NativeFunction::StringPrototypeFontsize),
    ("includes", 1, NativeFunction::StringPrototypeIncludes),
    ("indexOf", 1, NativeFunction::StringPrototypeIndexOf),
    ("italics", 0, NativeFunction::StringPrototypeItalics),
    (
        "isWellFormed",
        0,
        NativeFunction::StringPrototypeIsWellFormed,
    ),
    ("lastIndexOf", 1, NativeFunction::StringPrototypeLastIndexOf),
    ("link", 1, NativeFunction::StringPrototypeLink),
    (
        "localeCompare",
        1,
        NativeFunction::StringPrototypeLocaleCompare,
    ),
    ("match", 1, NativeFunction::StringPrototypeMatch),
    ("padEnd", 1, NativeFunction::StringPrototypePadEnd),
    ("padStart", 1, NativeFunction::StringPrototypePadStart),
    ("repeat", 1, NativeFunction::StringPrototypeRepeat),
    ("replace", 2, NativeFunction::StringPrototypeReplace),
    ("replaceAll", 2, NativeFunction::StringPrototypeReplaceAll),
    ("search", 1, NativeFunction::StringPrototypeSearch),
    ("slice", 2, NativeFunction::StringPrototypeSlice),
    ("small", 0, NativeFunction::StringPrototypeSmall),
    ("split", 2, NativeFunction::StringPrototypeSplit),
    ("startsWith", 1, NativeFunction::StringPrototypeStartsWith),
    ("strike", 0, NativeFunction::StringPrototypeStrike),
    ("substr", 2, NativeFunction::StringPrototypeSubstr),
    ("substring", 2, NativeFunction::StringPrototypeSubstring),
    ("sub", 0, NativeFunction::StringPrototypeSub),
    ("sup", 0, NativeFunction::StringPrototypeSup),
    ("toLowerCase", 0, NativeFunction::StringPrototypeToLowerCase),
    (
        "toLocaleLowerCase",
        0,
        NativeFunction::StringPrototypeToLocaleLowerCase,
    ),
    (
        "toLocaleUpperCase",
        0,
        NativeFunction::StringPrototypeToLocaleUpperCase,
    ),
    (
        "toWellFormed",
        0,
        NativeFunction::StringPrototypeToWellFormed,
    ),
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
    define_object_alias(&string_prototype, "trimLeft", "trimStart");
    define_object_alias(&string_prototype, "trimRight", "trimEnd");
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
    define_function_property(
        &string_function,
        "fromCodePoint",
        1,
        NativeFunction::StringFromCodePoint,
    );
    define_function_property(&string_function, "raw", 1, NativeFunction::StringRaw);
    let string_value = Value::Function(string_function);
    env.insert("String".to_owned(), string_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("String".to_owned(), string_value);
    }
}

fn define_object_method(object: &ObjectRef, key: &str, length: usize, native: NativeFunction) {
    object.define_non_enumerable(
        key.to_owned(),
        Value::Function(Function::new_native(Some(key), length, native, false)),
    );
}

fn define_object_alias(object: &ObjectRef, key: &str, target: &str) {
    if let Some(value) = object.get(target) {
        object.define_non_enumerable(key.to_owned(), value);
    }
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
