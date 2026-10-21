use crate::CallEnv;
use std::collections::HashMap;

use crate::{Function, NativeFunction, ObjectRef, Property, Value};

pub(crate) fn install_object(env: &mut CallEnv, global_this: &Value) -> ObjectRef {
    let object_prototype = ObjectRef::new_in(env.realm().atoms(), HashMap::new());
    object_prototype.mark_immutable_prototype_exotic();
    env.realm()
        .initialize_object_prototype(object_prototype.clone());
    let object_function = Function::new_native(
        env.realm().atoms(),
        Some("Object"),
        1,
        NativeFunction::Object,
        true,
    );
    object_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(object_function.clone()),
    );
    object_prototype.define_native_methods(&[
        (
            "hasOwnProperty",
            1,
            NativeFunction::ObjectPrototypeHasOwnProperty,
        ),
        (
            "isPrototypeOf",
            1,
            NativeFunction::ObjectPrototypeIsPrototypeOf,
        ),
        (
            "propertyIsEnumerable",
            1,
            NativeFunction::ObjectPrototypePropertyIsEnumerable,
        ),
        (
            "toLocaleString",
            0,
            NativeFunction::ObjectPrototypeToLocaleString,
        ),
        ("toString", 0, NativeFunction::ObjectPrototypeToString),
        ("valueOf", 0, NativeFunction::ObjectPrototypeValueOf),
        (
            "__defineGetter__",
            2,
            NativeFunction::ObjectPrototypeDefineGetter,
        ),
        (
            "__defineSetter__",
            2,
            NativeFunction::ObjectPrototypeDefineSetter,
        ),
        (
            "__lookupGetter__",
            1,
            NativeFunction::ObjectPrototypeLookupGetter,
        ),
        (
            "__lookupSetter__",
            1,
            NativeFunction::ObjectPrototypeLookupSetter,
        ),
    ]);
    object_prototype.define_property(
        "__proto__".to_owned(),
        Property::accessor(
            Some(Value::Function(Function::new_native(
                env.realm().atoms(),
                Some("get __proto__"),
                0,
                NativeFunction::ObjectPrototypeGetProto,
                false,
            ))),
            Some(Value::Function(Function::new_native(
                env.realm().atoms(),
                Some("set __proto__"),
                1,
                NativeFunction::ObjectPrototypeSetProto,
                false,
            ))),
            false,
            true,
        ),
    );
    object_function.properties.borrow_mut().insert_in(
        env.realm().atoms(),
        "prototype",
        Property::fixed_non_enumerable(Value::Object(object_prototype.clone())),
    );
    let methods = [
        ("assign", 2, NativeFunction::ObjectAssign),
        ("create", 2, NativeFunction::ObjectCreate),
        ("defineProperty", 3, NativeFunction::ObjectDefineProperty),
        (
            "defineProperties",
            2,
            NativeFunction::ObjectDefineProperties,
        ),
        ("getPrototypeOf", 1, NativeFunction::ObjectGetPrototypeOf),
        (
            "getOwnPropertyDescriptor",
            2,
            NativeFunction::ObjectGetOwnPropertyDescriptor,
        ),
        (
            "getOwnPropertyDescriptors",
            1,
            NativeFunction::ObjectGetOwnPropertyDescriptors,
        ),
        (
            "getOwnPropertyNames",
            1,
            NativeFunction::ObjectGetOwnPropertyNames,
        ),
        (
            "getOwnPropertySymbols",
            1,
            NativeFunction::ObjectGetOwnPropertySymbols,
        ),
        ("fromEntries", 1, NativeFunction::ObjectFromEntries),
        ("groupBy", 2, NativeFunction::ObjectGroupBy),
        ("freeze", 1, NativeFunction::ObjectFreeze),
        ("hasOwn", 2, NativeFunction::ObjectHasOwn),
        ("is", 2, NativeFunction::ObjectIs),
        ("isExtensible", 1, NativeFunction::ObjectIsExtensible),
        ("isFrozen", 1, NativeFunction::ObjectIsFrozen),
        ("isSealed", 1, NativeFunction::ObjectIsSealed),
        (
            "preventExtensions",
            1,
            NativeFunction::ObjectPreventExtensions,
        ),
        ("seal", 1, NativeFunction::ObjectSeal),
        ("setPrototypeOf", 2, NativeFunction::ObjectSetPrototypeOf),
        ("entries", 1, NativeFunction::ObjectEntries),
        ("keys", 1, NativeFunction::ObjectKeys),
        ("values", 1, NativeFunction::ObjectValues),
    ];
    let entries = methods
        .into_iter()
        .map(|(key, length, native)| {
            (
                std::rc::Rc::from(key),
                Property::non_enumerable(Value::Function(Function::new_native(
                    env.realm().atoms(),
                    Some(key),
                    length,
                    native,
                    false,
                ))),
            )
        })
        .collect();
    object_function
        .properties
        .borrow_mut()
        .insert_batch_in(env.realm().atoms(), entries);

    let object_value = Value::Function(object_function);
    env.insert_realm("Object".to_owned(), object_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("Object".to_owned(), object_value);
    }

    object_prototype
}
