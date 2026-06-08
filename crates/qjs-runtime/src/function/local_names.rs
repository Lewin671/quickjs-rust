use std::collections::HashSet;

use qjs_ast::{ForInLeft, ForInit, FunctionParams, Stmt};

pub(crate) fn collect_function_local_names(
    name: Option<&String>,
    params: &FunctionParams,
    body: &[Stmt],
) -> Vec<String> {
    let mut names = HashSet::new();
    names.insert("this".to_owned());
    names.insert("arguments".to_owned());
    names.extend(params.names());
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
            Stmt::ForIn { left, body, .. } | Stmt::ForOf { left, body, .. } => {
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
