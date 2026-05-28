use std::collections::{HashMap, HashSet};

use qjs_ast::{ForInLeft, ForInit, Stmt, VarKind};

use crate::{Function, Value};

pub(crate) fn hoist_declarations(body: &[Stmt], env: &mut HashMap<String, Value>) {
    hoist_var_declarations(body, env);
    hoist_function_declarations(body, env);
}

fn hoist_var_declarations(body: &[Stmt], env: &mut HashMap<String, Value>) {
    for stmt in body {
        match stmt {
            Stmt::VarDecl {
                kind: VarKind::Var,
                declarations,
                ..
            } => {
                for declaration in declarations {
                    env.entry(declaration.name.clone())
                        .or_insert(Value::Undefined);
                }
            }
            Stmt::Block { body, .. } => hoist_var_declarations(body, env),
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                hoist_var_declarations(std::slice::from_ref(consequent.as_ref()), env);
                if let Some(alternate) = alternate {
                    hoist_var_declarations(std::slice::from_ref(alternate.as_ref()), env);
                }
            }
            Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
                hoist_var_declarations(std::slice::from_ref(body.as_ref()), env);
            }
            Stmt::For { init, body, .. } => {
                if let Some(ForInit::VarDecl {
                    kind: VarKind::Var,
                    declarations,
                    ..
                }) = init
                {
                    for declaration in declarations {
                        env.entry(declaration.name.clone())
                            .or_insert(Value::Undefined);
                    }
                }
                hoist_var_declarations(std::slice::from_ref(body.as_ref()), env);
            }
            Stmt::ForIn { left, body, .. } => {
                if let ForInLeft::VarDecl {
                    kind: VarKind::Var,
                    name,
                    ..
                } = left
                {
                    env.entry(name.clone()).or_insert(Value::Undefined);
                }
                hoist_var_declarations(std::slice::from_ref(body.as_ref()), env);
            }
            Stmt::Switch { cases, .. } => {
                for case in cases {
                    hoist_var_declarations(&case.consequent, env);
                }
            }
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                hoist_var_declarations(block, env);
                if let Some(handler) = handler {
                    hoist_var_declarations(&handler.body, env);
                }
                if let Some(finalizer) = finalizer {
                    hoist_var_declarations(finalizer, env);
                }
            }
            Stmt::FunctionDecl { .. } => {}
            _ => {}
        }
    }
}

fn hoist_function_declarations(body: &[Stmt], env: &mut HashMap<String, Value>) {
    for stmt in body {
        if let Stmt::FunctionDecl {
            name, params, body, ..
        } = stmt
        {
            env.insert(
                name.clone(),
                Value::Function(Function::new_user(
                    Some(name.clone()),
                    params.clone(),
                    body.clone(),
                    env.clone(),
                )),
            );
        }
    }
}

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
            _ => {}
        }
    }
}
