//! Call opcodes: plain, resolved and spread calls, and direct `eval`.
//!
//! A call takes one of three routes: a guarded native fast path
//! (`try_fast_global_native_call`), the direct-leaf path
//! (`function::call_direct_leaf_function`), or the general
//! `function::call_function` with a borrowed Context from `vm_context.rs`.
//! The direct-leaf path hands the callee no caller bindings, so only a
//! callee passing `is_direct_leaf_function` may take it.

#[cfg(test)]
use std::cell::Cell;

use crate::{
    NativeFunction, RuntimeError, Value,
    function::{call_direct_leaf_function, is_direct_leaf_function},
    to_int32_number,
};

use super::util::stack_underflow;
use super::vm::Vm;

#[cfg(test)]
thread_local! {
    static GUARDED_MATH_UNARY_HITS: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
pub(super) fn reset_guarded_math_unary_hits() {
    GUARDED_MATH_UNARY_HITS.with(|hits| hits.set(0));
}

#[cfg(test)]
pub(super) fn guarded_math_unary_hits() -> usize {
    GUARDED_MATH_UNARY_HITS.with(Cell::get)
}

impl Vm<'_, '_> {
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
        let fixed_multi_argument_direct_leaf = argc <= 3
            && self
                .stack
                .len()
                .checked_sub(argc + 1)
                .and_then(|index| self.stack.get(index))
                .is_some_and(is_direct_leaf_function);
        if fixed_multi_argument_direct_leaf {
            return self.call_fixed_multi_argument_direct_leaf(argc);
        }
        self.call_stack_arguments(argc, false, false, false)
    }

    fn call_stack_arguments(
        &mut self,
        argc: usize,
        resolved: bool,
        direct_eval: bool,
        direct_eval_strict: bool,
    ) -> Result<(), RuntimeError> {
        let prefix = usize::from(resolved) + 1;
        let base = self
            .stack
            .len()
            .checked_sub(argc + prefix)
            .ok_or_else(stack_underflow)?;
        let mut block = self.stack.take();
        let callee_index = base + usize::from(resolved);
        let callee = std::mem::replace(&mut block[callee_index], Value::Undefined);
        let this_value = if resolved {
            std::mem::replace(&mut block[base], Value::Undefined)
        } else {
            Value::Undefined
        };
        let result = self.invoke_callee(
            callee,
            this_value,
            &block[callee_index + 1..],
            direct_eval,
            direct_eval_strict,
        );
        block.truncate(base);
        // Restore the owner without recycling the empty take() placeholder.
        *self.stack = block;
        self.finish_callee(result)
    }

    // Keep the fixed-array setup out of the main opcode dispatch body. The
    // zero/one-argument path is hotter across the broad workload, and inlining
    // all three shapes makes that unrelated path measurably slower.
    #[inline(never)]
    fn call_fixed_multi_argument_direct_leaf(&mut self, argc: usize) -> Result<(), RuntimeError> {
        match argc {
            2 => {
                let second = self.pop()?;
                let first = self.pop()?;
                let callee = self.pop()?;
                self.call_direct_leaf_callee(callee, Value::Undefined, &[first, second])
            }
            3 => {
                let third = self.pop()?;
                let second = self.pop()?;
                let first = self.pop()?;
                let callee = self.pop()?;
                self.call_direct_leaf_callee(callee, Value::Undefined, &[first, second, third])
            }
            _ => unreachable!("fixed multi-argument calls contain two or three values"),
        }
    }

    pub(super) fn call_direct_eval(
        &mut self,
        argc: usize,
        is_strict: bool,
    ) -> Result<(), RuntimeError> {
        self.call_stack_arguments(argc, false, true, is_strict)
    }

    fn call_callee(
        &mut self,
        callee: Value,
        this_value: Value,
        arguments: Vec<Value>,
    ) -> Result<(), RuntimeError> {
        self.call_callee_with_eval_site(callee, this_value, arguments, false, false)
    }

    fn call_callee_with_direct_eval(
        &mut self,
        callee: Value,
        this_value: Value,
        arguments: Vec<Value>,
        is_strict: bool,
    ) -> Result<(), RuntimeError> {
        self.call_callee_with_eval_site(callee, this_value, arguments, true, is_strict)
    }

    fn call_callee_with_eval_site(
        &mut self,
        callee: Value,
        this_value: Value,
        arguments: Vec<Value>,
        direct_eval: bool,
        direct_eval_strict: bool,
    ) -> Result<(), RuntimeError> {
        let result = self.invoke_callee(
            callee,
            this_value,
            &arguments,
            direct_eval,
            direct_eval_strict,
        );
        self.finish_callee(result)
    }

    fn finish_callee(&mut self, result: Result<Value, RuntimeError>) -> Result<(), RuntimeError> {
        if let Some(value) = self.handle_call_result(result)? {
            self.stack.push(value);
        }
        Ok(())
    }

    fn invoke_callee(
        &mut self,
        callee: Value,
        this_value: Value,
        arguments: &[Value],
        direct_eval: bool,
        direct_eval_strict: bool,
    ) -> Result<Value, RuntimeError> {
        if matches!(&callee, Value::Function(function) if function.native.is_some()) {
            // Most fast native paths never touch an environment, so the realm
            // frame is built only by the arms that need one instead of once
            // per native call.
            if let Some(result) =
                try_fast_global_native_call(&callee, &this_value, arguments, &mut self.context())
            {
                return result;
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
        ) && let Some(result) =
            crate::function::apply_dense_native_fast_path(&this_value, arguments, &self.context())
        {
            return result;
        }
        if !direct_eval && is_direct_leaf_function(&callee) {
            return call_direct_leaf_function(callee, this_value, arguments, &mut self.context());
        }
        let effective_direct_eval = direct_eval
            && matches!(&callee, Value::Function(function) if function.native == Some(NativeFunction::Eval)
                && std::rc::Rc::ptr_eq(function.creation_realm(), self.env.realm()));
        if effective_direct_eval {
            let scope = self.prepare_scope_cursor();
            let private = self.current_private_environment();
            let site = crate::context::DirectEvalSite {
                strict: direct_eval_strict,
                in_parameters: self.in_parameter_prologue(),
                scope: scope.as_ref(),
                private: private.as_ref(),
            };
            let Value::Function(function) = &callee else {
                unreachable!("direct eval function");
            };
            return crate::native::call_native_function(
                function.native_entry.expect("eval entry"),
                crate::native::CallInfo {
                    callee: function,
                    this_value: &this_value,
                    is_construct: false,
                },
                arguments,
                None,
                Some(&site),
                &mut self.context(),
            );
        }
        crate::function::call_function_slice(&callee, this_value, arguments, &mut self.context())
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
        let fixed_multi_argument_direct_leaf = argc <= 3
            && self
                .stack
                .len()
                .checked_sub(argc + 1)
                .and_then(|index| self.stack.get(index))
                .is_some_and(is_direct_leaf_function);
        if fixed_multi_argument_direct_leaf {
            return self.call_resolved_fixed_multi_argument_direct_leaf(argc);
        }
        self.call_stack_arguments(argc, true, false, false)
    }

    /// Executes a compiler-marked unary-Math call without materializing the
    /// generic argument vector or realm call environment. The receiver,
    /// callee, and argument have already been evaluated through the ordinary
    /// member-call bytecode. Declines leave that exact stack untouched before
    /// replaying `CallResolved(1)`.
    pub(super) fn call_resolved_guarded_math_unary(&mut self) -> Result<(), RuntimeError> {
        let Some(receiver_index) = self.stack.len().checked_sub(3) else {
            return Err(stack_underflow());
        };
        let Some(Value::Function(function)) = self.stack.get(receiver_index + 1) else {
            return self.call_resolved(1);
        };
        let Some(native) = function.native_kind() else {
            return self.call_resolved(1);
        };
        let Some(Value::Number(argument)) = self.stack.get(receiver_index + 2) else {
            return self.call_resolved(1);
        };
        let Some(result) = super::vm_numeric_leaf::math_unary(native, *argument) else {
            return self.call_resolved(1);
        };

        let argument = self.pop()?;
        let callee = self.pop()?;
        let receiver = self.pop()?;
        drop(argument);
        drop(callee);
        drop(receiver);
        self.stack.push(Value::Number(result));
        #[cfg(test)]
        GUARDED_MATH_UNARY_HITS.with(|hits| hits.set(hits.get() + 1));
        Ok(())
    }

    // Method calls keep their receiver below the callee, so they need their
    // own fixed-argument pop sequence. Keep it out of the dispatch body for
    // the same reason as ordinary multi-argument direct calls: zero- and
    // one-argument calls are hotter, and the extra code otherwise perturbs
    // their generated path.
    #[inline(never)]
    fn call_resolved_fixed_multi_argument_direct_leaf(
        &mut self,
        argc: usize,
    ) -> Result<(), RuntimeError> {
        match argc {
            2 => {
                let second = self.pop()?;
                let first = self.pop()?;
                let callee = self.pop()?;
                let this_value = self.pop()?;
                self.call_direct_leaf_callee(callee, this_value, &[first, second])
            }
            3 => {
                let third = self.pop()?;
                let second = self.pop()?;
                let first = self.pop()?;
                let callee = self.pop()?;
                let this_value = self.pop()?;
                self.call_direct_leaf_callee(callee, this_value, &[first, second, third])
            }
            _ => unreachable!("fixed multi-argument resolved calls contain two or three values"),
        }
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
        let result = call_direct_leaf_function(callee, this_value, arguments, &mut self.context());
        if let Some(value) = self.handle_call_result(result)? {
            self.stack.push(value);
        }
        Ok(())
    }
}

pub(super) fn try_fast_global_native_call(
    callee: &Value,
    this_value: &Value,
    arguments: &[Value],
    context: &mut crate::context::Context<'_>,
) -> Option<Result<Value, RuntimeError>> {
    let Value::Function(function) = callee else {
        return None;
    };
    let native = function.native?;
    let info = crate::native::CallInfo {
        callee: function,
        this_value,
        is_construct: false,
    };
    let mut site = context.with_native(&info, None, None);
    let context = &mut site;
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
            if let [Value::Number(number)] = arguments {
                // ToUint16 of a finite number, sharing the cached ASCII
                // one-code-unit strings a string-building loop produces.
                let code_unit = crate::to_uint32_number(*number) as u16;
                return Some(Ok(Value::String(crate::string::js_string_from_code_unit(
                    code_unit,
                ))));
            }
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
        NativeFunction::MathAbs
        | NativeFunction::MathAcos
        | NativeFunction::MathAcosh
        | NativeFunction::MathAsin
        | NativeFunction::MathAsinh
        | NativeFunction::MathAtan
        | NativeFunction::MathAtanh
        | NativeFunction::MathCbrt
        | NativeFunction::MathCeil
        | NativeFunction::MathCos
        | NativeFunction::MathCosh
        | NativeFunction::MathExp
        | NativeFunction::MathExpm1
        | NativeFunction::MathFloor
        | NativeFunction::MathLog
        | NativeFunction::MathLog1p
        | NativeFunction::MathLog10
        | NativeFunction::MathLog2
        | NativeFunction::MathSin
        | NativeFunction::MathSinh
        | NativeFunction::MathSqrt
        | NativeFunction::MathTan
        | NativeFunction::MathTanh
        | NativeFunction::MathTrunc => {
            Ok(Value::Number(fast_primitive_unary_math(native, arguments)?))
        }
        NativeFunction::MathPow => Ok(Value::Number(fast_primitive_math_pow(arguments)?)),
        NativeFunction::MathRandom if arguments.is_empty() => crate::math::native_math_random(),
        NativeFunction::ArrayPrototypeIndexOf => Ok(crate::array::fast_dense_array_index_of(
            this_value, arguments, context,
        )?),
        NativeFunction::Eval => {
            let Some(Value::String(source)) = arguments.first() else {
                return None;
            };
            if crate::global::eval_source_is_only_comments_and_whitespace(source) {
                return Some(Ok(Value::Undefined));
            }
            match crate::global::try_eval_regexp_literal_source(source, context) {
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
        NativeFunction::DatePrototypeGetTime => {
            crate::date::native_date_prototype_get_time(this_value.clone())
        }
        NativeFunction::DatePrototypeGetTimezoneOffset => {
            crate::date::native_date_prototype_get_timezone_offset(this_value.clone())
        }
        NativeFunction::DatePrototypeGetYear => {
            crate::date::native_date_prototype_get_year(this_value.clone())
        }
        NativeFunction::DatePrototypeGetUtcDate => {
            crate::date::native_date_prototype_get_utc_date(this_value.clone())
        }
        NativeFunction::DatePrototypeGetUtcDay => {
            crate::date::native_date_prototype_get_utc_day(this_value.clone())
        }
        NativeFunction::DatePrototypeGetUtcFullYear => {
            crate::date::native_date_prototype_get_utc_full_year(this_value.clone())
        }
        NativeFunction::DatePrototypeGetUtcHours => {
            crate::date::native_date_prototype_get_utc_hours(this_value.clone())
        }
        NativeFunction::DatePrototypeGetUtcMilliseconds => {
            crate::date::native_date_prototype_get_utc_milliseconds(this_value.clone())
        }
        NativeFunction::DatePrototypeGetUtcMinutes => {
            crate::date::native_date_prototype_get_utc_minutes(this_value.clone())
        }
        NativeFunction::DatePrototypeGetUtcMonth => {
            crate::date::native_date_prototype_get_utc_month(this_value.clone())
        }
        NativeFunction::DatePrototypeGetUtcSeconds => {
            crate::date::native_date_prototype_get_utc_seconds(this_value.clone())
        }
        NativeFunction::DatePrototypeValueOf => {
            crate::date::native_date_prototype_value_of(this_value.clone())
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
            let env = &mut *context;
            crate::date::native_date_prototype_set_time(this_value.clone(), arguments, env)
        }
        NativeFunction::StringPrototypeSlice
        | NativeFunction::StringPrototypeSubstr
        | NativeFunction::StringPrototypeSubstring => {
            fast_string_sequence_native(native, this_value, arguments, context)?
        }
        NativeFunction::StringPrototypeCharAt => {
            let Value::String(text) = this_value else {
                return None;
            };
            // ToIntegerOrInfinity on a number needs no environment: NaN and
            // `undefined` select position 0, and any position outside the
            // string answers the empty string.
            let position = match arguments.first() {
                None | Some(Value::Undefined) => 0.0,
                Some(Value::Number(number)) if number.is_nan() => 0.0,
                Some(Value::Number(number)) => number.trunc(),
                _ => return None,
            };
            let code_unit = if position < 0.0 || position >= usize::MAX as f64 {
                None
            } else {
                crate::string::js_string_code_unit_at(text, position as usize)
            };
            Ok(Value::String(match code_unit {
                Some(code_unit) => crate::string::js_string_from_code_unit(code_unit),
                None => crate::JsString::default(),
            }))
        }
        NativeFunction::StringPrototypeCharCodeAt => {
            Ok(fast_primitive_string_char_code_at(this_value, arguments)?)
        }
        NativeFunction::StringPrototypeConcat => {
            let Value::String(_) = this_value else {
                return None;
            };
            if !arguments
                .iter()
                .all(|value| matches!(value, Value::String(_) | Value::Number(_)))
            {
                return None;
            }
            let env = &mut *context;
            crate::string::native_string_prototype_concat(this_value.clone(), arguments, env)
        }
        NativeFunction::RegExpPrototypeTest => {
            if !matches!(this_value, Value::Object(_))
                || !matches!(arguments.first(), Some(Value::String(_)))
            {
                return None;
            }
            let env = &mut *context;
            crate::regexp::native_regexp_prototype_test(this_value.clone(), arguments, env)
        }
        NativeFunction::Test262AssertSameValue => {
            crate::global::native_test262_assert_same_value(arguments)
        }
        _ => return None,
    };
    Some(result.map_err(|error| crate::error::capture_error_origin(error, context)))
}

fn fast_primitive_unary_math(native: NativeFunction, arguments: &[Value]) -> Option<f64> {
    let number = match arguments.first() {
        Some(Value::Number(number)) => *number,
        None | Some(Value::Undefined) => f64::NAN,
        _ => return None,
    };
    let result = match native {
        NativeFunction::MathAbs => number.abs(),
        NativeFunction::MathAcos => number.acos(),
        NativeFunction::MathAcosh => number.acosh(),
        NativeFunction::MathAsin => number.asin(),
        NativeFunction::MathAsinh => number.asinh(),
        NativeFunction::MathAtan => number.atan(),
        NativeFunction::MathAtanh => number.atanh(),
        NativeFunction::MathCbrt => number.cbrt(),
        NativeFunction::MathCeil => number.ceil(),
        NativeFunction::MathCos => number.cos(),
        NativeFunction::MathCosh => number.cosh(),
        NativeFunction::MathExp => number.exp(),
        NativeFunction::MathExpm1 => number.exp_m1(),
        NativeFunction::MathFloor => number.floor(),
        NativeFunction::MathLog => number.ln(),
        NativeFunction::MathLog1p => number.ln_1p(),
        NativeFunction::MathLog10 => number.log10(),
        NativeFunction::MathLog2 => number.log2(),
        NativeFunction::MathSin => number.sin(),
        NativeFunction::MathSinh => number.sinh(),
        NativeFunction::MathSqrt => number.sqrt(),
        NativeFunction::MathTan => number.tan(),
        NativeFunction::MathTanh => number.tanh(),
        NativeFunction::MathTrunc => number.trunc(),
        _ => return None,
    };
    Some(result)
}

fn fast_primitive_math_pow(arguments: &[Value]) -> Option<f64> {
    let primitive_number = |value: Option<&Value>| match value {
        Some(Value::Number(number)) => Some(*number),
        None | Some(Value::Undefined) => Some(f64::NAN),
        _ => None,
    };
    let base = primitive_number(arguments.first())?;
    let exponent = primitive_number(arguments.get(1))?;
    Some(crate::operations::number_exponentiate(base, exponent))
}

fn fast_primitive_string_char_code_at(this_value: &Value, arguments: &[Value]) -> Option<Value> {
    let Value::String(value) = this_value else {
        return None;
    };
    let number = match arguments.first() {
        Some(Value::Number(number)) => *number,
        None | Some(Value::Undefined) => 0.0,
        _ => return None,
    };
    let position = if number.is_nan() { 0.0 } else { number.trunc() };
    let code_unit = if position < 0.0 || !position.is_finite() {
        None
    } else {
        crate::string::js_string_code_unit_at(value, position as usize)
    };
    Some(Value::Number(code_unit.map_or(f64::NAN, f64::from)))
}

fn fast_string_sequence_native(
    native: NativeFunction,
    this_value: &Value,
    arguments: &[Value],
    context: &mut crate::context::Context<'_>,
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
    let env = &mut *context;
    let result = match native {
        NativeFunction::StringPrototypeSlice => {
            crate::string::native_string_prototype_slice(this_value.clone(), arguments, env)
        }
        NativeFunction::StringPrototypeSubstr => {
            crate::string::native_string_prototype_substr(this_value.clone(), arguments, env)
        }
        NativeFunction::StringPrototypeSubstring => {
            crate::string::native_string_prototype_substring(this_value.clone(), arguments, env)
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

#[cfg(test)]
mod tests {
    use super::super::{compiler::compile_script, eval_bytecode};

    #[test]
    fn native_argument_borrows_preserve_reusable_operand_capacity() {
        let script = qjs_parser::parse_script(&"Math.max(1, 2);".repeat(40)).unwrap();
        let code = compile_script(&script).unwrap();
        for _ in 0..2 {
            assert_eq!(eval_bytecode(&code).unwrap(), crate::Value::Number(2.0));
            let recycler = code.operand_stack_recycler();
            let buffer = recycler.take();
            assert!(
                buffer.capacity() > 0,
                "native return discarded the caller's reusable allocation"
            );
            recycler.recycle(buffer);
        }
    }
}
