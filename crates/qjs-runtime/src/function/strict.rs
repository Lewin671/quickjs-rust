use qjs_ast::{Expr, Literal, Stmt};

pub(crate) fn is_strict_function_body(body: &[Stmt]) -> bool {
    for stmt in body {
        let Stmt::Expr(Expr::Literal(Literal::String { value, .. })) = stmt else {
            return false;
        };
        if value == "use strict" {
            return true;
        }
    }
    false
}
