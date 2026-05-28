use std::collections::HashMap;

use crate::{Function, NativeFunction, ObjectRef, Property, Value};

pub(crate) fn install_function(
    env: &mut HashMap<String, Value>,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let function_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    let function_constructor =
        Function::new_native(Some("Function"), 1, NativeFunction::Function, true);
    function_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(function_constructor.clone()),
    );
    function_prototype.define_non_enumerable(
        "apply".to_owned(),
        Value::Function(Function::new_native(
            Some("apply"),
            2,
            NativeFunction::FunctionPrototypeApply,
            false,
        )),
    );
    function_prototype.define_non_enumerable(
        "call".to_owned(),
        Value::Function(Function::new_native(
            Some("call"),
            1,
            NativeFunction::FunctionPrototypeCall,
            false,
        )),
    );
    function_prototype.define_non_enumerable(
        "bind".to_owned(),
        Value::Function(Function::new_native(
            Some("bind"),
            1,
            NativeFunction::FunctionPrototypeBind,
            false,
        )),
    );
    function_constructor.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(function_prototype)),
    );

    let function_value = Value::Function(function_constructor);
    env.insert("Function".to_owned(), function_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Function".to_owned(), function_value);
    }
}
