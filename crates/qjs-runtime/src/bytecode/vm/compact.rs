use crate::{RuntimeError, Value, is_truthy, to_js_string_with_env};

use super::super::{
    compact::{CompactOpcode, decode_binary, decode_unary, decode_update},
    util::{stack_underflow, typeof_value},
    vm_frame::FrameRun,
    vm_result::Completion,
};
use super::Vm;

impl Vm<'_> {
    /// Runs a completely prevalidated compact child frame.
    ///
    /// `FrameState.ip` remains an index into the original `Vec<Op>` throughout
    /// execution. Selection happened before this frame started, so this loop
    /// never falls back to or replays the ordinary opcode stream. Keep this
    /// large dispatch loop out of line so the ordinary dispatcher retains its
    /// independent code size and stack frame.
    #[inline(never)]
    pub(super) fn run_compact_current_frame(&mut self) -> Result<FrameRun, RuntimeError> {
        let bytecode_owner = self.bytecode.clone();
        let bytecode = bytecode_owner.as_ref();
        let program = bytecode.compact_program().ok_or_else(|| {
            invalid_compact_program("compact frame has no completely lowered program")
        })?;

        loop {
            let instruction = *program.instructions.get(self.ip).ok_or_else(|| {
                invalid_compact_program("compact instruction pointer out of bounds")
            })?;
            self.ip += 1;

            match instruction.opcode {
                CompactOpcode::FunctionPrologueEnd => {
                    self.enter_body_deopt_scope();
                    if self.stop_at_prologue {
                        self.stop_at_prologue = false;
                        return Ok(FrameRun::Complete(Completion::PrologueEnd));
                    }
                }
                CompactOpcode::LoadConst => {
                    let index = compact_usize(instruction.a)?;
                    let value = bytecode.constants.get(index).cloned().ok_or_else(|| {
                        invalid_compact_program("compact constant index out of bounds")
                    })?;
                    self.stack.push(value);
                }
                CompactOpcode::LoadLocal => {
                    let slot = compact_usize(instruction.a)?;
                    let result =
                        if self.direct_eval_with_stack && self.bytecode.local_is_from_env(slot) {
                            let name = self.bytecode.locals[slot].name.clone();
                            self.load_ident_with(&name, Some(slot), self.bytecode.is_strict())
                        } else {
                            self.load_local(slot)
                        };
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                CompactOpcode::LoadLocalOrUndefined => {
                    let value = self.load_local_or_undefined(compact_usize(instruction.a)?)?;
                    self.stack.push(value);
                }
                CompactOpcode::StoreLocal => {
                    let value = self.pop()?;
                    let result = self.store_local(compact_usize(instruction.a)?, value);
                    self.handle_runtime_result(result)?;
                }
                CompactOpcode::AssignLocal => {
                    let slot = compact_usize(instruction.a)?;
                    let value = self.pop()?;
                    let result = if self.direct_eval_with_stack
                        && self.bytecode.local_is_from_env(slot)
                    {
                        let name = self.bytecode.locals[slot].name.clone();
                        self.store_ident_with(&name, Some(slot), self.bytecode.is_strict(), value)
                    } else {
                        self.assign_local(slot, value)
                    };
                    self.handle_runtime_result(result)?;
                }
                CompactOpcode::ClearLocal => {
                    self.clear_local(compact_usize(instruction.a)?)?;
                }
                CompactOpcode::LoadGlobal => {
                    let name = program
                        .names
                        .get(compact_usize(instruction.a)?)
                        .ok_or_else(|| {
                            invalid_compact_program("compact name index out of bounds")
                        })?;
                    let result = if self.direct_eval_with_stack {
                        self.load_ident_with(name, None, self.bytecode.is_strict())
                    } else {
                        self.load_global(name)
                    };
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                CompactOpcode::Pop => {
                    self.pop()?;
                }
                CompactOpcode::Dup => {
                    let value = self.stack.last().cloned().ok_or_else(stack_underflow)?;
                    self.stack.push(value);
                }
                CompactOpcode::Typeof => {
                    let value = self.pop()?;
                    self.stack.push(Value::String(typeof_value(value).into()));
                }
                CompactOpcode::ToString => {
                    let value = self.pop()?;
                    let mut env = self.current_env();
                    let result = to_js_string_with_env(value, &mut env);
                    self.apply_env(env);
                    if let Some(string) = self.handle_runtime_result(result)? {
                        self.stack.push(Value::String(string.into()));
                    }
                }
                CompactOpcode::ToNumeric => {
                    let result = self.eval_to_numeric();
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                CompactOpcode::Unary => {
                    let op = decode_unary(instruction.flags)
                        .ok_or_else(|| invalid_compact_program("invalid compact unary operator"))?;
                    let result = self.eval_unary(op);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                CompactOpcode::Update => {
                    let op = decode_update(instruction.flags).ok_or_else(|| {
                        invalid_compact_program("invalid compact update operator")
                    })?;
                    let result = self.eval_update(op);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                CompactOpcode::Binary => {
                    let op = decode_binary(instruction.flags).ok_or_else(|| {
                        invalid_compact_program("invalid compact binary operator")
                    })?;
                    let result = self.eval_binary(op);
                    if let Some(value) = self.handle_runtime_result(result)? {
                        self.stack.push(value);
                    }
                }
                CompactOpcode::Jump => {
                    let target = compact_usize(instruction.a)?;
                    let backedge = self.ip - 1;
                    if target >= backedge
                        || (!super::super::vm_numeric_mutation_loop::try_run_numeric_mutation_loop(
                            self, target, backedge,
                        ) && !super::super::vm_numeric_loop::try_run_numeric_loop(
                            self, target, backedge,
                        ) && !super::super::vm_control_loop::try_run_control_loop(
                            self, target, backedge,
                        ))
                    {
                        self.ip = target;
                    }
                }
                CompactOpcode::JumpIfFalse => {
                    if !is_truthy(self.stack.last().ok_or_else(stack_underflow)?) {
                        self.ip = compact_usize(instruction.a)?;
                    }
                }
                CompactOpcode::JumpIfTrue => {
                    if is_truthy(self.stack.last().ok_or_else(stack_underflow)?) {
                        self.ip = compact_usize(instruction.a)?;
                    }
                }
                CompactOpcode::JumpIfNotNullish => {
                    if !matches!(self.stack.last(), Some(Value::Null | Value::Undefined)) {
                        self.ip = compact_usize(instruction.a)?;
                    }
                }
                CompactOpcode::Call => {
                    let staged = self.call(compact_usize(instruction.a)?)?;
                    if let Some(run) = compact_direct_call_transition(staged) {
                        return Ok(run);
                    }
                }
                CompactOpcode::CallResolved => {
                    let staged = self.call_resolved(compact_usize(instruction.a)?)?;
                    if let Some(run) = compact_direct_call_transition(staged) {
                        return Ok(run);
                    }
                }
                CompactOpcode::Return => {
                    let value = self.stack.pop().unwrap_or(Value::Undefined);
                    if let Some(value) = self.return_value(value)? {
                        return Ok(FrameRun::Complete(Completion::Return(value)));
                    }
                }
            }
        }
    }
}

/// The only compact-executor dependency on the scheduler's staged-call ABI.
#[inline]
fn compact_direct_call_transition(staged: bool) -> Option<FrameRun> {
    staged.then_some(FrameRun::DirectCall)
}

fn compact_usize(value: u32) -> Result<usize, RuntimeError> {
    usize::try_from(value)
        .map_err(|_| invalid_compact_program("compact operand does not fit usize"))
}

fn invalid_compact_program(message: &str) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: message.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    use crate::bytecode::{
        DirectCallSlots,
        ir::{Bytecode, Op},
        vm_frame::FrameExecution,
    };

    #[test]
    fn frame_selection_is_fixed_before_execution() {
        let supported = Bytecode::new(
            vec![Value::Undefined],
            Vec::new(),
            vec![Op::LoadConst(0), Op::Return],
        );
        let root = Vm::new(&supported).expect("root VM should initialize");
        assert_eq!(root.execution, FrameExecution::Ordinary);
        let env = root.realm_env();
        drop(root);
        let direct_root = Vm::new_with_globals_upvalues_with_stack_and_direct_call_slots(
            &supported,
            env,
            Vec::new(),
            Vec::new(),
            Some(DirectCallSlots {
                this_value: None,
                parameter_slots: &[],
                arguments: &[],
                upvalues: &[],
                realm_upvalue_slots: 0,
            }),
        );
        assert_eq!(direct_root.execution, FrameExecution::Ordinary);
        assert_eq!(
            Vm::direct_leaf_execution(&supported),
            FrameExecution::Compact
        );

        let unsupported = Bytecode::new(
            Vec::new(),
            Vec::new(),
            vec![Op::NewObjectLiteral, Op::Return],
        );
        assert_eq!(
            Vm::direct_leaf_execution(&unsupported),
            FrameExecution::Ordinary
        );
    }

    #[test]
    fn compact_call_resolved_uses_canonical_receiver_callee_layout() {
        let bootstrap_bytecode = Bytecode::new(Vec::new(), Vec::new(), vec![Op::Return]);
        let bootstrap = Vm::new(&bootstrap_bytecode).expect("bootstrap VM should initialize");
        let number = bootstrap
            .env
            .get("Number")
            .expect("Number constructor should be installed");
        drop(bootstrap);

        let bytecode = Bytecode::new(
            vec![Value::Undefined, number, Value::Number(7.0)],
            Vec::new(),
            vec![
                Op::LoadConst(0),
                Op::LoadConst(1),
                Op::LoadConst(2),
                Op::CallResolved(1),
                Op::Return,
            ],
        );
        let mut vm = Vm::new(&bytecode).expect("compact VM should initialize");
        assert_eq!(vm.execution, FrameExecution::Ordinary);
        vm.execution = FrameExecution::Compact;

        assert_eq!(vm.run(), Ok(Value::Number(7.0)));
    }

    #[test]
    fn compact_call_resolved_stages_a_child_and_preserves_receiver() {
        let script = qjs_parser::parse_script(
            "function leaf(value) { if (value === 0) return 0; return this; }",
        )
        .expect("source should parse");
        let script_bytecode =
            super::super::super::compiler::compile_script(&script).expect("source should compile");
        let mut bootstrap =
            Vm::new(&script_bytecode).expect("bootstrap VM should initialize builtins");
        bootstrap
            .run()
            .expect("function declaration should evaluate");
        let callee = bootstrap
            .env
            .get("leaf")
            .expect("function declaration should install leaf");
        let Value::Function(function) = &callee else {
            panic!("leaf should be a function");
        };
        let child_bytecode = function
            .bytecode
            .as_ref()
            .expect("leaf should retain bytecode");
        assert_eq!(
            Vm::direct_leaf_execution(child_bytecode),
            FrameExecution::Compact
        );
        drop(bootstrap);

        let receiver = crate::ObjectRef::new(HashMap::new());
        let bytecode = Bytecode::new(
            vec![Value::Object(receiver.clone()), callee, Value::Number(1.0)],
            Vec::new(),
            vec![
                Op::LoadConst(0),
                Op::LoadConst(1),
                Op::LoadConst(2),
                Op::CallResolved(1),
                Op::Return,
            ],
        );
        let mut vm = Vm::new(&bytecode).expect("compact VM should initialize");
        vm.execution = FrameExecution::Compact;

        let Value::Object(result) = vm.run().expect("resolved call should complete") else {
            panic!("resolved call should return its receiver");
        };
        assert!(result.ptr_eq(&receiver));
    }

    #[test]
    fn semantic_shapes_used_by_runtime_tests_lower_completely() {
        let script = qjs_parser::parse_script(
            "function compactBinary(add, left, right) { \
               if (add) return left + right; return left - right; \
             } \
             function shortCircuit(kind, left) { \
               if (kind === 0) return left && zero(); \
               if (kind === 1) return left || zero(); \
               return left ?? zero(); \
             } \
             function readBeforeInit() { return hidden; let hidden = 1; } \
             function unaryOrUpdate(kind, value) { \
               if (kind === 0) return -value; \
               if (kind === 1) return typeof value; \
               return ++value; \
             } \
             function compactTemplate(value) { \
               if (value === null) return ''; return `${value}`; \
             } \
             function zero() { return 0; } \
             function dispatch(kind, first, second, third) { \
               if (kind === 0) return zero(); \
               if (kind === 1) return first; \
               if (kind === 2) return first + second; \
               return first + second + third; \
             }",
        )
        .expect("source should parse");
        let bytecode =
            super::super::super::compiler::compile_script(&script).expect("source should compile");
        let mut checked = 0;
        for op in &bytecode.code {
            let Op::NewFunction {
                name: Some(name),
                bytecode,
                ..
            } = op
            else {
                continue;
            };
            if [
                "compactBinary",
                "shortCircuit",
                "readBeforeInit",
                "unaryOrUpdate",
                "compactTemplate",
                "zero",
                "dispatch",
            ]
            .contains(&name.as_str())
            {
                assert_eq!(
                    Vm::direct_leaf_execution(bytecode),
                    FrameExecution::Compact,
                    "{name} unexpectedly fell back: {:#?}",
                    bytecode.code
                );
                checked += 1;
            }
        }
        assert_eq!(checked, 7);
    }
}
