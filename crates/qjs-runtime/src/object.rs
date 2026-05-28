use std::collections::HashMap;

use crate::{
    ArrayRef, BOOLEAN_DATA_PROPERTY, Function, NativeFunction, ObjectRef, Property, RuntimeError,
    Value, array_has_own_property, array_own_property_descriptor, array_own_property_keys,
    array_own_property_names, array_prototype, function_own_property_descriptor,
    function_own_property_keys, function_own_property_names, function_prototype, is_truthy, number,
    object_prototype, string, to_property_key, value_prototype,
};

pub(super) fn install_object(env: &mut HashMap<String, Value>, global_this: &Value) -> ObjectRef {
    let object_prototype = ObjectRef::new(HashMap::new());
    let object_function = Function::new_native(Some("Object"), 1, NativeFunction::Object, true);
    object_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(object_function.clone()),
    );
    object_prototype.define_non_enumerable(
        "hasOwnProperty".to_owned(),
        Value::Function(Function::new_native(
            Some("hasOwnProperty"),
            1,
            NativeFunction::ObjectPrototypeHasOwnProperty,
            false,
        )),
    );
    object_prototype.define_non_enumerable(
        "propertyIsEnumerable".to_owned(),
        Value::Function(Function::new_native(
            Some("propertyIsEnumerable"),
            1,
            NativeFunction::ObjectPrototypePropertyIsEnumerable,
            false,
        )),
    );
    object_prototype.define_non_enumerable(
        "isPrototypeOf".to_owned(),
        Value::Function(Function::new_native(
            Some("isPrototypeOf"),
            1,
            NativeFunction::ObjectPrototypeIsPrototypeOf,
            false,
        )),
    );
    object_prototype.define_non_enumerable(
        "toString".to_owned(),
        Value::Function(Function::new_native(
            Some("toString"),
            0,
            NativeFunction::ObjectPrototypeToString,
            false,
        )),
    );
    object_prototype.define_non_enumerable(
        "valueOf".to_owned(),
        Value::Function(Function::new_native(
            Some("valueOf"),
            0,
            NativeFunction::ObjectPrototypeValueOf,
            false,
        )),
    );
    object_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::non_enumerable(Value::Object(object_prototype.clone())),
    );
    define_object_function(&object_function, "assign", 2, NativeFunction::ObjectAssign);
    define_object_function(&object_function, "create", 1, NativeFunction::ObjectCreate);
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
        "getOwnPropertyNames",
        1,
        NativeFunction::ObjectGetOwnPropertyNames,
    );
    define_object_function(&object_function, "hasOwn", 2, NativeFunction::ObjectHasOwn);
    define_object_function(&object_function, "keys", 1, NativeFunction::ObjectKeys);

    let object_value = Value::Function(object_function);
    env.insert("Object".to_owned(), object_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.set("Object".to_owned(), object_value);
    }

    object_prototype
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

pub(super) fn native_object_assign(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    match target {
        Value::Object(_) | Value::Function(_) => {}
        Value::Null | Value::Undefined => {
            return Err(RuntimeError {
                message: "Object.assign target must not be null or undefined".to_owned(),
            });
        }
        Value::Array(_) | Value::String(_) | Value::Number(_) | Value::Boolean(_) => {
            return Err(RuntimeError {
                message: "Object.assign primitive targets are not implemented".to_owned(),
            });
        }
    }

    for source in argument_values.iter().skip(1).cloned() {
        if matches!(source, Value::Null | Value::Undefined) {
            continue;
        }
        for (key, value) in enumerable_property_entries(source)? {
            set_property(target.clone(), key, value)?;
        }
    }
    Ok(target)
}

fn enumerable_property_entries(value: Value) -> Result<Vec<(String, Value)>, RuntimeError> {
    let keys = match value.clone() {
        Value::Object(object) => object.own_property_keys(),
        Value::Array(elements) => array_own_property_keys(&elements),
        Value::Function(function) => function_own_property_keys(&function),
        Value::String(value) => string::string_own_property_keys(&value),
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined => Vec::new(),
    };
    let mut entries = Vec::with_capacity(keys.len());
    for key in keys {
        if let Some(property) = own_property_descriptor(value.clone(), &key)? {
            entries.push((key, property.value));
        }
    }
    Ok(entries)
}

fn set_property(target: Value, key: String, value: Value) -> Result<(), RuntimeError> {
    match target {
        Value::Object(object) => {
            object.set(key, value);
            Ok(())
        }
        Value::Function(function) => {
            function
                .properties
                .borrow_mut()
                .insert(key, Property::enumerable(value));
            Ok(())
        }
        _ => Err(RuntimeError {
            message: "property target is not mutable".to_owned(),
        }),
    }
}

pub(super) fn native_object(
    function: &Function,
    this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    match argument_values.first() {
        Some(Value::Array(_) | Value::Function(_) | Value::Object(_)) => {
            Ok(argument_values[0].clone())
        }
        _ if is_construct => Ok(this_value),
        _ => Ok(Value::Object(ObjectRef::with_prototype(
            HashMap::new(),
            function_prototype(function),
        ))),
    }
}

pub(super) fn native_object_create(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let object = match argument_values.first() {
        Some(Value::Object(prototype)) => Value::Object(ObjectRef::with_prototype(
            HashMap::new(),
            Some(prototype.clone()),
        )),
        Some(Value::Null) => Value::Object(ObjectRef::new(HashMap::new())),
        _ => {
            return Err(RuntimeError {
                message: "Object.create prototype must be an object or null".to_owned(),
            });
        }
    };

    if !matches!(argument_values.get(1), None | Some(Value::Undefined)) {
        native_object_define_properties(&[
            object.clone(),
            argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        ])?;
    }
    Ok(object)
}

pub(super) fn native_object_define_property(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let key = to_property_key(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    let descriptor =
        to_property_descriptor(argument_values.get(2).cloned().unwrap_or(Value::Undefined))?;

    define_property_on_value(target.clone(), key, descriptor)?;
    Ok(target)
}

pub(super) fn native_object_define_properties(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    ensure_define_property_target(&target)?;

    let descriptors = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if !matches!(descriptors, Value::Object(_) | Value::Function(_)) {
        return Err(RuntimeError {
            message: "property descriptors must be an object".to_owned(),
        });
    }

    for (key, descriptor_value) in enumerable_property_entries(descriptors)? {
        let descriptor = to_property_descriptor(descriptor_value)?;
        define_property_on_value(target.clone(), key, descriptor)?;
    }
    Ok(target)
}

fn define_property_on_value(
    target: Value,
    key: String,
    descriptor: Property,
) -> Result<(), RuntimeError> {
    match &target {
        Value::Object(object) => {
            object.define_property(key, descriptor);
            Ok(())
        }
        Value::Function(function) => {
            function.properties.borrow_mut().insert(key, descriptor);
            Ok(())
        }
        _ => ensure_define_property_target(&target),
    }
}

fn ensure_define_property_target(target: &Value) -> Result<(), RuntimeError> {
    match target {
        Value::Object(_) | Value::Function(_) => Ok(()),
        Value::Array(_) | Value::String(_) | Value::Number(_) | Value::Boolean(_) => {
            Err(RuntimeError {
                message: "Object.defineProperty primitive targets are not implemented".to_owned(),
            })
        }
        Value::Null | Value::Undefined => Err(RuntimeError {
            message: "Object.defineProperty target must be an object".to_owned(),
        }),
    }
}

fn to_property_descriptor(value: Value) -> Result<Property, RuntimeError> {
    let Value::Object(descriptor) = value else {
        return Err(RuntimeError {
            message: "property descriptor must be an object".to_owned(),
        });
    };

    if descriptor.contains_property("get") || descriptor.contains_property("set") {
        return Err(RuntimeError {
            message: "accessor property descriptors are not implemented".to_owned(),
        });
    }

    Ok(Property {
        value: descriptor.get("value").unwrap_or(Value::Undefined),
        writable: descriptor
            .get("writable")
            .is_some_and(|value| is_truthy(&value)),
        enumerable: descriptor
            .get("enumerable")
            .is_some_and(|value| is_truthy(&value)),
        configurable: descriptor
            .get("configurable")
            .is_some_and(|value| is_truthy(&value)),
    })
}

pub(super) fn native_object_get_prototype_of(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match argument_values.first() {
        Some(Value::Object(object)) => {
            Ok(object.prototype().map(Value::Object).unwrap_or(Value::Null))
        }
        Some(Value::Array(_)) => Ok(array_prototype(env)
            .map(Value::Object)
            .unwrap_or(Value::Null)),
        Some(Value::Function(_)) => Ok(Value::Null),
        _ => Err(RuntimeError {
            message: "Object.getPrototypeOf target must be an object".to_owned(),
        }),
    }
}

pub(super) fn native_object_get_own_property_descriptor(
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let key = to_property_key(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    let Some(property) = own_property_descriptor(target, &key)? else {
        return Ok(Value::Undefined);
    };
    Ok(Value::Object(property_descriptor_object(
        property,
        object_prototype(env),
    )))
}

fn own_property_descriptor(value: Value, key: &str) -> Result<Option<Property>, RuntimeError> {
    match value {
        Value::Object(object) => Ok(object.own_property(key)),
        Value::Function(function) => Ok(function_own_property_descriptor(&function, key)),
        Value::Array(elements) => Ok(array_own_property_descriptor(&elements, key)),
        Value::String(value) => Ok(string::string_own_property_descriptor(&value, key)),
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined => Ok(None),
    }
}

fn property_descriptor_object(property: Property, prototype: Option<ObjectRef>) -> ObjectRef {
    ObjectRef::with_prototype(
        HashMap::from([
            ("value".to_owned(), property.value),
            ("writable".to_owned(), Value::Boolean(property.writable)),
            ("enumerable".to_owned(), Value::Boolean(property.enumerable)),
            (
                "configurable".to_owned(),
                Value::Boolean(property.configurable),
            ),
        ]),
        prototype,
    )
}

pub(super) fn native_object_keys(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let keys = match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Object(object) => object.own_property_keys(),
        Value::Array(elements) => array_own_property_keys(&elements),
        Value::Function(function) => function_own_property_keys(&function),
        Value::String(value) => string::string_own_property_keys(&value),
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined => Vec::new(),
    };
    Ok(Value::Array(ArrayRef::new(
        keys.into_iter().map(Value::String).collect(),
    )))
}

pub(super) fn native_object_get_own_property_names(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let names = match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Object(object) => object.own_property_names(),
        Value::Array(elements) => array_own_property_names(&elements),
        Value::Function(function) => function_own_property_names(&function),
        Value::String(value) => string::string_own_property_names(&value),
        Value::Number(_) | Value::Boolean(_) | Value::Null | Value::Undefined => Vec::new(),
    };
    Ok(Value::Array(ArrayRef::new(
        names.into_iter().map(Value::String).collect(),
    )))
}

pub(super) fn native_object_has_own(argument_values: &[Value]) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if matches!(target, Value::Null | Value::Undefined) {
        return Err(RuntimeError {
            message: "Object.hasOwn target must not be null or undefined".to_owned(),
        });
    }

    let key = to_property_key(argument_values.get(1).cloned().unwrap_or(Value::Undefined))?;
    Ok(Value::Boolean(
        own_property_descriptor(target, &key)?.is_some(),
    ))
}

pub(super) fn native_object_prototype_has_own_property(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let key = to_property_key(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    match this_value {
        Value::Object(object) => Ok(Value::Boolean(object.has_own_property(&key))),
        Value::Function(function) => Ok(Value::Boolean(
            function_own_property_descriptor(&function, &key).is_some(),
        )),
        Value::Array(elements) => Ok(Value::Boolean(array_has_own_property(&elements, &key))),
        Value::String(value) => Ok(Value::Boolean(string::string_has_own_property(
            &value, &key,
        ))),
        Value::Null | Value::Undefined => Err(RuntimeError {
            message: "hasOwnProperty called on null or undefined".to_owned(),
        }),
        Value::Number(_) | Value::Boolean(_) => Ok(Value::Boolean(false)),
    }
}

pub(super) fn native_object_prototype_property_is_enumerable(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let key = to_property_key(argument_values.first().cloned().unwrap_or(Value::Undefined))?;
    match this_value {
        Value::Null | Value::Undefined => Err(RuntimeError {
            message: "propertyIsEnumerable called on null or undefined".to_owned(),
        }),
        value => Ok(Value::Boolean(
            own_property_descriptor(value, &key)?.is_some_and(|property| property.enumerable),
        )),
    }
}

pub(super) fn native_object_prototype_is_prototype_of(
    this_value: Value,
    argument_values: &[Value],
    env: &HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let Some(target_prototype) = value_prototype(target, env) else {
        return Ok(Value::Boolean(false));
    };
    let Value::Object(prototype) = this_value else {
        return Err(RuntimeError {
            message: "isPrototypeOf called on non-object".to_owned(),
        });
    };
    Ok(Value::Boolean(
        target_prototype.ptr_eq(&prototype) || target_prototype.has_prototype(&prototype),
    ))
}

pub(super) fn native_object_prototype_to_string(this_value: Value) -> Result<Value, RuntimeError> {
    let tag = match this_value {
        Value::Undefined => "Undefined",
        Value::Null => "Null",
        Value::Array(_) => "Array",
        Value::Function(_) => "Function",
        Value::String(_) => "String",
        Value::Number(_) => "Number",
        Value::Boolean(_) => "Boolean",
        Value::Object(object) => {
            if matches!(
                object.own_property(BOOLEAN_DATA_PROPERTY),
                Some(Property {
                    value: Value::Boolean(_),
                    ..
                })
            ) {
                "Boolean"
            } else if number::is_number_object(&object) {
                "Number"
            } else {
                "Object"
            }
        }
    };
    Ok(Value::String(format!("[object {tag}]")))
}

pub(super) fn native_object_prototype_value_of(this_value: Value) -> Result<Value, RuntimeError> {
    match this_value {
        Value::Null | Value::Undefined => Err(RuntimeError {
            message: "valueOf called on null or undefined".to_owned(),
        }),
        _ => Ok(this_value),
    }
}
