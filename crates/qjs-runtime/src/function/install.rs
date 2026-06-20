use crate::CallEnv;
use crate::{Function, NativeFunction, ObjectRef, Property, Prototype, Value};

pub(crate) fn install_function(
    env: &mut CallEnv,
    global_this: &Value,
    object_prototype: ObjectRef,
) {
    let function_prototype =
        Function::new_native(Some(""), 0, NativeFunction::FunctionPrototype, false);
    let _ =
        function_prototype.set_internal_prototype_slot(Some(Prototype::Object(object_prototype)));
    let function_constructor =
        Function::new_native(Some("Function"), 1, NativeFunction::Function, true);
    function_prototype.define_property(
        "length".to_owned(),
        Property::data(Value::Number(0.0), false, false, true),
    );
    // %Function.prototype% has an empty `name`, defined immediately after
    // `length` so the built-in property order matches the spec.
    function_prototype.define_property(
        "name".to_owned(),
        Property::data(Value::String(String::new().into()), false, false, true),
    );
    function_prototype.define_property(
        "constructor".to_owned(),
        Property::non_enumerable(Value::Function(function_constructor.clone())),
    );
    function_prototype.define_property(
        "apply".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("apply"),
            2,
            NativeFunction::FunctionPrototypeApply,
            false,
        ))),
    );
    function_prototype.define_property(
        "call".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("call"),
            1,
            NativeFunction::FunctionPrototypeCall,
            false,
        ))),
    );
    function_prototype.define_property(
        "bind".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("bind"),
            1,
            NativeFunction::FunctionPrototypeBind,
            false,
        ))),
    );
    function_prototype.define_property(
        "toString".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("toString"),
            0,
            NativeFunction::FunctionPrototypeToString,
            false,
        ))),
    );
    // %ThrowTypeError% is a single shared intrinsic: the same function object
    // backs `Function.prototype.arguments`/`caller` and the strict
    // `arguments.callee` poison accessor, so their getters compare equal. Stash
    // it in the realm (under a name no source identifier can spell) so the
    // arguments-object builder reuses this exact object.
    let throw_type_error = Value::Function(Function::new_native(
        Some("ThrowTypeError"),
        0,
        NativeFunction::ThrowTypeError,
        false,
    ));
    env.insert_realm(
        super::THROW_TYPE_ERROR_INTRINSIC.to_owned(),
        throw_type_error.clone(),
    );
    let restricted_property = Property::accessor(
        Some(throw_type_error.clone()),
        Some(throw_type_error),
        false,
        true,
    );
    function_prototype.define_property("arguments".to_owned(), restricted_property.clone());
    function_prototype.define_property("caller".to_owned(), restricted_property);
    function_constructor.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Function(function_prototype)),
    );

    let function_value = Value::Function(function_constructor);
    env.insert_realm("Function".to_owned(), function_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("Function".to_owned(), function_value);
    }
}
