//! Bytecode compiler and stack VM for the runtime's fast path.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

mod compiler;
mod compiler_assign;
mod compiler_binding;
mod compiler_class;
mod compiler_control;
mod compiler_expr;
mod compiler_function;
mod compiler_lexical;
mod compiler_params;
mod compiler_pattern;
mod compiler_try;
mod compiler_values;
mod ir;
mod ir_names;
mod upvalue_resolver;
mod util;
mod vm;
mod vm_bindings;
mod vm_call;
mod vm_capture;
mod vm_class;
mod vm_dispose;
mod vm_errors;
mod vm_generator;
mod vm_import;
mod vm_iter;
mod vm_jobs;
mod vm_literals;
mod vm_module;
mod vm_ops;
mod vm_private;
mod vm_props;
mod vm_result;
mod vm_set;
mod vm_string_append;
mod vm_try;

use qjs_ast::{FunctionParams, Script};
use qjs_parser::parse_script;

use crate::{RuntimeError, Value};

pub use ir::Bytecode;
pub(crate) use vm_class::install_field_value;
pub(crate) use vm_generator::{
    CaptureWriteback, GeneratorOutcome, GeneratorStart, GeneratorState, Resume, resume_generator,
    start_suspended_at_body,
};
pub(crate) use vm_iter::sync_iterator_for_value;
pub(crate) use vm_private::apply_instance_private_element;
pub(crate) use vm_result::FunctionBytecodeResult;
pub(crate) use vm_set::set_property as set_object_property;

/// Compiles an AST script into runtime bytecode.
///
/// # Errors
///
/// Returns an error for syntax currently outside the bytecode compiler subset.
pub fn compile_script(script: &Script) -> Result<Bytecode, RuntimeError> {
    compiler::compile_script(script).map_err(|error| error.error)
}

pub(crate) fn compile_direct_eval_script(
    script: &Script,
    strict: bool,
) -> Result<Bytecode, RuntimeError> {
    compiler::compile_direct_eval_script(script, strict)
}

/// A bytecode-compilation failure tagged with the stage a conformance harness
/// should attribute it to.
#[derive(Clone, Debug, PartialEq)]
pub struct CompileError {
    /// The underlying compiler error.
    pub error: RuntimeError,
    /// `true` when the failure is an invalid `/pattern/flags` regexp literal,
    /// which JavaScript rejects at parse phase rather than at evaluation.
    pub parse_stage: bool,
}

/// Compiles an AST script, preserving whether the failure is a parse-phase
/// error (an invalid regexp literal) for stage-sensitive harnesses.
///
/// # Errors
///
/// Returns a [`CompileError`] for syntax outside the bytecode compiler subset
/// or for a statically invalid regexp literal.
pub fn compile_script_classified(script: &Script) -> Result<Bytecode, CompileError> {
    compiler::compile_script(script)
}

pub(crate) fn compile_function_body(
    params: &FunctionParams,
    body: &[qjs_ast::Stmt],
) -> Result<Bytecode, RuntimeError> {
    compiler::compile_function_body(params, body)
}

pub(crate) fn compile_function_body_with_kind(
    params: &FunctionParams,
    body: &[qjs_ast::Stmt],
    parent_strict: bool,
    is_generator: bool,
    is_async: bool,
) -> Result<Bytecode, RuntimeError> {
    compiler::compile_function_body_with_strict_generator(
        params,
        body,
        parent_strict,
        is_generator,
        is_async,
    )
}

pub(crate) fn eval_function_bytecode(
    bytecode: &Bytecode,
    env: crate::CallEnv,
    captured_env: Rc<RefCell<HashMap<String, Value>>>,
    with_stack: Vec<Value>,
    capture_writeback: Option<CaptureWriteback>,
) -> FunctionBytecodeResult<'_> {
    vm::eval_function_bytecode(bytecode, env, captured_env, with_stack, capture_writeback)
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

/// Evaluates compiled script bytecode with a dynamic-import host installed on
/// its environment, so a dynamic `import()` in the script resolves and loads
/// modules through `resolver`. Drains the promise job queue (including any
/// import jobs) before returning the completion value.
///
/// # Errors
///
/// Returns runtime failures or malformed bytecode failures.
pub fn eval_bytecode_with_module_resolver(
    bytecode: &Bytecode,
    referrer: &str,
    resolver: Box<dyn crate::ModuleResolver>,
) -> Result<Value, RuntimeError> {
    vm_import::eval_bytecode_with_module_resolver(bytecode, referrer, resolver)
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
    env: crate::CallEnv,
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

/// A shared realm for a module graph (see [`vm_module::new_module_realm`]).
pub(crate) type ModuleRealm = crate::function::Realm;

pub(crate) struct ModuleEvaluation {
    pub(crate) env: crate::CallEnv,
    pub(crate) captured_env: Rc<RefCell<HashMap<String, Value>>>,
    pub(crate) async_result_promise: Option<crate::ObjectRef>,
}

pub(crate) struct ModuleLiveExports {
    pub(crate) names: Vec<String>,
    pub(crate) bindings: Rc<RefCell<HashMap<String, Value>>>,
    pub(crate) seed_tdz_markers: bool,
    pub(crate) imports: Vec<ModuleLiveImport>,
}

pub(crate) struct ModuleLiveImport {
    pub(crate) local_name: String,
    pub(crate) bindings: Rc<RefCell<HashMap<String, Value>>>,
    pub(crate) binding_name: String,
}

/// Builds the shared realm for a module graph. See
/// [`vm_module::new_module_realm`].
pub(crate) fn new_module_realm() -> ModuleRealm {
    vm_module::new_module_realm()
}

/// Compiles a module body to strict global-scope bytecode.
///
/// # Errors
///
/// Returns a compiler error for syntax outside the bytecode subset.
pub(crate) fn compile_module(script: &Script) -> Result<Bytecode, RuntimeError> {
    compiler::compile_module(script)
}

pub(crate) fn compile_module_function_hoists(script: &Script) -> Result<Bytecode, RuntimeError> {
    compiler::compile_module_function_hoists(script)
}

/// Compiles and evaluates a prelude *script* against the shared graph realm
/// before any module body runs, so its top-level bindings are visible to every
/// module. See [`vm_module::eval_prelude_script`].
///
/// # Errors
///
/// Returns parser, compiler, or VM runtime failures.
pub(crate) fn eval_prelude_script(
    source: &str,
    realm: &ModuleRealm,
) -> Result<(), crate::RuntimeError> {
    let script = parse_script(source).map_err(|error| crate::RuntimeError {
        thrown: None,
        message: error.message,
    })?;
    let bytecode = compile_script(&script)?;
    vm_module::eval_prelude_script(&bytecode, realm)
}

/// Evaluates a module body against the shared graph realm seeded with the
/// module's resolved imports. Returns the module's frame environment so the
/// caller can read its exported bindings. See [`vm_module::eval_module_body`].
pub(crate) fn eval_module_body(
    bytecode: &Bytecode,
    realm: &ModuleRealm,
    imports: HashMap<String, Value>,
    host: Option<crate::module::ModuleHostRef>,
    live_exports: ModuleLiveExports,
    drain: bool,
) -> Result<ModuleEvaluation, RuntimeError> {
    vm_module::eval_module_body(bytecode, realm, imports, host, live_exports, drain)
}

pub(crate) fn eval_module_function_hoists(
    bytecode: &Bytecode,
    realm: &ModuleRealm,
    host: Option<crate::module::ModuleHostRef>,
    live_exports: ModuleLiveExports,
) -> Result<(), RuntimeError> {
    vm_module::eval_module_function_hoists(bytecode, realm, host, live_exports)
}

pub(crate) fn seed_module_live_bindings(bytecode: &Bytecode, live_exports: &ModuleLiveExports) {
    vm_module::seed_live_bindings(
        &live_exports.bindings,
        bytecode,
        live_exports.names.clone(),
        live_exports.seed_tdz_markers,
    );
}

pub(crate) fn eval_bytecode_with_env(
    bytecode: &Bytecode,
    env: crate::CallEnv,
) -> FunctionBytecodeResult<'_> {
    let captured_env = Rc::new(RefCell::new(env.snapshot_locals()));
    vm::eval_function_bytecode(bytecode, env, captured_env, Vec::new(), None)
}
