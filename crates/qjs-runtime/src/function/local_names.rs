use std::collections::HashSet;

use qjs_ast::{BindingPattern, ForInLeft, ForInit, FunctionParams, Stmt};

/// Returns true for compiler-internal binding names that must never cross
/// call frames (destructuring temporaries and raw pattern-argument slots).
///
/// Frame-local temporaries use a double-NUL prefix. Single-NUL names are
/// runtime singletons (for example the symbol registry binding) that must
/// keep flowing between frames.
pub(crate) fn is_internal_binding_name(name: &str) -> bool {
    name.starts_with("\u{0}\u{0}")
}

/// Returns the call-frame binding name for a positional parameter.
///
/// Plain identifier parameters bind under their own name; destructured
/// parameters bind the raw argument under an internal name that the function
/// prologue destructures.
pub(crate) fn parameter_binding_name(binding: &BindingPattern, index: usize) -> String {
    match binding {
        BindingPattern::Identifier { name, .. } => name.clone(),
        BindingPattern::Array { .. } | BindingPattern::Object { .. } => {
            format!("\u{0}\u{0}param_pattern_{index}")
        }
    }
}

/// Returns the call-frame binding name for the rest parameter.
pub(crate) fn rest_parameter_binding_name(binding: &BindingPattern) -> String {
    match binding {
        BindingPattern::Identifier { name, .. } => name.clone(),
        BindingPattern::Array { .. } | BindingPattern::Object { .. } => {
            "\u{0}\u{0}rest_pattern".to_owned()
        }
    }
}

pub(crate) fn collect_function_local_names(
    name: Option<&String>,
    params: &FunctionParams,
    body: &[Stmt],
    has_own_arguments: bool,
) -> Vec<String> {
    let mut names = HashSet::new();
    names.insert("this".to_owned());
    if has_own_arguments {
        names.insert("arguments".to_owned());
    }
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
                        .flat_map(|declaration| declaration.binding.names()),
                );
            }
            Stmt::FunctionDecl { name, .. } => {
                names.insert(name.clone());
            }
            Stmt::Labelled { body, .. } | Stmt::With { body, .. } => {
                collect_statement_local_names(std::slice::from_ref(body.as_ref()), names);
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
                            .flat_map(|declaration| declaration.binding.names()),
                    );
                }
                collect_statement_local_names(std::slice::from_ref(body.as_ref()), names);
            }
            Stmt::ForIn { left, body, .. } | Stmt::ForOf { left, body, .. } => {
                if let ForInLeft::VarDecl { binding, .. } = left {
                    names.extend(binding.names());
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
                        names.extend(param.names());
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
            | Stmt::ClassDecl { .. }
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
            collect_function_local_names(Some(name), params, body, true),
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
