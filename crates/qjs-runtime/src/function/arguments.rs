use std::collections::HashMap;

use qjs_ast::BindingPattern;

use crate::{
    Function, NativeFunction, ObjectRef, Property, RuntimeError, Value, object_prototype, symbol,
};

use super::{CallEnv, Upvalue};

pub(super) fn arguments_object(
    function: &Function,
    argument_values: &[Value],
    env: &CallEnv,
) -> Value {
    let object = ObjectRef::with_prototype(HashMap::new(), object_prototype(env));
    object.define_property(
        "length".to_owned(),
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
                .unwrap_or_else(|| Upvalue::new(value.clone()));
            object.define_property(index.to_string(), mapped_argument_property(parameter));
        } else {
            object.define_property(index.to_string(), Property::enumerable(value));
        }
    }
    if function.is_strict || !function.params.is_simple() {
        define_restricted_callee(&object, function, env);
    } else {
        // A sloppy-mode simple-parameter function's `arguments.callee` is a
        // data property holding the executing function (CreateUnmappedArguments
        // / CreateMappedArguments): `{value: F, writable, enumerable: false,
        // configurable}`.
        object.define_property(
            "callee".to_owned(),
            Property::non_enumerable(Value::Function(function.clone())),
        );
    }
    define_arguments_iterator(&object, env);
    object.set_to_string_tag("Arguments");
    Value::Object(object)
}

fn mapped_argument_parameter(function: &Function, index: usize) -> Option<&str> {
    if function.is_strict || !function.params.is_simple() {
        return None;
    }
    let element = function.params.positional.get(index)?;
    let BindingPattern::Identifier {
        name: parameter_name,
        ..
    } = &element.binding
    else {
        return None;
    };
    if parameter_name.is_empty() {
        return None;
    }
    if function
        .params
        .positional
        .iter()
        .skip(index + 1)
        .any(|element| {
            matches!(
                &element.binding,
                BindingPattern::Identifier { name, .. } if name == parameter_name
            )
        })
    {
        None
    } else {
        Some(parameter_name)
    }
}

fn mapped_argument_property(parameter: Upvalue) -> Property {
    Property::accessor(
        Some(mapped_argument_getter(parameter.clone())),
        Some(mapped_argument_setter(parameter)),
        true,
        true,
    )
}

fn mapped_argument_getter(parameter: Upvalue) -> Value {
    let mut getter = Function::new_native(
        Some("[[MappedArgumentGet]]"),
        0,
        NativeFunction::MappedArgumentGet,
        false,
    );
    getter.set_single_upvalue(parameter);
    Value::Function(getter)
}

fn mapped_argument_setter(parameter: Upvalue) -> Value {
    let mut setter = Function::new_native(
        Some("[[MappedArgumentSet]]"),
        1,
        NativeFunction::MappedArgumentSet,
        false,
    );
    setter.set_single_upvalue(parameter);
    Value::Function(setter)
}

fn define_restricted_callee(object: &ObjectRef, function: &Function, env: &CallEnv) {
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
                ))
            })
    });
    object.define_property(
        "callee".to_owned(),
        Property::accessor(
            Some(throw_type_error.clone()),
            Some(throw_type_error),
            false,
            false,
        ),
    );
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
    ));
    let throw_type_error = Function::new_bound(
        target,
        Value::Undefined,
        vec![Value::Object(prototype.clone())],
        0,
    );
    throw_type_error.freeze();
    let value = Value::Function(throw_type_error);
    prototype.define_property(
        super::CROSS_REALM_THROW_TYPE_ERROR_INTRINSIC.to_owned(),
        Property::fixed_non_enumerable(value.clone()),
    );
    Some(value)
}

fn define_arguments_iterator(object: &ObjectRef, env: &CallEnv) {
    let Some(iterator) = symbol::iterator_symbol(env) else {
        return;
    };
    // Reuse the shared %Array.prototype.values% so `arguments[Symbol.iterator]`
    // has the same identity as `Array.prototype.values`, falling back to a fresh
    // native if the realm intrinsic is somehow absent.
    let values = env
        .get_realm(super::ARRAY_PROTO_VALUES_INTRINSIC)
        .unwrap_or_else(|| {
            Value::Function(Function::new_native(
                Some("[Symbol.iterator]"),
                0,
                NativeFunction::ArrayPrototypeValues,
                false,
            ))
        });
    object.define_symbol_property(iterator, Property::non_enumerable(values));
}

pub(crate) fn native_mapped_argument_get(function: &Function) -> Result<Value, RuntimeError> {
    Ok(function
        .upvalues
        .first()
        .map_or(Value::Undefined, Upvalue::get))
}

pub(crate) fn native_mapped_argument_set(
    function: &Function,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    if let Some(parameter) = function.upvalues.first() {
        parameter.set(argument_values.first().cloned().unwrap_or(Value::Undefined));
    }
    Ok(Value::Undefined)
}
