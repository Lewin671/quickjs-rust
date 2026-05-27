//! Early interpreter for the Rust QuickJS rewrite.

use std::{cell::RefCell, collections::HashMap, rc::Rc};

use qjs_ast::{
    AssignmentOp, AssignmentTarget, BinaryOp, CatchClause, Expr, ForInLeft, ForInit, Literal,
    MemberProperty, Script, Stmt, SwitchCase, UnaryOp, UpdateOp,
};
use qjs_parser::parse_script;

const GLOBAL_THIS_BINDING: &str = "\0global_this";

/// A JavaScript value supported by the current runtime subset.
#[derive(Clone, Debug)]
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
    /// Object value.
    Object(ObjectRef),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Number(left), Self::Number(right)) => left == right,
            (Self::String(left), Self::String(right)) => left == right,
            (Self::Boolean(left), Self::Boolean(right)) => left == right,
            (Self::Null, Self::Null) | (Self::Undefined, Self::Undefined) => true,
            (Self::Function(left), Self::Function(right)) => left == right,
            (Self::Array(left), Self::Array(right)) => left == right,
            (Self::Object(left), Self::Object(right)) => left.ptr_eq(right),
            _ => false,
        }
    }
}

/// Object storage reference.
#[derive(Clone, Debug)]
pub struct ObjectRef {
    properties: Rc<RefCell<HashMap<String, Value>>>,
}

impl ObjectRef {
    fn new(properties: HashMap<String, Value>) -> Self {
        Self {
            properties: Rc::new(RefCell::new(properties)),
        }
    }

    fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.properties, &other.properties)
    }
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
    let global_this = Value::Object(ObjectRef::new(HashMap::new()));
    env.insert("this".to_owned(), global_this.clone());
    env.insert(GLOBAL_THIS_BINDING.to_owned(), global_this);
    env.insert("undefined".to_owned(), Value::Undefined);
    let mut last = Value::Undefined;
    for stmt in &script.body {
        match eval_stmt(stmt, &mut env)? {
            Completion::Normal(value) => last = value,
            Completion::Return(value) => return Ok(value),
            Completion::Break | Completion::Continue => {
                return Err(RuntimeError {
                    message: "break or continue outside loop".to_owned(),
                });
            }
            Completion::Throw(value) => {
                return Err(RuntimeError {
                    message: format!("throw statement executed: {}", error_value(value)),
                });
            }
        }
    }
    Ok(last)
}

enum Completion {
    Normal(Value),
    Return(Value),
    Break,
    Continue,
    Throw(Value),
}

fn eval_stmt(stmt: &Stmt, env: &mut HashMap<String, Value>) -> Result<Completion, RuntimeError> {
    match stmt {
        Stmt::Expr(expr) => eval_expr(expr, env).map(Completion::Normal),
        Stmt::Block { body, .. } => eval_statement_list(body, env),
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
                    Completion::Break => break,
                    Completion::Continue => {}
                    Completion::Throw(value) => return Ok(Completion::Throw(value)),
                }
            }
            Ok(Completion::Normal(last))
        }
        Stmt::DoWhile { body, test, .. } => {
            let mut last = Value::Undefined;
            loop {
                match eval_stmt(body, env)? {
                    Completion::Normal(value) => last = value,
                    Completion::Return(value) => return Ok(Completion::Return(value)),
                    Completion::Break => break,
                    Completion::Continue => {}
                    Completion::Throw(value) => return Ok(Completion::Throw(value)),
                }
                if !is_truthy(&eval_expr(test, env)?) {
                    break;
                }
            }
            Ok(Completion::Normal(last))
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            if let Some(init) = init {
                eval_for_init(init, env)?;
            }
            let mut last = Value::Undefined;
            while test.as_ref().map_or(Ok(true), |test| {
                eval_expr(test, env).map(|value| is_truthy(&value))
            })? {
                match eval_stmt(body, env)? {
                    Completion::Normal(value) => last = value,
                    Completion::Return(value) => return Ok(Completion::Return(value)),
                    Completion::Break => break,
                    Completion::Continue => {}
                    Completion::Throw(value) => return Ok(Completion::Throw(value)),
                }
                if let Some(update) = update {
                    eval_expr(update, env)?;
                }
            }
            Ok(Completion::Normal(last))
        }
        Stmt::ForIn {
            left, right, body, ..
        } => {
            let keys = enumerable_keys(eval_expr(right, env)?)?;
            let mut last = Value::Undefined;
            for key in keys {
                assign_for_in_left(left, Value::String(key), env)?;
                match eval_stmt(body, env)? {
                    Completion::Normal(value) => last = value,
                    Completion::Return(value) => return Ok(Completion::Return(value)),
                    Completion::Break => break,
                    Completion::Continue => {}
                    Completion::Throw(value) => return Ok(Completion::Throw(value)),
                }
            }
            Ok(Completion::Normal(last))
        }
        Stmt::Switch {
            discriminant,
            cases,
            ..
        } => eval_switch(discriminant, cases, env),
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => eval_try(block, handler.as_ref(), finalizer.as_deref(), env),
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
        Stmt::Throw { argument, .. } => {
            let value = if let Some(argument) = argument {
                eval_expr(argument, env)?
            } else {
                Value::Undefined
            };
            Ok(Completion::Throw(value))
        }
        Stmt::Debugger { .. } => Ok(Completion::Normal(Value::Undefined)),
        Stmt::Break { .. } => Ok(Completion::Break),
        Stmt::Continue { .. } => Ok(Completion::Continue),
        Stmt::VarDecl { declarations, .. } => {
            for declaration in declarations {
                let value = if let Some(init) = &declaration.init {
                    eval_expr(init, env)?
                } else {
                    Value::Undefined
                };
                env.insert(declaration.name.clone(), value);
            }
            Ok(Completion::Normal(Value::Undefined))
        }
        Stmt::Empty => Ok(Completion::Normal(Value::Undefined)),
    }
}

fn eval_statement_list(
    body: &[Stmt],
    env: &mut HashMap<String, Value>,
) -> Result<Completion, RuntimeError> {
    let mut last = Value::Undefined;
    for stmt in body {
        match eval_stmt(stmt, env)? {
            Completion::Normal(value) => last = value,
            Completion::Return(value) => return Ok(Completion::Return(value)),
            Completion::Break => return Ok(Completion::Break),
            Completion::Continue => return Ok(Completion::Continue),
            Completion::Throw(value) => return Ok(Completion::Throw(value)),
        }
    }
    Ok(Completion::Normal(last))
}

fn eval_switch(
    discriminant: &Expr,
    cases: &[SwitchCase],
    env: &mut HashMap<String, Value>,
) -> Result<Completion, RuntimeError> {
    let discriminant = eval_expr(discriminant, env)?;
    let mut default_index = None;
    let mut selected_index = None;

    for (index, case) in cases.iter().enumerate() {
        if let Some(test) = &case.test {
            if eval_expr(test, env)? == discriminant {
                selected_index = Some(index);
                break;
            }
        } else {
            default_index = Some(index);
        }
    }

    let Some(start_index) = selected_index.or(default_index) else {
        return Ok(Completion::Normal(Value::Undefined));
    };

    let mut last = Value::Undefined;
    for case in &cases[start_index..] {
        for stmt in &case.consequent {
            match eval_stmt(stmt, env)? {
                Completion::Normal(value) => last = value,
                Completion::Break => return Ok(Completion::Normal(last)),
                Completion::Return(value) => return Ok(Completion::Return(value)),
                Completion::Continue => return Ok(Completion::Continue),
                Completion::Throw(value) => return Ok(Completion::Throw(value)),
            }
        }
    }
    Ok(Completion::Normal(last))
}

fn eval_try(
    block: &[Stmt],
    handler: Option<&CatchClause>,
    finalizer: Option<&[Stmt]>,
    env: &mut HashMap<String, Value>,
) -> Result<Completion, RuntimeError> {
    let mut completion = match eval_statement_list(block, env)? {
        Completion::Throw(value) => {
            if let Some(handler) = handler {
                eval_catch(handler, value, env)?
            } else {
                Completion::Throw(value)
            }
        }
        other => other,
    };

    if let Some(finalizer) = finalizer {
        let final_completion = eval_statement_list(finalizer, env)?;
        completion = match final_completion {
            Completion::Normal(_) => completion,
            abrupt => abrupt,
        };
    }

    Ok(completion)
}

fn eval_catch(
    handler: &CatchClause,
    thrown: Value,
    env: &mut HashMap<String, Value>,
) -> Result<Completion, RuntimeError> {
    let previous = if let Some(param) = &handler.param {
        env.insert(param.clone(), thrown)
    } else {
        None
    };
    let completion = eval_statement_list(&handler.body, env);
    if let Some(param) = &handler.param {
        if let Some(value) = previous {
            env.insert(param.clone(), value);
        } else {
            env.remove(param);
        }
    }
    completion
}

fn eval_for_init(init: &ForInit, env: &mut HashMap<String, Value>) -> Result<(), RuntimeError> {
    match init {
        ForInit::VarDecl { declarations, .. } => {
            for declaration in declarations {
                let value = if let Some(init) = &declaration.init {
                    eval_expr(init, env)?
                } else {
                    Value::Undefined
                };
                env.insert(declaration.name.clone(), value);
            }
            Ok(())
        }
        ForInit::Expr(expr) => eval_expr(expr, env).map(|_| ()),
    }
}

fn assign_for_in_left(
    left: &ForInLeft,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    match left {
        ForInLeft::VarDecl { name, .. } => {
            env.insert(name.clone(), value);
            Ok(())
        }
        ForInLeft::Target(target) => assign_target(target, value, env),
    }
}

fn enumerable_keys(value: Value) -> Result<Vec<String>, RuntimeError> {
    match value {
        Value::Object(object) => Ok(object.properties.borrow().keys().cloned().collect()),
        Value::Array(elements) => Ok((0..elements.len()).map(|index| index.to_string()).collect()),
        Value::Null | Value::Undefined => Ok(Vec::new()),
        _ => Err(RuntimeError {
            message: "for-in target is not enumerable".to_owned(),
        }),
    }
}

fn eval_call(
    callee: &Expr,
    arguments: &[Expr],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let (callee, this_value) = match callee {
        Expr::Member {
            object, property, ..
        } => {
            let object = eval_expr(object, env)?;
            let callee = eval_member(object.clone(), property, env)?;
            (callee, object)
        }
        _ => {
            let callee = eval_expr(callee, env)?;
            let this_value = env
                .get(GLOBAL_THIS_BINDING)
                .cloned()
                .unwrap_or(Value::Undefined);
            (callee, this_value)
        }
    };

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
    local_env.insert("this".to_owned(), this_value);
    for (param, argument) in function.params.iter().zip(arguments) {
        let value = eval_expr(argument, env)?;
        local_env.insert(param.clone(), value);
    }

    let mut last = Value::Undefined;
    for stmt in &function.body {
        match eval_stmt(stmt, &mut local_env)? {
            Completion::Normal(value) => last = value,
            Completion::Return(value) => return Ok(value),
            Completion::Break | Completion::Continue => {
                return Err(RuntimeError {
                    message: "break or continue outside loop".to_owned(),
                });
            }
            Completion::Throw(value) => {
                return Err(RuntimeError {
                    message: format!("throw statement executed: {}", error_value(value)),
                });
            }
        }
    }
    Ok(last)
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
        Expr::Object { properties, .. } => {
            let mut values = HashMap::new();
            for property in properties {
                values.insert(property.key.clone(), eval_expr(&property.value, env)?);
            }
            Ok(Value::Object(ObjectRef::new(values)))
        }
        Expr::Sequence { expressions, .. } => {
            let mut last = Value::Undefined;
            for expression in expressions {
                last = eval_expr(expression, env)?;
            }
            Ok(last)
        }
        Expr::This { .. } => env.get("this").cloned().ok_or_else(|| RuntimeError {
            message: "missing this binding".to_owned(),
        }),
        Expr::Identifier { name, .. } => env.get(name).cloned().ok_or_else(|| RuntimeError {
            message: format!("undefined identifier `{name}`"),
        }),
        Expr::Unary {
            op: UnaryOp::Typeof,
            argument,
            ..
        } => eval_typeof(argument, env),
        Expr::Unary {
            op: UnaryOp::Delete,
            argument,
            ..
        } => eval_delete(argument, env),
        Expr::Unary { op, argument, .. } => {
            let argument = eval_expr(argument, env)?;
            eval_unary(*op, argument)
        }
        Expr::Assignment {
            target, op, value, ..
        } => eval_assignment(target, *op, value, env),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            let test = eval_expr(test, env)?;
            if is_truthy(&test) {
                eval_expr(consequent, env)
            } else {
                eval_expr(alternate, env)
            }
        }
        Expr::Update {
            target, op, prefix, ..
        } => eval_update(target, *op, *prefix, env),
        Expr::Call {
            callee, arguments, ..
        } => eval_call(callee, arguments, env),
        Expr::Member {
            object, property, ..
        } => {
            let object = eval_expr(object, env)?;
            eval_member(object, property, env)
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
        } if *op == BinaryOp::NullishCoalescing => {
            let left = eval_expr(left, env)?;
            if matches!(left, Value::Null | Value::Undefined) {
                eval_expr(right, env)
            } else {
                Ok(left)
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

fn assign_target(
    target: &AssignmentTarget,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    match target {
        AssignmentTarget::Identifier { name, .. } => {
            if !env.contains_key(name) {
                return Err(RuntimeError {
                    message: format!("undefined identifier `{name}`"),
                });
            }
            env.insert(name.clone(), value);
            Ok(())
        }
        AssignmentTarget::Member {
            object, property, ..
        } => {
            let object = eval_expr(object, env)?;
            assign_member(object, property, value, env)
        }
    }
}

fn read_target(
    target: &AssignmentTarget,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match target {
        AssignmentTarget::Identifier { name, .. } => {
            env.get(name).cloned().ok_or_else(|| RuntimeError {
                message: format!("undefined identifier `{name}`"),
            })
        }
        AssignmentTarget::Member {
            object, property, ..
        } => {
            let object = eval_expr(object, env)?;
            eval_member(object, property, env)
        }
    }
}

fn eval_assignment(
    target: &AssignmentTarget,
    op: AssignmentOp,
    right: &Expr,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let old_value = match op {
        AssignmentOp::LogicalAndAssign
        | AssignmentOp::LogicalOrAssign
        | AssignmentOp::NullishAssign => read_target(target, env)?,
        _ => Value::Undefined,
    };

    match op {
        AssignmentOp::LogicalAndAssign if !is_truthy(&old_value) => return Ok(old_value),
        AssignmentOp::LogicalOrAssign if is_truthy(&old_value) => return Ok(old_value),
        AssignmentOp::NullishAssign if !matches!(old_value, Value::Null | Value::Undefined) => {
            return Ok(old_value);
        }
        _ => {}
    }

    let right = eval_expr(right, env)?;
    let value = match op {
        AssignmentOp::Assign => right,
        AssignmentOp::AddAssign => eval_binary(read_target(target, env)?, BinaryOp::Add, right)?,
        AssignmentOp::SubAssign => eval_binary(read_target(target, env)?, BinaryOp::Sub, right)?,
        AssignmentOp::MulAssign => eval_binary(read_target(target, env)?, BinaryOp::Mul, right)?,
        AssignmentOp::PowAssign => eval_binary(read_target(target, env)?, BinaryOp::Pow, right)?,
        AssignmentOp::DivAssign => eval_binary(read_target(target, env)?, BinaryOp::Div, right)?,
        AssignmentOp::RemAssign => eval_binary(read_target(target, env)?, BinaryOp::Rem, right)?,
        AssignmentOp::ShlAssign => eval_binary(read_target(target, env)?, BinaryOp::Shl, right)?,
        AssignmentOp::ShrAssign => eval_binary(read_target(target, env)?, BinaryOp::Shr, right)?,
        AssignmentOp::UShrAssign => eval_binary(read_target(target, env)?, BinaryOp::UShr, right)?,
        AssignmentOp::BitwiseAndAssign => {
            eval_binary(read_target(target, env)?, BinaryOp::BitwiseAnd, right)?
        }
        AssignmentOp::BitwiseXorAssign => {
            eval_binary(read_target(target, env)?, BinaryOp::BitwiseXor, right)?
        }
        AssignmentOp::BitwiseOrAssign => {
            eval_binary(read_target(target, env)?, BinaryOp::BitwiseOr, right)?
        }
        AssignmentOp::LogicalAndAssign
        | AssignmentOp::LogicalOrAssign
        | AssignmentOp::NullishAssign => right,
    };
    assign_target(target, value.clone(), env)?;
    Ok(value)
}

fn eval_update(
    target: &AssignmentTarget,
    op: UpdateOp,
    prefix: bool,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let old_number = to_number(read_target(target, env)?)?;
    let new = match op {
        UpdateOp::Increment => Value::Number(old_number + 1.0),
        UpdateOp::Decrement => Value::Number(old_number - 1.0),
    };
    assign_target(target, new.clone(), env)?;
    if prefix {
        Ok(new)
    } else {
        Ok(Value::Number(old_number))
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

fn eval_member(
    object: Value,
    property: &MemberProperty,
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    match (object, property) {
        (Value::Array(elements), MemberProperty::Named(name)) if name == "length" => {
            Ok(Value::Number(elements.len() as f64))
        }
        (Value::Array(elements), MemberProperty::Computed(index)) => {
            let index = eval_expr(index, env)?;
            let index = to_array_index(index)?;
            Ok(elements.get(index).cloned().unwrap_or(Value::Undefined))
        }
        (Value::Object(object), property) => {
            let key = property_key(property, env)?;
            Ok(object
                .properties
                .borrow()
                .get(&key)
                .cloned()
                .unwrap_or(Value::Undefined))
        }
        (_, MemberProperty::Named(name)) => Err(RuntimeError {
            message: format!("unsupported property `{name}`"),
        }),
        (_, MemberProperty::Computed(_)) => Err(RuntimeError {
            message: "unsupported computed member access".to_owned(),
        }),
    }
}

fn assign_member(
    object: Value,
    property: &MemberProperty,
    value: Value,
    env: &mut HashMap<String, Value>,
) -> Result<(), RuntimeError> {
    let Value::Object(object) = object else {
        return Err(RuntimeError {
            message: "member assignment target is not an object".to_owned(),
        });
    };
    let key = property_key(property, env)?;
    object.properties.borrow_mut().insert(key, value);
    Ok(())
}

fn property_key(
    property: &MemberProperty,
    env: &mut HashMap<String, Value>,
) -> Result<String, RuntimeError> {
    match property {
        MemberProperty::Named(name) => Ok(name.clone()),
        MemberProperty::Computed(expr) => to_property_key(eval_expr(expr, env)?),
    }
}

fn to_property_key(value: Value) -> Result<String, RuntimeError> {
    match value {
        Value::String(value) => Ok(value),
        Value::Number(number) if number.fract() == 0.0 => Ok(format!("{number:.0}")),
        Value::Number(number) => Ok(number.to_string()),
        Value::Boolean(true) => Ok("true".to_owned()),
        Value::Boolean(false) => Ok("false".to_owned()),
        Value::Null => Ok("null".to_owned()),
        Value::Undefined => Ok("undefined".to_owned()),
        Value::Function(_) | Value::Array(_) | Value::Object(_) => Err(RuntimeError {
            message: "unsupported property key".to_owned(),
        }),
    }
}

fn to_array_index(value: Value) -> Result<usize, RuntimeError> {
    let number = to_number(value)?;
    if !number.is_finite() || number < 0.0 || number.fract() != 0.0 {
        return Err(RuntimeError {
            message: "array index must be a non-negative integer".to_owned(),
        });
    }
    Ok(number as usize)
}

fn eval_unary(op: UnaryOp, argument: Value) -> Result<Value, RuntimeError> {
    match op {
        UnaryOp::Not => Ok(Value::Boolean(!is_truthy(&argument))),
        UnaryOp::Plus => Ok(Value::Number(to_number(argument)?)),
        UnaryOp::Minus => Ok(Value::Number(-to_number(argument)?)),
        UnaryOp::BitwiseNot => Ok(Value::Number(f64::from(!to_int32(argument)?))),
        UnaryOp::Void => Ok(Value::Undefined),
        UnaryOp::Typeof | UnaryOp::Delete => {
            unreachable!("operator requires unevaluated operand handling")
        }
    }
}

fn eval_delete(expr: &Expr, env: &mut HashMap<String, Value>) -> Result<Value, RuntimeError> {
    let Expr::Member {
        object, property, ..
    } = expr
    else {
        return Ok(Value::Boolean(true));
    };

    let object = eval_expr(object, env)?;
    match object {
        Value::Object(object) => {
            let key = property_key(property, env)?;
            object.properties.borrow_mut().remove(&key);
            Ok(Value::Boolean(true))
        }
        Value::Array(_) => Ok(Value::Boolean(true)),
        _ => Err(RuntimeError {
            message: "delete target is not an object".to_owned(),
        }),
    }
}

fn eval_typeof(expr: &Expr, env: &mut HashMap<String, Value>) -> Result<Value, RuntimeError> {
    let value = match expr {
        Expr::Identifier { name, .. } => env.get(name).cloned().unwrap_or(Value::Undefined),
        _ => eval_expr(expr, env)?,
    };
    let type_name = match value {
        Value::Undefined => "undefined",
        Value::Boolean(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Function(_) => "function",
        Value::Null | Value::Array(_) | Value::Object(_) => "object",
    };
    Ok(Value::String(type_name.to_owned()))
}

fn eval_binary(left: Value, op: BinaryOp, right: Value) -> Result<Value, RuntimeError> {
    if op == BinaryOp::In {
        return eval_in(left, right);
    }

    match op {
        BinaryOp::Eq | BinaryOp::StrictEq => return Ok(Value::Boolean(left == right)),
        BinaryOp::Ne | BinaryOp::StrictNe => return Ok(Value::Boolean(left != right)),
        BinaryOp::Add if matches!(left, Value::String(_)) || matches!(right, Value::String(_)) => {
            return Ok(Value::String(format!(
                "{}{}",
                to_js_string(left)?,
                to_js_string(right)?
            )));
        }
        _ => {}
    }

    let left = to_number(left)?;
    let right = to_number(right)?;

    let value = match op {
        BinaryOp::Add => left + right,
        BinaryOp::Sub => left - right,
        BinaryOp::Mul => left * right,
        BinaryOp::Pow => left.powf(right),
        BinaryOp::Div => left / right,
        BinaryOp::Rem => left % right,
        BinaryOp::Shl => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) << (to_uint32_number(right) & 0x1f),
            )));
        }
        BinaryOp::Shr => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) >> (to_uint32_number(right) & 0x1f),
            )));
        }
        BinaryOp::UShr => {
            return Ok(Value::Number(f64::from(
                to_uint32_number(left) >> (to_uint32_number(right) & 0x1f),
            )));
        }
        BinaryOp::BitwiseAnd => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) & to_int32_number(right),
            )));
        }
        BinaryOp::BitwiseXor => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) ^ to_int32_number(right),
            )));
        }
        BinaryOp::BitwiseOr => {
            return Ok(Value::Number(f64::from(
                to_int32_number(left) | to_int32_number(right),
            )));
        }
        BinaryOp::Lt => return Ok(Value::Boolean(left < right)),
        BinaryOp::Le => return Ok(Value::Boolean(left <= right)),
        BinaryOp::Gt => return Ok(Value::Boolean(left > right)),
        BinaryOp::Ge => return Ok(Value::Boolean(left >= right)),
        BinaryOp::Eq
        | BinaryOp::StrictEq
        | BinaryOp::Ne
        | BinaryOp::StrictNe
        | BinaryOp::In
        | BinaryOp::LogicalAnd
        | BinaryOp::LogicalOr
        | BinaryOp::NullishCoalescing => unreachable!("handled before numeric binary evaluation"),
    };
    Ok(Value::Number(value))
}

fn to_js_string(value: Value) -> Result<String, RuntimeError> {
    match value {
        Value::Number(number) if number.fract() == 0.0 => Ok(format!("{number:.0}")),
        Value::Number(number) => Ok(number.to_string()),
        Value::String(value) => Ok(value),
        Value::Boolean(true) => Ok("true".to_owned()),
        Value::Boolean(false) => Ok("false".to_owned()),
        Value::Null => Ok("null".to_owned()),
        Value::Undefined => Ok("undefined".to_owned()),
        Value::Function(_) | Value::Array(_) | Value::Object(_) => Err(RuntimeError {
            message: "cannot convert object to string".to_owned(),
        }),
    }
}

fn error_value(value: Value) -> String {
    match value {
        Value::Number(number) if number.fract() == 0.0 => format!("{number:.0}"),
        Value::Number(number) => number.to_string(),
        Value::String(value) => value,
        Value::Boolean(true) => "true".to_owned(),
        Value::Boolean(false) => "false".to_owned(),
        Value::Null => "null".to_owned(),
        Value::Undefined => "undefined".to_owned(),
        Value::Function(_) => "function".to_owned(),
        Value::Array(_) => "array".to_owned(),
        Value::Object(_) => "object".to_owned(),
    }
}

fn eval_in(left: Value, right: Value) -> Result<Value, RuntimeError> {
    let key = to_property_key(left)?;
    match right {
        Value::Object(object) => Ok(Value::Boolean(
            object.properties.borrow().contains_key(&key),
        )),
        Value::Array(elements) => {
            let index = key.parse::<usize>().ok();
            Ok(Value::Boolean(
                index.is_some_and(|index| index < elements.len()) || key == "length",
            ))
        }
        _ => Err(RuntimeError {
            message: "right operand of in is not an object".to_owned(),
        }),
    }
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
        Value::Array(_) | Value::Object(_) => Err(RuntimeError {
            message: "cannot convert object to number".to_owned(),
        }),
    }
}

fn to_int32(value: Value) -> Result<i32, RuntimeError> {
    to_number(value).map(to_int32_number)
}

fn to_int32_number(number: f64) -> i32 {
    let int = to_uint32_number(number);
    if int >= 0x8000_0000 {
        (i64::from(int) - 0x1_0000_0000) as i32
    } else {
        int as i32
    }
}

fn to_uint32_number(number: f64) -> u32 {
    if !number.is_finite() || number == 0.0 {
        return 0;
    }
    const TWO_32: f64 = 4_294_967_296.0;
    number.trunc().rem_euclid(TWO_32) as u32
}

fn is_truthy(value: &Value) -> bool {
    match value {
        Value::Number(number) => *number != 0.0 && !number.is_nan(),
        Value::String(value) => !value.is_empty(),
        Value::Boolean(value) => *value,
        Value::Null | Value::Undefined => false,
        Value::Function(_) | Value::Array(_) | Value::Object(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{Value, eval};

    #[test]
    fn evaluates_arithmetic() {
        assert_eq!(eval("1 + 2 * 3;"), Ok(Value::Number(7.0)));
        assert_eq!(eval("true + true;"), Ok(Value::Number(2.0)));
        assert_eq!(eval("true * 2;"), Ok(Value::Number(2.0)));
        assert_eq!(eval("2 ** 3;"), Ok(Value::Number(8.0)));
        assert_eq!(eval("2 ** 3 ** 2;"), Ok(Value::Number(512.0)));
        assert_eq!(eval("3 * 2 ** 3;"), Ok(Value::Number(24.0)));
        assert_eq!(eval("2 ** -1 * 2;"), Ok(Value::Number(1.0)));
    }

    #[test]
    fn evaluates_bitwise_and_shift_expressions() {
        assert_eq!(eval("5 & 3;"), Ok(Value::Number(1.0)));
        assert_eq!(eval("5 | 2;"), Ok(Value::Number(7.0)));
        assert_eq!(eval("5 ^ 3;"), Ok(Value::Number(6.0)));
        assert_eq!(eval("2 << 3;"), Ok(Value::Number(16.0)));
        assert_eq!(eval("-8 >> 1;"), Ok(Value::Number(-4.0)));
        assert_eq!(eval("-1 >>> 0;"), Ok(Value::Number(4_294_967_295.0)));
        assert_eq!(eval("~false;"), Ok(Value::Number(-1.0)));
        assert_eq!(eval("1 + 2 << 3;"), Ok(Value::Number(24.0)));
    }

    #[test]
    fn evaluates_string_addition() {
        assert_eq!(eval("'x' + 1;"), Ok(Value::String("x1".to_owned())));
        assert_eq!(eval("1 + 'x';"), Ok(Value::String("1x".to_owned())));
        assert_eq!(eval("'x' + true;"), Ok(Value::String("xtrue".to_owned())));
        assert_eq!(eval("'x' + null;"), Ok(Value::String("xnull".to_owned())));
        assert_eq!(
            eval("'x' + undefined;"),
            Ok(Value::String("xundefined".to_owned()))
        );
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
    fn evaluates_nullish_coalescing_expressions() {
        assert_eq!(eval("null ?? 42;"), Ok(Value::Number(42.0)));
        assert_eq!(eval("undefined ?? 42;"), Ok(Value::Number(42.0)));
        assert_eq!(eval("0 ?? 42;"), Ok(Value::Number(0.0)));
        assert_eq!(eval("false ?? 42;"), Ok(Value::Boolean(false)));
        assert_eq!(eval("42 ?? missing;"), Ok(Value::Number(42.0)));
        assert_eq!(eval("null ?? 0 ?? 1;"), Ok(Value::Number(0.0)));
    }

    #[test]
    fn evaluates_conditional_expressions() {
        assert_eq!(eval("true ? 1 : 2;"), Ok(Value::Number(1.0)));
        assert_eq!(eval("false ? 1 : 2;"), Ok(Value::Number(2.0)));
        assert_eq!(
            eval("let x = true ? 'yes' : 'no'; x;"),
            Ok(Value::String("yes".to_owned()))
        );
        assert_eq!(eval("true ? 1 : missing;"), Ok(Value::Number(1.0)));
        assert_eq!(eval("false ? missing : 2;"), Ok(Value::Number(2.0)));
    }

    #[test]
    fn evaluates_sequence_expressions() {
        assert_eq!(eval("1, 2;"), Ok(Value::Number(2.0)));
        assert_eq!(
            eval("let x = 0; x = 1, x = x + 2, x;"),
            Ok(Value::Number(3.0))
        );
        assert_eq!(
            eval("let x = 0; while ((x = x + 1, x < 3)) { } x;"),
            Ok(Value::Number(3.0))
        );
    }

    #[test]
    fn evaluates_variable_declarations() {
        assert_eq!(
            eval("let x = 2; const y = 3; x * y;"),
            Ok(Value::Number(6.0))
        );
        assert_eq!(eval("var missing; missing;"), Ok(Value::Undefined));
        assert_eq!(
            eval("var x = 1, y = 2, missing; x + y;"),
            Ok(Value::Number(3.0))
        );
    }

    #[test]
    fn evaluates_assignment_expressions() {
        assert_eq!(eval("let x = 2; x = x + 3; x;"), Ok(Value::Number(5.0)));
    }

    #[test]
    fn evaluates_update_and_compound_assignment() {
        assert_eq!(eval("let x = 1; x++; x;"), Ok(Value::Number(2.0)));
        assert_eq!(eval("let x = 1; ++x;"), Ok(Value::Number(2.0)));
        assert_eq!(eval("let x = 1; x++;"), Ok(Value::Number(1.0)));
        assert_eq!(eval("let x = false; x++;"), Ok(Value::Number(0.0)));
        assert_eq!(eval("let x = 3; x--; x;"), Ok(Value::Number(2.0)));
        assert_eq!(eval("let x = 1; x += 2; x;"), Ok(Value::Number(3.0)));
        assert_eq!(eval("let x = -3; x **= 3; x;"), Ok(Value::Number(-27.0)));
        assert_eq!(eval("let x = 2; x <<= 3; x;"), Ok(Value::Number(16.0)));
        assert_eq!(eval("let x = -8; x >>= 1; x;"), Ok(Value::Number(-4.0)));
        assert_eq!(
            eval("let x = -1; x >>>= 0; x;"),
            Ok(Value::Number(4_294_967_295.0))
        );
        assert_eq!(eval("let x = 5; x &= 3; x;"), Ok(Value::Number(1.0)));
        assert_eq!(eval("let x = 5; x ^= 3; x;"), Ok(Value::Number(6.0)));
        assert_eq!(eval("let x = 5; x |= 2; x;"), Ok(Value::Number(7.0)));
        assert_eq!(
            eval("let x = 'a'; x += 1; x;"),
            Ok(Value::String("a1".to_owned()))
        );
        assert_eq!(
            eval("let o = { count: 1 }; o.count++; o.count;"),
            Ok(Value::Number(2.0))
        );
    }

    #[test]
    fn evaluates_logical_assignment() {
        assert_eq!(eval("let x = 0; x &&= missing; x;"), Ok(Value::Number(0.0)));
        assert_eq!(eval("let x = 2; x &&= 7; x;"), Ok(Value::Number(7.0)));
        assert_eq!(eval("let x = 0; x ||= 7; x;"), Ok(Value::Number(7.0)));
        assert_eq!(eval("let x = 2; x ||= missing; x;"), Ok(Value::Number(2.0)));
        assert_eq!(eval("let x = null; x ??= 7; x;"), Ok(Value::Number(7.0)));
        assert_eq!(
            eval("let x = undefined; x ??= 8; x;"),
            Ok(Value::Number(8.0))
        );
        assert_eq!(
            eval("let x = false; x ??= missing; x;"),
            Ok(Value::Boolean(false))
        );
        assert_eq!(
            eval("let o = { value: 0 }; o.value ||= 3; o.value;"),
            Ok(Value::Number(3.0))
        );
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
    fn evaluates_do_while_statements() {
        assert_eq!(
            eval("let x = 0; do { x = x + 1; } while (false); x;"),
            Ok(Value::Number(1.0))
        );
        assert_eq!(
            eval("let x = 0; do { x++; } while (x < 3); x;"),
            Ok(Value::Number(3.0))
        );
        assert_eq!(
            eval("let x = 0; do { x++; if (x === 2) continue; } while (x < 3); x;"),
            Ok(Value::Number(3.0))
        );
    }

    #[test]
    fn evaluates_for_statements() {
        assert_eq!(
            eval("let sum = 0; for (var i = 0; i < 4; i = i + 1) { sum = sum + i; } sum;"),
            Ok(Value::Number(6.0))
        );
        assert_eq!(
            eval("let i = 0; for (; i < 3; ) i = i + 1; i;"),
            Ok(Value::Number(3.0))
        );
    }

    #[test]
    fn evaluates_for_in_statements() {
        assert_eq!(
            eval("let count = 0; for (var key in { a: 1, b: 2 }) { count++; } count;"),
            Ok(Value::Number(2.0))
        );
        assert_eq!(
            eval(
                "let total = 0; let item; let values = [1, 2, 3]; for (item in values) { total += values[item]; } total;"
            ),
            Ok(Value::Number(6.0))
        );
        assert_eq!(
            eval("let count = 0; for (var key in null) { count++; } count;"),
            Ok(Value::Number(0.0))
        );
    }

    #[test]
    fn evaluates_break_and_continue() {
        assert_eq!(
            eval("let i = 0; while (true) { i = i + 1; if (i === 3) break; } i;"),
            Ok(Value::Number(3.0))
        );
        assert_eq!(
            eval(
                "let sum = 0; for (var i = 0; i < 5; i = i + 1) { if (i === 2) continue; sum = sum + i; } sum;"
            ),
            Ok(Value::Number(8.0))
        );
    }

    #[test]
    fn evaluates_switch_statements() {
        assert_eq!(
            eval(
                "let x = 2; let out = 0; switch (x) { case 1: out = 1; break; case 2: out = 2; break; default: out = 3; } out;"
            ),
            Ok(Value::Number(2.0))
        );
        assert_eq!(
            eval(
                "let x = 4; let out = 0; switch (x) { case 1: out = 1; break; default: out = 3; } out;"
            ),
            Ok(Value::Number(3.0))
        );
        assert_eq!(
            eval(
                "let x = 1; let out = 0; switch (x) { case 1: out += 1; case 2: out += 2; default: out += 4; } out;"
            ),
            Ok(Value::Number(7.0))
        );
        assert_eq!(
            eval(
                "let x = '1'; let out = 0; switch (x) { case 1: out = 1; break; default: out = 2; } out;"
            ),
            Ok(Value::Number(2.0))
        );
    }

    #[test]
    fn evaluates_throw_statement_only_when_reached() {
        assert_eq!(eval("if (false) { throw; } 1;"), Ok(Value::Number(1.0)));
        assert_eq!(
            eval("if (false) { throw 'no'; } 1;"),
            Ok(Value::Number(1.0))
        );
        let error = eval("throw;").expect_err("throw should fail evaluation");
        assert_eq!(error.message, "throw statement executed: undefined");
        let error = eval("throw 'expected';").expect_err("throw should fail evaluation");
        assert_eq!(error.message, "throw statement executed: expected");
        let error = eval("throw 42;").expect_err("throw should fail evaluation");
        assert_eq!(error.message, "throw statement executed: 42");
    }

    #[test]
    fn evaluates_try_catch_finally_statements() {
        assert_eq!(
            eval("try { throw 'caught'; } catch (error) { error; }"),
            Ok(Value::String("caught".to_owned()))
        );
        assert_eq!(
            eval("let x = 1; try { throw 2; } catch (error) { x = error; } x;"),
            Ok(Value::Number(2.0))
        );
        assert_eq!(
            eval("let x = 1; try { x += 1; } finally { x += 2; } x;"),
            Ok(Value::Number(4.0))
        );
        assert_eq!(
            eval(
                "let x = 1; try { throw 1; } catch (error) { x += error; } finally { x += 2; } x;"
            ),
            Ok(Value::Number(4.0))
        );
        let error = eval("try { throw 'try'; } finally { throw 'finally'; }")
            .expect_err("throw should fail");
        assert_eq!(error.message, "throw statement executed: finally");
        assert_eq!(
            eval("let error = 'outer'; try { throw 'inner'; } catch (error) { error; } error;"),
            Ok(Value::String("outer".to_owned()))
        );
    }

    #[test]
    fn evaluates_debugger_statement_as_noop() {
        assert_eq!(eval("debugger; 1;"), Ok(Value::Number(1.0)));
        assert_eq!(
            eval("let x = 0; if (true) debugger; x = 2; x;"),
            Ok(Value::Number(2.0))
        );
    }

    #[test]
    fn evaluates_unary_expressions() {
        assert_eq!(eval("-1 + 3;"), Ok(Value::Number(2.0)));
        assert_eq!(eval("!0;"), Ok(Value::Boolean(true)));
        assert_eq!(eval("+true;"), Ok(Value::Number(1.0)));
        assert_eq!(eval("void 0;"), Ok(Value::Undefined));
        assert_eq!(eval("let x = 0; void (x = 1); x;"), Ok(Value::Number(1.0)));
    }

    #[test]
    fn evaluates_typeof_expressions() {
        assert_eq!(
            eval("typeof undefined;"),
            Ok(Value::String("undefined".to_owned()))
        );
        assert_eq!(
            eval("typeof neverDeclared;"),
            Ok(Value::String("undefined".to_owned()))
        );
        assert_eq!(
            eval("typeof true;"),
            Ok(Value::String("boolean".to_owned()))
        );
        assert_eq!(eval("typeof 1;"), Ok(Value::String("number".to_owned())));
        assert_eq!(eval("typeof 'x';"), Ok(Value::String("string".to_owned())));
        assert_eq!(eval("typeof null;"), Ok(Value::String("object".to_owned())));
        assert_eq!(eval("typeof {};"), Ok(Value::String("object".to_owned())));
        assert_eq!(eval("typeof this;"), Ok(Value::String("object".to_owned())));
        assert_eq!(
            eval("function f() { return 1; } typeof f;"),
            Ok(Value::String("function".to_owned()))
        );
    }

    #[test]
    fn evaluates_delete_operator() {
        assert_eq!(eval("let o = {}; delete o.x;"), Ok(Value::Boolean(true)));
        assert_eq!(
            eval("let o = { red: 1 }; delete o.red; o.red;"),
            Ok(Value::Undefined)
        );
        assert_eq!(
            eval("let o = { 2: 2 }; delete o[2]; o['2'];"),
            Ok(Value::Undefined)
        );
    }

    #[test]
    fn evaluates_in_operator() {
        assert_eq!(
            eval("'answer' in { answer: 42 };"),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval("'missing' in { answer: 42 };"),
            Ok(Value::Boolean(false))
        );
        assert_eq!(
            eval("let o = {}; o.present = undefined; 'present' in o;"),
            Ok(Value::Boolean(true))
        );
        assert_eq!(eval("'length' in [1, 2];"), Ok(Value::Boolean(true)));
    }

    #[test]
    fn evaluates_function_declarations_and_calls() {
        assert_eq!(
            eval("function add(a, b) { return a + b; } add(2, 3);"),
            Ok(Value::Number(5.0))
        );
        assert_eq!(
            eval("function getThis() { return this; } getThis() === this;"),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(
                "function getThis() { return this; } let o = {}; o.getThis = getThis; o.getThis() === o;"
            ),
            Ok(Value::Boolean(true))
        );
        assert_eq!(
            eval(
                "function getGlobal() { return this; } function method() { return getGlobal(); } let o = {}; o.method = method; o.method() === this;"
            ),
            Ok(Value::Boolean(true))
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

    #[test]
    fn evaluates_array_member_access() {
        assert_eq!(eval("let xs = [1, 2 + 3]; xs[1];"), Ok(Value::Number(5.0)));
        assert_eq!(eval("[1, 2, 3].length;"), Ok(Value::Number(3.0)));
    }

    #[test]
    fn evaluates_object_literals_and_member_access() {
        assert_eq!(
            eval("let o = { answer: 40 + 2 }; o.answer;"),
            Ok(Value::Number(42.0))
        );
        assert_eq!(eval("({ 'a': 1 })['a'];"), Ok(Value::Number(1.0)));
        assert_eq!(eval("({ true: 1 }).true;"), Ok(Value::Number(1.0)));
        assert_eq!(eval("({}).missing;"), Ok(Value::Undefined));
    }

    #[test]
    fn evaluates_member_assignment() {
        assert_eq!(
            eval("let o = {}; o.answer = 42; o.answer;"),
            Ok(Value::Number(42.0))
        );
        assert_eq!(
            eval("let key = 'answer'; let o = {}; o[key] = 7; o.answer;"),
            Ok(Value::Number(7.0))
        );
        assert_eq!(
            eval("this.answer = 42; this.answer;"),
            Ok(Value::Number(42.0))
        );
        assert_eq!(eval("this === this;"), Ok(Value::Boolean(true)));
    }

    #[test]
    fn evaluates_global_undefined_binding() {
        assert_eq!(eval("undefined;"), Ok(Value::Undefined));
        assert_eq!(eval("undefined === undefined;"), Ok(Value::Boolean(true)));
    }
}
