use std::collections::HashMap;

use crate::{Function, NativeFunction, ObjectRef, Value};

pub(crate) fn install_json(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let json_object = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    define_json_function(&json_object, "parse", 2, NativeFunction::JsonParse);
    define_json_function(&json_object, "stringify", 3, NativeFunction::JsonStringify);

    let json_value = Value::Object(json_object);
    env.insert("JSON".to_owned(), json_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("JSON".to_owned(), json_value);
    }
}

fn define_json_function(object: &ObjectRef, key: &str, length: usize, native: NativeFunction) {
    object.define_non_enumerable(
        key.to_owned(),
        Value::Function(Function::new_native(Some(key), length, native, false)),
    );
}
