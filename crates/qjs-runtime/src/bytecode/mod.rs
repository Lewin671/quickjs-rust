//! Bytecode compiler and stack VM for the runtime's fast path.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

mod compiler;
mod compiler_assign;
mod compiler_binding;
mod compiler_class;
mod compiler_control;
mod compiler_expr;
mod compiler_pattern;
mod compiler_try;
mod compiler_values;
mod ir;
mod util;
mod vm;
mod vm_bindings;
mod vm_call;
mod vm_class;
mod vm_errors;
mod vm_generator;
mod vm_iter;
mod vm_jobs;
mod vm_literals;
mod vm_ops;
mod vm_private;
mod vm_props;
mod vm_result;
mod vm_try;

use qjs_ast::{FunctionParams, Script};
use qjs_parser::parse_script;

use crate::{RuntimeError, Value};

pub use ir::Bytecode;
pub(crate) use vm_class::install_field_value;
pub(crate) use vm_generator::{
    GeneratorOutcome, GeneratorStart, GeneratorState, Resume, resume_generator,
};
pub(crate) use vm_iter::sync_iterator_for_value;
pub(crate) use vm_private::apply_instance_private_elements;
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
    env: HashMap<String, Value>,
    captured_env: Rc<RefCell<HashMap<String, Value>>>,
) -> FunctionBytecodeResult<'_> {
    vm::eval_function_bytecode(bytecode, env, captured_env)
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

/// Evaluates compiled bytecode, draining the promise job queue before
/// returning the script completion value.
///
/// # Errors
///
/// Returns runtime failures or malformed bytecode failures.
pub fn eval_bytecode(bytecode: &Bytecode) -> Result<Value, RuntimeError> {
    vm::eval_bytecode(bytecode)
}

/// Script completion paired with its realm's pending microtask queue.
///
/// Produced by [`eval_bytecode_keep_jobs`] for callers (the Test262 async
/// harness, the CLI) that need to evaluate further code or inspect results
/// before promise reactions run. Call [`EvalOutcome::run_jobs`] to drain the
/// queue when ready; the queue is realm-owned, so it is preserved here rather
/// than living in any global mutable state.
#[derive(Clone, Debug)]
pub struct EvalOutcome {
    /// The script's completion value, before any promise reactions ran.
    pub value: Value,
    env: HashMap<String, Value>,
}

impl EvalOutcome {
    /// Drains the realm's pending promise job queue in FIFO order, running
    /// queued microtasks (including ones they enqueue) until the queue is
    /// empty.
    ///
    /// # Errors
    ///
    /// Returns the first runtime failure raised while running a job.
    pub fn run_jobs(&mut self) -> Result<(), RuntimeError> {
        vm_jobs::run_pending_jobs(&mut self.env)
    }
}

/// Evaluates compiled bytecode without draining the promise job queue.
///
/// Returns the script completion value alongside the realm environment that
/// still owns any enqueued microtasks, so the caller controls when reactions
/// run via [`EvalOutcome::run_jobs`].
///
/// # Errors
///
/// Returns runtime failures or malformed bytecode failures.
pub fn eval_bytecode_keep_jobs(bytecode: &Bytecode) -> Result<EvalOutcome, RuntimeError> {
    let (value, env) = vm_jobs::eval_bytecode_keep_jobs(bytecode)?;
    Ok(EvalOutcome { value, env })
}

pub(crate) fn eval_bytecode_with_env(
    bytecode: &Bytecode,
    env: HashMap<String, Value>,
) -> FunctionBytecodeResult<'_> {
    let captured_env = Rc::new(RefCell::new(env.clone()));
    vm::eval_function_bytecode(bytecode, env, captured_env)
}
