use std::collections::HashMap;

use crate::{Function, NativeFunction, ObjectRef, Property, Value};

pub(crate) fn install_array(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let array_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    let array_function = Function::new_native(Some("Array"), 1, NativeFunction::Array, true);
    array_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(array_function.clone()),
    );
    define_array_prototype_function(&array_prototype, "at", 1, NativeFunction::ArrayPrototypeAt);
    define_array_prototype_function(
        &array_prototype,
        "concat",
        1,
        NativeFunction::ArrayPrototypeConcat,
    );
    define_array_prototype_function(
        &array_prototype,
        "copyWithin",
        2,
        NativeFunction::ArrayPrototypeCopyWithin,
    );
    define_array_prototype_function(
        &array_prototype,
        "fill",
        1,
        NativeFunction::ArrayPrototypeFill,
    );
    define_array_prototype_function(
        &array_prototype,
        "includes",
        1,
        NativeFunction::ArrayPrototypeIncludes,
    );
    define_array_prototype_function(
        &array_prototype,
        "join",
        1,
        NativeFunction::ArrayPrototypeJoin,
    );
    define_array_prototype_function(
        &array_prototype,
        "indexOf",
        1,
        NativeFunction::ArrayPrototypeIndexOf,
    );
    define_array_prototype_function(
        &array_prototype,
        "lastIndexOf",
        1,
        NativeFunction::ArrayPrototypeLastIndexOf,
    );
    define_array_prototype_function(
        &array_prototype,
        "pop",
        0,
        NativeFunction::ArrayPrototypePop,
    );
    define_array_prototype_function(
        &array_prototype,
        "push",
        1,
        NativeFunction::ArrayPrototypePush,
    );
    define_array_prototype_function(
        &array_prototype,
        "reverse",
        0,
        NativeFunction::ArrayPrototypeReverse,
    );
    define_array_prototype_function(
        &array_prototype,
        "shift",
        0,
        NativeFunction::ArrayPrototypeShift,
    );
    define_array_prototype_function(
        &array_prototype,
        "slice",
        2,
        NativeFunction::ArrayPrototypeSlice,
    );
    define_array_prototype_function(
        &array_prototype,
        "toString",
        0,
        NativeFunction::ArrayPrototypeToString,
    );
    define_array_prototype_function(
        &array_prototype,
        "unshift",
        1,
        NativeFunction::ArrayPrototypeUnshift,
    );
    array_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(array_prototype)),
    );
    define_array_function(&array_function, "isArray", 1, NativeFunction::ArrayIsArray);

    let array_value = Value::Function(array_function);
    env.insert("Array".to_owned(), array_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Array".to_owned(), array_value);
    }
}

fn define_array_prototype_function(
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

fn define_array_function(function: &Function, key: &str, length: usize, native: NativeFunction) {
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
