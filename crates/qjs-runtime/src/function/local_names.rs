use std::collections::HashSet;

use qjs_ast::{AssignmentTarget, ForInLeft, ForInit, Stmt};

pub(crate) fn collect_function_local_names(
    name: Option<&String>,
    params: &[String],
    body: &[Stmt],
) -> Vec<String> {
    let mut names = HashSet::new();
    names.insert("this".to_owned());
    names.insert("arguments".to_owned());
    names.extend(params.iter().cloned());
    if let Some(name) = name {
        names.insert(name.clone());
    }
    collect_statement_local_names(body, &mut names);
    let mut names = names.into_iter().collect::<Vec<_>>();
    names.sort();
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
            Stmt::ClassDecl { name, methods, .. } => {
                names.insert(name.clone());
                for method in methods {
                    collect_statement_local_names(&method.body, names);
                }
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
            Stmt::While { body, .. } | Stmt::With { body, .. } | Stmt::DoWhile { body, .. } => {
                collect_statement_local_names(std::slice::from_ref(body.as_ref()), names);
            }
            Stmt::For { init, body, .. } => {
                if let Some(init) = init {
                    match init {
                        ForInit::VarDecl { declarations, .. } => {
                            names.extend(
                                declarations
                                    .iter()
                                    .map(|declaration| declaration.name.clone()),
                            );
                        }
                        ForInit::Binding { target, .. } => {
                            collect_target_local_names(target, names);
                        }
                        ForInit::Expr(_) => {}
                    }
                }
                collect_statement_local_names(std::slice::from_ref(body.as_ref()), names);
            }
            Stmt::ForIn { left, body, .. } | Stmt::ForOf { left, body, .. } => {
                match left {
                    ForInLeft::VarDecl { name, .. } => {
                        names.insert(name.clone());
                    }
                    ForInLeft::Binding { target, .. } => {
                        collect_target_local_names(target, names);
                    }
                    ForInLeft::Target(_) => {}
                }
                collect_statement_local_names(std::slice::from_ref(body.as_ref()), names);
            }
            Stmt::Label { body, .. } => {
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

fn collect_target_local_names(target: &AssignmentTarget, names: &mut HashSet<String>) {
    match target {
        AssignmentTarget::Identifier { name, .. } => {
            names.insert(name.clone());
        }
        AssignmentTarget::Array { elements, .. } => {
            for element in elements.iter().flatten() {
                collect_target_local_names(&element.target, names);
            }
        }
        AssignmentTarget::Object { properties, .. } => {
            for property in properties {
                collect_target_local_names(&property.target, names);
            }
        }
        AssignmentTarget::Member { .. } => {}
    }
}

#[cfg(test)]
mod tests {
    use qjs_ast::Stmt;
    use qjs_parser::parse_script;

    use super::collect_function_local_names;

    #[test]
    fn collects_function_local_names_in_sorted_order() {
        let script = parse_script(
            "function outer(param) { var z; let a; try {} catch (caught) {} function inner() {} }",
        )
        .expect("function should parse");
        let Stmt::FunctionDecl {
            name, params, body, ..
        } = &script.body[0]
        else {
            panic!("expected function declaration");
        };

        assert_eq!(
            collect_function_local_names(Some(name), params, body),
            vec![
                "a",
                "arguments",
                "caught",
                "inner",
                "outer",
                "param",
                "this",
                "z"
            ]
        );
    }
}
