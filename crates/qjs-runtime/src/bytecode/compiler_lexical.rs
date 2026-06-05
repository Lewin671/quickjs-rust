use qjs_ast::{Stmt, VarKind};

use super::compiler::Compiler;

impl Compiler {
    pub(super) fn collect_lexical_locals(&mut self, body: &[Stmt]) {
        for stmt in body {
            match stmt {
                Stmt::Block { body, .. } => self.collect_lexical_locals(body),
                Stmt::If {
                    consequent,
                    alternate,
                    ..
                } => {
                    self.collect_lexical_locals(std::slice::from_ref(consequent));
                    if let Some(alternate) = alternate {
                        self.collect_lexical_locals(std::slice::from_ref(alternate));
                    }
                }
                Stmt::While { body, .. } | Stmt::With { body, .. } | Stmt::DoWhile { body, .. } => {
                    self.collect_lexical_locals(std::slice::from_ref(body));
                }
                Stmt::For { body, .. } => {
                    self.collect_lexical_locals(std::slice::from_ref(body));
                }
                Stmt::ForIn { body, .. } | Stmt::ForOf { body, .. } => {
                    self.collect_lexical_locals(std::slice::from_ref(body));
                }
                Stmt::VarDecl {
                    kind, declarations, ..
                } if *kind != VarKind::Var => {
                    for declaration in declarations {
                        if *kind == VarKind::Const {
                            self.immutable_local_slot(&declaration.name, false);
                        } else {
                            self.local_slot(&declaration.name, false);
                        }
                    }
                }
                Stmt::Switch { cases, .. } => {
                    for case in cases {
                        self.collect_lexical_locals(&case.consequent);
                    }
                }
                Stmt::Try {
                    block,
                    handler,
                    finalizer,
                    ..
                } => {
                    self.collect_lexical_locals(block);
                    if let Some(handler) = handler {
                        self.collect_lexical_locals(&handler.body);
                    }
                    if let Some(finalizer) = finalizer {
                        self.collect_lexical_locals(finalizer);
                    }
                }
                Stmt::Label { body, .. } => {
                    self.collect_lexical_locals(std::slice::from_ref(body));
                }
                Stmt::FunctionDecl { .. }
                | Stmt::ClassDecl { .. }
                | Stmt::Expr(_)
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
}
