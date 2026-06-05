use std::collections::HashMap;

use crate::{RuntimeError, Value};

use super::ir::Bytecode;
use super::vm::Vm;
use super::vm_result::FunctionBytecodeResult;

pub(super) fn eval_bytecode(bytecode: &Bytecode) -> Result<Value, RuntimeError> {
    let mut vm = Vm::new(bytecode);
    vm.run()
}

pub(super) fn eval_function_bytecode(
    bytecode: &Bytecode,
    env: HashMap<String, Value>,
) -> FunctionBytecodeResult<'_> {
    eval_function_bytecode_with_stack(bytecode, env, Vec::new())
}

pub(super) fn eval_function_bytecode_with_stack(
    bytecode: &Bytecode,
    env: HashMap<String, Value>,
    with_stack: Vec<Value>,
) -> FunctionBytecodeResult<'_> {
    let mut vm = Vm::new_with_globals(bytecode, env, false);
    vm.with_stack = with_stack;
    vm.with_cleanup_stack = vec![usize::MAX; vm.with_stack.len()];
    let value = vm.run();
    FunctionBytecodeResult {
        value,
        bytecode,
        globals: vm.globals,
        locals: vm.locals,
        binding_overrides: vm.binding_overrides,
    }
}
