use qjs_ast::{AssignmentTarget, ForInLeft, ForInit, Stmt, VarKind};

use super::compiler::Compiler;

impl Compiler {
    pub(super) fn collect_hoisted_locals(&mut self, body: &[Stmt]) {
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
}
