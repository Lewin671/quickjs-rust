//! Per-call arguments values/accessors with realm-owned shared descriptor schemas.
//! Parameter cells, poison callee and iterator identity retain their existing paths.
use crate::{
    Function, NativeFunction, Property, Prototype, RuntimeError, Value, object_prototype, symbol,
};

use super::{BindingCell, CallEnv};

const CROSS_REALM_FUNCTION_PROTOTYPE: &str = "__quickjsRustRealmFunctionPrototype";

pub(super) fn arguments_object(
    function: &Function,
    argument_values: &[Value],
    env: &CallEnv,
) -> Value {
    let prototype = object_prototype(env).map(Prototype::Object);
    let iterator = symbol::iterator_symbol(env);
    let width = argument_values.len() + 2 + usize::from(iterator.is_some());
    let prediction = env
        .realm()
        .predict_arguments_layout(width, prototype.as_ref());
    let mut sequence =
        crate::value::layout::DataSequence::new(prediction, env.realm().atoms(), prototype);
    sequence.insert_property(
        "length",
        Property::data(
            Value::Number(argument_values.len() as f64),
            false,
            true,
            true,
        ),
    );
    for (index, value) in argument_values.iter().cloned().enumerate() {
        if let Some(parameter_name) = mapped_argument_parameter(function, index) {
            let parameter = env
                .frame_binding_cell(parameter_name)
                .unwrap_or_else(|| BindingCell::new(value.clone()));
            sequence.insert_property(
                &index.to_string(),
                mapped_argument_property(env.realm(), parameter, env.realm().atoms()),
            );
        } else {
            sequence.insert(&index.to_string(), value);
        }
    }
    if function.is_strict || !function.params.is_simple() {
        sequence.insert_property("callee", restricted_callee_property(function, env));
    } else {
        // A sloppy-mode simple-parameter function's `arguments.callee` is a
        // data property holding the executing function (CreateUnmappedArguments
        // / CreateMappedArguments): `{value: F, writable, enumerable: false,
        // configurable}`.
        sequence.insert_property(
            "callee",
            Property::non_enumerable(Value::Function(function.clone())),
        );
    }
    if let Some(iterator) = iterator {
        sequence.insert_symbol(&iterator, arguments_iterator_property(function, env));
    }
    let object = crate::value::OrderedDataPropertyBuilder::finish_unobserved(sequence);
    object.set_to_string_tag("Arguments");
    env.realm().retain_arguments_layout(&object);
    Value::Object(object)
}

fn mapped_argument_parameter(function: &Function, index: usize) -> Option<&str> {
    if function.is_strict || !function.params.is_simple() {
        return None;
    }
    function.bytecode.as_ref()?.mapped_argument_parameter(index)
}

fn mapped_argument_property(
    realm: &crate::function::Realm,
    parameter: BindingCell,
    domain: &crate::property::atom::AtomDomain,
) -> Property {
    Property::accessor(
        Some(mapped_argument_getter(realm, parameter.clone(), domain)),
        Some(mapped_argument_setter(realm, parameter, domain)),
        true,
        true,
    )
}

fn mapped_argument_getter(
    realm: &crate::function::Realm,
    parameter: BindingCell,
    _domain: &crate::property::atom::AtomDomain,
) -> Value {
    let mut getter = Function::new_native(
        Some("[[MappedArgumentGet]]"),
        0,
        NativeFunction::MappedArgumentGet,
        false,
        realm,
    );
    getter.push_upvalue(parameter);
    Value::Function(getter)
}

fn mapped_argument_setter(
    realm: &crate::function::Realm,
    parameter: BindingCell,
    _domain: &crate::property::atom::AtomDomain,
) -> Value {
    let mut setter = Function::new_native(
        Some("[[MappedArgumentSet]]"),
        1,
        NativeFunction::MappedArgumentSet,
        false,
        realm,
    );
    setter.push_upvalue(parameter);
    Value::Function(setter)
}

fn restricted_callee_property(function: &Function, env: &CallEnv) -> Property {
    // Reuse the realm's shared %ThrowTypeError% so the strict `callee` poison
    // getter is the same object as `Function.prototype.arguments`/`caller`'s.
    let throw_type_error = cross_realm_throw_type_error(function).unwrap_or_else(|| {
        env.get_realm(super::THROW_TYPE_ERROR_INTRINSIC)
            .unwrap_or_else(|| {
                Value::Function(Function::new_native(
                    Some("ThrowTypeError"),
                    0,
                    NativeFunction::ThrowTypeError,
                    false,
                    env.realm(),
                ))
            })
    });
    Property::accessor(
        Some(throw_type_error.clone()),
        Some(throw_type_error),
        false,
        false,
    )
}

fn cross_realm_throw_type_error(function: &Function) -> Option<Value> {
    let crate::Property {
        value: Value::Object(prototype),
        ..
    } = function.own_property(super::CROSS_REALM_TYPE_ERROR_PROTOTYPE)?
    else {
        return None;
    };
    if let Some(crate::Property {
        value: Value::Function(throw_type_error),
        ..
    }) = prototype.own_property(super::CROSS_REALM_THROW_TYPE_ERROR_INTRINSIC)
    {
        return Some(Value::Function(throw_type_error));
    }

    let target = Value::Function(Function::new_native(
        Some(""),
        0,
        NativeFunction::RealmThrowTypeError,
        false,
        function.creation_realm(),
    ));
    let throw_type_error = Function::new_bound(
        target,
        Value::Undefined,
        vec![Value::Object(prototype.clone())],
        0,
        function.property_domain(),
    );
    if let Some(property) = function.own_property(CROSS_REALM_FUNCTION_PROTOTYPE) {
        if let Some(function_prototype) = cross_realm_function_prototype(&property.value) {
            // The host's synthetic Realm functions execute on the caller's
            // native Realm. Preserve the creation Realm's %Function.prototype%
            // on the synthesized %ThrowTypeError% instead of letting its
            // implicit prototype resolve through that caller Realm.
            let _ = throw_type_error.set_internal_prototype_slot(Some(function_prototype));
        }
    }
    throw_type_error.freeze();
    let value = Value::Function(throw_type_error);
    prototype.define_property(
        super::CROSS_REALM_THROW_TYPE_ERROR_INTRINSIC.to_owned(),
        Property::fixed_non_enumerable(value.clone()),
    );
    Some(value)
}

fn cross_realm_function_prototype(value: &Value) -> Option<Prototype> {
    let Value::Function(prototype) = value else {
        return None;
    };
    Some(Prototype::Function(prototype.clone()))
}

fn arguments_iterator_property(function: &Function, env: &CallEnv) -> Property {
    // Reuse the shared %Array.prototype.values% so `arguments[Symbol.iterator]`
    // has the same identity as `Array.prototype.values`, falling back to a fresh
    // native if the realm intrinsic is somehow absent.
    let values = cross_realm_array_proto_values(function)
        .or_else(|| {
            function
                .realm()
                .and_then(|realm| realm.get_value(super::ARRAY_PROTO_VALUES_INTRINSIC))
        })
        .or_else(|| env.get_realm(super::ARRAY_PROTO_VALUES_INTRINSIC))
        .unwrap_or_else(|| {
            Value::Function(Function::new_native(
                Some("[Symbol.iterator]"),
                0,
                NativeFunction::ArrayPrototypeValues,
                false,
                env.realm(),
            ))
        });
    Property::non_enumerable(values)
}

fn cross_realm_array_proto_values(function: &Function) -> Option<Value> {
    if !function.has_dynamic_function_realm && !function.has_dynamic_function_realm_override.get() {
        return None;
    }
    match function
        .own_property(super::CROSS_REALM_ARRAY_PROTO_VALUES_INTRINSIC)?
        .value
    {
        Value::Function(values) => Some(Value::Function(values)),
        _ => None,
    }
}

pub(crate) fn native_mapped_argument_get(function: &Function) -> Result<Value, RuntimeError> {
    Ok(function
        .upvalues
        .first()
        .map_or(Value::Undefined, BindingCell::get))
}

pub(crate) fn native_mapped_argument_set(
    function: &Function,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    if let Some(parameter) = function.upvalues.first() {
        parameter.assign(
            argument_values.first().cloned().unwrap_or(Value::Undefined),
            false,
        )?;
    }
    Ok(Value::Undefined)
}
