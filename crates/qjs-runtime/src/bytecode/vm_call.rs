use std::collections::HashMap;

use crate::{Function, Value};

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
