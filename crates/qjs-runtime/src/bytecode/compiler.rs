use std::collections::HashMap;

use qjs_ast::{AssignmentTarget, ForInLeft, ForInit, Script, Stmt, VarKind};

use crate::{RuntimeError, Value, function::is_strict_function_body};

use super::ir::{Bytecode, Local, Op};

#[derive(Default)]
pub(super) struct Compiler {
    pub(super) constants: Vec<Value>,
    pub(super) locals: Vec<Local>,
    pub(super) local_slots: HashMap<String, usize>,
    pub(super) code: Vec<Op>,
    loop_stack: Vec<LoopContext>,
    label_stack: Vec<String>,
    next_temp: usize,
    pub(super) strict: bool,
    pub(super) dynamic_scope_depth: usize,
    pub(super) direct_eval: bool,
}

#[derive(Default)]
pub(super) struct LoopContext {
    result_slot: usize,
    allows_continue: bool,
    labels: Vec<String>,
    breaks: Vec<usize>,
    continues: Vec<usize>,
}

pub(super) fn compile_script(script: &Script) -> Result<Bytecode, RuntimeError> {
    Compiler::default().compile(script)
}

pub(super) fn compile_eval_script(script: &Script) -> Result<Bytecode, RuntimeError> {
    Compiler {
        direct_eval: true,
        ..Compiler::default()
    }
    .compile(script)
}

pub(super) fn compile_function_body(
    params: &[String],
    body: &[Stmt],
) -> Result<Bytecode, RuntimeError> {
    compile_function_body_with_strict(params, body, false)
}

pub(super) fn compile_function_body_with_strict(
    params: &[String],
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
        self.compile_hoisted_function_decls(&script.body)?;
        for stmt in &script.body {
            self.compile_stmt(stmt)?;
        }
        self.code.push(Op::Return);
        Ok(Bytecode::new(self.constants, self.locals, self.code))
    }

    fn compile_function(
        mut self,
        params: &[String],
        body: &[Stmt],
    ) -> Result<Bytecode, RuntimeError> {
        self.strict = self.strict || is_strict_function_body(body);
        for param in params {
            self.local_slot(param, true);
        }
        self.collect_hoisted_locals(body);
        self.compile_hoisted_function_decls(body)?;
        for stmt in body {
            self.compile_stmt(stmt)?;
        }
        self.emit_load_undefined();
        self.code.push(Op::Return);
        Ok(Bytecode::new(self.constants, self.locals, self.code))
    }

    fn collect_hoisted_locals(&mut self, body: &[Stmt]) {
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
                Stmt::While { body, .. } | Stmt::With { body, .. } | Stmt::DoWhile { body, .. } => {
                    self.collect_hoisted_locals(std::slice::from_ref(body));
                }
                Stmt::For { init, body, .. } => {
                    if let Some(init) = init {
                        match init {
                            ForInit::VarDecl {
                                declarations,
                                kind: VarKind::Var,
                                ..
                            } => {
                                for declaration in declarations {
                                    self.local_slot(&declaration.name, true);
                                }
                            }
                            ForInit::Binding {
                                target,
                                kind: VarKind::Var,
                                ..
                            } => {
                                self.ensure_target_local_slots(target, true);
                            }
                            _ => {}
                        }
                    }
                    self.collect_hoisted_locals(std::slice::from_ref(body));
                }
                Stmt::ForIn { left, body, .. } | Stmt::ForOf { left, body, .. } => {
                    match left {
                        ForInLeft::VarDecl {
                            name,
                            kind: VarKind::Var,
                            ..
                        } => {
                            self.local_slot(name, true);
                        }
                        ForInLeft::Binding {
                            kind: VarKind::Var,
                            target,
                            ..
                        } => {
                            self.collect_hoisted_target_locals(target);
                        }
                        _ => {}
                    }
                    self.collect_hoisted_locals(std::slice::from_ref(body));
                }
                Stmt::FunctionDecl { name, .. } => {
                    self.local_slot(name, true);
                }
                Stmt::ClassDecl { name, .. } => {
                    self.local_slot(name, true);
                }
                Stmt::Label { body, .. } => {
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
                    for case in cases {
                        self.collect_hoisted_locals(&case.consequent);
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
                        self.collect_hoisted_locals(&handler.body);
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
    }

    fn collect_hoisted_target_locals(&mut self, target: &AssignmentTarget) {
        match target {
            AssignmentTarget::Identifier { name, .. } => {
                self.local_slot(name, true);
            }
            AssignmentTarget::Array { elements, .. } => {
                for element in elements.iter().flatten() {
                    self.collect_hoisted_target_locals(&element.target);
                }
            }
            AssignmentTarget::Object { properties, .. } => {
                for property in properties {
                    self.collect_hoisted_target_locals(&property.target);
                }
            }
            AssignmentTarget::Member { .. } => {}
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
        });
        self.local_slots.insert(name.to_owned(), slot);
        slot
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
            | Op::JumpIfNotUndefined(dest) => *dest = target,
            _ => unreachable!("attempted to patch a non-jump instruction"),
        }
    }

    pub(super) fn temp_local(&mut self, prefix: &str) -> usize {
        let name = format!("\0{prefix}_{}", self.next_temp);
        self.next_temp += 1;
        self.local_slot(&name, true)
    }

    pub(super) fn push_loop(&mut self, result_slot: usize) {
        self.loop_stack.push(LoopContext {
            result_slot,
            allows_continue: true,
            labels: std::mem::take(&mut self.label_stack),
            breaks: Vec::new(),
            continues: Vec::new(),
        });
    }

    pub(super) fn push_breakable(&mut self, result_slot: usize) {
        self.loop_stack.push(LoopContext {
            result_slot,
            allows_continue: false,
            labels: std::mem::take(&mut self.label_stack),
            breaks: Vec::new(),
            continues: Vec::new(),
        });
    }

    pub(super) fn pop_loop(&mut self) -> LoopContext {
        self.loop_stack
            .pop()
            .expect("loop context should be balanced")
    }

    pub(super) fn current_result_slot(&self) -> Option<usize> {
        self.loop_stack.last().map(|context| context.result_slot)
    }

    pub(super) fn compile_break(&mut self, label: Option<&str>) -> Result<(), RuntimeError> {
        let index = if let Some(label) = label {
            self.loop_stack
                .iter()
                .rposition(|context| context.labels.iter().any(|candidate| candidate == label))
        } else {
            self.loop_stack.len().checked_sub(1)
        };
        let Some(index) = index else {
            return Err(RuntimeError {
                thrown: None,
                message: "break outside loop".to_owned(),
            });
        };
        if let (Some(source), target) = (
            self.loop_stack.last().map(|context| context.result_slot),
            self.loop_stack[index].result_slot,
        ) && source != target
        {
            self.emit(Op::LoadLocal(source));
            self.emit(Op::StoreLocal(target));
        }
        let result_slot = self.loop_stack[index].result_slot;
        self.emit(Op::LoadLocal(result_slot));
        let jump = self.emit(Op::Jump(usize::MAX));
        self.loop_stack[index].breaks.push(jump);
        Ok(())
    }

    pub(super) fn compile_continue(&mut self, label: Option<&str>) -> Result<(), RuntimeError> {
        let index = if let Some(label) = label {
            self.loop_stack.iter().rposition(|context| {
                context.allows_continue && context.labels.iter().any(|candidate| candidate == label)
            })
        } else {
            self.loop_stack
                .iter()
                .rposition(|context| context.allows_continue)
        };
        let Some(index) = index else {
            return Err(RuntimeError {
                thrown: None,
                message: "continue outside loop".to_owned(),
            });
        };
        if let (Some(source), target) = (
            self.loop_stack.last().map(|context| context.result_slot),
            self.loop_stack[index].result_slot,
        ) && source != target
        {
            self.emit(Op::LoadLocal(source));
            self.emit(Op::StoreLocal(target));
        }
        let jump = self.emit(Op::Jump(usize::MAX));
        self.loop_stack[index].continues.push(jump);
        Ok(())
    }

    pub(super) fn patch_loop_breaks(&mut self, context: &LoopContext, target: usize) {
        for jump in &context.breaks {
            self.patch_jump(*jump, target);
        }
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
                if body.is_empty() {
                    self.emit_load_undefined();
                    return Ok(());
                }
                self.compile_hoisted_function_decls(body)?;
                for (index, stmt) in body.iter().enumerate() {
                    self.compile_stmt(stmt)?;
                    if index + 1 != body.len() {
                        if let Some(result_slot) =
                            self.loop_stack.last().map(|context| context.result_slot)
                        {
                            self.emit(Op::StoreLocal(result_slot));
                        } else {
                            self.emit(Op::Pop);
                        }
                    }
                }
                Ok(())
            }
            Stmt::If {
                test,
                consequent,
                alternate,
                ..
            } => self.compile_if(test, consequent, alternate.as_deref()),
            Stmt::While { test, body, .. } => self.compile_while(test, body),
            Stmt::With { object, body, .. } => self.compile_with(object, body),
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
                let is_hoisted = *kind == VarKind::Var;
                for declaration in declarations {
                    let slot = self.local_slot(&declaration.name, is_hoisted);
                    if let Some(init) = &declaration.init {
                        self.compile_expr(init)?;
                        self.emit(Op::StoreLocal(slot));
                    } else if *kind != VarKind::Var || self.direct_eval {
                        self.emit_load_undefined();
                        self.emit(Op::StoreLocal(slot));
                    }
                }
                self.emit_load_undefined();
                Ok(())
            }
            Stmt::Break { label, .. } => self.compile_break(label.as_deref()),
            Stmt::Continue { label, .. } => self.compile_continue(label.as_deref()),
            Stmt::Label { label, body, .. } => self.compile_label(label, body),
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
            Stmt::ClassDecl { name, methods, .. } => self.compile_class_decl(name, methods),
        }
    }

    fn compile_label(&mut self, label: &str, body: &Stmt) -> Result<(), RuntimeError> {
        if matches!(
            body,
            Stmt::While { .. }
                | Stmt::DoWhile { .. }
                | Stmt::For { .. }
                | Stmt::ForIn { .. }
                | Stmt::ForOf { .. }
                | Stmt::Switch { .. }
        ) {
            self.label_stack.push(label.to_owned());
            let result = self.compile_stmt(body);
            if self
                .label_stack
                .last()
                .is_some_and(|active| active == label)
            {
                self.label_stack.pop();
            }
            return result;
        }

        let result_slot = self.temp_local("label_result");
        self.emit_load_undefined();
        self.emit(Op::StoreLocal(result_slot));
        self.label_stack.push(label.to_owned());
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
