use std::collections::HashMap;

use crate::{
    Function, NativeFunction, ObjectRef, Property, Value, boolean, global, math, number, object,
};

pub(crate) fn initialize_builtins(env: &mut HashMap<String, Value>, global_this: &Value) {
    let object_prototype = object::install_object(env, global_this);

    global::install_globals(env, global_this);

    number::install_number(env, global_this, object_prototype.clone());
    install_string(env, global_this, object_prototype.clone());
    boolean::install_boolean(env, global_this, object_prototype.clone());
    math::install_math(env, global_this, object_prototype.clone());
    install_array(env, global_this, object_prototype);
}

fn install_string(
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
    string_prototype.define_non_enumerable(
        "at".to_owned(),
        Value::Function(Function::new_native(
            Some("at"),
            1,
            NativeFunction::StringPrototypeAt,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "charAt".to_owned(),
        Value::Function(Function::new_native(
            Some("charAt"),
            1,
            NativeFunction::StringPrototypeCharAt,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "charCodeAt".to_owned(),
        Value::Function(Function::new_native(
            Some("charCodeAt"),
            1,
            NativeFunction::StringPrototypeCharCodeAt,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "codePointAt".to_owned(),
        Value::Function(Function::new_native(
            Some("codePointAt"),
            1,
            NativeFunction::StringPrototypeCodePointAt,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "concat".to_owned(),
        Value::Function(Function::new_native(
            Some("concat"),
            1,
            NativeFunction::StringPrototypeConcat,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "endsWith".to_owned(),
        Value::Function(Function::new_native(
            Some("endsWith"),
            1,
            NativeFunction::StringPrototypeEndsWith,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "includes".to_owned(),
        Value::Function(Function::new_native(
            Some("includes"),
            1,
            NativeFunction::StringPrototypeIncludes,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "indexOf".to_owned(),
        Value::Function(Function::new_native(
            Some("indexOf"),
            1,
            NativeFunction::StringPrototypeIndexOf,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "lastIndexOf".to_owned(),
        Value::Function(Function::new_native(
            Some("lastIndexOf"),
            1,
            NativeFunction::StringPrototypeLastIndexOf,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "padEnd".to_owned(),
        Value::Function(Function::new_native(
            Some("padEnd"),
            1,
            NativeFunction::StringPrototypePadEnd,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "padStart".to_owned(),
        Value::Function(Function::new_native(
            Some("padStart"),
            1,
            NativeFunction::StringPrototypePadStart,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "repeat".to_owned(),
        Value::Function(Function::new_native(
            Some("repeat"),
            1,
            NativeFunction::StringPrototypeRepeat,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "slice".to_owned(),
        Value::Function(Function::new_native(
            Some("slice"),
            2,
            NativeFunction::StringPrototypeSlice,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "split".to_owned(),
        Value::Function(Function::new_native(
            Some("split"),
            2,
            NativeFunction::StringPrototypeSplit,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "startsWith".to_owned(),
        Value::Function(Function::new_native(
            Some("startsWith"),
            1,
            NativeFunction::StringPrototypeStartsWith,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "substring".to_owned(),
        Value::Function(Function::new_native(
            Some("substring"),
            2,
            NativeFunction::StringPrototypeSubstring,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "toLowerCase".to_owned(),
        Value::Function(Function::new_native(
            Some("toLowerCase"),
            0,
            NativeFunction::StringPrototypeToLowerCase,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "trim".to_owned(),
        Value::Function(Function::new_native(
            Some("trim"),
            0,
            NativeFunction::StringPrototypeTrim,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "trimEnd".to_owned(),
        Value::Function(Function::new_native(
            Some("trimEnd"),
            0,
            NativeFunction::StringPrototypeTrimEnd,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "trimStart".to_owned(),
        Value::Function(Function::new_native(
            Some("trimStart"),
            0,
            NativeFunction::StringPrototypeTrimStart,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "toString".to_owned(),
        Value::Function(Function::new_native(
            Some("toString"),
            0,
            NativeFunction::StringPrototypeToString,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "toUpperCase".to_owned(),
        Value::Function(Function::new_native(
            Some("toUpperCase"),
            0,
            NativeFunction::StringPrototypeToUpperCase,
            false,
        )),
    );
    string_prototype.define_non_enumerable(
        "valueOf".to_owned(),
        Value::Function(Function::new_native(
            Some("valueOf"),
            0,
            NativeFunction::StringPrototypeValueOf,
            false,
        )),
    );
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

fn install_array(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let array_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype.clone()));
    let array_function = Function::new_native(Some("Array"), 1, NativeFunction::Array, true);
    array_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(array_function.clone()),
    );
    array_prototype.define_non_enumerable(
        "at".to_owned(),
        Value::Function(Function::new_native(
            Some("at"),
            1,
            NativeFunction::ArrayPrototypeAt,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "concat".to_owned(),
        Value::Function(Function::new_native(
            Some("concat"),
            1,
            NativeFunction::ArrayPrototypeConcat,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "copyWithin".to_owned(),
        Value::Function(Function::new_native(
            Some("copyWithin"),
            2,
            NativeFunction::ArrayPrototypeCopyWithin,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "fill".to_owned(),
        Value::Function(Function::new_native(
            Some("fill"),
            1,
            NativeFunction::ArrayPrototypeFill,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "includes".to_owned(),
        Value::Function(Function::new_native(
            Some("includes"),
            1,
            NativeFunction::ArrayPrototypeIncludes,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "join".to_owned(),
        Value::Function(Function::new_native(
            Some("join"),
            1,
            NativeFunction::ArrayPrototypeJoin,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "indexOf".to_owned(),
        Value::Function(Function::new_native(
            Some("indexOf"),
            1,
            NativeFunction::ArrayPrototypeIndexOf,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "lastIndexOf".to_owned(),
        Value::Function(Function::new_native(
            Some("lastIndexOf"),
            1,
            NativeFunction::ArrayPrototypeLastIndexOf,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "pop".to_owned(),
        Value::Function(Function::new_native(
            Some("pop"),
            0,
            NativeFunction::ArrayPrototypePop,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "push".to_owned(),
        Value::Function(Function::new_native(
            Some("push"),
            1,
            NativeFunction::ArrayPrototypePush,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "reverse".to_owned(),
        Value::Function(Function::new_native(
            Some("reverse"),
            0,
            NativeFunction::ArrayPrototypeReverse,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "shift".to_owned(),
        Value::Function(Function::new_native(
            Some("shift"),
            0,
            NativeFunction::ArrayPrototypeShift,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "slice".to_owned(),
        Value::Function(Function::new_native(
            Some("slice"),
            2,
            NativeFunction::ArrayPrototypeSlice,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "toString".to_owned(),
        Value::Function(Function::new_native(
            Some("toString"),
            0,
            NativeFunction::ArrayPrototypeToString,
            false,
        )),
    );
    array_prototype.define_non_enumerable(
        "unshift".to_owned(),
        Value::Function(Function::new_native(
            Some("unshift"),
            1,
            NativeFunction::ArrayPrototypeUnshift,
            false,
        )),
    );
    array_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(array_prototype)),
    );
    array_function.properties.borrow_mut().insert(
        "isArray".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("isArray"),
            1,
            NativeFunction::ArrayIsArray,
            false,
        ))),
    );

    let array_value = Value::Function(array_function);
    env.insert("Array".to_owned(), array_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Array".to_owned(), array_value);
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
