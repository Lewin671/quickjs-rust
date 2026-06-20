use std::collections::HashMap;

use crate::CallEnv;
use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, function_prototype,
    to_js_string_with_env,
};

const SYMBOL_DATA_PROPERTY: &str = "\0SymbolData";
const SYMBOL_DESCRIPTION_PROPERTY: &str = "\0SymbolDescription";
const SYMBOL_BOXED_PROPERTY: &str = "\0SymbolBoxed";
const SYMBOL_WRAPPED_PROPERTY: &str = "\0SymbolWrapped";
pub(crate) const SYMBOL_REGISTRY_BINDING: &str = "\0SymbolRegistry";
const WELL_KNOWN_SYMBOL_NAMES: &[&str] = &[
    "asyncDispose",
    "asyncIterator",
    "dispose",
    "hasInstance",
    "isConcatSpreadable",
    "iterator",
    "match",
    "matchAll",
    "replace",
    "search",
    "species",
    "split",
    "toPrimitive",
    "toStringTag",
    "unscopables",
];

pub(crate) fn install_symbol(env: &mut CallEnv, global_this: &Value, object_prototype: ObjectRef) {
    let symbol_prototype = ObjectRef::with_prototype(HashMap::new(), Some(object_prototype));
    symbol_prototype.set_to_string_tag("Symbol");

    let symbol_function = Function::new_native(Some("Symbol"), 0, NativeFunction::Symbol, true);
    install_well_known_symbols(&symbol_function, &symbol_prototype);
    install_function_has_instance(env, &symbol_function);
    if let Some(to_string_tag) = well_known_symbol_from_function(&symbol_function, "toStringTag") {
        define_to_string_tag_property(&symbol_prototype, to_string_tag, "Symbol");
    }
    if let Some(to_primitive) = well_known_symbol_from_function(&symbol_function, "toPrimitive") {
        symbol_prototype.define_symbol_property(
            to_primitive,
            Property::data(
                Value::Function(Function::new_native(
                    Some("[Symbol.toPrimitive]"),
                    1,
                    NativeFunction::SymbolPrototypeToPrimitive,
                    false,
                )),
                false,
                false,
                true,
            ),
        );
    }
    symbol_function.properties.borrow_mut().insert(
        "for".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("for"),
            1,
            NativeFunction::SymbolFor,
            false,
        ))),
    );
    symbol_function.properties.borrow_mut().insert(
        "keyFor".to_owned(),
        Property::non_enumerable(Value::Function(Function::new_native(
            Some("keyFor"),
            1,
            NativeFunction::SymbolKeyFor,
            false,
        ))),
    );
    symbol_prototype.define_non_enumerable(
        "constructor".to_owned(),
        Value::Function(symbol_function.clone()),
    );
    symbol_prototype.define_non_enumerable(
        "toString".to_owned(),
        Value::Function(Function::new_native(
            Some("toString"),
            0,
            NativeFunction::SymbolPrototypeToString,
            false,
        )),
    );
    symbol_prototype.define_non_enumerable(
        "valueOf".to_owned(),
        Value::Function(Function::new_native(
            Some("valueOf"),
            0,
            NativeFunction::SymbolPrototypeValueOf,
            false,
        )),
    );
    symbol_prototype.define_property(
        "description".to_owned(),
        Property::accessor(
            Some(Value::Function(Function::new_native(
                Some("get description"),
                0,
                NativeFunction::SymbolPrototypeDescription,
                false,
            ))),
            None,
            false,
            true,
        ),
    );
    symbol_function.properties.borrow_mut().insert(
        "prototype".to_owned(),
        Property::fixed_non_enumerable(Value::Object(symbol_prototype)),
    );

    let symbol_value = Value::Function(symbol_function);
    env.insert_realm("Symbol".to_owned(), symbol_value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable("Symbol".to_owned(), symbol_value);
    }
}

fn install_well_known_symbols(symbol_function: &Function, symbol_prototype: &ObjectRef) {
    let mut properties = symbol_function.properties.borrow_mut();
    for name in WELL_KNOWN_SYMBOL_NAMES {
        let symbol = Value::Object(symbol_object(
            Some(symbol_prototype.clone()),
            Value::String(format!("Symbol.{name}").into()),
        ));
        properties.insert(
            name.to_string(),
            Property::data(symbol, false, false, false),
        );
    }
}

pub(crate) fn native_symbol(
    function: &Function,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if is_construct {
        return Err(RuntimeError {
            message: "TypeError: Symbol is not a constructor".to_owned(),
            thrown: None,
        });
    }

    let description = match argument_values.first().cloned().unwrap_or(Value::Undefined) {
        Value::Undefined => Value::Undefined,
        value => Value::String(to_js_string_with_env(value, env)?.into()),
    };
    Ok(Value::Object(symbol_object(
        function_prototype(function),
        description,
    )))
}

pub(crate) fn native_symbol_for(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let key = to_js_string_with_env(
        argument_values.first().cloned().unwrap_or(Value::Undefined),
        env,
    )?;
    let registry = symbol_registry(env);
    if let Some(property) = registry.own_property(&key) {
        return Ok(property.value);
    }

    let symbol = Value::Object(symbol_object(
        symbol_prototype(env),
        Value::String(key.clone().into()),
    ));
    registry.define_non_enumerable(key, symbol.clone());
    Ok(symbol)
}

pub(crate) fn native_symbol_key_for(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let target = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let Value::Object(target_object) = target else {
        return Err(symbol_method_error());
    };
    if !is_symbol_primitive(&target_object) {
        return Err(symbol_method_error());
    }

    let registry = symbol_registry(env);
    for key in registry.own_property_names() {
        if matches!(
            registry.own_property(&key).map(|property| property.value),
            Some(Value::Object(symbol)) if symbol.ptr_eq(&target_object)
        ) {
            return Ok(Value::String(key.into()));
        }
    }
    Ok(Value::Undefined)
}

pub(crate) fn is_symbol_object(object: &ObjectRef) -> bool {
    object.own_property(SYMBOL_DATA_PROPERTY).is_some()
}

pub(crate) fn is_symbol_primitive(object: &ObjectRef) -> bool {
    is_symbol_object(object) && object.own_property(SYMBOL_BOXED_PROPERTY).is_none()
}

pub(crate) fn is_registered_symbol(object: &ObjectRef, env: &CallEnv) -> bool {
    let Some(Value::Object(registry)) = env.get(SYMBOL_REGISTRY_BINDING) else {
        return false;
    };
    registry.own_property_names().into_iter().any(|key| {
        matches!(
            registry.own_property(&key).map(|property| property.value),
            Some(Value::Object(symbol)) if symbol.ptr_eq(object)
        )
    })
}

pub(crate) fn boxed_symbol(object: &ObjectRef, env: &CallEnv) -> Value {
    let description = symbol_description(object);
    let boxed = symbol_object(symbol_prototype(env), description);
    boxed.define_non_enumerable(SYMBOL_BOXED_PROPERTY.to_owned(), Value::Boolean(true));
    boxed.define_non_enumerable(
        SYMBOL_WRAPPED_PROPERTY.to_owned(),
        Value::Object(object.clone()),
    );
    Value::Object(boxed)
}

pub(crate) fn symbol_descriptive_string(object: &ObjectRef) -> String {
    symbol_description_string(symbol_description(object))
}

pub(crate) fn symbol_function_name_description(object: &ObjectRef) -> Option<String> {
    match symbol_description(object) {
        Value::String(description) => Some(description.to_string()),
        _ => None,
    }
}

pub(crate) fn to_string_tag_symbol(env: &CallEnv) -> Option<ObjectRef> {
    well_known_symbol(env, "toStringTag")
}

pub(crate) fn iterator_symbol(env: &CallEnv) -> Option<ObjectRef> {
    well_known_symbol(env, "iterator")
}

pub(crate) fn async_iterator_symbol(env: &CallEnv) -> Option<ObjectRef> {
    well_known_symbol(env, "asyncIterator")
}

pub(crate) fn dispose_symbol(env: &CallEnv) -> Option<ObjectRef> {
    well_known_symbol(env, "dispose")
}

pub(crate) fn async_dispose_symbol(env: &CallEnv) -> Option<ObjectRef> {
    well_known_symbol(env, "asyncDispose")
}

pub(crate) fn to_primitive_symbol(env: &CallEnv) -> Option<ObjectRef> {
    well_known_symbol(env, "toPrimitive")
}

pub(crate) fn has_instance_symbol(env: &CallEnv) -> Option<ObjectRef> {
    well_known_symbol(env, "hasInstance")
}

pub(crate) fn is_concat_spreadable_symbol(env: &CallEnv) -> Option<ObjectRef> {
    well_known_symbol(env, "isConcatSpreadable")
}

pub(crate) fn match_symbol(env: &CallEnv) -> Option<ObjectRef> {
    well_known_symbol(env, "match")
}

pub(crate) fn match_all_symbol(env: &CallEnv) -> Option<ObjectRef> {
    well_known_symbol(env, "matchAll")
}

pub(crate) fn replace_symbol(env: &CallEnv) -> Option<ObjectRef> {
    well_known_symbol(env, "replace")
}

pub(crate) fn search_symbol(env: &CallEnv) -> Option<ObjectRef> {
    well_known_symbol(env, "search")
}

pub(crate) fn species_symbol(env: &CallEnv) -> Option<ObjectRef> {
    well_known_symbol(env, "species")
}

pub(crate) fn split_symbol(env: &CallEnv) -> Option<ObjectRef> {
    well_known_symbol(env, "split")
}

pub(crate) fn unscopables_symbol(env: &CallEnv) -> Option<ObjectRef> {
    well_known_symbol(env, "unscopables")
}

fn well_known_symbol(env: &CallEnv, name: &str) -> Option<ObjectRef> {
    let Some(Value::Function(symbol_function)) = env.get("Symbol") else {
        return None;
    };
    well_known_symbol_from_function(&symbol_function, name)
}

fn well_known_symbol_from_function(symbol_function: &Function, name: &str) -> Option<ObjectRef> {
    match symbol_function.properties.borrow().get(name) {
        Some(Property {
            value: Value::Object(symbol),
            ..
        }) => Some(symbol.clone()),
        _ => None,
    }
}

fn install_function_has_instance(env: &CallEnv, symbol_function: &Function) {
    let Some(symbol) = well_known_symbol_from_function(symbol_function, "hasInstance") else {
        return;
    };
    let Some(prototype) = crate::function_intrinsic_prototype_slot(env) else {
        return;
    };
    let property = Property::data(
        Value::Function(Function::new_native(
            Some("[Symbol.hasInstance]"),
            1,
            NativeFunction::FunctionPrototypeHasInstance,
            false,
        )),
        false,
        false,
        false,
    );
    match prototype {
        crate::Prototype::Object(prototype) => prototype.define_symbol_property(symbol, property),
        crate::Prototype::Function(prototype) => {
            prototype.define_symbol_property(symbol, property);
        }
        crate::Prototype::Proxy(_) => {}
    }
}

pub(crate) fn define_well_known_iterator_alias(
    env: &CallEnv,
    object: &ObjectRef,
    method_name: &str,
) {
    let Some(symbol) = iterator_symbol(env) else {
        return;
    };
    let Some(property) = object.own_property(method_name) else {
        return;
    };
    object.define_symbol_property(symbol, Property::non_enumerable(property.value));
}

pub(crate) fn define_species_accessor(env: &CallEnv, function: &Function) {
    let Some(symbol) = species_symbol(env) else {
        return;
    };
    function.define_symbol_property(
        symbol,
        Property::accessor(
            Some(Value::Function(Function::new_native(
                Some("get [Symbol.species]"),
                0,
                NativeFunction::SpeciesGetter,
                false,
            ))),
            None,
            false,
            true,
        ),
    );
}

pub(crate) fn define_well_known_to_string_tag(env: &CallEnv, object: &ObjectRef, tag: &str) {
    if let Some(symbol) = to_string_tag_symbol(env) {
        define_to_string_tag_property(object, symbol, tag);
    }
}

fn define_to_string_tag_property(object: &ObjectRef, symbol: ObjectRef, tag: &str) {
    object.define_symbol_property(
        symbol,
        Property::data(Value::String(tag.to_owned().into()), false, false, true),
    );
}

pub(crate) fn native_symbol_prototype_description(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    this_symbol_description(this_value)
}

pub(crate) fn native_symbol_prototype_to_primitive(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let Value::Object(object) = this_value else {
        return Err(symbol_method_error());
    };
    symbol_primitive_value(&object).ok_or_else(symbol_method_error)
}

pub(crate) fn native_symbol_prototype_to_string(this_value: Value) -> Result<Value, RuntimeError> {
    Ok(Value::String(
        symbol_description_string(this_symbol_description(this_value)?).into(),
    ))
}

pub(crate) fn native_symbol_prototype_value_of(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Object(object) = this_value else {
        return Err(symbol_method_error());
    };
    symbol_primitive_value(&object).ok_or_else(symbol_method_error)
}

fn this_symbol_description(this_value: Value) -> Result<Value, RuntimeError> {
    let Value::Object(object) = this_value else {
        return Err(symbol_method_error());
    };
    if !is_symbol_object(&object) {
        return Err(symbol_method_error());
    }
    Ok(symbol_description(&object))
}

fn symbol_description(object: &ObjectRef) -> Value {
    object
        .own_property(SYMBOL_DESCRIPTION_PROPERTY)
        .map(|property| property.value)
        .unwrap_or(Value::Undefined)
}

fn symbol_primitive_value(object: &ObjectRef) -> Option<Value> {
    if !is_symbol_object(object) {
        return None;
    }
    if is_symbol_primitive(object) {
        return Some(Value::Object(object.clone()));
    }
    match object.own_property(SYMBOL_WRAPPED_PROPERTY) {
        Some(Property {
            value: Value::Object(symbol),
            ..
        }) if is_symbol_primitive(&symbol) => Some(Value::Object(symbol)),
        _ => None,
    }
}

fn symbol_description_string(description: Value) -> String {
    match description {
        Value::String(description) => format!("Symbol({description})"),
        _ => "Symbol()".to_owned(),
    }
}

fn symbol_method_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "Symbol.prototype method called on non-symbol".to_owned(),
    }
}

fn symbol_object(prototype: Option<ObjectRef>, description: Value) -> ObjectRef {
    let object = ObjectRef::with_prototype(HashMap::new(), prototype);
    object.define_non_enumerable(SYMBOL_DATA_PROPERTY.to_owned(), Value::Boolean(true));
    object.define_non_enumerable(SYMBOL_DESCRIPTION_PROPERTY.to_owned(), description);
    object
}

fn symbol_prototype(env: &CallEnv) -> Option<ObjectRef> {
    let Some(Value::Function(symbol_function)) = env.get("Symbol") else {
        return None;
    };
    function_prototype(&symbol_function)
}

fn symbol_registry(env: &mut CallEnv) -> ObjectRef {
    if let Some(Value::Object(registry)) = env.get(SYMBOL_REGISTRY_BINDING) {
        return registry.clone();
    }
    let registry = ObjectRef::new(HashMap::new());
    env.insert_realm(
        SYMBOL_REGISTRY_BINDING.to_owned(),
        Value::Object(registry.clone()),
    );
    registry
}
