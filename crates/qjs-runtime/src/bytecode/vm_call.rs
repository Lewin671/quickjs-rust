use std::collections::HashMap;

use crate::{
    CallEnv, Function, NativeFunction, RuntimeError, Value, call_function,
    function::{call_direct_leaf_function, is_direct_leaf_function},
    to_int32_number,
};

use super::util::stack_underflow;
use super::vm::Vm;

const DYNAMIC_FUNCTION_REALM_GLOBAL: &str = "__quickjsRustDynamicFunctionRealm";

impl Vm<'_> {
    pub(super) fn require_callable(&self) -> Result<(), RuntimeError> {
        let callee = self.stack.last().ok_or_else(stack_underflow)?;
        if matches!(callee, Value::Function(_))
            || matches!(callee, Value::Proxy(proxy) if crate::proxy::proxy_is_callable(proxy))
        {
            return Ok(());
        }
        Err(RuntimeError {
            thrown: None,
            message: "value is not callable".to_owned(),
        })
    }

    pub(super) fn call(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let direct_leaf = argc <= 1
            && self
                .stack
                .len()
                .checked_sub(argc + 1)
                .and_then(|index| self.stack.get(index))
                .is_some_and(is_direct_leaf_function);
        if direct_leaf {
            let argument = if argc == 1 { Some(self.pop()?) } else { None };
            let callee = self.pop()?;
            let arguments = argument.as_slice();
            return self.call_direct_leaf_callee(callee, Value::Undefined, arguments);
        }
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
        if matches!(&callee, Value::Function(function) if function.native.is_some()) {
            let realm_env = self.realm_env();
            if let Some(result) =
                try_fast_global_native_call(&callee, &this_value, &arguments, &realm_env)
            {
                if let Some(value) = self.handle_runtime_result(result)? {
                    self.stack.push(value);
                }
                return Ok(());
            }
        }
        // `fn.apply(this, denseArray)` for a self-contained native target reads
        // its arguments straight out of the array and needs nothing from the
        // caller frame. Take that fast path before `call_env`, whose
        // forwarding-native branch materializes and deep-clones every caller
        // local: with a large accumulating string local (the `buildString`
        // harness loop behind RegExp property-escape tests) that clone is what
        // turns repeated `String.fromCodePoint.apply` quadratic.
        if matches!(
            &callee,
            Value::Function(function)
                if function.native == Some(NativeFunction::FunctionPrototypeApply)
        ) && let Some(result) = crate::function::apply_dense_native_fast_path(
            &this_value,
            &arguments,
            &self.realm_env(),
        ) {
            if let Some(value) = self.handle_runtime_result(result)? {
                self.stack.push(value);
            }
            return Ok(());
        }
        if !direct_eval && is_direct_leaf_function(&callee) {
            return self.call_direct_leaf_callee(callee, this_value, &arguments);
        }
        let effective_direct_eval = direct_eval
            && matches!(&callee, Value::Function(function) if function.native == Some(NativeFunction::Eval));
        let in_parameter_scope = effective_direct_eval && self.in_parameter_prologue();
        let mut env = self.call_env(&callee);
        if effective_direct_eval {
            env.env
                .insert(crate::DIRECT_EVAL_BINDING.to_owned(), Value::Boolean(true));
            env.env.insert(
                crate::DIRECT_EVAL_STRICT_BINDING.to_owned(),
                Value::Boolean(direct_eval_strict),
            );
            if in_parameter_scope {
                env.env.insert(
                    crate::DIRECT_EVAL_IN_PARAMETER_SCOPE_BINDING.to_owned(),
                    Value::Boolean(true),
                );
            }
            env.env.set_direct_eval_with_stack(self.with_stack.clone());
        } else {
            env.env.remove(crate::DIRECT_EVAL_BINDING);
            env.env.remove(crate::DIRECT_EVAL_STRICT_BINDING);
            env.env
                .remove(crate::DIRECT_EVAL_IN_PARAMETER_SCOPE_BINDING);
        }
        let restore_dynamic_realm_after_call = !matches!(&callee, Value::Function(function) if function.native == Some(NativeFunction::Eval));
        let dynamic_realm_snapshot = restore_dynamic_realm_after_call
            .then(|| self.marked_dynamic_realm_snapshot())
            .flatten();
        let result = call_function(callee, this_value, arguments, &mut env.env, false);
        env.env.remove(crate::DIRECT_EVAL_BINDING);
        env.env.remove(crate::DIRECT_EVAL_STRICT_BINDING);
        env.env
            .remove(crate::DIRECT_EVAL_IN_PARAMETER_SCOPE_BINDING);
        env.env.set_direct_eval_with_stack(Vec::new());
        self.apply_call_env(env);
        if let Some(snapshot) = dynamic_realm_snapshot {
            self.restore_marked_dynamic_realm(snapshot);
        }
        if let Some(result) = self.handle_call_result(result)? {
            self.stack.push(result);
        }
        Ok(())
    }

    fn marked_dynamic_realm_snapshot(&self) -> Option<HashMap<String, Value>> {
        let Some(Value::Object(global)) = self.env.get(DYNAMIC_FUNCTION_REALM_GLOBAL) else {
            return None;
        };
        let Some(Value::Object(global_this)) = self.env.get(crate::GLOBAL_THIS_BINDING) else {
            return None;
        };
        if !global.ptr_eq(&global_this) {
            return None;
        }
        let mut snapshot = HashMap::new();
        snapshot.insert(
            DYNAMIC_FUNCTION_REALM_GLOBAL.to_owned(),
            Value::Object(global.clone()),
        );
        snapshot.insert(
            crate::GLOBAL_THIS_BINDING.to_owned(),
            Value::Object(global.clone()),
        );
        snapshot.insert("globalThis".to_owned(), Value::Object(global.clone()));
        snapshot.insert("this".to_owned(), Value::Object(global.clone()));
        for name in global.own_property_names() {
            if name.starts_with('\0') {
                continue;
            }
            if let Some(property) = global.own_property(&name) {
                snapshot.insert(name, property.value);
            }
        }
        Some(snapshot)
    }

    fn restore_marked_dynamic_realm(&mut self, snapshot: HashMap<String, Value>) {
        if let Some(Value::Object(global)) = snapshot.get(DYNAMIC_FUNCTION_REALM_GLOBAL) {
            for name in global.own_property_names() {
                if !snapshot.contains_key(&name) {
                    self.env.remove(&name);
                }
            }
        }
        for (name, value) in snapshot {
            self.env.insert(name, value);
        }
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

    /// Calls a pre-resolved callee whose receiver and callee are already on the
    /// stack as `[receiver, callee, args...]`.
    pub(super) fn call_resolved(&mut self, argc: usize) -> Result<(), RuntimeError> {
        let direct_leaf = argc <= 1
            && self
                .stack
                .len()
                .checked_sub(argc + 1)
                .and_then(|index| self.stack.get(index))
                .is_some_and(is_direct_leaf_function);
        if direct_leaf {
            let argument = if argc == 1 { Some(self.pop()?) } else { None };
            let callee = self.pop()?;
            let this_value = self.pop()?;
            let arguments = argument.as_slice();
            return self.call_direct_leaf_callee(callee, this_value, arguments);
        }
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

    fn call_direct_leaf_callee(
        &mut self,
        callee: Value,
        this_value: Value,
        arguments: &[Value],
    ) -> Result<(), RuntimeError> {
        let result = call_direct_leaf_function(
            callee,
            this_value,
            arguments,
            &self.env,
            self.module_host.clone(),
            #[cfg(feature = "agents")]
            self.agent_context.clone(),
        );
        if let Some(value) = self.handle_call_result(result)? {
            self.stack.push(value);
        }
        Ok(())
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
        NativeFunction::DecodeUri
        | NativeFunction::DecodeUriComponent
        | NativeFunction::EncodeUri
        | NativeFunction::EncodeUriComponent => {
            let source = match arguments.first().cloned().unwrap_or(Value::Undefined) {
                Value::String(source) => source,
                Value::Undefined => "undefined".to_owned().into(),
                _ => return None,
            };
            let result = match native {
                NativeFunction::DecodeUri => crate::global::decode_uri_string(&source),
                NativeFunction::DecodeUriComponent => {
                    crate::global::decode_uri_component_string(&source)
                }
                NativeFunction::EncodeUri => crate::global::encode_uri_string(&source),
                NativeFunction::EncodeUriComponent => {
                    crate::global::encode_uri_component_string(&source)
                }
                _ => unreachable!("URI native matched above"),
            };
            result.map(|s| Value::String(s.into()))
        }
        NativeFunction::StringFromCharCode => {
            let result = fast_string_from_char_code_primitives(arguments)?;
            result.map(|s| Value::String(s.into()))
        }
        NativeFunction::ParseInt => {
            let source = match arguments.first().cloned().unwrap_or(Value::Undefined) {
                Value::String(source) => source,
                Value::Undefined => "undefined".to_owned().into(),
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
                Value::Undefined => "undefined".to_owned().into(),
                _ => return None,
            };
            Ok(Value::Number(crate::number::parse_float_string(&source)))
        }
        NativeFunction::MathAbs => {
            let number = match arguments.first().cloned().unwrap_or(Value::Undefined) {
                Value::Number(number) => number,
                Value::Undefined => f64::NAN,
                _ => return None,
            };
            Ok(Value::Number(number.abs()))
        }
        NativeFunction::Eval => {
            let Some(Value::String(source)) = arguments.first() else {
                return None;
            };
            if crate::global::eval_source_is_only_comments_and_whitespace(source) {
                return Some(Ok(Value::Undefined));
            }
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
            crate::number::number_to_radix_string(*number, radix).map(|s| Value::String(s.into()))
        }
        NativeFunction::DatePrototypeGetTimezoneOffset if arguments.is_empty() => {
            crate::date::native_date_prototype_get_timezone_offset(this_value.clone())
        }
        NativeFunction::DatePrototypeSetTime => {
            if !matches!(arguments.first(), Some(Value::Number(_)))
                || arguments
                    .iter()
                    .skip(1)
                    .any(|value| !matches!(value, Value::Undefined))
            {
                return None;
            }
            let mut env = realm_env.clone();
            crate::date::native_date_prototype_set_time(this_value.clone(), arguments, &mut env)
        }
        NativeFunction::StringPrototypeSlice
        | NativeFunction::StringPrototypeSubstr
        | NativeFunction::StringPrototypeSubstring => {
            fast_string_sequence_native(native, this_value, arguments, realm_env)?
        }
        NativeFunction::Test262AssertSameValue => {
            crate::global::native_test262_assert_same_value(arguments)
        }
        _ => return None,
    };
    Some(result)
}

fn fast_string_sequence_native(
    native: NativeFunction,
    this_value: &Value,
    arguments: &[Value],
    realm_env: &CallEnv,
) -> Option<Result<Value, RuntimeError>> {
    if !matches!(this_value, Value::String(_)) {
        return None;
    }
    if !arguments
        .iter()
        .all(|value| matches!(value, Value::Number(_) | Value::Undefined))
    {
        return None;
    }
    let mut env = realm_env.clone();
    let result = match native {
        NativeFunction::StringPrototypeSlice => {
            crate::string::native_string_prototype_slice(this_value.clone(), arguments, &mut env)
        }
        NativeFunction::StringPrototypeSubstr => {
            crate::string::native_string_prototype_substr(this_value.clone(), arguments, &mut env)
        }
        NativeFunction::StringPrototypeSubstring => {
            crate::string::native_string_prototype_substring(
                this_value.clone(),
                arguments,
                &mut env,
            )
        }
        _ => unreachable!("string sequence native fast path only accepts sequence natives"),
    };
    Some(result)
}

fn fast_string_from_char_code_numbers(arguments: &[Value]) -> String {
    let mut result = String::with_capacity(arguments.len());
    for value in arguments {
        let code_unit = match value {
            Value::Number(number) if number.is_finite() && *number != 0.0 => {
                number.trunc().rem_euclid(65_536.0) as u16
            }
            Value::Number(_) => 0,
            _ => unreachable!("fast path only accepts numeric arguments"),
        };
        crate::string::push_code_unit(&mut result, code_unit);
    }
    result
}

fn fast_string_from_char_code_primitives(
    arguments: &[Value],
) -> Option<Result<String, RuntimeError>> {
    if arguments
        .iter()
        .all(|value| matches!(value, Value::Number(_)))
    {
        return Some(Ok(fast_string_from_char_code_numbers(arguments)));
    }

    let mut result = String::with_capacity(arguments.len());
    for value in arguments {
        let number = match value {
            Value::Number(number) => *number,
            Value::String(source) => match crate::conversion::string_to_number(source) {
                Ok(number) => number,
                Err(error) => return Some(Err(error)),
            },
            Value::Boolean(true) => 1.0,
            Value::Boolean(false) | Value::Null => 0.0,
            Value::Undefined => f64::NAN,
            Value::BigInt(_) => {
                return Some(Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: cannot convert BigInt to number".to_owned(),
                }));
            }
            Value::Object(_)
            | Value::Function(_)
            | Value::Array(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Proxy(_) => return None,
        };
        let code_unit = if number.is_finite() && number != 0.0 {
            number.trunc().rem_euclid(65_536.0) as u16
        } else {
            0
        };
        crate::string::push_code_unit(&mut result, code_unit);
    }
    Some(Ok(result))
}
