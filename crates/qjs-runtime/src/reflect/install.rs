use crate::CallEnv;
use std::collections::HashMap;

use crate::{Function, NativeFunction, ObjectRef, Value};

pub(crate) fn install_reflect(env: &mut CallEnv, global_this: &Value, object_prototype: ObjectRef) {
    let reflect_object = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    define_reflect_function(&reflect_object, "apply", 3, NativeFunction::ReflectApply);
    define_reflect_function(
        &reflect_object,
        "construct",
        2,
        NativeFunction::ReflectConstruct,
    );
    define_reflect_function(
        &reflect_object,
        "defineProperty",
        3,
        NativeFunction::ReflectDefineProperty,
    );
    define_reflect_function(
        &reflect_object,
        "deleteProperty",
        2,
        NativeFunction::ReflectDeleteProperty,
    );
    define_reflect_function(&reflect_object, "get", 2, NativeFunction::ReflectGet);
    define_reflect_function(
        &reflect_object,
        "getPrototypeOf",
        1,
        NativeFunction::ReflectGetPrototypeOf,
    );
    define_reflect_function(
        &reflect_object,
        "getOwnPropertyDescriptor",
        2,
        NativeFunction::ReflectGetOwnPropertyDescriptor,
    );
    define_reflect_function(&reflect_object, "has", 2, NativeFunction::ReflectHas);
    define_reflect_function(
        &reflect_object,
        "isExtensible",
        1,
        NativeFunction::ReflectIsExtensible,
    );
    define_reflect_function(
        &reflect_object,
        "ownKeys",
        1,
        NativeFunction::ReflectOwnKeys,
    );
    define_reflect_function(
        &reflect_object,
        "preventExtensions",
        1,
        NativeFunction::ReflectPreventExtensions,
    );
    define_reflect_function(&reflect_object, "set", 3, NativeFunction::ReflectSet);
    define_reflect_function(
        &reflect_object,
        "setPrototypeOf",
        2,
        NativeFunction::ReflectSetPrototypeOf,
    );

    let reflect_value = Value::Object(reflect_object);
    env.insert_realm("Reflect".to_owned(), reflect_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("Reflect".to_owned(), reflect_value);
    }
}

fn define_reflect_function(object: &ObjectRef, key: &str, length: usize, native: NativeFunction) {
    object.define_non_enumerable(
        key.to_owned(),
        Value::Function(Function::new_native(Some(key), length, native, false)),
    );
}
