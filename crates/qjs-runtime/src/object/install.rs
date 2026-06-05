use std::collections::HashMap;

use crate::{Function, NativeFunction, ObjectRef, Property, Value};

pub(crate) fn install_object(env: &mut HashMap<String, Value>, global_this: &Value) -> ObjectRef {
    let object_prototype = ObjectRef::new(HashMap::new());
    let object_function = Function::new_native(Some("Object"), 1, NativeFunction::Object, true);
    object_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(object_function.clone()),
    );
    define_object_prototype_function(
        &object_prototype,
        "hasOwnProperty",
        1,
        NativeFunction::ObjectPrototypeHasOwnProperty,
    );
    define_object_prototype_function(
        &object_prototype,
        "propertyIsEnumerable",
        1,
        NativeFunction::ObjectPrototypePropertyIsEnumerable,
    );
    define_object_prototype_function(
        &object_prototype,
        "isPrototypeOf",
        1,
        NativeFunction::ObjectPrototypeIsPrototypeOf,
    );
    define_object_prototype_function(
        &object_prototype,
        "toString",
        0,
        NativeFunction::ObjectPrototypeToString,
    );
    define_object_prototype_function(
        &object_prototype,
        "toLocaleString",
        0,
        NativeFunction::ObjectPrototypeToLocaleString,
    );
    define_object_prototype_function(
        &object_prototype,
        "valueOf",
        0,
        NativeFunction::ObjectPrototypeValueOf,
    );
    object_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::data(Value::Object(object_prototype.clone()), false, false, false),
    );
    define_object_function(&object_function, "assign", 2, NativeFunction::ObjectAssign);
    define_object_function(&object_function, "create", 2, NativeFunction::ObjectCreate);
    define_object_function(
        &object_function,
        "defineProperty",
        3,
        NativeFunction::ObjectDefineProperty,
    );
    define_object_function(
        &object_function,
        "defineProperties",
        2,
        NativeFunction::ObjectDefineProperties,
    );
    define_object_function(
        &object_function,
        "getPrototypeOf",
        1,
        NativeFunction::ObjectGetPrototypeOf,
    );
    define_object_function(
        &object_function,
        "getOwnPropertyDescriptor",
        2,
        NativeFunction::ObjectGetOwnPropertyDescriptor,
    );
    define_object_function(
        &object_function,
        "getOwnPropertyDescriptors",
        1,
        NativeFunction::ObjectGetOwnPropertyDescriptors,
    );
    define_object_function(
        &object_function,
        "getOwnPropertyNames",
        1,
        NativeFunction::ObjectGetOwnPropertyNames,
    );
    define_object_function(
        &object_function,
        "fromEntries",
        1,
        NativeFunction::ObjectFromEntries,
    );
    define_object_function(&object_function, "freeze", 1, NativeFunction::ObjectFreeze);
    define_object_function(&object_function, "hasOwn", 2, NativeFunction::ObjectHasOwn);
    define_object_function(&object_function, "is", 2, NativeFunction::ObjectIs);
    define_object_function(
        &object_function,
        "isExtensible",
        1,
        NativeFunction::ObjectIsExtensible,
    );
    define_object_function(
        &object_function,
        "isFrozen",
        1,
        NativeFunction::ObjectIsFrozen,
    );
    define_object_function(
        &object_function,
        "isSealed",
        1,
        NativeFunction::ObjectIsSealed,
    );
    define_object_function(
        &object_function,
        "preventExtensions",
        1,
        NativeFunction::ObjectPreventExtensions,
    );
    define_object_function(&object_function, "seal", 1, NativeFunction::ObjectSeal);
    define_object_function(
        &object_function,
        "setPrototypeOf",
        2,
        NativeFunction::ObjectSetPrototypeOf,
    );
    define_object_function(
        &object_function,
        "entries",
        1,
        NativeFunction::ObjectEntries,
    );
    define_object_function(&object_function, "keys", 1, NativeFunction::ObjectKeys);
    define_object_function(&object_function, "values", 1, NativeFunction::ObjectValues);

    let object_value = Value::Function(object_function);
    env.insert("Object".to_owned(), object_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Object".to_owned(), object_value);
    }

    object_prototype
}

fn define_object_prototype_function(
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

fn define_object_function(function: &Function, key: &str, length: usize, native: NativeFunction) {
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
