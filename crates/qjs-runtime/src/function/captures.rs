use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::{
    ACTIVE_CONSTRUCTOR_BINDING, Bytecode, DIRECT_EVAL_ARGUMENTS_BINDING,
    DIRECT_EVAL_FUNCTION_CONTEXT_BINDING, DIRECT_EVAL_STRICT_BINDING,
    FIELD_INITIALIZER_EVAL_BINDING, Function, GLOBAL_THIS_BINDING, HOME_OBJECT_BINDING,
    NEW_TARGET_BINDING, SUPER_CONSTRUCTOR_BINDING, Value, function::CallEnv,
};

const DYNAMIC_FUNCTION_REALM_GLOBAL: &str = "__quickjsRustDynamicFunctionRealm";

pub(super) fn sync_global_var_captures(
    function: &Function,
    bytecode: &Bytecode,
    env: &mut CallEnv,
) {
    let captured = function.captured_env.borrow();
    let Some(Value::Object(global_this)) = captured.get(GLOBAL_THIS_BINDING) else {
        return;
    };
    if captured
        .get(DYNAMIC_FUNCTION_REALM_GLOBAL)
        .is_some_and(|value| matches!(value, Value::Object(realm) if realm.ptr_eq(global_this)))
        && !matches!(
            env.get(GLOBAL_THIS_BINDING),
            Some(Value::Object(caller_global)) if caller_global.ptr_eq(global_this)
        )
    {
        return;
    }
    for (name, value) in captured.iter() {
        if function.immutable_name_binding && function.name.as_deref() == Some(name.as_str()) {
            continue;
        }
        // The captured env also holds the ~48 realm intrinsics seeded at
        // function creation. Only a binding this function or one of its nested
        // closures could have reassigned needs syncing back; skipping the rest
        // avoids deep-cloning constant intrinsic function objects (their entire
        // property map) on every leaf call — the dominant call-path cost.
        if is_call_frame_binding(name) || !global_this.has_own_property(name) {
            continue;
        }
        if !bytecode.writes_binding(name) {
            continue;
        }
        if bytecode
            .local_slot(name)
            .is_some_and(|slot| !bytecode.local_is_sloppy_global_fallback(slot))
        {
            continue;
        }
        let global_value = global_this
            .own_property(name)
            .map(|property| property.value);
        if global_value.as_ref() != Some(value) {
            continue;
        }
        if let Some(binding) = env.get_local_mut(name) {
            *binding = value.clone();
        }
        if env.realm_contains(name) {
            env.insert_realm(name.clone(), value.clone());
        }
        global_this.set(name.clone(), value.clone());
    }
}

pub(super) fn refresh_class_constructor_captures_from_caller(function: &Function, env: &CallEnv) {
    let names = function
        .captured_env
        .borrow()
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    let mut captured = function.captured_env.borrow_mut();
    for name in names {
        if let Some(value) = refreshed_class_capture_value(function, env, &name) {
            captured.insert(name, value);
        }
    }
}

fn refreshed_class_capture_value(function: &Function, env: &CallEnv, name: &str) -> Option<Value> {
    env.locals()
        .get(name)
        .cloned()
        .or_else(|| {
            function
                .capture_writeback
                .as_ref()
                .and_then(|writeback| capture_writeback_value(writeback, name))
        })
        .or_else(|| {
            env.captured_binding_source_env()
                .and_then(|source| source.borrow().get(name).cloned())
        })
}

fn capture_writeback_value(
    writeback: &crate::bytecode::CaptureWriteback,
    name: &str,
) -> Option<Value> {
    {
        let target = writeback.target.borrow();
        if writeback.names.iter().any(|candidate| candidate == name)
            && let Some(value) = target.get(name).cloned()
        {
            return Some(value);
        }
        for (source_name, target_name) in &writeback.aliases {
            if source_name == name
                && let Some(value) = target
                    .get(source_name)
                    .or_else(|| target.get(target_name))
                    .cloned()
            {
                return Some(value);
            }
            if target_name == name
                && let Some(value) = target
                    .get(target_name)
                    .or_else(|| target.get(source_name))
                    .cloned()
            {
                return Some(value);
            }
        }
    }
    writeback
        .parent
        .as_deref()
        .and_then(|parent| capture_writeback_value(parent, name))
}

pub(super) fn propagate_function_captures(
    function: &Function,
    bytecode: &Bytecode,
    function_capture_names: &[String],
    caller_env: &mut CallEnv,
    result: &crate::bytecode::FunctionBytecodeResult<'_>,
) {
    let mut written_capture_names = function_capture_names
        .iter()
        .filter(|name| bytecode.writes_binding(name))
        .cloned()
        .collect::<Vec<_>>();
    for name in bytecode.written_binding_names() {
        if !written_capture_names
            .iter()
            .any(|existing| existing == &name)
        {
            written_capture_names.push(name);
        }
    }
    write_function_capture_values(
        &function.captured_env,
        function_capture_names,
        &[],
        result,
        Some(bytecode),
    );
    if let Some(writeback) = &function.capture_writeback {
        write_function_capture_values(
            &writeback.target,
            &writeback.names,
            &writeback.aliases,
            result,
            None,
        );
        write_parent_capture_values(writeback.parent.as_deref(), result);
    }
    for name in &written_capture_names {
        if is_call_frame_binding(name) {
            continue;
        }
        if function.immutable_name_binding && function.name.as_deref() == Some(name.as_str()) {
            continue;
        }
        let Some(final_value) = result.frame_binding(name).or_else(|| result.binding(name)) else {
            continue;
        };
        let writeback_targets_caller =
            function
                .capture_writeback
                .as_ref()
                .is_some_and(|writeback| {
                    capture_writeback_targets_caller(writeback, caller_env)
                        && capture_writeback_contains_name(writeback, name)
                });
        let writes_global_capture = bytecode
            .local_slot(name)
            .is_none_or(|slot| bytecode.local_is_sloppy_global_fallback(slot))
            && (captured_global_this_has_own_property(&function.captured_env, name)
                || caller_global_this_has_own_property(caller_env, name))
            && captured_value_matches_global_this(&function.captured_env, caller_env, name);
        let parent_writeback_targets_caller = function
            .capture_writeback
            .as_ref()
            .and_then(|writeback| writeback.parent.as_deref())
            .is_some_and(|parent| {
                capture_writeback_targets_caller(parent, caller_env)
                    && capture_writeback_contains_name(parent, name)
            });
        let parent_writeback_reaches_activation_name = function
            .capture_writeback
            .as_ref()
            .and_then(|writeback| writeback.parent.as_deref())
            .is_some_and(|parent| capture_writeback_contains_name(parent, name))
            && caller_env
                .activation_captured_env()
                .is_some_and(|source| source.borrow().contains_key(name));
        if (writeback_targets_caller
            || parent_writeback_targets_caller
            || parent_writeback_reaches_activation_name
            || writes_global_capture)
            && let Some(binding) = caller_env.get_local_mut(name)
        {
            *binding = final_value.clone();
        } else if (writeback_targets_caller
            || parent_writeback_targets_caller
            || parent_writeback_reaches_activation_name
            || writes_global_capture)
            && caller_env.realm_contains(name)
        {
            caller_env.insert_realm(name.clone(), final_value.clone());
        } else if writeback_targets_caller {
            caller_env.insert(name.clone(), final_value.clone());
        }
        if (writeback_targets_caller
            || parent_writeback_targets_caller
            || parent_writeback_reaches_activation_name
            || writes_global_capture)
            && let Some(source) = caller_env.captured_binding_source_env()
            && source.borrow().contains_key(name)
        {
            source.borrow_mut().insert(name.clone(), final_value);
        }
    }
}

fn capture_writeback_targets_caller(
    writeback: &crate::bytecode::CaptureWriteback,
    caller_env: &CallEnv,
) -> bool {
    caller_env
        .activation_captured_env()
        .is_some_and(|activation| Rc::ptr_eq(activation, &writeback.target))
        || caller_env
            .captured_binding_source_env()
            .is_some_and(|source| Rc::ptr_eq(source, &writeback.target))
        || writeback
            .parent
            .as_deref()
            .is_some_and(|parent| capture_writeback_targets_caller(parent, caller_env))
}

fn capture_writeback_contains_name(
    writeback: &crate::bytecode::CaptureWriteback,
    name: &str,
) -> bool {
    writeback.names.iter().any(|candidate| candidate == name)
        || writeback
            .aliases
            .iter()
            .any(|(source, target)| source == name || target == name)
        || writeback
            .parent
            .as_deref()
            .is_some_and(|parent| capture_writeback_contains_name(parent, name))
}

pub(super) fn captured_global_this_has_own_property(
    captured_env: &Rc<RefCell<HashMap<String, Value>>>,
    name: &str,
) -> bool {
    matches!(
        captured_env.borrow().get(GLOBAL_THIS_BINDING),
        Some(Value::Object(global)) if global.has_own_property(name)
    )
}

pub(super) fn caller_global_this_has_own_property(caller_env: &CallEnv, name: &str) -> bool {
    matches!(
        caller_env.get(GLOBAL_THIS_BINDING),
        Some(Value::Object(global)) if global.has_own_property(name)
    )
}

pub(super) fn caller_capture_matches_existing(
    local_env: &HashMap<String, Value>,
    env: &CallEnv,
    name: &str,
    caller_shares_capture_source: bool,
    allow_caller_local_match: bool,
    callee: &Value,
) -> bool {
    !env.locals()
        .get(name)
        .is_some_and(|value| value == callee && local_env.get(name) != Some(value))
        && (caller_shares_capture_source
            || !local_env.contains_key(name)
            || (allow_caller_local_match && env.locals().contains_key(name))
            || env
                .get(name)
                .is_some_and(|value| local_env.get(name) == Some(&value)))
}

fn captured_value_matches_global_this(
    captured_env: &Rc<RefCell<HashMap<String, Value>>>,
    caller_env: &CallEnv,
    name: &str,
) -> bool {
    let captured_value = captured_env.borrow().get(name).cloned();
    let global_value = match caller_env.get(GLOBAL_THIS_BINDING) {
        Some(Value::Object(global)) => global.own_property(name).map(|property| property.value),
        _ => None,
    };
    captured_value.is_some() && captured_value == global_value
}

fn write_function_capture_values(
    target: &Rc<RefCell<HashMap<String, Value>>>,
    names: &[String],
    aliases: &[(String, String)],
    result: &crate::bytecode::FunctionBytecodeResult<'_>,
    bytecode: Option<&Bytecode>,
) {
    if names.is_empty() && aliases.is_empty() {
        return;
    }
    let realm_global = target
        .borrow()
        .get(DYNAMIC_FUNCTION_REALM_GLOBAL)
        .and_then(|value| match value {
            Value::Object(object) => Some(object.clone()),
            _ => None,
        });
    let mut captured_env = target.borrow_mut();
    for name in names {
        if !is_call_frame_binding(name)
            && let Some(final_value) = result.frame_binding(name).or_else(|| result.binding(name))
        {
            if bytecode.is_some_and(|bytecode| !bytecode.writes_binding(name))
                && captured_env
                    .get(name)
                    .is_some_and(|existing| existing != &final_value)
            {
                continue;
            }
            captured_env.insert(name.clone(), final_value.clone());
            if bytecode.is_some_and(|bytecode| bytecode.sloppy_global_fallback_binding(name))
                && let Some(global) = &realm_global
                && global.has_own_property(name)
            {
                global.set(name.clone(), final_value);
            }
        }
    }
    for (source_name, target_name) in aliases {
        if !is_call_frame_binding(target_name)
            && let Some(final_value) = result
                .frame_binding(source_name)
                .or_else(|| result.binding(source_name))
        {
            captured_env.insert(target_name.clone(), final_value.clone());
        }
    }
}

fn write_parent_capture_values(
    writeback: Option<&crate::bytecode::CaptureWriteback>,
    result: &crate::bytecode::FunctionBytecodeResult<'_>,
) {
    let Some(writeback) = writeback else {
        return;
    };
    write_function_capture_values(
        &writeback.target,
        &writeback.names,
        &writeback.aliases,
        result,
        None,
    );
    write_parent_capture_values(writeback.parent.as_deref(), result);
}

pub(super) fn is_call_frame_binding(name: &str) -> bool {
    matches!(
        name,
        GLOBAL_THIS_BINDING
            | DIRECT_EVAL_STRICT_BINDING
            | DIRECT_EVAL_ARGUMENTS_BINDING
            | DIRECT_EVAL_FUNCTION_CONTEXT_BINDING
            | FIELD_INITIALIZER_EVAL_BINDING
            | HOME_OBJECT_BINDING
            | NEW_TARGET_BINDING
            | SUPER_CONSTRUCTOR_BINDING
            | ACTIVE_CONSTRUCTOR_BINDING
            | "this"
            | "arguments"
    )
}
