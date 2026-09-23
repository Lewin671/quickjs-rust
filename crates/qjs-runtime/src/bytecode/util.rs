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
    static TYPEOF_NAMES: [JsString; 8] = [
        "undefined", "boolean", "number", "bigint", "string", "function", "symbol", "object",
    ]
    .map(JsString::from);
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

/// Module `import`/`export` items parse under the Module goal but the runtime
/// does not yet support module linking or evaluation (T012).
pub(super) fn unsupported_module_item() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "modules are not yet supported".to_owned(),
    }
}

/// Whether `stmt`'s value updates a statement list's completion value (used to
/// decide between storing and popping the result of each statement).
pub(super) fn stmt_updates_statement_list_completion(stmt: &Stmt) -> bool {
    if let Stmt::Block { body, .. } = stmt {
        return body.iter().any(stmt_updates_statement_list_completion);
    }
    !matches!(
        stmt,
        Stmt::Debugger { .. }
            | Stmt::Empty
            | Stmt::FunctionDecl { .. }
            | Stmt::ClassDecl { .. }
            | Stmt::VarDecl { .. }
    )
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
