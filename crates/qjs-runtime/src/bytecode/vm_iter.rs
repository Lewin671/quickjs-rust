//! Iterator-protocol and destructuring helper ops.

use std::collections::HashMap;

use crate::{
    ArrayRef, ObjectRef, PropertyKey, RuntimeError, Value, call_function, is_truthy, object,
    object_prototype, property_value, property_value_key, symbol,
};

use super::util::is_object_value;
use super::vm::Vm;
use super::vm_result::ResumeMode;
use crate::CallEnv;

/// The outcome of one pass through the `yield*` delegation op.
pub(super) enum DelegateStep {
    /// The outer generator must suspend, yielding the inner result object
    /// unwrapped. The `Op::YieldDelegate` ip has been rewound so the resume
    /// re-enters the same op.
    Suspend(Value),
    /// The delegation produced a return completion (an outer `return(v)` was
    /// forwarded into an inner iterator with no `return`, or the inner
    /// `return` reported done): the outer body's enclosing `finally` blocks
    /// have already run and this value is the body's return value.
    Return(Value),
    /// The delegation finished normally: the `yield*` expression value is on
    /// the stack and execution continues past the op.
    Continue,
}

impl Vm<'_> {
    pub(super) fn get_iterator(&mut self) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let mut env = self.current_env();
        let result = iterator_for_value(value, &mut env);
        self.apply_env(env);
        if let Some(iterator) = self.handle_runtime_result(result)? {
            self.stack.push(iterator);
        }
        Ok(())
    }

    pub(super) fn get_async_iterator(&mut self) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let mut env = self.current_env();
        let result = crate::async_generator::get_async_iterator(value, &mut env);
        self.apply_env(env);
        if let Some(iterator) = self.handle_runtime_result(result)? {
            self.stack.push(iterator);
        }
        Ok(())
    }

    /// Processes the awaited result of an async iterator `next()` (`for await`):
    /// the result object is on top of the stack. Validates it is an object,
    /// records `done`, and replaces it with the `value`.
    pub(super) fn async_iterator_complete(&mut self, done_slot: usize) -> Result<(), RuntimeError> {
        let result = self.pop()?;
        self.store_local(done_slot, Value::Boolean(true))?;
        if !is_object_value(&result) {
            let error: Result<(), RuntimeError> = Err(RuntimeError {
                thrown: None,
                message: "TypeError: iterator result is not an object".to_owned(),
            });
            self.handle_runtime_result(error)?;
            return Ok(());
        }
        let mut env = self.current_env();
        let done = property_value(result.clone(), "done", &mut env).map(|value| is_truthy(&value));
        self.apply_env(env);
        let Some(done) = self.handle_runtime_result(done)? else {
            return Ok(());
        };
        self.store_local(done_slot, Value::Boolean(done))?;
        if done {
            self.stack.push(Value::Undefined);
            return Ok(());
        }
        let mut env = self.current_env();
        let value = property_value(result, "value", &mut env);
        self.apply_env(env);
        if let Some(value) = self.handle_runtime_result(value)? {
            self.stack.push(value);
        }
        Ok(())
    }

    pub(super) fn iterator_step(&mut self, done_slot: usize) -> Result<(), RuntimeError> {
        let next = self.pop()?;
        let iterator = self.pop()?;
        // Pessimistically mark the iterator done: errors raised by the step
        // itself must not trigger a close on the abrupt path.
        self.store_local(done_slot, Value::Boolean(true))?;
        let mut env = self.current_env();
        let result = iterator_step_value(&iterator, &next, &mut env);
        self.apply_env(env);
        match self.handle_runtime_result(result)? {
            Some(Some(value)) => {
                self.store_local(done_slot, Value::Boolean(false))?;
                self.stack.push(value);
            }
            Some(None) => self.stack.push(Value::Undefined),
            None => {}
        }
        Ok(())
    }

    pub(super) fn iterator_rest(&mut self, done_slot: usize) -> Result<(), RuntimeError> {
        let next = self.pop()?;
        let iterator = self.pop()?;
        if matches!(self.load_local(done_slot)?, Value::Boolean(true)) {
            self.stack.push(Value::Array(ArrayRef::new(Vec::new())));
            return Ok(());
        }
        self.store_local(done_slot, Value::Boolean(true))?;
        let mut env = self.current_env();
        let result = iterator_rest_values(&iterator, &next, &mut env);
        self.apply_env(env);
        if let Some(values) = self.handle_runtime_result(result)? {
            self.stack.push(Value::Array(ArrayRef::new(values)));
        }
        Ok(())
    }

    pub(super) fn iterator_close(&mut self, swallow: bool) -> Result<(), RuntimeError> {
        let iterator = self.pop()?;
        let mut env = self.current_env();
        let result = close_iterator(&iterator, &mut env);
        self.apply_env(env);
        if swallow {
            return Ok(());
        }
        self.handle_runtime_result(result)?;
        Ok(())
    }

    pub(super) fn object_rest_excluding(
        &mut self,
        excluded: &[String],
    ) -> Result<(), RuntimeError> {
        let value = self.pop()?;
        let mut env = self.current_env();
        let result = object::enumerable_property_entries(value, &mut env);
        self.apply_env(env);
        let Some(entries) = self.handle_runtime_result(result)? else {
            return Ok(());
        };
        let rest = ObjectRef::with_prototype(HashMap::new(), object_prototype(&self.env));
        for (key, value) in entries {
            if !excluded.iter().any(|name| name == &key) {
                rest.set(key, value);
            }
        }
        self.stack.push(Value::Object(rest));
        Ok(())
    }

    /// Runs one pass of the `yield*` delegation algorithm (ES2023 14.4.14).
    ///
    /// On first entry the iterable is on top of the stack: it is resolved to an
    /// iterator and its `next` method, both stored in the op's slots so they
    /// survive a suspension. Each subsequent entry is a resume carrying a
    /// `next`/`return`/`throw` in [`Vm::resume_mode`], which is forwarded to the
    /// inner iterator. The loop runs until the inner iterator is done (returning
    /// [`DelegateStep::Continue`] with the result value on the stack), the
    /// delegation becomes a return completion ([`DelegateStep::Return`]), an
    /// error is routed into the body's try/finally (returns `Continue` after the
    /// ip is redirected), or the inner iterator yields again
    /// ([`DelegateStep::Suspend`]).
    pub(super) fn yield_delegate(
        &mut self,
        iterator_slot: usize,
        next_slot: usize,
    ) -> Result<DelegateStep, RuntimeError> {
        // Resolve the inner iterator on first entry; on a resume the iterator
        // and its `next` method are restored from the slots and `resume_mode`
        // carries the forwarded completion.
        let mode = match self.resume_mode.take() {
            Some(mode) => mode,
            None => {
                let iterable = self.pop()?;
                let mut env = self.current_env();
                let resolved = resolve_delegate_iterator(iterable, &mut env);
                self.apply_env(env);
                let Some((iterator, next)) = self.handle_runtime_result(resolved)? else {
                    // The GetIterator failure was routed into a try/finally.
                    return Ok(DelegateStep::Continue);
                };
                self.store_local(iterator_slot, iterator)?;
                self.store_local(next_slot, next)?;
                ResumeMode::Next(Value::Undefined)
            }
        };
        let iterator = self.load_local(iterator_slot)?;

        // Each pass forwards one completion to the inner iterator. Throw and
        // return have their own close/complete handling; `next` validates the
        // result and either continues, suspends, or routes an error.
        match mode {
            ResumeMode::Throw(value) => self.delegate_throw(&iterator, value),
            ResumeMode::Return(value) => self.delegate_return(&iterator, value),
            ResumeMode::Next(value) => {
                let next = self.load_local(next_slot)?;
                let outcome = self.call_inner(&next, &iterator, value);
                match self.classify_inner(outcome)? {
                    Some(InnerStep::Suspend(result)) => Ok(self.suspend_delegate(result)),
                    Some(InnerStep::Done(value)) => {
                        self.stack.push(value);
                        Ok(DelegateStep::Continue)
                    }
                    // The call's error was routed into the body's try/finally.
                    None => Ok(DelegateStep::Continue),
                }
            }
        }
    }

    /// Forwards an outer `throw(v)` into the inner iterator's `throw` method, or
    /// closes the inner iterator and throws a TypeError when it has none.
    fn delegate_throw(
        &mut self,
        iterator: &Value,
        value: Value,
    ) -> Result<DelegateStep, RuntimeError> {
        let mut env = self.current_env();
        let method = get_iterator_method(iterator, "throw", &mut env);
        self.apply_env(env);
        let Some(method) = self.handle_runtime_result(method)? else {
            return Ok(DelegateStep::Continue);
        };
        if matches!(method, Value::Undefined | Value::Null) {
            // No inner `throw`: close the inner iterator (its return is called,
            // errors swallowed) and raise a TypeError at the `yield*` site.
            let mut env = self.current_env();
            let _ = close_iterator(iterator, &mut env);
            self.apply_env(env);
            let result: Result<(), RuntimeError> = Err(RuntimeError {
                thrown: None,
                message: "TypeError: the iterator does not provide a 'throw' method".to_owned(),
            });
            self.handle_runtime_result(result)?;
            return Ok(DelegateStep::Continue);
        }
        let outcome = self.call_inner(&method, iterator, value);
        match self.classify_inner(outcome)? {
            Some(InnerStep::Suspend(result)) => Ok(self.suspend_delegate(result)),
            Some(InnerStep::Done(value)) => {
                self.stack.push(value);
                Ok(DelegateStep::Continue)
            }
            None => Ok(DelegateStep::Continue),
        }
    }

    /// Forwards an outer `return(v)` into the inner iterator's `return` method,
    /// or completes the outer generator with a return completion when it has
    /// none (running the body's enclosing `finally` blocks).
    fn delegate_return(
        &mut self,
        iterator: &Value,
        value: Value,
    ) -> Result<DelegateStep, RuntimeError> {
        let mut env = self.current_env();
        let method = get_iterator_method(iterator, "return", &mut env);
        self.apply_env(env);
        let Some(method) = self.handle_runtime_result(method)? else {
            return Ok(DelegateStep::Continue);
        };
        if matches!(method, Value::Undefined | Value::Null) {
            // No inner `return`: the `yield*` is itself a return completion.
            return self.complete_delegate_return(value);
        }
        let outcome = self.call_inner(&method, iterator, value);
        match self.classify_inner(outcome)? {
            Some(InnerStep::Suspend(result)) => Ok(self.suspend_delegate(result)),
            // A done inner `return` makes the `yield*` a return completion
            // carrying the inner result's value.
            Some(InnerStep::Done(value)) => self.complete_delegate_return(value),
            None => Ok(DelegateStep::Continue),
        }
    }

    /// Turns a delegating return completion into either a body return (when no
    /// enclosing `finally` intervenes) or a redirected ip into a `finally`.
    fn complete_delegate_return(&mut self, value: Value) -> Result<DelegateStep, RuntimeError> {
        match self.return_value(value)? {
            Some(returned) => Ok(DelegateStep::Return(returned)),
            // `return_value` redirected the ip into an enclosing `finally`.
            None => Ok(DelegateStep::Continue),
        }
    }

    /// Calls an inner iterator method, returning the (validated-later) result.
    fn call_inner(
        &mut self,
        method: &Value,
        iterator: &Value,
        argument: Value,
    ) -> Result<Value, RuntimeError> {
        let mut env = self.current_env();
        let result = call_function(
            method.clone(),
            iterator.clone(),
            vec![argument],
            &mut env,
            false,
        );
        self.apply_env(env);
        result
    }

    /// Validates an inner iterator result, classifying it as done or not, and
    /// routes a failing call into the body's try/finally (returning `None`).
    fn classify_inner(
        &mut self,
        outcome: Result<Value, RuntimeError>,
    ) -> Result<Option<InnerStep>, RuntimeError> {
        let validated = outcome.and_then(|result| {
            if is_object_value(&result) {
                Ok(result)
            } else {
                Err(RuntimeError {
                    thrown: None,
                    message: "TypeError: iterator result is not an object".to_owned(),
                })
            }
        });
        let Some(result) = self.handle_runtime_result(validated)? else {
            return Ok(None);
        };
        let mut env = self.current_env();
        let done = property_value(result.clone(), "done", &mut env).map(|v| is_truthy(&v));
        self.apply_env(env);
        let Some(done) = self.handle_runtime_result(done)? else {
            return Ok(None);
        };
        if done {
            let mut env = self.current_env();
            let value = property_value(result, "value", &mut env);
            self.apply_env(env);
            let Some(value) = self.handle_runtime_result(value)? else {
                return Ok(None);
            };
            Ok(Some(InnerStep::Done(value)))
        } else {
            Ok(Some(InnerStep::Suspend(result)))
        }
    }

    /// Rewinds the ip to the `Op::YieldDelegate` so the resume re-enters it,
    /// then yields the inner result object unwrapped.
    fn suspend_delegate(&mut self, result: Value) -> DelegateStep {
        self.ip -= 1;
        DelegateStep::Suspend(result)
    }

    pub(super) fn require_object_coercible(&mut self) -> Result<(), RuntimeError> {
        if matches!(self.stack.last(), Some(Value::Undefined | Value::Null)) {
            let result: Result<(), RuntimeError> = Err(RuntimeError {
                thrown: None,
                message: "TypeError: cannot destructure undefined or null".to_owned(),
            });
            self.handle_runtime_result(result)?;
        }
        Ok(())
    }
}

/// One inner-iterator step, classified by its `done` flag.
enum InnerStep {
    /// `{ done: false }`: suspend the outer generator yielding this result.
    Suspend(Value),
    /// `{ done: true, value }`: the inner iterator finished with `value`.
    Done(Value),
}

/// Resolves `yield*`'s operand to an inner iterator and its `next` method,
/// mirroring GetIterator(value, sync) plus the `next` lookup that ES2023
/// 14.4.14 performs once up front.
fn resolve_delegate_iterator(
    value: Value,
    env: &mut CallEnv,
) -> Result<(Value, Value), RuntimeError> {
    let iterator = iterator_for_value(value, env)?;
    let next = property_value(iterator.clone(), "next", env)?;
    Ok((iterator, next))
}

/// Reads a named method off an iterator, normalizing `undefined`/`null` (no
/// such method) and erroring when present but not callable.
fn get_iterator_method(
    iterator: &Value,
    name: &str,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let method = property_value(iterator.clone(), name, env)?;
    if matches!(method, Value::Undefined | Value::Null) {
        return Ok(Value::Undefined);
    }
    if !matches!(method, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: format!("TypeError: iterator '{name}' is not a function"),
        });
    }
    Ok(method)
}

/// Public wrapper around the sync `GetIterator(value)` algorithm, reused by the
/// async-from-sync iterator path (`for await` over a non-async iterable).
pub(crate) fn sync_iterator_for_value(
    value: Value,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    iterator_for_value(value, env)
}

fn iterator_for_value(value: Value, env: &mut CallEnv) -> Result<Value, RuntimeError> {
    if matches!(value, Value::Undefined | Value::Null) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: value is not iterable".to_owned(),
        });
    }
    let Some(iterator_symbol) = symbol::iterator_symbol(env) else {
        return Err(RuntimeError {
            thrown: None,
            message: "iterator symbol is unavailable".to_owned(),
        });
    };
    let method = property_value_key(value.clone(), &PropertyKey::Symbol(iterator_symbol), env)?;
    if !matches!(method, Value::Function(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: value is not iterable".to_owned(),
        });
    }
    let iterator = call_function(method, value, Vec::new(), env, false)?;
    if !is_object_value(&iterator) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: iterator method must return an object".to_owned(),
        });
    }
    Ok(iterator)
}

fn iterator_step_value(
    iterator: &Value,
    next: &Value,
    env: &mut CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    let result = call_function(next.clone(), iterator.clone(), Vec::new(), env, false)?;
    if !is_object_value(&result) {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: iterator result is not an object".to_owned(),
        });
    }
    if is_truthy(&property_value(result.clone(), "done", env)?) {
        return Ok(None);
    }
    Ok(Some(property_value(result, "value", env)?))
}

fn iterator_rest_values(
    iterator: &Value,
    next: &Value,
    env: &mut CallEnv,
) -> Result<Vec<Value>, RuntimeError> {
    let mut values = Vec::new();
    while let Some(value) = iterator_step_value(iterator, next, env)? {
        values.push(value);
    }
    Ok(values)
}

fn close_iterator(iterator: &Value, env: &mut CallEnv) -> Result<(), RuntimeError> {
    let return_method = property_value(iterator.clone(), "return", env)?;
    if matches!(return_method, Value::Null | Value::Undefined) {
        return Ok(());
    }
    let result = call_function(return_method, iterator.clone(), Vec::new(), env, false)?;
    if is_object_value(&result) {
        return Ok(());
    }
    Err(RuntimeError {
        thrown: None,
        message: "TypeError: iterator return result must be an object".to_owned(),
    })
}
