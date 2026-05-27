//! Early interpreter for the Rust QuickJS rewrite.

use std::collections::HashMap;

use qjs_ast::{BinaryOp, Expr, Literal, Script, Stmt, UnaryOp};
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
    /// User-defined function.
    Function(Function),
    /// Array value.
    Array(Vec<Value>),
}

/// User-defined function value.
#[derive(Clone, Debug, PartialEq)]
pub struct Function {
    /// Parameter names.
    pub params: Vec<String>,
    /// Function body statements.
    pub body: Vec<Stmt>,
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
    let mut env = HashMap::new();
    let mut last = Value::Undefined;
    for stmt in &script.body {
        match eval_stmt(stmt, &mut env)? {
            Completion::Normal(value) => last = value,
            Completion::Return(value) => return Ok(value),
        }
    }
    Ok(last)
}

enum Completion {
    Normal(Value),
    Return(Value),
}

fn eval_stmt(stmt: &Stmt, env: &mut HashMap<String, Value>) -> Result<Completion, RuntimeError> {
    match stmt {
        Stmt::Expr(expr) => eval_expr(expr, env).map(Completion::Normal),
        Stmt::Block { body, .. } => {
            let mut last = Value::Undefined;
            for stmt in body {
                match eval_stmt(stmt, env)? {
                    Completion::Normal(value) => last = value,
                    Completion::Return(value) => return Ok(Completion::Return(value)),
                }
            }
            Ok(Completion::Normal(last))
        }
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            let test = eval_expr(test, env)?;
            if is_truthy(&test) {
                eval_stmt(consequent, env)
            } else if let Some(alternate) = alternate {
                eval_stmt(alternate, env)
            } else {
                Ok(Completion::Normal(Value::Undefined))
            }
        }
        Stmt::While { test, body, .. } => {
            let mut last = Value::Undefined;
            while is_truthy(&eval_expr(test, env)?) {
                match eval_stmt(body, env)? {
                    Completion::Normal(value) => last = value,
                    Completion::Return(value) => return Ok(Completion::Return(value)),
                }
            }
            Ok(Completion::Normal(last))
        }
        Stmt::FunctionDecl {
            name, params, body, ..
        } => {
            env.insert(
                name.clone(),
                Value::Function(Function {
                    params: params.clone(),
                    body: body.clone(),
                }),
            );
            Ok(Completion::Normal(Value::Undefined))
        }
        Stmt::Return { argument, .. } => {
            let value = if let Some(argument) = argument {
                eval_expr(argument, env)?
            } else {
                Value::Undefined
            };
            Ok(Completion::Return(value))
        }
        Stmt::VarDecl { name, init, .. } => {
            let value = if let Some(init) = init {
                eval_expr(init, env)?
            } else {
                Value::Undefined
            };
            env.insert(name.clone(), value);
            Ok(Completion::Normal(Value::Undefined))
        }
        Stmt::Empty => Ok(Completion::Normal(Value::Undefined)),
    }
}

fn eval_expr(expr: &Expr, env: &mut HashMap<String, Value>) -> Result<Value, RuntimeError> {
    match expr {
        Expr::Literal(literal) => eval_literal(literal),
        Expr::Array { elements, .. } => {
            let mut values = Vec::with_capacity(elements.len());
            for element in elements {
                values.push(eval_expr(element, env)?);
            }
            Ok(Value::Array(values))
        }
        Expr::Identifier { name, .. } => env.get(name).cloned().ok_or_else(|| RuntimeError {
            message: format!("undefined identifier `{name}`"),
        }),
        Expr::Unary { op, argument, .. } => {
            let argument = eval_expr(argument, env)?;
            eval_unary(*op, argument)
        }
        Expr::Assignment { name, value, .. } => {
            if !env.contains_key(name) {
                return Err(RuntimeError {
                    message: format!("undefined identifier `{name}`"),
                });
            }
            let value = eval_expr(value, env)?;
            env.insert(name.clone(), value.clone());
            Ok(value)
        }
        Expr::Call {
            callee, arguments, ..
        } => {
            let callee = eval_expr(callee, env)?;
            let Value::Function(function) = callee else {
                return Err(RuntimeError {
                    message: "value is not callable".to_owned(),
                });
            };
            if arguments.len() != function.params.len() {
                return Err(RuntimeError {
                    message: format!(
                        "expected {} arguments, got {}",
                        function.params.len(),
                        arguments.len()
                    ),
                });
            }

            let mut local_env = env.clone();
            for (param, argument) in function.params.iter().zip(arguments) {
                let value = eval_expr(argument, env)?;
                local_env.insert(param.clone(), value);
            }

            let mut last = Value::Undefined;
            for stmt in &function.body {
                match eval_stmt(stmt, &mut local_env)? {
                    Completion::Normal(value) => last = value,
                    Completion::Return(value) => return Ok(value),
                }
            }
            Ok(last)
        }
        Expr::Binary {
            left, op, right, ..
        } if *op == BinaryOp::LogicalAnd => {
            let left = eval_expr(left, env)?;
            if is_truthy(&left) {
                eval_expr(right, env)
            } else {
                Ok(left)
            }
        }
        Expr::Binary {
            left, op, right, ..
        } if *op == BinaryOp::LogicalOr => {
            let left = eval_expr(left, env)?;
            if is_truthy(&left) {
                Ok(left)
            } else {
                eval_expr(right, env)
            }
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            let left = eval_expr(left, env)?;
            let right = eval_expr(right, env)?;
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

fn eval_unary(op: UnaryOp, argument: Value) -> Result<Value, RuntimeError> {
    match op {
        UnaryOp::Not => Ok(Value::Boolean(!is_truthy(&argument))),
        UnaryOp::Plus => Ok(Value::Number(to_number(argument)?)),
        UnaryOp::Minus => Ok(Value::Number(-to_number(argument)?)),
    }
}

fn eval_binary(left: Value, op: BinaryOp, right: Value) -> Result<Value, RuntimeError> {
    match op {
        BinaryOp::Eq | BinaryOp::StrictEq => return Ok(Value::Boolean(left == right)),
        BinaryOp::Ne | BinaryOp::StrictNe => return Ok(Value::Boolean(left != right)),
        _ => {}
    }

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
        BinaryOp::Rem => left % right,
        BinaryOp::Lt => return Ok(Value::Boolean(left < right)),
        BinaryOp::Le => return Ok(Value::Boolean(left <= right)),
        BinaryOp::Gt => return Ok(Value::Boolean(left > right)),
        BinaryOp::Ge => return Ok(Value::Boolean(left >= right)),
        BinaryOp::Eq
        | BinaryOp::StrictEq
        | BinaryOp::Ne
        | BinaryOp::StrictNe
        | BinaryOp::LogicalAnd
        | BinaryOp::LogicalOr => unreachable!("handled before numeric binary evaluation"),
    };
    Ok(Value::Number(value))
}

fn to_number(value: Value) -> Result<f64, RuntimeError> {
    match value {
        Value::Number(number) => Ok(number),
        Value::Boolean(true) => Ok(1.0),
        Value::Boolean(false) | Value::Null => Ok(0.0),
        Value::String(value) => value.parse::<f64>().map_err(|_| RuntimeError {
            message: format!("cannot convert string `{value}` to number"),
        }),
        Value::Undefined => Ok(f64::NAN),
        Value::Function(_) => Err(RuntimeError {
            message: "cannot convert function to number".to_owned(),
        }),
        Value::Array(_) => Err(RuntimeError {
            message: "cannot convert array to number".to_owned(),
        }),
    }
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Number(number) => *number != 0.0 && !number.is_nan(),
        Value::String(value) => !value.is_empty(),
        Value::Boolean(value) => *value,
        Value::Null | Value::Undefined => false,
        Value::Function(_) | Value::Array(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{Value, eval};

    #[test]
    fn evaluates_arithmetic() {
        assert_eq!(eval("1 + 2 * 3;"), Ok(Value::Number(7.0)));
    }

    #[test]
    fn evaluates_comparison_and_equality() {
        assert_eq!(eval("1 + 2 * 3 >= 7;"), Ok(Value::Boolean(true)));
        assert_eq!(eval("1 + 1 === 2;"), Ok(Value::Boolean(true)));
        assert_eq!(eval("1 !== 2;"), Ok(Value::Boolean(true)));
    }

    #[test]
    fn evaluates_logical_expressions() {
        assert_eq!(eval("0 || 5;"), Ok(Value::Number(5.0)));
        assert_eq!(eval("1 && 7;"), Ok(Value::Number(7.0)));
    }

    #[test]
    fn evaluates_variable_declarations() {
        assert_eq!(
            eval("let x = 2; const y = 3; x * y;"),
            Ok(Value::Number(6.0))
        );
        assert_eq!(eval("var missing; missing;"), Ok(Value::Undefined));
    }

    #[test]
    fn evaluates_assignment_expressions() {
        assert_eq!(eval("let x = 2; x = x + 3; x;"), Ok(Value::Number(5.0)));
    }

    #[test]
    fn evaluates_if_else_statements() {
        assert_eq!(
            eval("let x = 1; if (x > 0) { x = 7; } else { x = 3; } x;"),
            Ok(Value::Number(7.0))
        );
        assert_eq!(
            eval("let x = 1; if (x < 0) x = 7; else x = 3; x;"),
            Ok(Value::Number(3.0))
        );
    }

    #[test]
    fn evaluates_while_statements() {
        assert_eq!(
            eval("let x = 0; while (x < 3) { x = x + 1; } x;"),
            Ok(Value::Number(3.0))
        );
    }

    #[test]
    fn evaluates_unary_expressions() {
        assert_eq!(eval("-1 + 3;"), Ok(Value::Number(2.0)));
        assert_eq!(eval("!0;"), Ok(Value::Boolean(true)));
        assert_eq!(eval("+true;"), Ok(Value::Number(1.0)));
    }

    #[test]
    fn evaluates_function_declarations_and_calls() {
        assert_eq!(
            eval("function add(a, b) { return a + b; } add(2, 3);"),
            Ok(Value::Number(5.0))
        );
    }

    #[test]
    fn evaluates_array_literals() {
        assert_eq!(
            eval("[1, 2 + 3, true];"),
            Ok(Value::Array(vec![
                Value::Number(1.0),
                Value::Number(5.0),
                Value::Boolean(true),
            ]))
        );
    }
}
