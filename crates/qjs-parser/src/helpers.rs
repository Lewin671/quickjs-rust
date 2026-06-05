use qjs_ast::{
    ArrayAssignmentElement, AssignmentOp, AssignmentTarget, Expr, ObjectAssignmentProperty,
    ObjectPropertyKind, Stmt, VarKind,
};
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
        TokenKind::With => Some("with".to_owned()),
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
        Expr::Object { properties, span } => {
            let properties = properties
                .into_iter()
                .map(|property| {
                    if property.kind != ObjectPropertyKind::Data {
                        return Err(ParseError {
                            message: "invalid destructuring assignment property".to_owned(),
                            span: property.span,
                        });
                    }
                    let target = destructuring_property_target(property.value)?;
                    Ok(ObjectAssignmentProperty {
                        key: property.key,
                        target,
                        span: property.span,
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            Ok(AssignmentTarget::Object { properties, span })
        }
        Expr::Array { elements, span } => {
            let elements = elements
                .into_iter()
                .map(|element| element.map(destructuring_array_element).transpose())
                .collect::<Result<Vec<_>, _>>()?;
            Ok(AssignmentTarget::Array { elements, span })
        }
        other => Err(ParseError {
            message: "invalid assignment target".to_owned(),
            span: other.span(),
        }),
    }
}

fn destructuring_array_element(expr: Expr) -> Result<ArrayAssignmentElement, ParseError> {
    let span = expr.span();
    match expr {
        Expr::Assignment {
            target,
            op: AssignmentOp::Assign,
            value,
            ..
        } => Ok(ArrayAssignmentElement {
            target,
            default: Some(*value),
            span,
        }),
        other => Ok(ArrayAssignmentElement {
            target: assignment_target(other)?,
            default: None,
            span,
        }),
    }
}

fn destructuring_property_target(expr: Expr) -> Result<AssignmentTarget, ParseError> {
    match expr {
        Expr::Assignment {
            target,
            op: AssignmentOp::Assign,
            value: _,
            ..
        } => Ok(target),
        other => assignment_target(other),
    }
}

pub(crate) fn stmt_end(stmt: &Stmt) -> usize {
    match stmt {
        Stmt::Expr(expr) => expr.span().end,
        Stmt::Block { span, .. }
        | Stmt::If { span, .. }
        | Stmt::While { span, .. }
        | Stmt::With { span, .. }
        | Stmt::DoWhile { span, .. }
        | Stmt::For { span, .. }
        | Stmt::ForIn { span, .. }
        | Stmt::ForOf { span, .. }
        | Stmt::Switch { span, .. }
        | Stmt::Try { span, .. }
        | Stmt::FunctionDecl { span, .. }
        | Stmt::ClassDecl { span, .. }
        | Stmt::Label { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::Throw { span, .. }
        | Stmt::Debugger { span }
        | Stmt::Break { span, .. }
        | Stmt::Continue { span, .. }
        | Stmt::VarDecl { span, .. } => span.end,
        Stmt::Empty => 0,
    }
}
