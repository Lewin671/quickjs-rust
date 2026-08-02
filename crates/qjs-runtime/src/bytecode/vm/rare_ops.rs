//! Opcodes the dispatch loop keeps out of its own machine code.
//!
//! `Vm::run_current_activation` disassembled as a single 25,020-instruction
//! function with a 4.3 KB stack frame and 335 distinct spill slots: the
//! register allocator had given up, so every dispatch reloaded `self`, the code
//! pointer and the code length *from the stack* before any opcode did any work.
//! That preamble is paid by the opcodes a benchmark actually executes, and it
//! is caused by the ones it never executes -- `NewFunction`, `TypeofGlobal`,
//! the `super` family and the generator suspensions each inline a large body
//! into the same function and compete for the same registers.
//!
//! Everything here is reached at most once per closure, class, `with` block,
//! iterator protocol step or suspension, so one extra call is not measurable on
//! it; what matters is that its registers are no longer the dispatch loop's
//! problem. The split is by *dispatch frequency*, not by semantic family, which
//! is why this module reads as a list rather than as a subsystem.

use super::{FrameExit, Vm};
use crate::bytecode::ir::Op;
use crate::bytecode::util::typeof_value;
use crate::bytecode::vm_iter::DelegateStep;
use crate::bytecode::vm_props::array_index_from_number;
use crate::bytecode::vm_result::Completion;
use crate::{
    Function, HOME_OBJECT_BINDING, PropertyKey, RuntimeError, SUPER_CONSTRUCTOR_BINDING, Value,
    function::{CompiledUserFunction, Upvalue},
    to_js_string_with_env,
};
use std::rc::Rc;

impl Vm<'_> {
    /// Runs one opcode the dispatch loop routed here.
    ///
    /// `Ok(None)` continues the activation; `Ok(Some(exit))` is the exit the
    /// dispatch loop returns. `self.ip` is authoritative on entry and on exit,
    /// so an opcode that jumps or suspends behaves exactly as it did inline.
    #[cold]
    #[inline(never)]
    pub(super) fn run_rare_op(&mut self, op: &Op) -> Result<Option<FrameExit>, RuntimeError> {
        match op {
            Op::LoadNewTarget => {
                let value = self.load_new_target();
                self.stack.push(value);
            }
            Op::ClearLocal(slot) => self.clear_local(*slot)?,
            Op::DefineGlobalVar(name) => {
                let value = self.pop()?;
                let result = self.define_global_var(name.clone(), value);
                self.handle_runtime_result(result)?;
            }
            Op::StoreLocalOrGlobalSloppy { slot, name } => {
                let value = self.pop()?;
                let result = self.store_local_or_global_sloppy(*slot, name, value);
                self.handle_runtime_result(result)?;
            }
            Op::TypeofGlobal(name) => {
                let result: Result<Value, RuntimeError> = (|| {
                    if self.direct_eval_with_stack {
                        return self.typeof_ident_with(name, None);
                    }
                    let value = if let Some(value) = self.env.module_import_value(name) {
                        if value.is_uninitialized_lexical_marker() {
                            return Err(RuntimeError {
                                thrown: None,
                                message: format!("ReferenceError: undefined identifier `{name}`"),
                            });
                        }
                        value
                    } else if let Some(value) = self.env.get(name) {
                        value
                    } else {
                        // A bare global name may resolve to a property on
                        // globalThis added via assignment or defineProperty;
                        // reading it invokes any getter. typeof yields
                        // "undefined" only when the reference is genuinely
                        // unresolvable.
                        self.global_this_own_value(name)?
                            .unwrap_or(Value::Undefined)
                    };
                    let value = if matches!(
                        &value,
                        Value::Function(function) if function.is_uninitialized_lexical_marker()
                    ) {
                        Value::Undefined
                    } else {
                        value
                    };
                    Ok(Value::String(typeof_value(value).into()))
                })();
                if let Some(value) = self.handle_runtime_result(result)? {
                    self.stack.push(value);
                }
            }
            op @ (Op::EnterWith
            | Op::ExitWith
            | Op::LoadIdentWith { .. }
            | Op::ResolveIdentWith { .. }
            | Op::LoadResolvedIdentWith { .. }
            | Op::StoreIdentWith { .. }
            | Op::StoreResolvedIdentWith { .. }
            | Op::TypeofIdentWith { .. }
            | Op::DeleteIdentWith { .. }) => {
                self.run_with_op(op.clone())?;
            }
            Op::NewTemplateObject { site, cooked, raw } => {
                self.new_template_object(*site, cooked, raw)
            }
            op @ (Op::EnterDisposableScope
            | Op::RegisterDisposable
            | Op::RegisterAsyncDisposable
            | Op::DisposeScope { .. }) => {
                self.run_disposal_op(op)?;
            }
            Op::SetComputedFunctionName(kind) => self.set_computed_function_name(*kind)?,
            Op::CopyObjectSpread => self.copy_object_spread()?,
            Op::GetIterator => self.get_iterator()?,
            Op::GetAsyncIterator => self.get_async_iterator()?,
            Op::AsyncIteratorComplete { done_slot } => self.async_iterator_complete(*done_slot)?,
            Op::IteratorStep { done_slot } => self.iterator_step(*done_slot)?,
            Op::IteratorRest { done_slot } => self.iterator_rest(*done_slot)?,
            Op::ObjectRestExcluding { excluded } => self.object_rest_excluding(excluded)?,
            Op::RequireObjectCoercible => self.require_object_coercible()?,
            Op::GetPrivate(name) => {
                let result = self.get_private(name);
                if let Some(value) = self.handle_runtime_result(result)? {
                    self.stack.push(value);
                }
            }
            Op::SetPrivate(name) => {
                let result = self.set_private(name);
                if let Some(value) = self.handle_runtime_result(result)? {
                    self.stack.push(value);
                }
            }
            Op::PrivateIn(name) => {
                let result = self.private_in(name);
                if let Some(value) = self.handle_runtime_result(result)? {
                    self.stack.push(value);
                }
            }
            Op::DeleteProp { is_strict } => {
                let result = self.delete_prop(*is_strict);
                self.handle_runtime_result(result)?;
            }
            Op::DeleteIdent(name) => {
                let result = self.delete_ident(name);
                self.stack.push(Value::Boolean(result));
            }
            Op::RequireCallable => {
                let result = self.require_callable();
                self.handle_runtime_result(result)?;
            }
            Op::CallDirectEval { argc, is_strict } => self.call_direct_eval(*argc, *is_strict)?,
            Op::CallSpread => self.call_spread()?,
            Op::CallDirectEvalSpread { is_strict } => self.call_direct_eval_spread(*is_strict)?,
            Op::IteratorClose { swallow } => self.iterator_close(*swallow)?,
            Op::NewSpread => self.construct_spread()?,
            Op::NewFunction {
                name,
                has_name_binding,
                immutable_name_binding,
                params,
                local_names,
                lexical_captures,
                bytecode,
                constructable,
                is_strict,
                lexical_this,
                lexical_arguments,
                is_generator,
                is_async,
                source_text,
            } => {
                let (home_object, super_constructor) = if *lexical_this {
                    let home_object = self.env.get_local(HOME_OBJECT_BINDING);
                    let mut super_constructor = self.env.get(SUPER_CONSTRUCTOR_BINDING);
                    if self.load_global("this").is_err() && super_constructor.is_none() {
                        super_constructor = Some(Value::Undefined);
                    }
                    (home_object, super_constructor)
                } else {
                    (None, None)
                };
                let upvalues = self.captured_upvalues_for_function(bytecode, lexical_captures);
                let immutable_env_binding =
                    self.captured_immutable_function_name(bytecode, local_names);
                let immutable_env_value = immutable_env_binding
                    .as_deref()
                    .and_then(|name| self.env.get(name))
                    .map(Upvalue::new);
                let lexical_new_target = if *lexical_this {
                    self.env.get(crate::NEW_TARGET_BINDING).map(Upvalue::new)
                } else {
                    None
                };
                let deopt_bindings = self.frame_deopt_bindings();
                let function = Function::new_user_compiled(CompiledUserFunction {
                    name: name.clone(),
                    has_name_binding: *has_name_binding,
                    immutable_name_binding: *immutable_name_binding,
                    immutable_env_binding,
                    immutable_env_value,
                    params: Rc::clone(params),
                    realm: Rc::clone(&self.realm),
                    module_host: self.module_host.clone(),
                    module_imports: self.env.module_imports(),
                    bytecode: Rc::clone(bytecode),
                    source_text: source_text.clone(),
                    local_names: Rc::clone(local_names),
                    constructable: *constructable,
                    is_strict: *is_strict,
                    lexical_this: *lexical_this,
                    lexical_arguments: *lexical_arguments,
                    lexical_new_target,
                    is_generator: *is_generator,
                    is_async: *is_async,
                    is_class_constructor: false,
                    is_derived_constructor: false,
                    is_field_initializer: *lexical_this
                        && matches!(
                            self.env.get(crate::FIELD_INITIALIZER_EVAL_BINDING),
                            Some(Value::Boolean(true))
                        ),
                    home_object,
                    super_constructor,
                    deopt_bindings,
                    with_stack: self.with_stack.clone(),
                    upvalues,
                });
                self.capture_private_environment(&function);
                if *is_generator && *is_async {
                    crate::async_generator::wire_async_generator_function_intrinsics(
                        &function,
                        &self.realm_env(),
                    );
                } else if *is_generator {
                    self.wire_generator_function_intrinsics(&function);
                } else if *is_async {
                    self.wire_async_function_intrinsics(&function);
                }
                self.stack.push(Value::Function(function));
            }
            Op::NewClass { definition } => {
                let result = self.new_class(
                    definition.name.as_deref(),
                    &definition.constructor,
                    &definition.elements,
                    &definition.private_elements,
                    &definition.computed_keys,
                    definition.has_heritage,
                );
                if let Some(value) = self.handle_runtime_result(result)? {
                    self.stack.push(value);
                }
            }
            Op::SuperGet { key } => {
                let result = self.super_get(&PropertyKey::String(key.clone()));
                if let Some(value) = self.handle_runtime_result(result)? {
                    self.stack.push(value);
                }
            }
            Op::SuperReference => {
                let result = self.super_reference();
                if let Some((receiver, lookup_base)) = self.handle_runtime_result(result)? {
                    self.stack.push(receiver);
                    self.stack.push(lookup_base);
                }
            }
            Op::SuperGetComputed => {
                let key_value = self.pop()?;
                let key = self.coerce_property_key(key_value);
                if let Some(key) = self.handle_runtime_result(key)? {
                    let lookup_base = self.pop()?;
                    let receiver = self.pop()?;
                    let result = self.super_get_from(lookup_base, receiver, &key);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
            }
            Op::SuperSet { key, is_strict } => {
                let result = self.super_set(&PropertyKey::String(key.clone()), *is_strict);
                if let Some(value) = self.handle_runtime_result(result)? {
                    self.stack.push(value);
                }
            }
            Op::SuperSetComputed { is_strict } => {
                let value = self.pop()?;
                let key_value = self.pop()?;
                let key = self.coerce_property_key(key_value);
                if let Some(key) = self.handle_runtime_result(key)? {
                    let lookup_base = self.pop()?;
                    let receiver = self.pop()?;
                    let result =
                        self.super_set_value_from(lookup_base, receiver, key, value, *is_strict);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
            }
            Op::SuperMethod { key } => {
                let result = self.super_method(PropertyKey::String(key.clone()));
                self.handle_runtime_result(result)?;
            }
            Op::SuperMethodComputed => {
                let key_value = self.pop()?;
                let key = self.coerce_property_key(key_value);
                if let Some(key) = self.handle_runtime_result(key)? {
                    let lookup_base = self.pop()?;
                    let receiver = self.pop()?;
                    let result = self.super_method_from(lookup_base, receiver, key);
                    self.handle_runtime_result(result)?;
                }
            }
            Op::CallResolvedSpread => self.call_resolved_spread()?,
            Op::SuperCall(argc) => {
                let arguments = self.pop_arguments(*argc)?;
                self.super_call(arguments)?;
            }
            Op::SuperCallSpread => {
                let arguments = self.pop_argument_array("super call spread")?;
                self.super_call(arguments)?;
            }
            Op::ToString => {
                let value = self.pop()?;
                let mut env = self.callee_env();
                let result = to_js_string_with_env(value, &mut env);
                self.apply_env(env);
                // Route a throwing toString/Symbol.toPrimitive through the
                // try-handler stack so `` `${bad}` `` is catchable, instead of
                // escaping the VM loop.
                if let Some(string) = self.handle_runtime_result(result)? {
                    self.stack.push(Value::String(string.into()));
                }
            }
            Op::ToPropertyKey => {
                let value = self.pop()?;
                let key = self.coerce_property_key(value)?;
                self.stack.push(key.into_value());
            }
            Op::ToPropertyKeyForAccess => {
                let value = self.pop()?;
                if matches!(&value, Value::Number(number) if array_index_from_number(*number).is_some())
                {
                    self.stack.push(value);
                } else {
                    let key = self.coerce_property_key(value)?;
                    self.stack.push(key.into_value());
                }
            }
            Op::AbruptJump(target) => self.abrupt_jump(*target)?,
            Op::EnterTry {
                catch,
                finally,
                catch_scope,
                cleanup_slots,
            } => self.enter_try(*catch, *finally, catch_scope.clone(), cleanup_slots.clone()),
            Op::ExitTry => self.exit_try()?,
            Op::EndFinally => {
                if let Some(value) = self.end_finally()? {
                    return Ok(Some(FrameExit::Completed(Completion::Return(value))));
                }
            }
            Op::DiscardPendingAbrupt => {
                self.pending_throw = None;
                self.pending_return = None;
            }
            Op::Throw => {
                let value = self.pop()?;
                self.throw_value(value)?;
            }
            Op::ThrowReferenceError(message) => {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("ReferenceError: {message}"),
                });
            }
            Op::FunctionPrologueEnd => {
                self.enter_body_deopt_scope();
                if self.stop_at_prologue {
                    self.stop_at_prologue = false;
                    return Ok(Some(FrameExit::Completed(Completion::PrologueEnd)));
                }
            }
            Op::Yield => {
                let value = self.pop()?;
                return Ok(Some(FrameExit::Completed(Completion::Yield(value))));
            }
            Op::Await => {
                let value = self.pop()?;
                return Ok(Some(FrameExit::Completed(Completion::Await(value))));
            }
            Op::YieldDelegate {
                iterator_slot,
                next_slot,
                async_delegate,
            } => match self.yield_delegate(*iterator_slot, *next_slot, *async_delegate)? {
                DelegateStep::Suspend(value) if *async_delegate => {
                    return Ok(Some(FrameExit::Completed(Completion::YieldDelegateAsync(
                        value,
                    ))));
                }
                DelegateStep::Suspend(value) => {
                    return Ok(Some(FrameExit::Completed(Completion::YieldDelegate(value))));
                }
                DelegateStep::Await(value) => {
                    return Ok(Some(FrameExit::Completed(Completion::YieldDelegateAwait(
                        value,
                    ))));
                }
                DelegateStep::AwaitReturn(value) => {
                    return Ok(Some(FrameExit::Completed(
                        Completion::YieldDelegateAwaitReturn(value),
                    )));
                }
                DelegateStep::AwaitReturnValue(value) => {
                    return Ok(Some(FrameExit::Completed(
                        Completion::YieldDelegateAwaitReturnValue(value),
                    )));
                }
                DelegateStep::Return(value) => {
                    return Ok(Some(FrameExit::Completed(Completion::Return(value))));
                }
                DelegateStep::Continue => {}
            },
            Op::ImportCall { has_options } => self.import_call(*has_options)?,
            Op::ImportMeta => {
                let Some(host) = self.current.module_host.as_ref() else {
                    return Err(RuntimeError {
                        thrown: None,
                        message: "SyntaxError: 'import.meta' is only valid in a module".to_owned(),
                    });
                };
                let import_meta = host.borrow_mut().import_meta();
                self.current.stack.push(Value::Object(import_meta));
            }
            // The dispatch loop routes exactly the variants above; its own match
            // stays exhaustive, so a new opcode is a compile error there rather
            // than a silent arrival here.
            op => {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!("bytecode opcode is not routed to the rare path: {op:?}"),
                });
            }
        }
        Ok(None)
    }
}
