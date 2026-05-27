//! Early interpreter for the Rust QuickJS rewrite.

use qjs_ast::{BinaryOp, Expr, Literal, Script, Stmt};
use qjs_parser::parse_script;

/// A JavaScript value supported by the current runtime subset.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    /// Number value.
    Number(f64),
    /// String value.
    String(String),
    /// Boolean value.
    Boolean(bool),
    /// Null value.
    Null,
    /// Undefined value.
    Undefined,
}

/// Runtime error.
#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeError {
    /// Human-readable message.
    pub message: String,
}

/// Evaluates source text and returns the last statement value.
///
/// # Errors
///
/// Returns parser or runtime failures.
pub fn eval(source: &str) -> Result<Value, RuntimeError> {
    let script = parse_script(source).map_err(|error| RuntimeError {
        message: error.message,
    })?;
    eval_script(&script)
}

/// Evaluates an AST script.
///
/// # Errors
///
/// Returns runtime failures for unsupported operations.
pub fn eval_script(script: &Script) -> Result<Value, RuntimeError> {
    let mut last = Value::Undefined;
    for stmt in &script.body {
        last = eval_stmt(stmt)?;
    }
    Ok(last)
}

fn eval_stmt(stmt: &Stmt) -> Result<Value, RuntimeError> {
    match stmt {
        Stmt::Expr(expr) => eval_expr(expr),
        Stmt::Empty => Ok(Value::Undefined),
    }
}

fn eval_expr(expr: &Expr) -> Result<Value, RuntimeError> {
    match expr {
        Expr::Literal(literal) => eval_literal(literal),
        Expr::Identifier { name, .. } => Err(RuntimeError {
            message: format!("undefined identifier `{name}`"),
        }),
        Expr::Binary {
            left, op, right, ..
        } => {
            let left = eval_expr(left)?;
            let right = eval_expr(right)?;
            eval_binary(left, *op, right)
        }
    }
}

fn eval_literal(literal: &Literal) -> Result<Value, RuntimeError> {
    match literal {
        Literal::Number { raw, .. } => {
            raw.parse::<f64>()
                .map(Value::Number)
                .map_err(|_| RuntimeError {
                    message: format!("invalid number literal `{raw}`"),
                })
        }
        Literal::String { value, .. } => Ok(Value::String(value.clone())),
        Literal::Boolean { value, .. } => Ok(Value::Boolean(*value)),
        Literal::Null { .. } => Ok(Value::Null),
    }
}

fn eval_binary(left: Value, op: BinaryOp, right: Value) -> Result<Value, RuntimeError> {
    let (Value::Number(left), Value::Number(right)) = (left, right) else {
        return Err(RuntimeError {
            message: "only numeric binary operations are supported".to_owned(),
        });
    };

    let value = match op {
        BinaryOp::Add => left + right,
        BinaryOp::Sub => left - right,
        BinaryOp::Mul => left * right,
        BinaryOp::Div => left / right,
    };
    Ok(Value::Number(value))
}

#[cfg(test)]
mod tests {
    use super::{Value, eval};

    #[test]
    fn evaluates_arithmetic() {
        assert_eq!(eval("1 + 2 * 3;"), Ok(Value::Number(7.0)));
    }
}
