use std::collections::HashMap;

use qjs_ast::{ForInLeft, ForInit, FunctionParams, Script, Stmt, VarKind};

use crate::{
    RuntimeError, Value,
    function::{is_strict_function_body, parameter_binding_name, rest_parameter_binding_name},
};

use super::compiler_lexical::{
    catch_param_annex_b_blocked_names, current_scope_lexical_declared_bindings,
    function_body_annex_b_blocked_names, function_param_names, lexical_declared_names,
    nested_block_annex_b_blocked_names, switch_lexical_declared_names,
};
use super::ir::{Bytecode, Local, Op};
use super::util::{stmt_accepts_pending_label, stmt_updates_statement_list_completion};

pub(super) struct Compiler {
    pub(super) constants: Vec<Value>,
    pub(super) locals: Vec<Local>,
    pub(super) local_slots: HashMap<String, usize>,
    lexical_scopes: Vec<HashMap<String, usize>>,
    pub(super) code: Vec<Op>,
    loop_stack: Vec<LoopContext>,
    pending_labels: Vec<String>,
    next_temp: usize,
    pub(super) strict: bool,
    pub(super) global_scope: bool,
    /// Names of `var`/function declarations hoisted at global script scope.
    /// They live in the realm (and on `globalThis`), not in frame slots, so
    /// deferred closures, direct eval, and promise jobs observe one binding.
    global_hoisted: std::collections::HashSet<String>,
    annex_b_blocked_function_names: Vec<Vec<String>>,
    /// Count of `with` scopes currently open around the code being compiled.
    /// Break/continue/return that leave one or more of them emit `Op::ExitWith`
    /// for each scope crossed, keeping the VM's with-object stack balanced.
    pub(super) with_depth: usize,
    /// Set when an invalid `/.../` regexp literal aborted compilation; such
    /// literals are parse-phase errors, not generic early errors.
    pub(super) regexp_literal_error: bool,
    /// Whether the current function body is an async generator. `yield*` uses
    /// the async iterator protocol only in this context.
    pub(super) async_generator_body: bool,
}

#[derive(Default)]
pub(super) struct LoopContext {
    result_slot: usize,
    allows_continue: bool,
    labels: Vec<String>,
    breaks: Vec<usize>,
    continues: Vec<usize>,
    iterator: Option<LoopIterator>,
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
            regexp_literal_error: false,
            async_generator_body: false,
        }
    }
}

pub(super) fn compile_script(script: &Script) -> Result<Bytecode, super::CompileError> {
    let mut compiler = Compiler::default();
    let result = compiler.compile_into(script);
    let parse_stage = compiler.regexp_literal_error;
    result.map_err(|error| super::CompileError { error, parse_stage })
}

pub(super) fn compile_direct_eval_script(script: &Script) -> Result<Bytecode, RuntimeError> {
    let mut compiler = Compiler {
        global_scope: false,
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

/// Compiles a function body. Generator bodies compile through the same path:
/// `yield` is already gated by the parser and lowers to `Op::Yield`, while the
/// generator-ness is carried on the resulting function value, not the bytecode.
/// This wrapper documents the intent at the call sites that handle `*`.
pub(super) fn compile_function_body_with_strict_generator(
    params: &FunctionParams,
    body: &[Stmt],
    parent_strict: bool,
    is_generator: bool,
    is_async: bool,
) -> Result<Bytecode, RuntimeError> {
    Compiler {
        strict: parent_strict,
        async_generator_body: is_generator && is_async,
        ..Compiler::default()
    }
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

    fn compile_into(&mut self, script: &Script) -> Result<Bytecode, RuntimeError> {
        self.strict = self.strict || is_strict_function_body(&script.body);
        self.collect_hoisted_locals(&script.body);
        self.predeclare_current_scope_lexicals(&script.body);
        let blocked = lexical_declared_names(&script.body);
        let global_lexical_names = blocked.clone();
        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
            compiler.compile_hoisted_function_decls(&script.body)?;
            for stmt in &script.body {
                compiler.compile_stmt(stmt)?;
            }
            Ok(())
        })?;
        self.code.push(Op::Return);
        Ok(Bytecode::with_scope_and_global_lexical_names(
            std::mem::take(&mut self.constants),
            std::mem::take(&mut self.locals),
            std::mem::take(&mut self.code),
            true,
            global_lexical_names,
        ))
    }

    fn compile_eval_into(&mut self, script: &Script) -> Result<Bytecode, RuntimeError> {
        self.strict = self.strict || is_strict_function_body(&script.body);
        self.collect_hoisted_locals(&script.body);
        self.predeclare_current_scope_lexicals(&script.body);
        let blocked = lexical_declared_names(&script.body);
        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
            compiler.compile_hoisted_function_decls(&script.body)?;
            for stmt in &script.body {
                compiler.compile_stmt(stmt)?;
            }
            Ok(())
        })?;
        self.code.push(Op::Return);
        Ok(Bytecode::with_scope_and_global_lexical_names(
            std::mem::take(&mut self.constants),
            std::mem::take(&mut self.locals),
            std::mem::take(&mut self.code),
            false,
            blocked,
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
            self.local_slot(&binding_name, true);
        }
        if let Some(rest) = &params.rest {
            let binding_name = rest_parameter_binding_name(rest);
            self.local_slot(&binding_name, true);
        }
        let non_simple_params = !params.is_simple();
        if non_simple_params {
            self.snapshot_non_simple_parameter_arguments(params)?;
        }
        self.compile_parameter_bindings(params, non_simple_params)?;
        // Mark the end of parameter instantiation. Generators/async generators
        // run their prologue synchronously at the call and suspend here; other
        // functions skip past it.
        self.emit(Op::FunctionPrologueEnd);
        let param_blocked = function_param_names(params);
        self.with_annex_b_blocked_function_names(&param_blocked, |compiler| {
            compiler.collect_hoisted_locals(body);
            Ok(())
        })?;
        let blocked = function_body_annex_b_blocked_names(params, body);
        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
            compiler.predeclare_current_scope_lexicals(body);
            compiler.compile_hoisted_function_decls(body)?;
            for stmt in body {
                compiler.compile_stmt(stmt)?;
            }
            Ok(())
        })?;
        self.emit_load_undefined();
        self.code.push(Op::Return);
        Ok(Bytecode::new(self.constants, self.locals, self.code))
    }

    fn collect_hoisted_locals(&mut self, body: &[Stmt]) {
        let blocked = lexical_declared_names(body);
        let pushed_blocked = !blocked.is_empty();
        if pushed_blocked {
            self.annex_b_blocked_function_names.push(blocked);
        }
        for stmt in body {
            match stmt {
                Stmt::Block { body, .. } => self.collect_hoisted_locals(body),
                Stmt::If {
                    consequent,
                    alternate,
                    ..
                } => {
                    self.collect_hoisted_locals(std::slice::from_ref(consequent));
                    if let Some(alternate) = alternate {
                        self.collect_hoisted_locals(std::slice::from_ref(alternate));
                    }
                }
                Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
                    self.collect_hoisted_locals(std::slice::from_ref(body));
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
                    self.collect_hoisted_locals(std::slice::from_ref(body));
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
                    self.collect_hoisted_locals(std::slice::from_ref(body));
                }
                Stmt::FunctionDecl { name, .. } => {
                    if !self.annex_b_function_name_blocked(name) {
                        self.local_slot(name, true);
                    }
                }
                Stmt::Labelled { body, .. } | Stmt::With { body, .. } => {
                    self.collect_hoisted_locals(std::slice::from_ref(body));
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
                        self.collect_hoisted_locals(&case.consequent);
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
                    self.collect_hoisted_locals(block);
                    if let Some(handler) = handler {
                        let blocked = catch_param_annex_b_blocked_names(handler.param.as_ref());
                        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
                            compiler.collect_hoisted_locals(&handler.body);
                            Ok(())
                        })
                        .expect("hoisted local collection should not fail");
                    }
                    if let Some(finalizer) = finalizer {
                        self.collect_hoisted_locals(finalizer);
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
            mutable: true,
            from_env: true,
        });
        self.local_slots.insert(name.to_owned(), slot);
        slot
    }

    pub(super) fn declare_lexical_slot(&mut self, name: &str, mutable: bool) -> usize {
        if let Some(slot) = self.current_lexical_scope().get(name) {
            return *slot;
        }
        let slot = self.locals.len();
        self.locals.push(Local {
            name: name.to_owned(),
            hoisted: false,
            mutable,
            from_env: false,
        });
        self.current_lexical_scope_mut()
            .insert(name.to_owned(), slot);
        slot
    }

    pub(super) fn declare_captured_lexical_slot(&mut self, name: &str, mutable: bool) -> usize {
        if let Some(slot) = self.current_lexical_scope().get(name) {
            return *slot;
        }
        let slot = self.locals.len();
        self.locals.push(Local {
            name: name.to_owned(),
            hoisted: false,
            mutable,
            from_env: true,
        });
        self.current_lexical_scope_mut()
            .insert(name.to_owned(), slot);
        slot
    }

    pub(super) fn resolve_local_slot(&self, name: &str) -> Option<usize> {
        let lexical = self
            .lexical_scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied());
        if lexical.is_some() {
            return lexical;
        }
        // Global-scope `var`/function bindings live in the realm, not in
        // frame slots: every identifier reference compiles to a global op so
        // eval'd code and deferred jobs share the same binding.
        if self.global_scope && self.global_hoisted.contains(name) {
            return None;
        }
        self.local_slots.get(name).copied()
    }

    pub(super) fn is_global_hoisted(&self, name: &str) -> bool {
        self.global_scope && self.global_hoisted.contains(name)
    }

    pub(super) fn assignment_slot(&mut self, name: &str) -> usize {
        self.resolve_local_slot(name)
            .unwrap_or_else(|| self.local_slot(name, false))
    }

    pub(super) fn with_lexical_scope<T>(
        &mut self,
        compile: impl FnOnce(&mut Self) -> Result<T, RuntimeError>,
    ) -> Result<T, RuntimeError> {
        self.lexical_scopes.push(HashMap::new());
        let result = compile(self);
        self.lexical_scopes
            .pop()
            .expect("lexical scope stack should be balanced");
        result
    }

    pub(super) fn current_lexical_slots_for_names(&self, names: &[String]) -> Vec<usize> {
        let Some(scope) = self.lexical_scopes.last() else {
            return Vec::new();
        };
        let mut slots = Vec::new();
        for name in names {
            if let Some(slot) = scope.get(name)
                && !slots.contains(slot)
            {
                slots.push(*slot);
            }
        }
        slots
    }

    pub(super) fn active_lexical_captures(
        &self,
        function_bytecode: &Bytecode,
        function_local_names: &[String],
    ) -> Vec<(String, usize)> {
        let mut captures = Vec::new();
        for name in function_bytecode
            .global_names()
            .iter()
            .map(String::as_str)
            .chain(function_bytecode.local_names())
        {
            if function_local_names
                .binary_search_by(|local| local.as_str().cmp(name))
                .is_ok()
            {
                continue;
            }
            if let Some(slot) = self.resolve_active_lexical_slot(name)
                && !captures.iter().any(|(_, existing)| *existing == slot)
            {
                captures.push((name.to_owned(), slot));
            }
        }
        captures
    }

    pub(super) fn predeclare_current_scope_lexicals(&mut self, body: &[Stmt]) {
        for (name, mutable) in current_scope_lexical_declared_bindings(body) {
            self.declare_lexical_slot(&name, mutable);
        }
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

    pub(super) fn annex_b_arguments_function_name_blocked(&self, name: &str) -> bool {
        name == "arguments" && self.annex_b_function_name_blocked(name)
    }

    fn current_lexical_scope(&self) -> &HashMap<String, usize> {
        self.lexical_scopes
            .last()
            .expect("compiler should always have a lexical scope")
    }

    fn current_lexical_scope_mut(&mut self) -> &mut HashMap<String, usize> {
        self.lexical_scopes
            .last_mut()
            .expect("compiler should always have a lexical scope")
    }

    fn resolve_active_lexical_slot(&self, name: &str) -> Option<usize> {
        self.lexical_scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
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
            | Op::JumpIfNotNullish(dest) => *dest = target,
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

    pub(super) fn compile_break(&mut self, label: Option<&str>) -> Result<(), RuntimeError> {
        let Some(index) = self.break_context_index(label) else {
            return Err(RuntimeError {
                thrown: None,
                message: label.map_or_else(
                    || "break outside loop".to_owned(),
                    |label| format!("undefined break label: {label}"),
                ),
            });
        };
        self.propagate_current_completion_to(index);
        self.emit_loop_iterator_closes_above(index);
        self.emit_with_exits_above(self.loop_stack[index].with_depth);
        let result_slot = self.loop_stack[index].result_slot;
        self.emit(Op::LoadLocal(result_slot));
        let jump = self.emit(Op::Jump(usize::MAX));
        self.loop_stack[index].breaks.push(jump);
        Ok(())
    }

    pub(super) fn compile_continue(&mut self, label: Option<&str>) -> Result<(), RuntimeError> {
        let Some(index) = self.continue_context_index(label) else {
            return Err(RuntimeError {
                thrown: None,
                message: label.map_or_else(
                    || "continue outside loop".to_owned(),
                    |label| format!("undefined continue label: {label}"),
                ),
            });
        };
        self.propagate_current_completion_to(index);
        self.emit_loop_iterator_closes_above(index);
        self.emit_with_exits_above(self.loop_stack[index].with_depth);
        let jump = self.emit(Op::Jump(usize::MAX));
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
            self.emit(Op::LoadLocal(result_slot));
            let done = self.code.len();
            self.patch_loop_breaks(context, done);
            return done;
        }

        for slot in cleanup_slots {
            self.emit(Op::ClearLocal(*slot));
        }
        self.emit(Op::LoadLocal(result_slot));
        let normal_done = self.emit(Op::Jump(usize::MAX));

        let break_cleanup = self.code.len();
        for slot in cleanup_slots {
            self.emit(Op::ClearLocal(*slot));
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
            Stmt::Block { body, .. } => self.with_lexical_scope(|compiler| {
                let blocked = lexical_declared_names(body);
                compiler.with_annex_b_blocked_function_names(&blocked, |compiler| {
                    compiler.predeclare_current_scope_lexicals(body);
                    if body.is_empty() {
                        compiler.emit_load_undefined();
                        return Ok(());
                    }
                    compiler.compile_hoisted_function_decls(body)?;
                    let nested_blocked = nested_block_annex_b_blocked_names(body);
                    compiler.with_annex_b_blocked_function_names(&nested_blocked, |compiler| {
                        for (index, stmt) in body.iter().enumerate() {
                            compiler.compile_stmt(stmt)?;
                            if index + 1 != body.len() {
                                compiler.store_or_pop_statement_list_completion(stmt);
                            }
                        }
                        Ok(())
                    })?;
                    for slot in compiler.current_lexical_slots_for_names(&blocked) {
                        compiler.emit(Op::ClearLocal(slot));
                    }
                    Ok(())
                })
            }),
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
                self.emit_with_exits_above(0);
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
                        if matches!(kind, VarKind::Let | VarKind::Const) {
                            self.emit(Op::ClearLocal(slot));
                        }
                    }
                    let has_init = declaration.init.is_some();
                    if let Some(init) = &declaration.init {
                        self.compile_declaration_init(&declaration.binding, init)?;
                    } else {
                        self.emit_load_undefined();
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
                self.compile_class(Some(name), body)?;
                self.emit(Op::StoreLocal(slot));
                self.emit_load_undefined();
                Ok(())
            }
            Stmt::ModuleDecl(_) => Err(super::util::unsupported_module_item()),
        }
    }

    fn store_or_pop_statement_list_completion(&mut self, stmt: &Stmt) {
        if let Some(result_slot) = self.current_loop_result_slot()
            && stmt_updates_statement_list_completion(stmt)
        {
            self.emit(Op::StoreLocal(result_slot));
        } else {
            self.emit(Op::Pop);
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
