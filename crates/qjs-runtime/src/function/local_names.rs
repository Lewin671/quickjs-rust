use std::collections::HashSet;

use qjs_ast::{ForInLeft, ForInit, Stmt};

use crate::Function;

pub(crate) fn collect_function_local_names(function: &Function) -> HashSet<String> {
    let mut names = HashSet::new();
    names.insert("this".to_owned());
    names.insert("arguments".to_owned());
    names.extend(function.params.iter().cloned());
    if let Some(name) = &function.name {
        names.insert(name.clone());
    }
    collect_statement_local_names(&function.body, &mut names);
    names
}

fn collect_statement_local_names(body: &[Stmt], names: &mut HashSet<String>) {
    for stmt in body {
        match stmt {
            Stmt::VarDecl { declarations, .. } => {
                names.extend(
                    declarations
                        .iter()
                        .map(|declaration| declaration.name.clone()),
                );
            }
            Stmt::FunctionDecl { name, .. } => {
                names.insert(name.clone());
            }
            Stmt::Block { body, .. } => collect_statement_local_names(body, names),
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                collect_statement_local_names(std::slice::from_ref(consequent.as_ref()), names);
                if let Some(alternate) = alternate {
                    collect_statement_local_names(std::slice::from_ref(alternate.as_ref()), names);
                }
            }
            Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
                collect_statement_local_names(std::slice::from_ref(body.as_ref()), names);
            }
            Stmt::For { init, body, .. } => {
                if let Some(ForInit::VarDecl { declarations, .. }) = init {
                    names.extend(
                        declarations
                            .iter()
                            .map(|declaration| declaration.name.clone()),
                    );
                }
                collect_statement_local_names(std::slice::from_ref(body.as_ref()), names);
            }
            Stmt::ForIn { left, body, .. } => {
                if let ForInLeft::VarDecl { name, .. } = left {
                    names.insert(name.clone());
                }
                collect_statement_local_names(std::slice::from_ref(body.as_ref()), names);
            }
            Stmt::Switch { cases, .. } => {
                for case in cases {
                    collect_statement_local_names(&case.consequent, names);
                }
            }
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                collect_statement_local_names(block, names);
                if let Some(handler) = handler {
                    if let Some(param) = &handler.param {
                        names.insert(param.clone());
                    }
                    collect_statement_local_names(&handler.body, names);
                }
                if let Some(finalizer) = finalizer {
                    collect_statement_local_names(finalizer, names);
                }
            }
            Stmt::Expr(_)
            | Stmt::Return { .. }
            | Stmt::Throw { .. }
            | Stmt::Debugger { .. }
            | Stmt::Break { .. }
            | Stmt::Continue { .. }
            | Stmt::Empty => {}
        }
    }
}
