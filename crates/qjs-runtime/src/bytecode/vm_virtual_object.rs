//! Handlers for scalar-replaced object and array bytecodes.
//!
//! These helpers execute one instruction selected by the ordinary VM dispatch;
//! they deliberately do not form a second bytecode or loop executor.

use super::{ir::Op, vm::Vm, vm_props::fast_number_binary};
use crate::{RuntimeError, Value, is_truthy};

impl<'a> Vm<'a> {
    /// Re-selects lowered code after generator setup/resume changes the frame's
    /// real binding authority. Instruction offsets are identical in both
    /// streams, and the analysis never carries a virtual candidate across a
    /// suspension point.
    pub(super) fn refresh_virtual_object_execution(&mut self) {
        let program = self
            .bytecode
            .virtual_object_program
            .get_or_init(|| super::virtual_object::lower(self.bytecode));
        let virtual_function_context_safe = self.env.deopt_bindings().is_none()
            && self.env.immutable_function_name().is_none()
            && self.with_stack.is_empty();
        self.execution_code = program.code_for_frame(
            &self.bytecode.code,
            self.authoritative_slots,
            virtual_function_context_safe,
        );
        self.virtual_values.clear();
    }

    pub(super) fn run_virtual_object_op(&mut self, op: &Op) -> Result<(), RuntimeError> {
        match op {
            Op::InitVirtualObject {
                slot,
                count,
                local,
                skip,
            } => {
                if *count != 0 {
                    let end = slot.checked_add(*count).ok_or_else(|| RuntimeError {
                        thrown: None,
                        message: "virtual object slot range out of bounds".to_owned(),
                    })?;
                    if self.virtual_values.len() < end {
                        self.virtual_values.resize(end, Value::Undefined);
                    }
                    for target in (*slot..end).rev() {
                        let value = self.pop()?;
                        self.virtual_values[target] = value;
                    }
                }
                if let Some(local) = local {
                    let target = self.locals.get_mut(*local).ok_or_else(|| RuntimeError {
                        thrown: None,
                        message: "virtual object alias slot out of bounds".to_owned(),
                    })?;
                    *target = Some(Value::Undefined);
                } else {
                    self.stack.push(Value::Undefined);
                }
                self.ip += *skip;
                #[cfg(test)]
                super::virtual_object::record_virtual_init_for_test(*count);
            }
            Op::InitVirtualConstants {
                slot,
                constants,
                local,
                skip,
            } => {
                let end = slot
                    .checked_add(constants.len())
                    .ok_or_else(|| RuntimeError {
                        thrown: None,
                        message: "virtual constant slot range out of bounds".to_owned(),
                    })?;
                if self.virtual_values.len() < end {
                    self.virtual_values.resize(end, Value::Undefined);
                }
                for (offset, index) in constants.iter().enumerate() {
                    let value = self
                        .bytecode
                        .constants
                        .get(*index)
                        .cloned()
                        .ok_or_else(|| RuntimeError {
                            thrown: None,
                            message: "virtual constant index out of bounds".to_owned(),
                        })?;
                    self.virtual_values[*slot + offset] = value;
                }
                if let Some(local) = local {
                    let target = self.locals.get_mut(*local).ok_or_else(|| RuntimeError {
                        thrown: None,
                        message: "virtual object alias slot out of bounds".to_owned(),
                    })?;
                    *target = Some(Value::Undefined);
                } else {
                    self.stack.push(Value::Undefined);
                }
                self.ip += *skip;
                #[cfg(test)]
                super::virtual_object::record_virtual_init_for_test(constants.len());
            }
            Op::InitVirtualFunction { local, skip } => {
                if let Some(local) = local {
                    let target = self.locals.get_mut(*local).ok_or_else(|| RuntimeError {
                        thrown: None,
                        message: "virtual function alias slot out of bounds".to_owned(),
                    })?;
                    *target = Some(Value::Undefined);
                } else {
                    self.stack.push(Value::Undefined);
                }
                self.ip += *skip;
                #[cfg(test)]
                super::virtual_object::record_virtual_function_init_for_test();
            }
            Op::LoadVirtualValue { slot, discard } => {
                #[cfg(test)]
                super::virtual_object::record_virtual_load_for_test(1);
                for _ in 0..*discard {
                    self.pop()?;
                }
                let value =
                    self.virtual_values
                        .get(*slot)
                        .cloned()
                        .ok_or_else(|| RuntimeError {
                            thrown: None,
                            message: "virtual object slot out of bounds".to_owned(),
                        })?;
                self.stack.push(value);
            }
            Op::StoreVirtualValue { slot, discard } => {
                let value = self.pop()?;
                for _ in 0..*discard {
                    self.pop()?;
                }
                let target = self
                    .virtual_values
                    .get_mut(*slot)
                    .ok_or_else(|| RuntimeError {
                        thrown: None,
                        message: "virtual object slot out of bounds".to_owned(),
                    })?;
                *target = value.clone();
                self.stack.push(value);
            }
            Op::LoadVirtualLength { length, discard } => {
                for _ in 0..*discard {
                    self.pop()?;
                }
                self.stack.push(Value::Number(*length as f64));
            }
            Op::GuardVirtualObject => {}
            Op::LoadVirtualBinary {
                left,
                right,
                op,
                skip,
            } => {
                #[cfg(test)]
                super::virtual_object::record_virtual_load_for_test(2);
                let direct = self
                    .virtual_values
                    .get(*left)
                    .zip(self.virtual_values.get(*right))
                    .and_then(|(left, right)| fast_number_binary(left, *op, right));
                if let Some(value) = direct {
                    self.stack.push(value);
                    self.ip += *skip;
                    return Ok(());
                }
                let left = self
                    .virtual_values
                    .get(*left)
                    .cloned()
                    .ok_or_else(|| RuntimeError {
                        thrown: None,
                        message: "virtual object slot out of bounds".to_owned(),
                    })?;
                let right =
                    self.virtual_values
                        .get(*right)
                        .cloned()
                        .ok_or_else(|| RuntimeError {
                            thrown: None,
                            message: "virtual object slot out of bounds".to_owned(),
                        })?;
                self.stack.push(left);
                self.stack.push(right);
                let result = self.eval_binary(*op);
                if let Some(value) = self.handle_runtime_result(result)? {
                    self.stack.push(value);
                    self.ip += *skip;
                }
            }
            Op::BinaryAssignLocals {
                op,
                target,
                stores,
                skip,
            } => {
                let direct = self.stack.len().checked_sub(2).and_then(|start| {
                    fast_number_binary(&self.stack[start], *op, &self.stack[start + 1])
                        .map(|value| (start, value))
                });
                let value = if let Some((start, value)) = direct {
                    self.stack.truncate(start);
                    Some(value)
                } else {
                    let result = self.eval_binary(*op);
                    self.handle_runtime_result(result)?
                };
                if let Some(value) = value {
                    if [*target, stores[0], stores[1]]
                        .into_iter()
                        .any(|slot| slot >= self.locals.len())
                    {
                        return Err(RuntimeError {
                            thrown: None,
                            message: "fused assignment slot out of bounds".to_owned(),
                        });
                    }
                    self.locals[*target] = Some(value.clone());
                    self.locals[stores[0]] = Some(value.clone());
                    self.locals[stores[1]] = Some(value);
                    self.ip += *skip;
                }
            }
            Op::IncrementLocal { slot, skip, jump } => {
                let direct = match self.locals.get(*slot) {
                    Some(Some(Value::Number(value))) => Some(*value),
                    _ => None,
                };
                if let Some(value) = direct {
                    self.locals[*slot] = Some(Value::Number(value + 1.0));
                    if let Some(target) = jump {
                        let backedge = self.ip + *skip;
                        self.jump_with_loop_plans(*target, backedge);
                    } else {
                        self.ip += *skip;
                    }
                    return Ok(());
                }
                let result = self.load_local(*slot);
                let Some(value) = self.handle_runtime_result(result)? else {
                    return Ok(());
                };
                self.stack.push(value);
                let result = self.eval_to_numeric();
                let Some(value) = self.handle_runtime_result(result)? else {
                    return Ok(());
                };
                self.stack.push(value.clone());
                self.stack.push(value);
                let result = self.eval_update(qjs_ast::UpdateOp::Increment);
                let Some(value) = self.handle_runtime_result(result)? else {
                    return Ok(());
                };
                let result = self.assign_local(*slot, value);
                if self.handle_runtime_result(result)?.is_none() {
                    return Ok(());
                }
                self.pop()?;
                if let Some(target) = jump {
                    let backedge = self.ip + *skip;
                    self.jump_with_loop_plans(*target, backedge);
                } else {
                    self.ip += *skip;
                }
            }
            Op::CopyLocal { from, to, skip } => {
                if let Some(Some(value)) = self.locals.get(*from).cloned()
                    && let Some(target) = self.locals.get_mut(*to)
                {
                    *target = Some(value);
                    self.ip += *skip;
                    return Ok(());
                }
                let result = self.load_local(*from);
                let Some(value) = self.handle_runtime_result(result)? else {
                    return Ok(());
                };
                let result = self.store_local(*to, value);
                if self.handle_runtime_result(result)?.is_none() {
                    return Ok(());
                }
                self.ip += *skip;
            }
            Op::CompareLocalsJumpFalse {
                left,
                right,
                op,
                target,
                skip,
                discard,
            } => {
                let direct = (self.slot_is_authoritative(*left)
                    && self.slot_is_authoritative(*right))
                .then(|| {
                    self.locals
                        .get(*left)
                        .and_then(Option::as_ref)
                        .zip(self.locals.get(*right).and_then(Option::as_ref))
                        .and_then(|(left, right)| fast_number_binary(left, *op, right))
                })
                .flatten();
                if let Some(value) = direct {
                    self.finish_virtual_comparison(value, *target, *skip, *discard);
                    return Ok(());
                }
                let result = self.load_local(*left);
                let Some(value) = self.handle_runtime_result(result)? else {
                    return Ok(());
                };
                self.stack.push(value);
                let result = self.load_local(*right);
                let Some(value) = self.handle_runtime_result(result)? else {
                    return Ok(());
                };
                self.stack.push(value);
                let result = self.eval_binary(*op);
                let Some(value) = self.handle_runtime_result(result)? else {
                    return Ok(());
                };
                self.finish_virtual_comparison(value, *target, *skip, *discard);
            }
            Op::CallVirtualFunction {
                allocation_ip,
                argc,
            } => self.call_virtual_function(*allocation_ip, *argc)?,
            _ => unreachable!("non-virtual opcode routed to virtual-object handler"),
        }
        Ok(())
    }

    fn call_virtual_function(
        &mut self,
        allocation_ip: usize,
        argc: usize,
    ) -> Result<(), RuntimeError> {
        let (params, bytecode) = match self.bytecode.code.get(allocation_ip) {
            Some(Op::NewFunction {
                params, bytecode, ..
            }) => (params.clone(), bytecode.clone()),
            _ => {
                return Err(RuntimeError {
                    thrown: None,
                    message: "virtual function template is unavailable".to_owned(),
                });
            }
        };

        let result = match argc {
            0 => {
                self.pop()?;
                crate::function::call_direct_function_literal(&params, &bytecode, &[], &self.env)
            }
            1 => {
                let first = self.pop()?;
                self.pop()?;
                crate::function::call_direct_function_literal(
                    &params,
                    &bytecode,
                    std::slice::from_ref(&first),
                    &self.env,
                )
            }
            2 => {
                let second = self.pop()?;
                let first = self.pop()?;
                self.pop()?;
                crate::function::call_direct_function_literal(
                    &params,
                    &bytecode,
                    &[first, second],
                    &self.env,
                )
            }
            3 => {
                let third = self.pop()?;
                let second = self.pop()?;
                let first = self.pop()?;
                self.pop()?;
                crate::function::call_direct_function_literal(
                    &params,
                    &bytecode,
                    &[first, second, third],
                    &self.env,
                )
            }
            _ => {
                let arguments = self.pop_arguments(argc)?;
                self.pop()?;
                crate::function::call_direct_function_literal(
                    &params, &bytecode, &arguments, &self.env,
                )
            }
        };
        if let Some(value) = self.handle_call_result(result)? {
            self.stack.push(value);
        }
        Ok(())
    }

    fn finish_virtual_comparison(
        &mut self,
        value: Value,
        target: usize,
        skip: usize,
        discard: bool,
    ) {
        let jump = !is_truthy(&value);
        if discard {
            if jump {
                self.ip = target + 1;
            } else {
                self.ip += skip + 1;
            }
        } else {
            self.stack.push(value);
            if jump {
                self.ip = target;
            } else {
                self.ip += skip;
            }
        }
    }
}
