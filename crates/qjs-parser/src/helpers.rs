use qjs_ast::{AssignmentTarget, Expr, Stmt, VarKind};
use qjs_lexer::TokenKind;

use crate::ParseError;

pub(crate) fn property_name(kind: TokenKind) -> Option<String> {
    match kind {
        TokenKind::Identifier(name) => Some(name),
        TokenKind::True => Some("true".to_owned()),
        TokenKind::False => Some("false".to_owned()),
        TokenKind::Null => Some("null".to_owned()),
        TokenKind::This => Some("this".to_owned()),
        TokenKind::Var => Some("var".to_owned()),
        TokenKind::Let => Some("let".to_owned()),
        TokenKind::Const => Some("const".to_owned()),
        TokenKind::If => Some("if".to_owned()),
        TokenKind::Else => Some("else".to_owned()),
        TokenKind::While => Some("while".to_owned()),
        TokenKind::Do => Some("do".to_owned()),
        TokenKind::For => Some("for".to_owned()),
        TokenKind::Switch => Some("switch".to_owned()),
        TokenKind::Case => Some("case".to_owned()),
        TokenKind::Default => Some("default".to_owned()),
        TokenKind::Try => Some("try".to_owned()),
        TokenKind::Catch => Some("catch".to_owned()),
        TokenKind::Finally => Some("finally".to_owned()),
        TokenKind::Break => Some("break".to_owned()),
        TokenKind::Continue => Some("continue".to_owned()),
        TokenKind::Function => Some("function".to_owned()),
        TokenKind::Return => Some("return".to_owned()),
        TokenKind::Throw => Some("throw".to_owned()),
        TokenKind::Debugger => Some("debugger".to_owned()),
        TokenKind::Typeof => Some("typeof".to_owned()),
        TokenKind::Void => Some("void".to_owned()),
        TokenKind::In => Some("in".to_owned()),
        TokenKind::Delete => Some("delete".to_owned()),
        TokenKind::Instanceof => Some("instanceof".to_owned()),
        _ => None,
    }
}

pub(crate) fn var_kind(kind: &TokenKind) -> Option<VarKind> {
    match kind {
        TokenKind::Var => Some(VarKind::Var),
        TokenKind::Let => Some(VarKind::Let),
        TokenKind::Const => Some(VarKind::Const),
        _ => None,
    }
}

pub(crate) fn assignment_target(expr: Expr) -> Result<AssignmentTarget, ParseError> {
    match expr {
        Expr::Identifier { name, span } => Ok(AssignmentTarget::Identifier { name, span }),
        Expr::Member {
            object,
            property,
            span,
        } => Ok(AssignmentTarget::Member {
            object,
            property,
            span,
        }),
        other => Err(ParseError {
            message: "invalid assignment target".to_owned(),
            span: other.span(),
        }),
    }
}

pub(crate) fn stmt_end(stmt: &Stmt) -> usize {
    match stmt {
        Stmt::Expr(expr) => expr.span().end,
        Stmt::Block { span, .. }
        | Stmt::If { span, .. }
        | Stmt::While { span, .. }
        | Stmt::DoWhile { span, .. }
        | Stmt::For { span, .. }
        | Stmt::ForIn { span, .. }
        | Stmt::ForOf { span, .. }
        | Stmt::Switch { span, .. }
        | Stmt::Try { span, .. }
        | Stmt::FunctionDecl { span, .. }
        | Stmt::Labelled { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::Throw { span, .. }
        | Stmt::Debugger { span }
        | Stmt::Break { span, .. }
        | Stmt::Continue { span, .. }
        | Stmt::VarDecl { span, .. } => span.end,
        Stmt::Empty => 0,
    }
}
