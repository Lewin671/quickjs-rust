use std::collections::HashMap;

use qjs_ast::{ForInit, Script, Stmt, VarKind};

use crate::{RuntimeError, Value};

use super::ir::{Bytecode, Local, Op};
use super::util::unsupported_stmt;

#[derive(Default)]
pub(super) struct Compiler {
    pub(super) constants: Vec<Value>,
    pub(super) locals: Vec<Local>,
    pub(super) local_slots: HashMap<String, usize>,
    pub(super) code: Vec<Op>,
}

pub(super) fn compile_script(script: &Script) -> Result<Bytecode, RuntimeError> {
    Compiler::default().compile(script)
}

impl Compiler {
    fn compile(mut self, script: &Script) -> Result<Bytecode, RuntimeError> {
        self.collect_hoisted_locals(&script.body);
        for stmt in &script.body {
            self.compile_stmt(stmt)?;
        }
        self.code.push(Op::Return);
        Ok(Bytecode {
            constants: self.constants,
            locals: self.locals,
            code: self.code,
        })
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
                Stmt::FunctionDecl { name, .. } => {
                    self.local_slot(name, true);
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
                | Stmt::ForIn { .. }
                | Stmt::Empty => {}
            }
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
            | Op::JumpIfNotNullish(dest) => *dest = target,
            _ => unreachable!("attempted to patch a non-jump instruction"),
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
                for (index, stmt) in body.iter().enumerate() {
                    self.compile_stmt(stmt)?;
                    if index + 1 != body.len() {
                        self.emit(Op::Pop);
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
            Stmt::DoWhile { body, test, .. } => self.compile_do_while(body, test),
            Stmt::For {
                init,
                test,
                update,
                body,
                ..
            } => self.compile_for(init.as_ref(), test.as_ref(), update.as_ref(), body),
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
                    } else {
                        self.emit_load_undefined();
                    }
                    self.emit(Op::StoreLocal(slot));
                }
                self.emit_load_undefined();
                Ok(())
            }
            Stmt::Break { .. }
            | Stmt::Continue { .. }
            | Stmt::ForIn { .. }
            | Stmt::Switch { .. }
            | Stmt::Try { .. }
            | Stmt::FunctionDecl { .. } => Err(unsupported_stmt(stmt)),
        }
    }
}
