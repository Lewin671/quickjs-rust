//! `using` disposal for the bytecode VM (Explicit Resource Management, sync).
//!
//! A block containing `using` declarations is compiled with an implicit
//! try/finally: `EnterDisposableScope` opens a scope, each `using`/`await using`
//! initializer is registered, and the finally runs `DisposeScope`, which
//! disposes the resources LIFO on every completion path. A dispose failure that
//! overrides a pending throw is chained with `SuppressedError`.

use crate::{
    PropertyKey, RuntimeError, Value, call_function, error::create_suppressed_error,
    error::runtime_error_to_value, property_value_key, symbol,
};

use super::vm::Vm;

/// A resource registered by a `using` declaration: the value and disposal
/// method resolved once at registration time.
pub(super) struct DisposeResource {
    pub(super) value: Value,
    pub(super) method: Value,
    hint: DisposeHint,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DisposeHint {
    Sync,
    Async,
}

impl Vm<'_> {
    /// Dispatches the three `using` disposal ops, routing register/dispose
    /// failures through the throw machinery so a `try` can catch them.
    pub(super) fn run_disposal_op(&mut self, op: &super::ir::Op) -> Result<(), RuntimeError> {
        use super::ir::Op;
        match op {
            Op::EnterDisposableScope => {
                self.disposable_scopes.push(Vec::new());
                Ok(())
            }
            Op::RegisterDisposable => {
                let result = self.register_disposable(DisposeHint::Sync);
                self.handle_runtime_result(result).map(|_| ())
            }
            Op::RegisterAsyncDisposable => {
                let result = self.register_disposable(DisposeHint::Async);
                self.handle_runtime_result(result).map(|_| ())
            }
            Op::DisposeScope { await_async } => {
                let result = self.dispose_scope(*await_async);
                self.handle_runtime_result(result).map(|_| ())
            }
            _ => Ok(()),
        }
    }

    fn register_disposable(&mut self, hint: DisposeHint) -> Result<(), RuntimeError> {
        // The resource value stays on the stack (it is also bound to the
        // declaration); only inspect it here.
        let value = match self.stack.last() {
            Some(value) => value.clone(),
            None => {
                return Err(RuntimeError {
                    thrown: None,
                    message: "missing `using` resource value on the stack".to_owned(),
                });
            }
        };
        if matches!(value, Value::Null | Value::Undefined) {
            if hint == DisposeHint::Async {
                self.disposable_scopes
                    .last_mut()
                    .expect("a disposable scope is open while registering")
                    .push(DisposeResource {
                        value,
                        method: Value::Undefined,
                        hint,
                    });
            }
            return Ok(());
        }
        if !is_disposable_object(&value) {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: `using` value is not an object".to_owned(),
            });
        }
        let mut env = self.current_env();
        let method = resolve_dispose_method(value.clone(), hint, &mut env)?;
        self.apply_env(env);
        if matches!(method, Value::Null | Value::Undefined) {
            return Err(RuntimeError {
                thrown: None,
                message: missing_dispose_message(hint).to_owned(),
            });
        }
        if !is_callable(&method) {
            return Err(RuntimeError {
                thrown: None,
                message: not_callable_message(hint).to_owned(),
            });
        }
        self.disposable_scopes
            .last_mut()
            .expect("a disposable scope is open while registering")
            .push(DisposeResource {
                value,
                method,
                hint,
            });
        Ok(())
    }

    fn dispose_scope(&mut self, await_async: bool) -> Result<(), RuntimeError> {
        let resources = self.disposable_scopes.pop().unwrap_or_default();
        // Seed the accumulated completion with any throw the block raised (the
        // finally was entered via throw_value, which stages pending_throw). A
        // dispose failure then suppresses it.
        let mut pending = self.pending_throw.take();
        let mut awaited = Value::Undefined;
        let mut did_await = false;
        for resource in resources.into_iter().rev() {
            let result = if matches!(resource.method, Value::Undefined) {
                Ok(Value::Undefined)
            } else {
                let mut env = self.current_env();
                let result =
                    call_function(resource.method, resource.value, Vec::new(), &mut env, false);
                self.apply_env(env);
                result
            };
            match result {
                Ok(value) => {
                    if resource.hint == DisposeHint::Async {
                        did_await = true;
                        if let Some(error) = async_dispose_rejection(&value) {
                            awaited = Value::Undefined;
                            pending = Some(self.suppress_dispose_error(error, pending.take())?);
                        } else {
                            awaited = value;
                        }
                    }
                }
                Err(error) => {
                    pending = Some(self.suppress_dispose_error(error, pending.take())?);
                }
            }
        }
        if let Some(error) = pending {
            // A throw (re-staged block throw or a dispose failure) overrides any
            // pending return/break that entered the finally.
            self.pending_return = None;
            self.pending_jump = None;
            self.pending_throw = Some(error);
        }
        if await_async {
            self.stack.push(awaited);
            self.stack.push(Value::Boolean(did_await));
        }
        Ok(())
    }

    fn suppress_dispose_error(
        &mut self,
        error: RuntimeError,
        pending: Option<Value>,
    ) -> Result<Value, RuntimeError> {
        let env = self.current_env();
        let thrown = runtime_error_to_value(error, &env);
        match pending {
            None => Ok(thrown),
            Some(previous) => {
                let mut env = self.current_env();
                let suppressed = create_suppressed_error(thrown, previous, &mut env)?;
                self.apply_env(env);
                Ok(suppressed)
            }
        }
    }
}

fn async_dispose_rejection(value: &Value) -> Option<RuntimeError> {
    let Value::Object(promise) = value else {
        return None;
    };
    match crate::promise::settled_outcome(promise) {
        Some(Err(reason)) => Some(RuntimeError {
            thrown: Some(Box::new(reason)),
            message: "throw statement executed".to_owned(),
        }),
        _ => None,
    }
}

fn resolve_dispose_method(
    value: Value,
    hint: DisposeHint,
    env: &mut crate::CallEnv,
) -> Result<Value, RuntimeError> {
    if hint == DisposeHint::Async {
        let Some(async_dispose_symbol) = symbol::async_dispose_symbol(env) else {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: Symbol.asyncDispose is not available".to_owned(),
            });
        };
        let method = property_value_key(
            value.clone(),
            &PropertyKey::Symbol(async_dispose_symbol),
            env,
        )?;
        if !matches!(method, Value::Null | Value::Undefined) {
            return Ok(method);
        }
    }
    let Some(dispose_symbol) = symbol::dispose_symbol(env) else {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Symbol.dispose is not available".to_owned(),
        });
    };
    property_value_key(value, &PropertyKey::Symbol(dispose_symbol), env)
}

fn missing_dispose_message(hint: DisposeHint) -> &'static str {
    match hint {
        DisposeHint::Sync => "TypeError: `using` value is missing Symbol.dispose",
        DisposeHint::Async => {
            "TypeError: `await using` value is missing Symbol.asyncDispose or Symbol.dispose"
        }
    }
}

fn not_callable_message(hint: DisposeHint) -> &'static str {
    match hint {
        DisposeHint::Sync => "TypeError: Symbol.dispose is not callable",
        DisposeHint::Async => "TypeError: Symbol.asyncDispose or Symbol.dispose is not callable",
    }
}

fn is_disposable_object(value: &Value) -> bool {
    matches!(
        value,
        Value::Object(_)
            | Value::Array(_)
            | Value::Function(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Proxy(_)
    )
}

fn is_callable(value: &Value) -> bool {
    match value {
        Value::Function(_) => true,
        Value::Proxy(proxy) => crate::proxy::proxy_is_callable(proxy),
        _ => false,
    }
}
