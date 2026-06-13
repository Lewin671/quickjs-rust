use std::collections::HashMap;

use crate::{CallEnv, Function, NativeFunction, RuntimeError, Value};

use super::ir::Bytecode;
use super::vm::Slot;

pub(super) fn user_bytecode_function(value: &Value) -> Option<&Function> {
    let Value::Function(function) = value else {
        return None;
    };
    if let Some(bound) = &function.bound {
        return user_bytecode_function(&bound.target);
    }
    if function.native.is_none() && function.bytecode.is_some() {
        Some(function)
    } else {
        None
    }
}

pub(super) fn native_error_message(message: &str) -> (&'static str, String) {
    for name in [
        "EvalError",
        "RangeError",
        "ReferenceError",
        "SyntaxError",
        "TypeError",
        "URIError",
    ] {
        if let Some(message) = message
            .strip_prefix(name)
            .and_then(|rest| rest.strip_prefix(": "))
        {
            return (name, message.to_owned());
        }
    }
    ("TypeError", message.to_owned())
}

pub(super) fn insert_scope_call_bindings(
    env: &mut HashMap<String, Value>,
    binding_names: &mut Vec<String>,
    bytecode: &Bytecode,
    locals: &[Slot],
    function_local_names: &[String],
) {
    // Only the caller's live frame slots ride into the callee; realm bindings
    // are visible through the shared cell and copying them would give the
    // callee a frozen snapshot that masks later realm writes.
    for (index, local) in bytecode.locals.iter().enumerate() {
        if function_local_names
            .binary_search_by(|name| name.as_str().cmp(&local.name))
            .is_ok()
        {
            continue;
        }
        if let Some(Some(value)) = locals.get(index) {
            insert_binding(env, binding_names, &local.name, value);
        }
    }
}

pub(super) fn call_forwarding_native_env(
    callee: &Value,
    env: CallEnv,
) -> Option<(CallEnv, HashMap<String, Value>, Vec<String>)> {
    if !is_call_forwarding_native(callee) {
        return None;
    }
    let locals = env.locals().clone();
    let binding_names = locals.keys().cloned().collect();
    Some((env, locals, binding_names))
}

pub(super) fn try_fast_global_native_call(
    callee: &Value,
    this_value: &Value,
    arguments: &[Value],
    realm_env: &CallEnv,
) -> Option<Result<Value, RuntimeError>> {
    let Value::Function(function) = callee else {
        return None;
    };
    let native = function.native?;
    let result = match native {
        NativeFunction::DecodeUri | NativeFunction::DecodeUriComponent => {
            let source = match arguments.first().cloned().unwrap_or(Value::Undefined) {
                Value::String(source) => source,
                Value::Undefined => "undefined".to_owned(),
                _ => return None,
            };
            let result = match native {
                NativeFunction::DecodeUri => crate::global::decode_uri_string(&source),
                NativeFunction::DecodeUriComponent => {
                    crate::global::decode_uri_component_string(&source)
                }
                _ => unreachable!("URI native matched above"),
            };
            result.map(Value::String)
        }
        NativeFunction::StringFromCharCode => {
            if !arguments
                .iter()
                .all(|value| matches!(value, Value::Number(_)))
            {
                return None;
            }
            Ok(Value::String(fast_string_from_char_code_numbers(arguments)))
        }
        NativeFunction::Eval => {
            let Some(Value::String(source)) = arguments.first() else {
                return None;
            };
            match crate::global::try_eval_regexp_literal_source(source, realm_env) {
                Ok(Some(value)) => Ok(value),
                Ok(None) => return None,
                Err(error) => Err(error),
            }
        }
        NativeFunction::NumberPrototypeToString => {
            let Value::Number(number) = this_value else {
                return None;
            };
            let radix = match arguments.first() {
                None | Some(Value::Undefined) => 10,
                Some(Value::Number(radix)) if radix.fract() == 0.0 => {
                    if !(2.0..=36.0).contains(radix) {
                        return None;
                    }
                    *radix as u32
                }
                _ => return None,
            };
            crate::number::number_to_radix_string(*number, radix).map(Value::String)
        }
        NativeFunction::Test262AssertSameValue => {
            crate::global::native_test262_assert_same_value(arguments)
        }
        _ => return None,
    };
    Some(result)
}

fn is_call_forwarding_native(callee: &Value) -> bool {
    let Value::Function(function) = callee else {
        return false;
    };
    matches!(
        function.native,
        Some(
            crate::NativeFunction::FunctionPrototypeApply
                | crate::NativeFunction::FunctionPrototypeCall
                | crate::NativeFunction::ReflectApply
        )
    )
}

fn fast_string_from_char_code_numbers(arguments: &[Value]) -> String {
    let code_units: Vec<u16> = arguments
        .iter()
        .map(|value| match value {
            Value::Number(number) if number.is_finite() && *number != 0.0 => {
                number.trunc().rem_euclid(65_536.0) as u16
            }
            Value::Number(_) => 0,
            _ => unreachable!("fast path only accepts numeric arguments"),
        })
        .collect();
    crate::string::string_from_code_units(&code_units)
}

fn insert_binding(
    env: &mut HashMap<String, Value>,
    binding_names: &mut Vec<String>,
    name: &str,
    value: &Value,
) {
    if crate::function::is_internal_binding_name(name) {
        return;
    }
    env.entry(name.to_owned()).or_insert_with(|| value.clone());
    if !binding_names.iter().any(|existing| existing == name) {
        binding_names.push(name.to_owned());
    }
}
