//! Handlers for scalar-replaced object and array bytecodes.
//!
//! These helpers execute one instruction selected by the ordinary VM dispatch;
//! they deliberately do not form a second bytecode or loop executor.

use super::{
    ir::Op,
    virtual_object::constant_binary::VIRTUAL_CONSTANT_BINARY_INIT_SLOT,
    vm::Vm,
    vm_props::{fast_number_binary, fast_number_binary_numbers},
};
use crate::{RuntimeError, Value, is_truthy};

#[cfg(test)]
thread_local! {
    static TEST_CONSTANT_BINARY_INIT_HITS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

#[cfg(test)]
fn record_constant_binary_init_for_test() {
    TEST_CONSTANT_BINARY_INIT_HITS.set(TEST_CONSTANT_BINARY_INIT_HITS.get().saturating_add(1));
}

#[cfg(test)]
fn reset_constant_binary_init_hits() {
    TEST_CONSTANT_BINARY_INIT_HITS.set(0);
}

#[cfg(test)]
fn constant_binary_init_hits() -> usize {
    TEST_CONSTANT_BINARY_INIT_HITS.get()
}

impl<'a> Vm<'a> {
    /// Re-selects lowered code after generator setup/resume changes the frame's
    /// real binding authority. Instruction offsets are identical in both
    /// streams, and the analysis never carries a virtual candidate across a
    /// suspension point.
    pub(super) fn refresh_virtual_object_execution(&mut self) {
        // Only the selection inputs are updated here. The instruction stream
        // they imply is derived when the frame is next activated, because a
        // live `FrameProgramView` must never have code replaced underneath it.
        // Every caller runs during generator setup or resume, before the
        // dispatch loop starts, so there is no live view to invalidate.
        self.current.virtual_function_context_safe = self.env.deopt_bindings().is_none()
            && self.env.immutable_function_name().is_none()
            && self.with_stack.is_empty();
        self.virtual_values.clear();
    }

    pub(super) fn run_virtual_object_op(
        &mut self,
        program: &super::frame_program::FrameProgramView<'_>,
        op: &Op,
    ) -> Result<(), RuntimeError> {
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
                    if *count == 0
                        && *slot == VIRTUAL_CONSTANT_BINARY_INIT_SLOT
                        && self.try_run_constant_binary_assign_after_virtual_init(
                            program.execution_code,
                            *skip,
                        )?
                    {
                        #[cfg(test)]
                        {
                            record_constant_binary_init_for_test();
                            super::virtual_object::record_virtual_init_for_test(*count);
                        }
                        return Ok(());
                    }
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
                        self.jump_with_loop_plans(program.loop_plans(), *target, backedge);
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
                    self.jump_with_loop_plans(program.loop_plans(), *target, backedge);
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

    /// Folds a dead scalar-replaced literal initializer into the immediately
    /// following binary assignment when its field or element read has already
    /// been forwarded to a primitive constant. The source stream is unchanged:
    /// this only recognizes adjacent lowered operations and preserves the
    /// `BinaryAssignLocals` generic fallback for non-numeric values.
    fn try_run_constant_binary_assign_after_virtual_init(
        &mut self,
        execution_code: &[Op],
        initializer_skip: usize,
    ) -> Result<bool, RuntimeError> {
        let Some(first) = self.ip.checked_add(initializer_skip) else {
            return Ok(false);
        };
        let Some(Op::LoadLocal(left)) = execution_code.get(first) else {
            return Ok(false);
        };
        let Some(right_ip) = first.checked_add(1) else {
            return Ok(false);
        };
        let (right, right_number, binary_ip) = match execution_code.get(right_ip) {
            Some(Op::LoadConst(index)) => {
                let Some(right) = self.bytecode.constants.get(*index) else {
                    return Ok(false);
                };
                let Some(binary_ip) = right_ip.checked_add(1) else {
                    return Ok(false);
                };
                (
                    right.clone(),
                    match right {
                        Value::Number(value) => Some(*value),
                        _ => None,
                    },
                    binary_ip,
                )
            }
            Some(Op::LoadVirtualNumber { value, skip }) => {
                let Some(binary_ip) = right_ip
                    .checked_add(1)
                    .and_then(|after_load| after_load.checked_add(*skip))
                else {
                    return Ok(false);
                };
                (Value::Number(*value), Some(*value), binary_ip)
            }
            _ => return Ok(false),
        };
        let Some(Op::BinaryAssignLocals {
            op,
            target,
            stores,
            skip,
        }) = execution_code.get(binary_ip)
        else {
            return Ok(false);
        };
        let (op, target, stores, binary_skip) = (*op, *target, *stores, *skip);
        if [target, stores[0], stores[1]]
            .into_iter()
            .any(|slot| slot >= self.locals.len())
        {
            return Err(RuntimeError {
                thrown: None,
                message: "fused assignment slot out of bounds".to_owned(),
            });
        }
        let next_ip = binary_ip
            .checked_add(binary_skip)
            .and_then(|ip| ip.checked_add(1))
            .ok_or_else(|| RuntimeError {
                thrown: None,
                message: "virtual constant binary instruction pointer overflow".to_owned(),
            })?;
        if self.slot_is_authoritative(*left)
            && let Some(Some(Value::Number(left))) = self.locals.get(*left)
            && let Some(right) = right_number
            && let Some(Value::Number(value)) = fast_number_binary_numbers(*left, op, right)
        {
            self.locals[target] = Some(Value::Number(value));
            self.locals[stores[0]] = Some(Value::Number(value));
            self.locals[stores[1]] = Some(Value::Number(value));
            self.ip = next_ip;
            return Ok(true);
        }
        let value = self.load_local(*left)?;
        self.stack.push(value);
        self.stack.push(right);
        let result = self.eval_binary(op);
        let Some(value) = self.handle_runtime_result(result)? else {
            return Ok(true);
        };
        self.locals[target] = Some(value.clone());
        self.locals[stores[0]] = Some(value.clone());
        self.locals[stores[1]] = Some(value);
        self.ip = next_ip;
        Ok(true)
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

#[cfg(test)]
mod tests {
    use super::{constant_binary_init_hits, reset_constant_binary_init_hits};
    use crate::{Value, eval};

    #[test]
    fn folds_forwarded_literal_constants_into_binary_assignments() {
        reset_constant_binary_init_hits();
        assert_eq!(
            eval(
                "function run(n) { var sum = 0; for (var i = 0; i < n; i++) { var values = [1, 2, 3]; sum += values[2]; } return sum; } run(5);"
            ),
            Ok(Value::Number(15.0))
        );
        assert_eq!(constant_binary_init_hits(), 5);
    }

    #[test]
    fn folds_precomputed_virtual_numbers_into_binary_assignments() {
        reset_constant_binary_init_hits();
        assert_eq!(
            eval(
                "function run(n) { var total = 0; for (var index = 0; index < n; index++) { var point = { x: 1, y: 2 }; total += point.x + point.y; } return total; } run(5);"
            ),
            Ok(Value::Number(15.0))
        );
        assert_eq!(constant_binary_init_hits(), 5);
    }

    #[test]
    fn folded_constant_binary_assignments_retain_generic_addition() {
        reset_constant_binary_init_hits();
        assert_eq!(
            eval(
                "function run(n) { var sum = ''; for (var i = 0; i < n; i++) { var values = [1, 2, 3]; sum += values[2]; } return sum; } run(2);"
            ),
            Ok(Value::String("33".to_owned().into()))
        );
        assert_eq!(constant_binary_init_hits(), 2);
    }

    #[test]
    fn folded_constant_binary_assignments_preserve_numeric_and_bigint_edges() {
        reset_constant_binary_init_hits();
        assert_eq!(
            eval(
                "function run() { var total = -0; for (var index = 0; index < 1; index++) { var values = [1, 2, 1]; total *= values[2]; } return Object.is(total, -0); } run();"
            ),
            Ok(Value::Boolean(true))
        );
        assert_eq!(constant_binary_init_hits(), 1);

        reset_constant_binary_init_hits();
        let error = eval(
            "function run(total) { for (var index = 0; index < 1; index++) { var values = [1, 2, 3]; total += values[2]; } return total; } run(0n);",
        )
        .expect_err("mixed bigint and number addition should throw");
        assert!(error.message.contains("BigInt"), "{}", error.message);
    }
}
