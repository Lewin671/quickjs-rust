//! Bytecode compiler and stack VM for the runtime's fast path.

mod compiler;
mod compiler_assign;
mod compiler_control;
mod compiler_expr;
mod compiler_try;
mod compiler_values;
mod ir;
mod util;
mod vm;
mod vm_bindings;
mod vm_call;
mod vm_errors;
mod vm_ops;
mod vm_props;
mod vm_result;
mod vm_try;

use qjs_ast::{FunctionParams, Script};
use qjs_parser::parse_script;

use crate::{RuntimeError, Value};

pub use ir::Bytecode;
pub(crate) use vm_result::FunctionBytecodeResult;

/// Compiles an AST script into runtime bytecode.
///
/// # Errors
///
/// Returns an error for syntax currently outside the bytecode compiler subset.
pub fn compile_script(script: &Script) -> Result<Bytecode, RuntimeError> {
    compiler::compile_script(script)
}

pub(crate) fn compile_function_body(
    params: &FunctionParams,
    body: &[qjs_ast::Stmt],
) -> Result<Bytecode, RuntimeError> {
    compiler::compile_function_body(params, body)
}

pub(crate) fn eval_function_bytecode(
    bytecode: &Bytecode,
    env: std::collections::HashMap<String, Value>,
) -> FunctionBytecodeResult<'_> {
    vm::eval_function_bytecode(bytecode, env)
}

/// Compiles and evaluates source text through the bytecode VM.
///
/// # Errors
///
/// Returns parser, compiler, or VM runtime failures.
pub fn eval_bytecode_source(source: &str) -> Result<Value, RuntimeError> {
    let script = parse_script(source).map_err(|error| RuntimeError {
        thrown: None,
        message: error.message,
    })?;
    let bytecode = compile_script(&script)?;
    eval_bytecode(&bytecode)
}

/// Evaluates compiled bytecode.
///
/// # Errors
///
/// Returns runtime failures or malformed bytecode failures.
pub fn eval_bytecode(bytecode: &Bytecode) -> Result<Value, RuntimeError> {
    vm::eval_bytecode(bytecode)
}

pub(crate) fn eval_bytecode_with_env(
    bytecode: &Bytecode,
    env: std::collections::HashMap<String, Value>,
) -> FunctionBytecodeResult<'_> {
    vm::eval_function_bytecode(bytecode, env)
}
