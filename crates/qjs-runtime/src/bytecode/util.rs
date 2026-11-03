use qjs_ast::{AssignmentOp, AssignmentTarget, BinaryOp, Stmt};

use crate::{JsString, RuntimeError, Value, symbol};

pub(super) fn assignment_binary_op(op: AssignmentOp) -> Result<BinaryOp, RuntimeError> {
    match op {
        AssignmentOp::AddAssign => Ok(BinaryOp::Add),
        AssignmentOp::SubAssign => Ok(BinaryOp::Sub),
        AssignmentOp::MulAssign => Ok(BinaryOp::Mul),
        AssignmentOp::PowAssign => Ok(BinaryOp::Pow),
        AssignmentOp::DivAssign => Ok(BinaryOp::Div),
        AssignmentOp::RemAssign => Ok(BinaryOp::Rem),
        AssignmentOp::ShlAssign => Ok(BinaryOp::Shl),
        AssignmentOp::ShrAssign => Ok(BinaryOp::Shr),
        AssignmentOp::UShrAssign => Ok(BinaryOp::UShr),
        AssignmentOp::BitwiseAndAssign => Ok(BinaryOp::BitwiseAnd),
        AssignmentOp::BitwiseXorAssign => Ok(BinaryOp::BitwiseXor),
        AssignmentOp::BitwiseOrAssign => Ok(BinaryOp::BitwiseOr),
        AssignmentOp::Assign
        | AssignmentOp::LogicalAndAssign
        | AssignmentOp::LogicalOrAssign
        | AssignmentOp::NullishAssign => Err(RuntimeError {
            thrown: None,
            message: "assignment operator has no binary equivalent".to_owned(),
        }),
    }
}

pub(super) fn parse_number_literal(raw: &str) -> Result<f64, RuntimeError> {
    let cleaned = raw.replace('_', "");
    if let Some(digits) = cleaned
        .strip_prefix("0x")
        .or_else(|| cleaned.strip_prefix("0X"))
    {
        Ok(parse_radix_number(digits, 16))
    } else if let Some(digits) = cleaned
        .strip_prefix("0b")
        .or_else(|| cleaned.strip_prefix("0B"))
    {
        Ok(parse_radix_number(digits, 2))
    } else if let Some(digits) = cleaned
        .strip_prefix("0o")
        .or_else(|| cleaned.strip_prefix("0O"))
    {
        Ok(parse_radix_number(digits, 8))
    } else if cleaned.len() > 1
        && cleaned.starts_with('0')
        && cleaned.bytes().all(|byte| matches!(byte, b'0'..=b'7'))
    {
        Ok(parse_radix_number(&cleaned[1..], 8))
    } else {
        cleaned.parse::<f64>().map_err(|_| RuntimeError {
            thrown: None,
            message: format!("invalid number literal `{raw}`"),
        })
    }
}

fn parse_radix_number(digits: &str, radix: u32) -> f64 {
    digits.chars().fold(0.0, |value, digit| {
        value * f64::from(radix) + f64::from(digit.to_digit(radix).unwrap_or(0))
    })
}

thread_local! {
    /// The eight `typeof` results, built on first use and never mutated.
    /// Every `typeof` used to allocate its result; sharing is unobservable
    /// because strings are immutable (see `ASCII_CODE_UNIT_STRINGS`).
    static TYPEOF_NAMES: [JsString; 8] = TYPEOF_TEXTS.map(JsString::atom);
}

const TYPEOF_TEXTS: [&str; 8] = [
    "undefined",
    "boolean",
    "number",
    "bigint",
    "string",
    "function",
    "symbol",
    "object",
];

/// The shared atom for a string literal naming a `typeof` result, so the
/// literal in `typeof x === "number"` is the very instance `typeof` returns
/// and the comparison is a pointer test.
pub(super) fn typeof_name_atom(text: &str) -> Option<JsString> {
    let index = TYPEOF_TEXTS.iter().position(|name| *name == text)?;
    Some(TYPEOF_NAMES.with(|names| names[index].clone()))
}

pub(super) fn typeof_value(value: Value) -> JsString {
    let index = if crate::html_dda::is_html_dda(&value) {
        0
    } else {
        match value {
            Value::Undefined => 0,
            Value::Boolean(_) => 1,
            Value::Number(_) => 2,
            Value::BigInt(_) => 3,
            Value::String(_) => 4,
            Value::Function(_) => 5,
            // A Proxy reports `function` exactly when its target is callable.
            Value::Proxy(ref proxy) if crate::proxy::proxy_is_callable(proxy) => 5,
            Value::Object(object) if symbol::is_symbol_primitive(&object) => 6,
            Value::Null
            | Value::Array(_)
            | Value::Map(_)
            | Value::Set(_)
            | Value::Object(_)
            | Value::Proxy(_) => 7,
        }
    };
    TYPEOF_NAMES.with(|names| names[index].clone())
}

pub(super) fn is_object_value(value: &Value) -> bool {
    matches!(
        value,
        Value::Array(_)
            | Value::Function(_)
            | Value::Map(_)
            | Value::Object(_)
            | Value::Proxy(_)
            | Value::Set(_)
    )
}

pub(super) fn stack_underflow() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "bytecode stack underflow".to_owned(),
    }
}

pub(super) fn unsupported_stmt(stmt: &Stmt) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("unsupported bytecode statement: {stmt:?}"),
    }
}

/// The error for a module `import`/`export` item reaching the script
/// statement compiler. Modules compile through `compiler::compile_module`.
pub(super) fn unsupported_module_item() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "modules are not yet supported".to_owned(),
    }
}

/// Whether evaluating `stmt` stores a value in the completion register before
/// it can complete normally or by `break`/`continue`, so a statement that
/// would first reset the register to `undefined` may skip the reset. Every
/// statement whose completion is never empty writes the register on entry or
/// first; a block does when its first statement that is not a declaration
/// does.
pub(super) fn stmt_writes_completion_first(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expr(_)
        | Stmt::If { .. }
        | Stmt::With { .. }
        | Stmt::While { .. }
        | Stmt::DoWhile { .. }
        | Stmt::For { .. }
        | Stmt::ForIn { .. }
        | Stmt::ForOf { .. }
        | Stmt::Switch { .. }
        | Stmt::Try { .. } => true,
        Stmt::Block { body, .. } => stmts_write_completion_first(body),
        Stmt::Labelled { body, .. } => stmt_writes_completion_first(body),
        _ => false,
    }
}

/// `stmt_writes_completion_first` for a statement list.
pub(super) fn stmts_write_completion_first(body: &[Stmt]) -> bool {
    for stmt in body {
        if stmt_writes_completion_first(stmt) {
            return true;
        }
        // Declarations complete empty and cannot jump.
        if !matches!(
            stmt,
            Stmt::VarDecl { .. }
                | Stmt::FunctionDecl { .. }
                | Stmt::ClassDecl { .. }
                | Stmt::Empty
                | Stmt::Debugger { .. }
        ) {
            return false;
        }
    }
    false
}

/// Whether a statement list contains a `break` or `continue` outside nested
/// functions -- the only way a `finally` block's own completion value can
/// escape it.
pub(super) fn stmts_contain_break_or_continue(body: &[Stmt]) -> bool {
    body.iter().any(stmt_contains_break_or_continue)
}

fn stmt_contains_break_or_continue(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Break { .. } | Stmt::Continue { .. } => true,
        Stmt::Block { body, .. } => stmts_contain_break_or_continue(body),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            stmt_contains_break_or_continue(consequent)
                || alternate
                    .as_deref()
                    .is_some_and(stmt_contains_break_or_continue)
        }
        Stmt::While { body, .. }
        | Stmt::DoWhile { body, .. }
        | Stmt::For { body, .. }
        | Stmt::ForIn { body, .. }
        | Stmt::ForOf { body, .. }
        | Stmt::Labelled { body, .. }
        | Stmt::With { body, .. } => stmt_contains_break_or_continue(body),
        Stmt::Switch { cases, .. } => cases
            .iter()
            .any(|case| stmts_contain_break_or_continue(&case.consequent)),
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            stmts_contain_break_or_continue(block)
                || handler
                    .as_ref()
                    .is_some_and(|handler| stmts_contain_break_or_continue(&handler.body))
                || finalizer
                    .as_deref()
                    .is_some_and(stmts_contain_break_or_continue)
        }
        _ => false,
    }
}

/// Whether a pending label may attach directly to `stmt` (the iteration and
/// switch statements that observe labelled break/continue).
pub(super) fn stmt_accepts_pending_label(stmt: &Stmt) -> bool {
    matches!(
        stmt,
        Stmt::Labelled { .. }
            | Stmt::While { .. }
            | Stmt::DoWhile { .. }
            | Stmt::For { .. }
            | Stmt::ForIn { .. }
            | Stmt::ForOf { .. }
            | Stmt::Switch { .. }
    )
}

pub(super) fn unsupported_target(target: &AssignmentTarget) -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: format!("unsupported bytecode assignment target: {target:?}"),
    }
}
