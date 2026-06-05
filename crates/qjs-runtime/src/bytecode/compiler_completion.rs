use qjs_ast::Stmt;

pub(super) fn statement_has_empty_completion(stmt: &Stmt) -> bool {
    matches!(
        stmt,
        Stmt::VarDecl { .. } | Stmt::FunctionDecl { .. } | Stmt::Empty
    )
}
