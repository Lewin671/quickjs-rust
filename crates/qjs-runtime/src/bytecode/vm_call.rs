use std::collections::HashMap;

use crate::{
    CallEnv, Function, NativeFunction, RuntimeError, Value, call_function, to_int32_number,
};

use super::ir::Bytecode;
use super::vm::{Slot, Vm};
use super::vm_props::get_property_key;

impl Vm<'_> {
    pub(super) fn call(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        let callee = self.pop()?;
        self.call_callee(callee, Value::Undefined, arguments)
    }

    pub(super) fn call_direct_eval(
        &mut self,
        argc: usize,
        is_strict: bool,
    ) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        let callee = self.pop()?;
        self.call_callee_with_direct_eval(callee, Value::Undefined, arguments, is_strict)
    }

    fn call_callee(
        &mut self,
        callee: Value,
        this_value: Value,
        arguments: Vec<Value>,
    ) -> Result<(), RuntimeError> {
        self.call_callee_with_marker(callee, this_value, arguments, false, false)
    }

    fn call_callee_with_direct_eval(
        &mut self,
        callee: Value,
        this_value: Value,
        arguments: Vec<Value>,
        is_strict: bool,
    ) -> Result<(), RuntimeError> {
        self.call_callee_with_marker(callee, this_value, arguments, true, is_strict)
    }

    fn call_callee_with_marker(
        &mut self,
        callee: Value,
        this_value: Value,
        arguments: Vec<Value>,
        direct_eval: bool,
        direct_eval_strict: bool,
    ) -> Result<(), RuntimeError> {
        if let Some(result) =
            try_fast_global_native_call(&callee, &this_value, &arguments, &self.realm_env())
        {
            if let Some(value) = self.handle_runtime_result(result)? {
                self.stack.push(value);
            }
            return Ok(());
        }
        let mut env = self.call_env(&callee);
        if direct_eval {
            env.env
                .insert(crate::DIRECT_EVAL_BINDING.to_owned(), Value::Boolean(true));
            env.env.insert(
                crate::DIRECT_EVAL_STRICT_BINDING.to_owned(),
                Value::Boolean(direct_eval_strict),
            );
        }
        let result = call_function(callee, this_value, arguments, &mut env.env, false);
        env.env.remove(crate::DIRECT_EVAL_BINDING);
        env.env.remove(crate::DIRECT_EVAL_STRICT_BINDING);
        self.apply_call_env(env);
        if let Some(result) = self.handle_call_result(result)? {
            self.stack.push(result);
        }
        Ok(())
    }

    pub(super) fn call_spread(&mut self) -> Result<(), RuntimeError> {
        let arguments = self.pop_argument_array("function call spread")?;
        let callee = self.pop()?;
        self.call_callee(callee, Value::Undefined, arguments)
    }

    pub(super) fn call_direct_eval_spread(&mut self, is_strict: bool) -> Result<(), RuntimeError> {
        let arguments = self.pop_argument_array("direct eval spread")?;
        let callee = self.pop()?;
        self.call_callee_with_direct_eval(callee, Value::Undefined, arguments, is_strict)
    }

    pub(super) fn call_method(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        // Method resolution errors (e.g. reading a property of undefined) are
        // catchable runtime errors, not VM faults.
        let resolved = self.pop_method_callee();
        let Some((callee, this_value)) = self.handle_runtime_result(resolved)? else {
            return Ok(());
        };
        self.call_callee(callee, this_value, arguments)
    }

    pub(super) fn call_method_spread(&mut self) -> Result<(), RuntimeError> {
        let arguments = self.pop_argument_array("method call spread")?;
        let resolved = self.pop_method_callee();
        let Some((callee, this_value)) = self.handle_runtime_result(resolved)? else {
            return Ok(());
        };
        self.call_callee(callee, this_value, arguments)
    }

    /// Calls a pre-resolved callee whose receiver and callee are already on the
    /// stack as `[receiver, callee, args...]`.
    pub(super) fn call_resolved(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let arguments = self.pop_arguments(argc)?;
        let callee = self.pop()?;
        let this_value = self.pop()?;
        self.call_callee(callee, this_value, arguments)
    }

    pub(super) fn call_resolved_spread(&mut self) -> Result<(), RuntimeError> {
        let arguments = self.pop_argument_array("super method call spread")?;
        let callee = self.pop()?;
        let this_value = self.pop()?;
        self.call_callee(callee, this_value, arguments)
    }

    fn pop_method_callee(&mut self) -> Result<(Value, Value), RuntimeError> {
        let key_value = self.pop()?;
        let key = self.coerce_property_key(key_value)?;
        let this_value = self.pop()?;
        let callee = if let Some(callee) = self.try_direct_get(&this_value, &key) {
            callee
        } else {
            let mut getter_env = self.current_env();
            let callee = get_property_key(this_value.clone(), &key, &mut getter_env)?;
            self.apply_env(getter_env);
            callee
        };
        Ok((callee, this_value))
    }
}

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
        NativeFunction::ParseInt => {
            let source = match arguments.first().cloned().unwrap_or(Value::Undefined) {
                Value::String(source) => source,
                Value::Undefined => "undefined".to_owned(),
                _ => return None,
            };
            let radix = match arguments.get(1).cloned().unwrap_or(Value::Undefined) {
                Value::Undefined => 0,
                Value::Number(number) => to_int32_number(number),
                _ => return None,
            };
            Ok(Value::Number(crate::number::parse_int_string(
                &source, radix,
            )))
        }
        NativeFunction::ParseFloat => {
            let source = match arguments.first().cloned().unwrap_or(Value::Undefined) {
                Value::String(source) => source,
                Value::Undefined => "undefined".to_owned(),
                _ => return None,
            };
            Ok(Value::Number(crate::number::parse_float_string(&source)))
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
