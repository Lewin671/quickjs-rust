use std::collections::HashMap;

use qjs_ast::{ForInLeft, ForInit, FunctionParams, Script, Stmt, VarKind};

use crate::{
    RuntimeError, Value,
    function::{is_strict_function_body, parameter_binding_name, rest_parameter_binding_name},
};

use super::compiler_lexical::{
    catch_param_annex_b_blocked_names, function_body_annex_b_blocked_names, function_param_names,
    lexical_declared_names, switch_lexical_declared_names,
};
use super::compiler_try::block_has_sync_using;
use super::ir::{Bytecode, Local, Op};
use super::util::{stmt_accepts_pending_label, stmt_updates_statement_list_completion};

pub(super) struct Compiler {
    pub(super) constants: Vec<Value>,
    pub(super) locals: Vec<Local>,
    pub(super) local_slots: HashMap<String, usize>,
    pub(super) lexical_scopes: Vec<HashMap<String, usize>>,
    pub(super) code: Vec<Op>,
    loop_stack: Vec<LoopContext>,
    pending_labels: Vec<String>,
    next_temp: usize,
    pub(super) strict: bool,
    pub(super) global_scope: bool,
    /// Names of `var`/function declarations hoisted at global script scope.
    /// They live in the realm (and on `globalThis`), not frame slots, so
    /// closures, direct eval, and promise jobs all observe one binding.
    pub(super) global_hoisted: std::collections::HashSet<String>,
    annex_b_blocked_function_names: Vec<Vec<String>>,
    /// Count of `with` scopes currently open around the code being compiled.
    /// Break/continue/return that leave one or more of them emit `Op::ExitWith`
    /// for each scope crossed, keeping the VM's with-object stack balanced.
    pub(super) with_depth: usize,
    /// `with` scopes captured from an enclosing function-creation environment;
    /// present when the frame starts, so returns must not `ExitWith` them.
    pub(super) with_base_depth: usize,
    /// Set when an invalid `/.../` regexp literal aborted compilation (a
    /// parse-phase error, not a generic early error).
    pub(super) regexp_literal_error: bool,
    /// Whether the current function body is an async generator. `yield*` uses
    /// the async iterator protocol only in this context.
    pub(super) async_generator_body: bool,
    /// Stack of try/catch/finally result slot contexts. When a `break` or
    /// `continue` crosses a try boundary, the innermost try result slot must
    /// be propagated to the target loop's result slot so UpdateEmpty semantics
    /// are preserved.
    pub(super) try_result_slots: Vec<TryResultEntry>,
    /// Open `using` disposal scopes around the code being compiled. A `using`
    /// initializer emits `RegisterDisposable` only when this is non-zero (i.e.
    /// its block opened a disposal scope), so contexts not yet wired for
    /// disposal never emit an unmatched register.
    pub(super) disposable_scope_depth: usize,
}

/// Tracks a try/catch/finally result slot for completion value propagation.
pub(super) struct TryResultEntry {
    /// The local slot tracking this block's accumulated completion value.
    pub(super) result_slot: usize,
    /// The loop stack depth when the try/finally block was entered. A
    /// break/continue targeting a loop below this depth propagates this slot.
    pub(super) loop_depth: usize,
    /// Whether this entry is for a `finally` block. Break/continue inside
    /// finally must emit `DiscardPendingAbrupt` to clear stale pending
    /// throw/return state.
    pub(super) is_finally: bool,
}

#[derive(Default)]
pub(super) struct LoopContext {
    result_slot: usize,
    allows_continue: bool,
    labels: Vec<String>,
    breaks: Vec<usize>,
    continues: Vec<usize>,
    iterator: Option<LoopIterator>,
    pub(super) captured_env_scope: bool,
    /// The compiler's `with_depth` when this loop was entered. A break or
    /// continue targeting it must close every `with` scope opened since.
    with_depth: usize,
}

/// Live iterator state for a `for-of` loop that must be closed when an
/// abrupt completion leaves the loop.
#[derive(Clone, Copy)]
pub(super) struct LoopIterator {
    pub(super) iterator_slot: usize,
    pub(super) done_slot: usize,
}

impl Default for Compiler {
    fn default() -> Self {
        Self {
            constants: Vec::new(),
            locals: Vec::new(),
            local_slots: HashMap::new(),
            lexical_scopes: vec![HashMap::new()],
            code: Vec::new(),
            loop_stack: Vec::new(),
            pending_labels: Vec::new(),
            next_temp: 0,
            strict: false,
            global_scope: true,
            global_hoisted: std::collections::HashSet::new(),
            annex_b_blocked_function_names: Vec::new(),
            with_depth: 0,
            with_base_depth: 0,
            regexp_literal_error: false,
            async_generator_body: false,
            try_result_slots: Vec::new(),
            disposable_scope_depth: 0,
        }
    }
}

pub(super) fn compile_script(script: &Script) -> Result<Bytecode, super::CompileError> {
    let mut compiler = Compiler::default();
    let result = compiler.compile_into(script);
    let parse_stage = compiler.regexp_literal_error;
    result.map_err(|error| super::CompileError { error, parse_stage })
}

pub(super) fn compile_direct_eval_script(
    script: &Script,
    strict: bool,
) -> Result<Bytecode, RuntimeError> {
    let mut compiler = Compiler {
        global_scope: false,
        strict,
        ..Compiler::default()
    };
    compiler.compile_eval_into(script)
}

/// Compiles a module body. Module code is global-scope bytecode (its top-level
/// `var`/function bindings live in the module realm) but always strict mode,
/// regardless of a leading directive prologue.
pub(super) fn compile_module(script: &Script) -> Result<Bytecode, RuntimeError> {
    let mut compiler = Compiler {
        strict: true,
        ..Compiler::default()
    };
    compiler.compile_into(script)
}

pub(super) fn compile_function_body(
    params: &FunctionParams,
    body: &[Stmt],
) -> Result<Bytecode, RuntimeError> {
    compile_function_body_with_strict(params, body, false)
}

pub(super) fn compile_function_body_with_strict(
    params: &FunctionParams,
    body: &[Stmt],
    parent_strict: bool,
) -> Result<Bytecode, RuntimeError> {
    Compiler {
        strict: parent_strict,
        ..Compiler::default()
    }
    .compile_function(params, body)
}

/// Compiles a function body, carrying generator/async-ness on the resulting
/// function value (`yield` is parser-gated and lowers to `Op::Yield`).
pub(super) fn compile_function_body_with_strict_generator(
    params: &FunctionParams,
    body: &[Stmt],
    parent_strict: bool,
    is_generator: bool,
    is_async: bool,
) -> Result<Bytecode, RuntimeError> {
    Compiler::function_compiler(parent_strict, is_generator && is_async)
        .compile_function(params, body)
}

impl Compiler {
    pub(super) fn strict_function_compiler() -> Self {
        Self {
            strict: true,
            global_scope: false,
            ..Self::default()
        }
    }

    pub(super) fn function_compiler(strict: bool, async_generator_body: bool) -> Self {
        Self {
            strict,
            async_generator_body,
            ..Self::default()
        }
    }

    pub(super) fn function_compiler_with_base_with_depth(
        strict: bool,
        async_generator_body: bool,
        with_base_depth: usize,
    ) -> Self {
        Self {
            strict,
            async_generator_body,
            with_depth: with_base_depth,
            with_base_depth,
            ..Self::default()
        }
    }

    fn compile_into(&mut self, script: &Script) -> Result<Bytecode, RuntimeError> {
        self.strict = self.strict || is_strict_function_body(&script.body);
        self.collect_hoisted_locals(&script.body, false);
        self.predeclare_current_scope_lexicals(&script.body);
        let blocked = lexical_declared_names(&script.body);
        let global_lexical_names = blocked.clone();
        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
            compiler.compile_hoisted_function_decls(&script.body)?;
            compiler.compile_script_statement_list(&script.body)?;
            Ok(())
        })?;
        self.code.push(Op::Return);
        Ok(Bytecode::with_scope_global_lexical_names_and_strict(
            std::mem::take(&mut self.constants),
            std::mem::take(&mut self.locals),
            std::mem::take(&mut self.code),
            true,
            global_lexical_names,
            self.strict,
        ))
    }

    fn compile_eval_into(&mut self, script: &Script) -> Result<Bytecode, RuntimeError> {
        self.strict = self.strict || is_strict_function_body(&script.body);
        self.collect_hoisted_locals(&script.body, false);
        self.predeclare_current_scope_lexicals(&script.body);
        let blocked = lexical_declared_names(&script.body);
        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
            compiler.compile_hoisted_function_decls(&script.body)?;
            compiler.compile_script_statement_list(&script.body)?;
            Ok(())
        })?;
        self.code.push(Op::Return);
        Ok(Bytecode::with_scope_global_lexical_names_and_strict(
            std::mem::take(&mut self.constants),
            std::mem::take(&mut self.locals),
            std::mem::take(&mut self.code),
            false,
            blocked,
            self.strict,
        ))
    }

    pub(super) fn compile_function(
        mut self,
        params: &FunctionParams,
        body: &[Stmt],
    ) -> Result<Bytecode, RuntimeError> {
        self.global_scope = false;
        self.strict = self.strict || is_strict_function_body(body);
        for (index, element) in params.positional.iter().enumerate() {
            let binding_name = parameter_binding_name(&element.binding, index);
            self.parameter_slot(&binding_name);
        }
        if let Some(rest) = &params.rest {
            let binding_name = rest_parameter_binding_name(rest);
            self.parameter_slot(&binding_name);
        }
        let non_simple_params = !params.is_simple();
        if non_simple_params {
            self.snapshot_non_simple_parameter_arguments(params)?;
        }
        self.compile_parameter_bindings(params, non_simple_params)?;
        // Mark the end of parameter instantiation: generators run the prologue
        // synchronously at the call and suspend here; others skip past it.
        self.emit(Op::FunctionPrologueEnd);
        let param_blocked = function_param_names(params);
        self.with_annex_b_blocked_function_names(&param_blocked, |compiler| {
            compiler.collect_hoisted_locals(body, false);
            Ok(())
        })?;
        let blocked = function_body_annex_b_blocked_names(params, body);
        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
            compiler.predeclare_current_scope_lexicals(body);
            compiler.compile_hoisted_function_decls(body)?;
            if block_has_sync_using(body) {
                compiler.compile_statements_with_disposal(body)?;
            } else {
                for stmt in body {
                    compiler.compile_stmt(stmt)?;
                }
            }
            Ok(())
        })?;
        self.emit_load_undefined();
        self.code.push(Op::Return);
        Ok(Bytecode::new(self.constants, self.locals, self.code))
    }

    /// `nested` is true once recursion descends below the top level of the body
    /// (into a block, `if`, loop, `switch` case, `try`, `with`, or label) --
    /// where a function declaration is only Annex B sloppy-mode hoistable. The
    /// top-level entry points pass `false`.
    fn collect_hoisted_locals(&mut self, body: &[Stmt], nested: bool) {
        let blocked = lexical_declared_names(body);
        let pushed_blocked = !blocked.is_empty();
        if pushed_blocked {
            self.annex_b_blocked_function_names.push(blocked);
        }
        for stmt in body {
            match stmt {
                Stmt::Block { body, .. } => self.collect_hoisted_locals(body, true),
                Stmt::If {
                    consequent,
                    alternate,
                    ..
                } => {
                    self.collect_hoisted_locals(std::slice::from_ref(consequent), true);
                    if let Some(alternate) = alternate {
                        self.collect_hoisted_locals(std::slice::from_ref(alternate), true);
                    }
                }
                Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
                    self.collect_hoisted_locals(std::slice::from_ref(body), true);
                }
                Stmt::For { init, body, .. } => {
                    if let Some(ForInit::VarDecl {
                        declarations, kind, ..
                    }) = init
                    {
                        for declaration in declarations {
                            if *kind == VarKind::Var {
                                for name in declaration.binding.names() {
                                    self.local_slot(&name, true);
                                }
                            }
                        }
                    }
                    self.collect_hoisted_locals(std::slice::from_ref(body), true);
                }
                Stmt::ForIn { left, body, .. } | Stmt::ForOf { left, body, .. } => {
                    if let ForInLeft::VarDecl {
                        binding,
                        kind: VarKind::Var,
                        ..
                    } = left
                    {
                        for name in binding.names() {
                            self.local_slot(&name, true);
                        }
                    }
                    self.collect_hoisted_locals(std::slice::from_ref(body), true);
                }
                Stmt::FunctionDecl { name, .. } => {
                    // Nested-block functions hoist to the var scope only under
                    // Annex B (sloppy mode); strict code keeps them block-scoped.
                    if (!nested || !self.strict) && !self.annex_b_function_name_blocked(name) {
                        if nested {
                            self.local_slot(name, true);
                        } else {
                            self.hoisted_function_slot(name);
                        }
                    }
                }
                Stmt::Labelled { body, .. } | Stmt::With { body, .. } => {
                    self.collect_hoisted_locals(std::slice::from_ref(body), true);
                }
                Stmt::VarDecl {
                    kind: VarKind::Var,
                    declarations,
                    ..
                } => {
                    for declaration in declarations {
                        for name in declaration.binding.names() {
                            self.local_slot(&name, true);
                        }
                    }
                }
                Stmt::Switch { cases, .. } => {
                    let blocked = switch_lexical_declared_names(cases);
                    let pushed_switch_blocked = !blocked.is_empty();
                    if pushed_switch_blocked {
                        self.annex_b_blocked_function_names.push(blocked);
                    }
                    for case in cases {
                        self.collect_hoisted_locals(&case.consequent, true);
                    }
                    if pushed_switch_blocked {
                        self.annex_b_blocked_function_names
                            .pop()
                            .expect("Annex B function blocklist stack should be balanced");
                    }
                }
                Stmt::Try {
                    block,
                    handler,
                    finalizer,
                    ..
                } => {
                    self.collect_hoisted_locals(block, true);
                    if let Some(handler) = handler {
                        let blocked = catch_param_annex_b_blocked_names(handler.param.as_ref());
                        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
                            compiler.collect_hoisted_locals(&handler.body, true);
                            Ok(())
                        })
                        .expect("hoisted local collection should not fail");
                    }
                    if let Some(finalizer) = finalizer {
                        self.collect_hoisted_locals(finalizer, true);
                    }
                }
                Stmt::Expr(_)
                | Stmt::Return { .. }
                | Stmt::Throw { .. }
                | Stmt::Debugger { .. }
                | Stmt::Break { .. }
                | Stmt::Continue { .. }
                | Stmt::VarDecl { .. }
                | Stmt::ClassDecl { .. }
                | Stmt::ModuleDecl(_)
                | Stmt::Empty => {}
            }
        }
        if pushed_blocked {
            self.annex_b_blocked_function_names
                .pop()
                .expect("Annex B function blocklist stack should be balanced");
        }
    }

    pub(super) fn local_slot(&mut self, name: &str, hoisted: bool) -> usize {
        if hoisted && self.global_scope {
            self.global_hoisted.insert(name.to_owned());
        }
        if let Some(slot) = self.local_slots.get(name) {
            if hoisted {
                self.locals[*slot].hoisted = true;
            }
            return *slot;
        }
        let slot = self.locals.len();
        self.locals.push(Local {
            name: name.to_owned(),
            hoisted,
            hoisted_function: false,
            parameter: false,
            mutable: true,
            from_env: true,
            sloppy_global_fallback: false,
        });
        self.local_slots.insert(name.to_owned(), slot);
        slot
    }

    fn hoisted_function_slot(&mut self, name: &str) -> usize {
        let slot = self.local_slot(name, true);
        self.locals[slot].hoisted_function = true;
        slot
    }

    pub(super) fn parameter_slot(&mut self, name: &str) -> usize {
        let slot = self.local_slot(name, true);
        self.locals[slot].parameter = true;
        slot
    }

    pub(super) fn with_annex_b_blocked_function_names<T>(
        &mut self,
        names: &[String],
        compile: impl FnOnce(&mut Self) -> Result<T, RuntimeError>,
    ) -> Result<T, RuntimeError> {
        if names.is_empty() {
            return compile(self);
        }
        self.annex_b_blocked_function_names.push(names.to_vec());
        let result = compile(self);
        self.annex_b_blocked_function_names
            .pop()
            .expect("Annex B function blocklist stack should be balanced");
        result
    }

    pub(super) fn annex_b_function_name_blocked(&self, name: &str) -> bool {
        self.annex_b_blocked_function_names
            .iter()
            .rev()
            .any(|names| names.iter().any(|blocked| blocked == name))
    }

    pub(super) fn annex_b_function_name_blocked_by_outer_scope(&self, name: &str) -> bool {
        self.annex_b_blocked_function_names
            .iter()
            .rev()
            .skip(1)
            .any(|names| names.iter().any(|blocked| blocked == name))
    }

    pub(super) fn annex_b_arguments_function_name_blocked(&self, name: &str) -> bool {
        name == "arguments" && self.annex_b_function_name_blocked(name)
    }

    pub(super) fn const_slot(&mut self, value: Value) -> usize {
        self.constants.push(value);
        self.constants.len() - 1
    }

    pub(super) fn emit(&mut self, op: Op) -> usize {
        self.code.push(op);
        self.code.len() - 1
    }

    pub(super) fn patch_jump(&mut self, at: usize, target: usize) {
        match &mut self.code[at] {
            Op::Jump(dest)
            | Op::JumpIfFalse(dest)
            | Op::JumpIfTrue(dest)
            | Op::JumpIfNotNullish(dest)
            | Op::AbruptJump(dest) => *dest = target,
            _ => unreachable!("attempted to patch a non-jump instruction"),
        }
    }

    pub(super) fn temp_local(&mut self, prefix: &str) -> usize {
        let name = format!("\0\0{prefix}_{}", self.next_temp);
        self.next_temp += 1;
        self.local_slot(&name, true)
    }

    pub(super) fn push_loop(&mut self, result_slot: usize) {
        let labels = self.take_pending_labels();
        self.loop_stack.push(LoopContext {
            result_slot,
            allows_continue: true,
            labels,
            with_depth: self.with_depth,
            ..LoopContext::default()
        });
    }

    pub(super) fn push_loop_with_iterator(&mut self, result_slot: usize, iterator: LoopIterator) {
        self.push_loop(result_slot);
        self.loop_stack
            .last_mut()
            .expect("loop context should exist after push")
            .iterator = Some(iterator);
    }

    pub(super) fn push_breakable(&mut self, result_slot: usize) {
        let labels = self.take_pending_labels();
        self.loop_stack.push(LoopContext {
            result_slot,
            allows_continue: false,
            labels,
            with_depth: self.with_depth,
            ..LoopContext::default()
        });
    }

    fn push_label(&mut self, label: String) {
        self.pending_labels.push(label);
    }

    fn take_pending_labels(&mut self) -> Vec<String> {
        std::mem::take(&mut self.pending_labels)
    }

    pub(super) fn pop_loop(&mut self) -> LoopContext {
        self.loop_stack
            .pop()
            .expect("loop context should be balanced")
    }

    pub(super) fn mark_loop_captured_env_scope(&mut self) {
        if let Some(context) = self.loop_stack.last_mut() {
            context.captured_env_scope = true;
        }
    }

    pub(super) fn compile_break(&mut self, label: Option<&str>) -> Result<(), RuntimeError> {
        let Some(index) = self.break_context_index(label) else {
            return Err(RuntimeError {
                thrown: None,
                message: label.map_or_else(
                    || "SyntaxError: Illegal break statement".to_owned(),
                    |label| format!("SyntaxError: undefined break label: {label}"),
                ),
            });
        };
        self.propagate_try_result_to_loop(index);
        self.propagate_current_completion_to(index);
        self.emit_loop_iterator_closes_above(index);
        self.emit_with_exits_above(self.loop_stack[index].with_depth);
        let result_slot = self.loop_stack[index].result_slot;
        self.emit(Op::LoadLocal(result_slot));
        let crosses_finally = self.crosses_finally_boundary(index);
        let jump = self.emit(if crosses_finally {
            Op::AbruptJump(usize::MAX)
        } else {
            Op::Jump(usize::MAX)
        });
        self.loop_stack[index].breaks.push(jump);
        Ok(())
    }

    pub(super) fn compile_continue(&mut self, label: Option<&str>) -> Result<(), RuntimeError> {
        let Some(index) = self.continue_context_index(label) else {
            return Err(RuntimeError {
                thrown: None,
                message: label.map_or_else(
                    || "SyntaxError: Illegal continue statement".to_owned(),
                    |label| format!("SyntaxError: undefined continue label: {label}"),
                ),
            });
        };
        self.propagate_try_result_to_loop(index);
        self.propagate_current_completion_to(index);
        self.emit_loop_iterator_closes_above(index);
        self.emit_with_exits_above(self.loop_stack[index].with_depth);
        let crosses_finally = self.crosses_finally_boundary(index);
        let jump = self.emit(if crosses_finally {
            Op::AbruptJump(usize::MAX)
        } else {
            Op::Jump(usize::MAX)
        });
        self.loop_stack[index].continues.push(jump);
        Ok(())
    }

    /// Closes the live for-of iterators of every loop context nested inside
    /// the target context, innermost first, exiting their protected regions.
    /// Used when a labelled break or continue crosses for-of loops.
    fn emit_loop_iterator_closes_above(&mut self, target_index: usize) {
        let iterators: Vec<LoopIterator> = self.loop_stack[target_index + 1..]
            .iter()
            .filter_map(|context| context.iterator)
            .collect();
        for iterator in iterators.into_iter().rev() {
            self.emit(Op::ExitTry);
            self.emit_close_unless_done(iterator.iterator_slot, iterator.done_slot, false);
        }
        let env_scope_count = self.loop_stack[target_index + 1..]
            .iter()
            .filter(|context| context.captured_env_scope)
            .count();
        for _ in 0..env_scope_count {
            self.emit(Op::PopCapturedEnv);
        }
    }

    /// Closes every live for-of iterator before a `return` leaves the
    /// function body, innermost first.
    fn emit_loop_iterator_closes_for_return(&mut self) {
        let iterators: Vec<LoopIterator> = self
            .loop_stack
            .iter()
            .filter_map(|context| context.iterator)
            .collect();
        for iterator in iterators.into_iter().rev() {
            self.emit(Op::ExitTry);
            self.emit_close_unless_done(iterator.iterator_slot, iterator.done_slot, false);
        }
        let env_scope_count = self
            .loop_stack
            .iter()
            .filter(|context| context.captured_env_scope)
            .count();
        for _ in 0..env_scope_count {
            self.emit(Op::PopCapturedEnv);
        }
    }

    fn break_context_index(&self, label: Option<&str>) -> Option<usize> {
        match label {
            Some(label) => self
                .loop_stack
                .iter()
                .rposition(|context| context.labels.iter().any(|item| item == label)),
            None => self.loop_stack.len().checked_sub(1),
        }
    }

    fn continue_context_index(&self, label: Option<&str>) -> Option<usize> {
        self.loop_stack.iter().rposition(|context| {
            context.allows_continue
                && label.is_none_or(|label| context.labels.iter().any(|item| item == label))
        })
    }

    /// Returns the current loop stack depth, used by try/catch/finally
    /// compilation to record the nesting level at which a try block was entered.
    pub(super) fn loop_stack_depth(&self) -> usize {
        self.loop_stack.len()
    }

    /// Pushes a try/finally result slot onto the tracking stack with the
    /// current loop nesting depth. Must be balanced with `pop_try_result_slot`.
    pub(super) fn push_try_result_slot(
        &mut self,
        result_slot: usize,
        loop_depth: usize,
        is_finally: bool,
    ) {
        self.try_result_slots.push(TryResultEntry {
            result_slot,
            loop_depth,
            is_finally,
        });
    }

    /// Pops the most recent try/finally result slot from the tracking stack.
    pub(super) fn pop_try_result_slot(&mut self) {
        self.try_result_slots
            .pop()
            .expect("try result slot stack should be balanced");
    }

    /// When a `break` or `continue` crosses a try/catch/finally boundary,
    /// propagate the innermost try result slot to the target loop's result
    /// slot. This implements the UpdateEmpty semantics for try statements:
    /// the try's accumulated completion value becomes the break's value.
    /// Also emits `DiscardPendingAbrupt` when crossing a finally boundary
    /// to clear any stale pending throw/return.
    fn propagate_try_result_to_loop(&mut self, target_loop_index: usize) {
        // The innermost try_result_slot whose loop_depth exceeds
        // target_loop_index was entered inside the target loop, so a
        // break/continue crosses its boundary.
        if let Some(entry) = self
            .try_result_slots
            .iter()
            .rev()
            .find(|e| e.loop_depth > target_loop_index)
        {
            let try_slot = entry.result_slot;
            let is_finally = entry.is_finally;
            let target_slot = self.loop_stack[target_loop_index].result_slot;
            self.emit(Op::LoadLocal(try_slot));
            self.emit(Op::StoreLocal(target_slot));
            if is_finally {
                self.emit(Op::DiscardPendingAbrupt);
            }
        }
    }

    /// Returns true if a break/continue targeting `target_loop_index` would
    /// cross a try block boundary (the VM will check at runtime whether that
    /// try has a finally and route through it if so).
    fn crosses_finally_boundary(&self, target_loop_index: usize) -> bool {
        self.try_result_slots
            .iter()
            .rev()
            .any(|e| e.loop_depth > target_loop_index && !e.is_finally)
    }

    fn propagate_current_completion_to(&mut self, target_index: usize) {
        let Some(current_index) = self.loop_stack.len().checked_sub(1) else {
            return;
        };
        if current_index == target_index {
            return;
        }
        let current_slot = self.loop_stack[current_index].result_slot;
        let target_slot = self.loop_stack[target_index].result_slot;
        self.emit(Op::LoadLocal(current_slot));
        self.emit(Op::StoreLocal(target_slot));
    }

    pub(super) fn patch_loop_breaks(&mut self, context: &LoopContext, target: usize) {
        for jump in &context.breaks {
            self.patch_jump(*jump, target);
        }
    }

    pub(super) fn emit_scoped_loop_completion(
        &mut self,
        result_slot: usize,
        cleanup_slots: &[usize],
        context: &LoopContext,
    ) -> usize {
        if cleanup_slots.is_empty() {
            if context.captured_env_scope {
                self.emit(Op::PopCapturedEnv);
            }
            self.emit(Op::LoadLocal(result_slot));
            let done = self.code.len();
            self.patch_loop_breaks(context, done);
            return done;
        }

        for slot in cleanup_slots {
            self.emit(Op::ClearLocal(*slot));
        }
        if context.captured_env_scope {
            self.emit(Op::PopCapturedEnv);
        }
        self.emit(Op::LoadLocal(result_slot));
        let normal_done = self.emit(Op::Jump(usize::MAX));

        let break_cleanup = self.code.len();
        for slot in cleanup_slots {
            self.emit(Op::ClearLocal(*slot));
        }
        if context.captured_env_scope {
            self.emit(Op::PopCapturedEnv);
        }
        let break_done = self.emit(Op::Jump(usize::MAX));

        let done = self.code.len();
        self.patch_jump(normal_done, done);
        self.patch_jump(break_done, done);
        self.patch_loop_breaks(context, break_cleanup);
        done
    }

    pub(super) fn patch_loop_continues(&mut self, context: &LoopContext, target: usize) {
        for jump in &context.continues {
            self.patch_jump(*jump, target);
        }
    }

    pub(super) fn compile_stmt(&mut self, stmt: &Stmt) -> Result<(), RuntimeError> {
        match stmt {
            Stmt::Expr(expr) => self.compile_expr(expr),
            Stmt::Block { body, .. } => {
                if block_has_sync_using(body) {
                    self.compile_disposable_block(body)
                } else {
                    self.compile_block_body(body)
                }
            }
            Stmt::If {
                test,
                consequent,
                alternate,
                ..
            } => self.compile_if(test, consequent, alternate.as_deref()),
            Stmt::While { test, body, .. } => self.compile_while(test, body),
            Stmt::DoWhile { body, test, .. } => self.compile_do_while(body, test),
            Stmt::For {
                init,
                test,
                update,
                body,
                ..
            } => self.compile_for(init.as_ref(), test.as_ref(), update.as_ref(), body),
            Stmt::ForIn {
                left, right, body, ..
            } => self.compile_for_in(left, right, body),
            Stmt::ForOf {
                left,
                right,
                body,
                is_await,
                ..
            } => {
                if *is_await {
                    return self.compile_for_await_of(left, right, body);
                }
                self.compile_for_of(left, right, body)
            }
            Stmt::Return { argument, .. } => {
                if let Some(argument) = argument {
                    self.compile_expr(argument)?;
                } else {
                    self.emit_load_undefined();
                }
                self.emit_loop_iterator_closes_for_return();
                self.emit_with_exits_above(self.with_base_depth);
                self.emit(Op::Return);
                Ok(())
            }
            Stmt::Throw { argument, .. } => {
                if let Some(argument) = argument {
                    self.compile_expr(argument)?;
                } else {
                    self.emit_load_undefined();
                }
                self.emit(Op::Throw);
                Ok(())
            }
            Stmt::Debugger { .. } | Stmt::Empty => {
                self.emit_load_undefined();
                Ok(())
            }
            Stmt::VarDecl {
                kind, declarations, ..
            } => {
                for declaration in declarations {
                    for name in declaration.binding.names() {
                        let slot = self.declare_var_kind_slot(&name, *kind);
                        if kind.is_lexical() {
                            self.emit(Op::ClearLocal(slot));
                        }
                    }
                    let has_init = declaration.init.is_some();
                    if let Some(init) = &declaration.init {
                        self.compile_declaration_init(&declaration.binding, init)?;
                    } else {
                        self.emit_load_undefined();
                    }
                    // A sync `using` registers its initializer value with the
                    // enclosing disposal scope before the binding store consumes
                    // it (RegisterDisposable inspects the stack top in place).
                    if *kind == VarKind::Using && self.disposable_scope_depth > 0 {
                        self.emit(Op::RegisterDisposable);
                    }
                    if has_init {
                        self.compile_binding_initializer(&declaration.binding, *kind)?;
                    } else {
                        self.compile_binding_uninitialized(&declaration.binding, *kind)?;
                    }
                }
                self.emit_load_undefined();
                Ok(())
            }
            Stmt::With { object, body, .. } => self.compile_with(object, body),
            Stmt::Labelled { label, body, .. } => self.compile_labelled(label, body),
            Stmt::Break { label, .. } => self.compile_break(label.as_deref()),
            Stmt::Continue { label, .. } => self.compile_continue(label.as_deref()),
            Stmt::Switch {
                discriminant,
                cases,
                ..
            } => self.compile_switch(discriminant, cases),
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => self.compile_try(block, handler.as_ref(), finalizer.as_deref()),
            Stmt::FunctionDecl { .. } => {
                self.emit_load_undefined();
                Ok(())
            }
            Stmt::ClassDecl { name, body, .. } => {
                // Class declarations create a lexical (`let`-like) binding.
                let slot = self.declare_lexical_slot(name, true);
                self.emit(Op::ClearLocal(slot));
                self.with_lexical_scope(|compiler| {
                    let storage_name =
                        format!("\0class_decl_inner:{}:{}", name, compiler.locals.len());
                    let inner_slot =
                        compiler.declare_lexical_slot_with_storage_name(name, &storage_name, false);
                    compiler.emit(Op::ClearLocal(inner_slot));
                    compiler.compile_class(Some(name), body)?;
                    compiler.emit(Op::Dup);
                    compiler.emit(Op::StoreLocal(inner_slot));
                    Ok(())
                })?;
                self.emit(Op::StoreLocal(slot));
                self.emit_load_undefined();
                Ok(())
            }
            Stmt::ModuleDecl(_) => Err(super::util::unsupported_module_item()),
        }
    }

    fn compile_script_statement_list(&mut self, body: &[Stmt]) -> Result<(), RuntimeError> {
        let result_slot = self.temp_local("script_result");
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
        for stmt in body {
            self.compile_stmt(stmt)?;
            if stmt_updates_statement_list_completion(stmt) {
                self.emit(Op::StoreLocal(result_slot));
            } else {
                self.emit(Op::Pop);
            }
        }
        self.emit(Op::LoadLocal(result_slot));
        Ok(())
    }

    pub(super) fn store_or_pop_statement_list_completion(&mut self, stmt: &Stmt) {
        if let Some(result_slot) = self.current_loop_result_slot()
            && stmt_updates_statement_list_completion(stmt)
        {
            self.emit(Op::StoreLocal(result_slot));
        } else {
            self.emit(Op::Pop);
        }
    }

    pub(super) fn reset_current_loop_completion_to_undefined(&mut self) {
        if let Some(result_slot) = self.current_loop_result_slot() {
            self.emit_load_undefined();
            self.emit(Op::StoreLocal(result_slot));
        }
    }

    fn current_loop_result_slot(&self) -> Option<usize> {
        self.loop_stack.last().map(|context| context.result_slot)
    }

    /// Whether the code currently being compiled is inside a `with` body, so
    /// identifier references must consult the with-object stack at runtime.
    pub(super) fn inside_with(&self) -> bool {
        self.with_depth > 0
    }

    pub(super) fn inside_current_with(&self) -> bool {
        self.with_depth > self.with_base_depth
    }

    pub(super) fn identifier_needs_with_resolution(&self, slot: Option<usize>) -> bool {
        self.inside_current_with() || (slot.is_none() && self.inside_with())
    }

    fn compile_labelled(&mut self, label: &str, body: &Stmt) -> Result<(), RuntimeError> {
        if stmt_accepts_pending_label(body) {
            self.push_label(label.to_owned());
            return self.compile_stmt(body);
        }

        let result_slot = self.temp_local("label_result");
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
        self.push_label(label.to_owned());
        self.push_breakable(result_slot);
        self.compile_stmt(body)?;
        self.emit(Op::StoreLocal(result_slot));
        let context = self.pop_loop();
        self.emit(Op::LoadLocal(result_slot));
        let done = self.code.len();
        self.patch_loop_breaks(&context, done);
        Ok(())
    }
}
