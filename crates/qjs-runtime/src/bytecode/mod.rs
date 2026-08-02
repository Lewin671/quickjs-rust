//! Bytecode compiler and stack VM for the runtime's fast path.

use std::collections::HashMap;

mod call_context;
mod compact_fn;
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
mod enumerate_keys_cache;
mod frame_program;
mod frame_stack;
mod ir;
mod ir_names;
mod named_property_cache;
mod operand_stack;
mod typed_loop;
mod upvalue_resolver;
mod util;
mod virtual_object;
mod vm;
mod vm_bindings;
mod vm_call;
mod vm_call_env;
mod vm_capture;
mod vm_class;
mod vm_control_loop;
mod vm_direct_upvalues;
mod vm_dispose;
mod vm_errors;
mod vm_frame_init;
mod vm_generator;
mod vm_import;
mod vm_iter;
mod vm_jobs;
mod vm_literals;
mod vm_loop_dispatch;
mod vm_module;
mod vm_numeric_leaf;
mod vm_numeric_loop;
mod vm_numeric_mutation_loop;
mod vm_ops;
mod vm_private;
mod vm_props;
mod vm_result;
mod vm_set;
mod vm_string_append;
mod vm_this_property_leaf;
mod vm_try;
mod vm_virtual_object;
mod vm_with;

use qjs_ast::{FunctionParams, Script};
use qjs_parser::parse_script;

use crate::{RuntimeError, Value};

pub use ir::Bytecode;
pub(crate) use vm_class::install_field_value;
pub(crate) use vm_generator::{
    GeneratorOutcome, GeneratorStart, GeneratorState, Resume, is_suspended_at_plain_yield,
    resume_generator, start_suspended_at_body,
};
pub(crate) use vm_iter::sync_iterator_for_value;
pub(crate) use vm_numeric_leaf::try_eval_numeric_leaf;
pub(crate) use vm_private::apply_instance_private_element;
pub(crate) use vm_result::FunctionBytecodeResult;
pub(crate) use vm_set::set_property as set_object_property;
pub(crate) use vm_this_property_leaf::try_eval_this_property_leaf;

pub(crate) fn delete_object_property(
    object: Value,
    key: &crate::PropertyKey,
    env: &mut crate::CallEnv,
) -> Result<bool, RuntimeError> {
    match vm_props::delete_property_key(object, key, env)? {
        Value::Boolean(deleted) => Ok(deleted),
        _ => unreachable!("property deletion must return a Boolean value"),
    }
}

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

/// Whether a direct-eval compilation can safely serve as a reusable blueprint.
///
/// Declaration-bearing sources mutate eval-instantiation metadata per caller;
/// nested function/class bodies and template literals carry per-evaluation
/// identity state. Keep those paths freshly compiled until they have their own
/// deep-clone representation.
pub(crate) fn direct_eval_bytecode_is_cacheable(bytecode: &Bytecode) -> bool {
    bytecode.hoisted_local_names().next().is_none()
        && bytecode.eval_lexical_local_names().next().is_none()
        && !bytecode.creates_closures()
        && !bytecode
            .code
            .iter()
            .any(|op| matches!(op, ir::Op::NewTemplateObject { .. }))
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
    upvalues: Vec<crate::function::Upvalue>,
    with_stack: Vec<Value>,
    persist_global_lexicals: bool,
) -> FunctionBytecodeResult<'_> {
    vm::eval_function_bytecode(
        bytecode,
        env,
        upvalues,
        with_stack,
        persist_global_lexicals,
        None,
    )
}

/// The immutable source of received-upvalue cells for a slot-seeded call.
///
/// User functions retain their upvalue vector for their whole lifetime. A
/// direct frame that statically only reads those cells can keep one `Function`
/// handle instead of allocating a local-sized option vector and cloning every
/// cell handle. Function literals without an allocated `Function` retain the
/// ordinary slice representation.
#[derive(Clone, Copy)]
pub(crate) enum DirectCallUpvalues<'a> {
    Function(&'a crate::Function),
    Slice(&'a [crate::function::Upvalue]),
}

impl<'a> DirectCallUpvalues<'a> {
    pub(crate) fn as_slice(self) -> &'a [crate::function::Upvalue] {
        match self {
            Self::Function(function) => &function.upvalues,
            Self::Slice(upvalues) => upvalues,
        }
    }

    pub(crate) fn function(self) -> Option<&'a crate::Function> {
        match self {
            Self::Function(function) => Some(function),
            Self::Slice(_) => None,
        }
    }
}

pub(crate) struct DirectCallSlots<'a> {
    pub(crate) this_value: Option<Value>,
    pub(crate) parameter_slots: &'a [usize],
    pub(crate) arguments: &'a [Value],
    pub(crate) upvalues: DirectCallUpvalues<'a>,
    pub(crate) realm_upvalue_slots: u128,
}

/// Runs a slot-seeded ordinary call whose caller needs only its completion
/// value. Direct-call eligibility excludes the compatibility bindings that a
/// general call must copy back to its caller, so materializing a completed
/// `FunctionBytecodeResult` would only move and then drop the finished frame.
pub(crate) fn eval_direct_call_bytecode(
    bytecode: &Bytecode,
    env: crate::CallEnv,
    direct_call_slots: DirectCallSlots<'_>,
) -> Result<Value, RuntimeError> {
    vm::eval_direct_call_bytecode(bytecode, env, direct_call_slots)
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

/// Evaluates script bytecode inside a Test262 `$262.agent` whose
/// `AgentCanSuspend()` is `can_block` (see
/// [`vm_import::eval_bytecode_with_module_resolver_in_agent`]).
///
/// # Errors
///
/// Returns runtime failures or malformed bytecode failures.
#[cfg(feature = "agents")]
pub fn eval_bytecode_with_module_resolver_in_agent(
    bytecode: &Bytecode,
    referrer: &str,
    resolver: Box<dyn crate::ModuleResolver>,
    can_block: bool,
) -> Result<Value, RuntimeError> {
    vm_import::eval_bytecode_with_module_resolver_in_agent(bytecode, referrer, resolver, can_block)
}

/// Evaluates a worker agent's script bytecode with `context` installed (see
/// [`vm_import::eval_bytecode_in_agent_context`]).
///
/// # Errors
///
/// Returns runtime failures or malformed bytecode failures.
#[cfg(feature = "agents")]
pub fn eval_bytecode_in_agent_context(
    bytecode: &Bytecode,
    context: crate::agent::AgentContextRef,
) -> Result<Value, RuntimeError> {
    vm_import::eval_bytecode_in_agent_context(bytecode, context)
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
    pub(crate) live_bindings: crate::function::DynamicBindings,
    pub(crate) async_result_promise: Option<crate::ObjectRef>,
}

pub(crate) struct ModuleLiveExports {
    pub(crate) names: Vec<String>,
    pub(crate) bindings: crate::function::DynamicBindings,
    pub(crate) seed_tdz_markers: bool,
    pub(crate) imports: Vec<ModuleLiveImport>,
}

pub(crate) struct ModuleLiveImport {
    pub(crate) local_name: String,
    pub(crate) bindings: crate::function::DynamicBindings,
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
    let with_stack = env.direct_eval_with_stack();
    vm::eval_function_bytecode(bytecode, env, Vec::new(), with_stack, true, None)
}

pub(crate) fn eval_bytecode_with_env_ephemeral_global_lexicals(
    bytecode: &Bytecode,
    env: crate::CallEnv,
) -> FunctionBytecodeResult<'_> {
    let with_stack = env.direct_eval_with_stack();
    vm::eval_function_bytecode(bytecode, env, Vec::new(), with_stack, false, None)
}
