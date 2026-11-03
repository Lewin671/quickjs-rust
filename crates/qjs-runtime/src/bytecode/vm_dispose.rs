//! `using` disposal for the bytecode VM (Explicit Resource Management).
//!
//! A block containing `using` declarations is compiled with an implicit
//! try/finally: `EnterDisposableScope` opens a scope, each `using`/`await using`
//! initializer is registered, and the finally disposes the resources LIFO on
//! every completion path. A sync scope runs `DisposeScope` once; an `await
//! using` scope loops over `DisposeAsyncStep`, awaiting each async dispose
//! before the next resource is touched (DisposeResources). A dispose failure
//! that overrides a pending throw is chained with `SuppressedError`.

use crate::{
    PropertyKey, RuntimeError, Value, call_function, error::create_suppressed_error,
    error::runtime_error_to_value, property_value_key, symbol,
};

use super::{vm::Vm, vm_try::PendingAbrupt};

/// A resource registered by a `using` declaration: the value and disposal
/// method resolved once at registration time.
pub(super) struct DisposeResource {
    pub(super) value: Value,
    pub(super) method: Value,
    hint: DisposeHint,
    /// An `await using` resource without `Symbol.asyncDispose` disposes
    /// through its `Symbol.dispose`, wrapped so the call's result is ignored
    /// and a throw surfaces only after the await (GetDisposeMethod).
    sync_fallback: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum DisposeHint {
    Sync,
    Async,
}

/// One open `using` scope: its resources in registration order, plus the
/// DisposeResources await bookkeeping an `await using` scope carries across
/// the suspensions of its disposal loop.
#[derive(Default)]
pub(super) struct DisposableScope {
    resources: Vec<DisposeResource>,
    /// A `null`/`undefined` `await using` was skipped; disposal still owes
    /// one await unless another resource already awaited.
    needs_await: bool,
    has_awaited: bool,
    /// A sync-fallback dispose threw; the error is chained once the await
    /// that the wrapper's rejected promise implies has resumed.
    deferred_error: Option<Value>,
}

impl<'a, F: crate::bytecode::vm::VmFrame<'a>> Vm<'a, '_, F> {
    /// Dispatches the `using` disposal ops, routing register/dispose failures
    /// through the throw machinery so a `try` can catch them.
    pub(super) fn run_disposal_op(&mut self, op: &super::ir::Op) -> Result<(), RuntimeError> {
        use super::ir::Op;
        match op {
            Op::EnterDisposableScope => {
                self.cold_mut()
                    .disposable_scopes
                    .push(DisposableScope::default());
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
            Op::DisposeScope => {
                let result = self.dispose_scope();
                self.handle_runtime_result(result).map(|_| ())
            }
            Op::DisposeAsyncStep => {
                let result = self.dispose_async_step();
                if let Some(step) = self.handle_runtime_result(result)? {
                    match step {
                        Some(awaited) => {
                            self.stack.push(awaited);
                            self.stack.push(Value::Boolean(true));
                        }
                        None => self.stack.push(Value::Boolean(false)),
                    }
                }
                Ok(())
            }
            Op::DisposeAsyncRejected => {
                let error = self.pop()?;
                let result = self.chain_dispose_error(error);
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
                self.cold_mut()
                    .disposable_scopes
                    .last_mut()
                    .expect("a disposable scope is open while registering")
                    .resources
                    .push(DisposeResource {
                        value,
                        method: Value::Undefined,
                        hint,
                        sync_fallback: false,
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
        let mut env = self.context();
        let (method, sync_fallback) = resolve_dispose_method(value.clone(), hint, &mut env)?;
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
        self.cold_mut()
            .disposable_scopes
            .last_mut()
            .expect("a disposable scope is open while registering")
            .resources
            .push(DisposeResource {
                value,
                method,
                hint,
                sync_fallback,
            });
        Ok(())
    }

    /// Disposes a sync scope's resources LIFO without suspending.
    fn dispose_scope(&mut self) -> Result<(), RuntimeError> {
        let resources = self
            .cold
            .as_deref_mut()
            .and_then(|cold| cold.disposable_scopes.pop())
            .unwrap_or_default()
            .resources;
        // The current finalizer owns the block's throw; disposal can suppress it.
        let mut pending = self
            .cold_mut()
            .finally_stack
            .last_mut()
            .and_then(|frame| {
                frame
                    .completion
                    .take_if(|completion| matches!(completion, PendingAbrupt::Throw(_)))
            })
            .and_then(|completion| match completion {
                PendingAbrupt::Throw(value) => Some(value),
                _ => None,
            });
        for resource in resources.into_iter().rev() {
            // `await using` is confined to blocks compiled with the async loop.
            debug_assert!(resource.hint == DisposeHint::Sync);
            let mut env = self.context();
            let result =
                call_function(resource.method, resource.value, Vec::new(), &mut env, false);
            if let Err(error) = result {
                pending = Some(self.suppress_dispose_error(error, pending.take())?);
            }
        }
        if let Some(error) = pending {
            // A throw (re-staged block throw or a dispose failure) overrides any
            // pending return/break that entered the finally.
            self.cold_mut()
                .finally_stack
                .last_mut()
                .expect("disposal runs inside a finalizer")
                .completion = Some(PendingAbrupt::Throw(error));
        }
        Ok(())
    }

    /// Runs an `await using` scope's DisposeResources up to its next await:
    /// returns the value the finally must await before stepping again, or
    /// `None` once every resource is disposed and the scope is closed. Sync
    /// dispose failures are chained into the finalizer's completion here; a
    /// rejected await is chained by `DisposeAsyncRejected`.
    fn dispose_async_step(&mut self) -> Result<Option<Value>, RuntimeError> {
        loop {
            let scope = self.current_disposable_scope();
            if let Some(error) = scope.deferred_error.take() {
                self.chain_dispose_error(error)?;
                continue;
            }
            let Some(resource) = scope.resources.last() else {
                if scope.needs_await && !scope.has_awaited {
                    scope.needs_await = false;
                    return Ok(Some(Value::Undefined));
                }
                self.cold_mut().disposable_scopes.pop();
                return Ok(None);
            };
            // A skipped `await using` still awaits before a sync dispose runs.
            if resource.hint == DisposeHint::Sync && scope.needs_await && !scope.has_awaited {
                scope.needs_await = false;
                return Ok(Some(Value::Undefined));
            }
            let resource = scope.resources.pop().expect("checked above");
            if matches!(resource.method, Value::Undefined) {
                scope.needs_await = true;
                continue;
            }
            let mut env = self.context();
            let result =
                call_function(resource.method, resource.value, Vec::new(), &mut env, false);
            match result {
                Ok(_) if resource.hint == DisposeHint::Sync => {}
                Ok(value) => {
                    self.current_disposable_scope().has_awaited = true;
                    let awaited = if resource.sync_fallback {
                        Value::Undefined
                    } else {
                        value
                    };
                    return Ok(Some(awaited));
                }
                Err(error) if resource.sync_fallback => {
                    let env = self.context();
                    let error = runtime_error_to_value(error, &env);
                    let scope = self.current_disposable_scope();
                    scope.has_awaited = true;
                    scope.deferred_error = Some(error);
                    return Ok(Some(Value::Undefined));
                }
                Err(error) => {
                    let env = self.context();
                    let error = runtime_error_to_value(error, &env);
                    self.chain_dispose_error(error)?;
                }
            }
        }
    }

    fn current_disposable_scope(&mut self) -> &mut DisposableScope {
        self.cold_mut()
            .disposable_scopes
            .last_mut()
            .expect("an `await using` scope is open while it is disposed")
    }

    /// Makes `error` the finalizer's completion: it suppresses a throw that
    /// is already pending and overrides a pending return or break.
    fn chain_dispose_error(&mut self, error: Value) -> Result<(), RuntimeError> {
        let previous = self
            .cold_mut()
            .finally_stack
            .last_mut()
            .expect("disposal runs inside a finalizer")
            .completion
            .take();
        let error = match previous {
            Some(PendingAbrupt::Throw(previous)) => {
                let mut env = self.context();
                create_suppressed_error(error, previous, &mut env)?
            }
            _ => error,
        };
        self.cold_mut()
            .finally_stack
            .last_mut()
            .expect("disposal runs inside a finalizer")
            .completion = Some(PendingAbrupt::Throw(error));
        Ok(())
    }

    fn suppress_dispose_error(
        &mut self,
        error: RuntimeError,
        pending: Option<Value>,
    ) -> Result<Value, RuntimeError> {
        let env = self.context();
        let thrown = runtime_error_to_value(error, &env);
        match pending {
            None => Ok(thrown),
            Some(previous) => {
                let mut env = self.context();
                let suppressed = create_suppressed_error(thrown, previous, &mut env)?;
                Ok(suppressed)
            }
        }
    }
}

/// Resolves the disposal method; the flag reports an `await using` falling
/// back to `Symbol.dispose`.
fn resolve_dispose_method(
    value: Value,
    hint: DisposeHint,
    env: &mut crate::context::Context<'_>,
) -> Result<(Value, bool), RuntimeError> {
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
            return Ok((method, false));
        }
    }
    let Some(dispose_symbol) = symbol::dispose_symbol(env) else {
        return Err(RuntimeError {
            thrown: None,
            message: "TypeError: Symbol.dispose is not available".to_owned(),
        });
    };
    let method = property_value_key(value, &PropertyKey::Symbol(dispose_symbol), env)?;
    Ok((method, hint == DisposeHint::Async))
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
