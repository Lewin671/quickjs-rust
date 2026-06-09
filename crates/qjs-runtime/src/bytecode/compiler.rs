use std::collections::HashMap;

use qjs_ast::{
    BinaryOp, CatchParam, ForInLeft, ForInit, FunctionParams, Script, Stmt, SwitchCase, VarKind,
};

use crate::{RuntimeError, Value, function::is_strict_function_body};

use super::ir::{Bytecode, Local, Op};

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
    annex_b_blocked_function_names: Vec<Vec<String>>,
}

#[derive(Default)]
pub(super) struct LoopContext {
    result_slot: usize,
    allows_continue: bool,
    labels: Vec<String>,
    breaks: Vec<usize>,
    continues: Vec<usize>,
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
            annex_b_blocked_function_names: Vec::new(),
        }
    }
}

pub(super) fn compile_script(script: &Script) -> Result<Bytecode, RuntimeError> {
    Compiler::default().compile(script)
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

impl Compiler {
    fn compile(mut self, script: &Script) -> Result<Bytecode, RuntimeError> {
        self.strict = is_strict_function_body(&script.body);
        self.collect_hoisted_locals(&script.body);
        let blocked = lexical_declared_names(&script.body);
        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
            compiler.compile_hoisted_function_decls(&script.body)?;
            for stmt in &script.body {
                compiler.compile_stmt(stmt)?;
            }
            Ok(())
        })?;
        self.code.push(Op::Return);
        Ok(Bytecode::new(self.constants, self.locals, self.code))
    }

    fn compile_function(
        mut self,
        params: &FunctionParams,
        body: &[Stmt],
    ) -> Result<Bytecode, RuntimeError> {
        self.global_scope = false;
        self.strict = self.strict || is_strict_function_body(body);
        for param in params.names() {
            self.local_slot(&param, true);
        }
        self.compile_parameter_defaults(params)?;
        let param_blocked = function_param_names(params);
        self.with_annex_b_blocked_function_names(&param_blocked, |compiler| {
            compiler.collect_hoisted_locals(body);
            Ok(())
        })?;
        let blocked = function_body_annex_b_blocked_names(params, body);
        self.with_annex_b_blocked_function_names(&blocked, |compiler| {
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

    fn compile_parameter_defaults(&mut self, params: &FunctionParams) -> Result<(), RuntimeError> {
        for (index, name) in params.positional.iter().enumerate() {
            let Some(default) = params.default_at(index) else {
                continue;
            };
            let slot = self
                .resolve_local_slot(name)
                .expect("parameter slot should be declared before defaults");
            self.emit(Op::LoadLocal(slot));
            self.emit_load_undefined();
            self.emit(Op::Binary(BinaryOp::StrictEq));
            let skip_default = self.emit(Op::JumpIfFalse(usize::MAX));
            self.emit(Op::Pop);
            self.compile_expr(default)?;
            self.emit(Op::StoreLocal(slot));
            let done = self.emit(Op::Jump(usize::MAX));
            let skip_target = self.code.len();
            self.patch_jump(skip_default, skip_target);
            self.emit(Op::Pop);
            let done_target = self.code.len();
            self.patch_jump(done, done_target);
        }
        Ok(())
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
                                self.local_slot(&declaration.name, true);
                            }
                        }
                    }
                    self.collect_hoisted_locals(std::slice::from_ref(body));
                }
                Stmt::ForIn { left, body, .. } | Stmt::ForOf { left, body, .. } => {
                    if let ForInLeft::VarDecl {
                        name,
                        kind: VarKind::Var,
                        ..
                    } = left
                    {
                        self.local_slot(name, true);
                    }
                    self.collect_hoisted_locals(std::slice::from_ref(body));
                }
                Stmt::FunctionDecl { name, .. } => {
                    if !self.annex_b_function_name_blocked(name) {
                        self.local_slot(name, true);
                    }
                }
                Stmt::Labelled { body, .. } => {
                    self.collect_hoisted_locals(std::slice::from_ref(body));
                }
                Stmt::VarDecl {
                    kind: VarKind::Var,
                    declarations,
                    ..
                } => {
                    for declaration in declarations {
                        self.local_slot(&declaration.name, true);
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

    pub(super) fn declare_var_kind_slot(&mut self, name: &str, kind: VarKind) -> usize {
        match kind {
            VarKind::Var => self.local_slot(name, true),
            VarKind::Let => self.declare_lexical_slot(name, true),
            VarKind::Const => self.declare_lexical_slot(name, false),
        }
    }

    fn var_initializer_slot(&self, name: &str, declared_slot: usize, kind: VarKind) -> usize {
        if kind != VarKind::Var {
            return declared_slot;
        }
        self.resolve_local_slot(name).unwrap_or(declared_slot)
    }

    pub(super) fn emit_store_var_initializer(&mut self, slot: usize, name: &str, kind: VarKind) {
        let store_slot = self.var_initializer_slot(name, slot, kind);
        if store_slot != slot && kind == VarKind::Var {
            self.emit(Op::StoreLocal(store_slot));
        } else {
            self.emit_store_var_binding(store_slot, name, kind);
        }
    }

    pub(super) fn emit_store_var_binding(&mut self, slot: usize, name: &str, kind: VarKind) {
        if self.global_scope && kind == VarKind::Var {
            self.emit(Op::Dup);
            self.emit(Op::StoreLocal(slot));
            self.emit(Op::DefineGlobalVar(name.to_owned()));
        } else {
            self.emit(Op::StoreLocal(slot));
        }
    }

    pub(super) fn resolve_local_slot(&self, name: &str) -> Option<usize> {
        self.lexical_scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
            .or_else(|| self.local_slots.get(name).copied())
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
        let name = format!("\0{prefix}_{}", self.next_temp);
        self.next_temp += 1;
        self.local_slot(&name, true)
    }

    pub(super) fn push_loop(&mut self, result_slot: usize) {
        let labels = self.take_pending_labels();
        self.loop_stack.push(LoopContext {
            result_slot,
            allows_continue: true,
            labels,
            breaks: Vec::new(),
            continues: Vec::new(),
        });
    }

    pub(super) fn push_breakable(&mut self, result_slot: usize) {
        let labels = self.take_pending_labels();
        self.loop_stack.push(LoopContext {
            result_slot,
            allows_continue: false,
            labels,
            breaks: Vec::new(),
            continues: Vec::new(),
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
        let jump = self.emit(Op::Jump(usize::MAX));
        self.loop_stack[index].continues.push(jump);
        Ok(())
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
                left, right, body, ..
            } => self.compile_for_of(left, right, body),
            Stmt::Return { argument, .. } => {
                if let Some(argument) = argument {
                    self.compile_expr(argument)?;
                } else {
                    self.emit_load_undefined();
                }
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
                    let slot = self.declare_var_kind_slot(&declaration.name, *kind);
                    if matches!(kind, VarKind::Let | VarKind::Const) {
                        self.emit(Op::ClearLocal(slot));
                    }
                    let has_init = declaration.init.is_some();
                    if let Some(init) = &declaration.init {
                        self.compile_expr(init)?;
                    } else {
                        self.emit_load_undefined();
                    }
                    if has_init {
                        self.emit_store_var_initializer(slot, &declaration.name, *kind);
                    } else {
                        self.emit_store_var_binding(slot, &declaration.name, *kind);
                    }
                }
                self.emit_load_undefined();
                Ok(())
            }
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

fn stmt_updates_statement_list_completion(stmt: &Stmt) -> bool {
    !matches!(
        stmt,
        Stmt::Debugger { .. } | Stmt::Empty | Stmt::FunctionDecl { .. } | Stmt::VarDecl { .. }
    )
}

fn stmt_accepts_pending_label(stmt: &Stmt) -> bool {
    matches!(
        stmt,
        Stmt::Labelled { .. }
            | Stmt::While { .. }
            | Stmt::DoWhile { .. }
            | Stmt::For { .. }
            | Stmt::ForIn { .. }
            | Stmt::ForOf { .. }
            | Stmt::Switch { .. }
    )
}

pub(super) fn catch_param_annex_b_blocked_names(param: Option<&CatchParam>) -> Vec<String> {
    match param {
        Some(CatchParam::Object { names }) => names.clone(),
        Some(CatchParam::Identifier(_)) | None => Vec::new(),
    }
}

pub(super) fn for_init_lexical_names(init: &ForInit) -> Vec<String> {
    match init {
        ForInit::VarDecl {
            kind: VarKind::Let | VarKind::Const,
            declarations,
            ..
        } => declarations
            .iter()
            .map(|declaration| declaration.name.clone())
            .collect(),
        ForInit::VarDecl { .. } | ForInit::Expr(_) => Vec::new(),
    }
}

pub(super) fn for_in_left_lexical_name(left: &ForInLeft) -> Option<String> {
    match left {
        ForInLeft::VarDecl {
            kind: VarKind::Let | VarKind::Const,
            name,
            ..
        } => Some(name.clone()),
        ForInLeft::VarDecl { .. } | ForInLeft::Target(_) => None,
    }
}

pub(super) fn switch_lexical_declared_names(cases: &[SwitchCase]) -> Vec<String> {
    let mut names = Vec::new();
    for case in cases {
        names.extend(lexical_declared_names(&case.consequent));
    }
    names
}

fn lexical_declared_names(body: &[Stmt]) -> Vec<String> {
    let mut names = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::VarDecl {
                kind: VarKind::Let | VarKind::Const,
                declarations,
                ..
            } => names.extend(
                declarations
                    .iter()
                    .map(|declaration| declaration.name.clone()),
            ),
            Stmt::For {
                init: Some(init), ..
            } => names.extend(for_init_lexical_names(init)),
            Stmt::ForIn { left, .. } | Stmt::ForOf { left, .. } => {
                if let Some(name) = for_in_left_lexical_name(left) {
                    names.push(name);
                }
            }
            Stmt::Switch { cases, .. } => names.extend(switch_lexical_declared_names(cases)),
            _ => {}
        }
    }
    names
}

fn nested_block_annex_b_blocked_names(body: &[Stmt]) -> Vec<String> {
    let mut names = lexical_declared_names(body);
    for stmt in body {
        if let Stmt::FunctionDecl { name, .. } = stmt
            && !names.iter().any(|existing| existing == name)
        {
            names.push(name.clone());
        }
    }
    names
}

fn function_body_annex_b_blocked_names(params: &FunctionParams, body: &[Stmt]) -> Vec<String> {
    let mut names = function_param_names(params);
    names.extend(lexical_declared_names(body));
    names
}

fn function_param_names(params: &FunctionParams) -> Vec<String> {
    let mut names = params.positional.clone();
    if let Some(rest) = &params.rest {
        names.push(rest.clone());
    }
    if !names.iter().any(|name| name == "arguments") {
        names.push("arguments".to_owned());
    }
    names
}
