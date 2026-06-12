use qjs_ast::{BindingPattern, ForInLeft, ForInit, FunctionParams, Stmt, SwitchCase, VarKind};

pub(super) fn catch_param_annex_b_blocked_names(param: Option<&BindingPattern>) -> Vec<String> {
    match param {
        Some(BindingPattern::Identifier { .. }) | None => Vec::new(),
        Some(pattern) => pattern.names(),
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
            .flat_map(|declaration| declaration.binding.names())
            .collect(),
        ForInit::VarDecl { .. } | ForInit::Expr(_) => Vec::new(),
    }
}

pub(super) fn for_in_left_lexical_names(left: &ForInLeft) -> Vec<String> {
    match left {
        ForInLeft::VarDecl {
            kind: VarKind::Let | VarKind::Const,
            binding,
            ..
        } => binding.names(),
        ForInLeft::VarDecl { .. } | ForInLeft::Target(_) => Vec::new(),
    }
}

pub(super) fn switch_lexical_declared_names(cases: &[SwitchCase]) -> Vec<String> {
    let mut names = Vec::new();
    for case in cases {
        names.extend(lexical_declared_names(&case.consequent));
    }
    names
}

pub(super) fn lexical_declared_names(body: &[Stmt]) -> Vec<String> {
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
                    .flat_map(|declaration| declaration.binding.names()),
            ),
            Stmt::For {
                init: Some(init), ..
            } => names.extend(for_init_lexical_names(init)),
            Stmt::ForIn { left, .. } | Stmt::ForOf { left, .. } => {
                names.extend(for_in_left_lexical_names(left));
            }
            Stmt::Switch { cases, .. } => names.extend(switch_lexical_declared_names(cases)),
            Stmt::ClassDecl { name, .. } => names.push(name.clone()),
            _ => {}
        }
    }
    names
}

pub(super) fn current_scope_lexical_declared_bindings(body: &[Stmt]) -> Vec<(String, bool)> {
    let mut names = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::VarDecl {
                kind: VarKind::Let | VarKind::Const,
                declarations,
                ..
            } => {
                let mutable = matches!(
                    stmt,
                    Stmt::VarDecl {
                        kind: VarKind::Let,
                        ..
                    }
                );
                names.extend(
                    declarations
                        .iter()
                        .flat_map(|declaration| declaration.binding.names())
                        .map(|name| (name, mutable)),
                );
            }
            Stmt::ClassDecl { name, .. } => names.push((name.clone(), true)),
            _ => {}
        }
    }
    names
}

pub(super) fn nested_block_annex_b_blocked_names(body: &[Stmt]) -> Vec<String> {
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

pub(super) fn function_body_annex_b_blocked_names(
    params: &FunctionParams,
    body: &[Stmt],
) -> Vec<String> {
    let mut names = function_param_names(params);
    names.extend(lexical_declared_names(body));
    names
}

pub(super) fn function_param_names(params: &FunctionParams) -> Vec<String> {
    let mut names = params.names();
    if !names.iter().any(|name| name == "arguments") {
        names.push("arguments".to_owned());
    }
    names
}
