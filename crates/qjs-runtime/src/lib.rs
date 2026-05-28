//! Early interpreter for the Rust QuickJS rewrite.

use std::collections::{HashMap, HashSet};

use qjs_ast::{
    AssignmentOp, AssignmentTarget, BinaryOp, CatchClause, Expr, ForInLeft, ForInit, Literal,
    MemberProperty, ObjectPropertyKey, Script, Stmt, SwitchCase, UnaryOp, UpdateOp, VarKind,
};
use qjs_parser::parse_script;

mod array;
mod boolean;
mod builtins;
mod conversion;
mod function;
mod global;
mod math;
mod native;
mod number;
mod object;
mod operations;
mod property;
mod string;
mod value;

use builtins::initialize_builtins;
pub(crate) use conversion::{
    error_value, is_truthy, to_int32, to_int32_number, to_js_string, to_length, to_number,
    to_uint16, to_uint32, to_uint32_number,
};
use function::{Function, NativeFunction};
use native::call_native_function;
pub(crate) use property::{
    array_has_own_property, array_own_property_descriptor, array_own_property_keys,
    array_own_property_names, array_prototype, constructor_prototype,
    function_own_property_descriptor, function_own_property_keys, function_own_property_names,
    function_prototype, inherited_array_prototype_property, inherited_object_prototype_property,
    inherited_string_prototype_property, object_prototype, string_prototype, to_array_index,
    to_property_key, value_prototype,
};
pub use value::Value;
use value::{ArrayRef, ObjectRef, Property};

const GLOBAL_THIS_BINDING: &str = "\0global_this";

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
    env.insert(GLOBAL_THIS_BINDING.to_owned(), global_this.clone());
    env.insert("undefined".to_owned(), Value::Undefined);
    initialize_builtins(&mut env, &global_this);
    hoist_declarations(&script.body, &mut env);
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
                Value::Function(Function::new_user(
                    Some(name.clone()),
                    params.clone(),
                    body.clone(),
                    env.clone(),
                )),
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
    hoist_declarations(body, env);
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

fn hoist_declarations(body: &[Stmt], env: &mut HashMap<String, Value>) {
    hoist_var_declarations(body, env);
    hoist_function_declarations(body, env);
}

fn hoist_var_declarations(body: &[Stmt], env: &mut HashMap<String, Value>) {
    for stmt in body {
        match stmt {
            Stmt::VarDecl {
                kind: VarKind::Var,
                declarations,
                ..
            } => {
                for declaration in declarations {
                    env.entry(declaration.name.clone())
                        .or_insert(Value::Undefined);
                }
            }
            Stmt::Block { body, .. } => hoist_var_declarations(body, env),
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                hoist_var_declarations(std::slice::from_ref(consequent.as_ref()), env);
                if let Some(alternate) = alternate {
                    hoist_var_declarations(std::slice::from_ref(alternate.as_ref()), env);
                }
            }
            Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
                hoist_var_declarations(std::slice::from_ref(body.as_ref()), env);
            }
            Stmt::For { init, body, .. } => {
                if let Some(ForInit::VarDecl {
                    kind: VarKind::Var,
                    declarations,
                    ..
                }) = init
                {
                    for declaration in declarations {
                        env.entry(declaration.name.clone())
                            .or_insert(Value::Undefined);
                    }
                }
                hoist_var_declarations(std::slice::from_ref(body.as_ref()), env);
            }
            Stmt::ForIn { left, body, .. } => {
                if let ForInLeft::VarDecl {
                    kind: VarKind::Var,
                    name,
                    ..
                } = left
                {
                    env.entry(name.clone()).or_insert(Value::Undefined);
                }
                hoist_var_declarations(std::slice::from_ref(body.as_ref()), env);
            }
            Stmt::Switch { cases, .. } => {
                for case in cases {
                    hoist_var_declarations(&case.consequent, env);
                }
            }
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                hoist_var_declarations(block, env);
                if let Some(handler) = handler {
                    hoist_var_declarations(&handler.body, env);
                }
                if let Some(finalizer) = finalizer {
                    hoist_var_declarations(finalizer, env);
                }
            }
            Stmt::FunctionDecl { .. } => {}
            _ => {}
        }
    }
}

fn hoist_function_declarations(body: &[Stmt], env: &mut HashMap<String, Value>) {
    for stmt in body {
        if let Stmt::FunctionDecl {
            name, params, body, ..
        } = stmt
        {
            env.insert(
                name.clone(),
                Value::Function(Function::new_user(
                    Some(name.clone()),
                    params.clone(),
                    body.clone(),
                    env.clone(),
                )),
            );
        }
    }
}

fn collect_function_local_names(function: &Function) -> HashSet<String> {
    let mut names = HashSet::new();
    names.insert("this".to_owned());
    names.insert("arguments".to_owned());
    names.extend(function.params.iter().cloned());
    if let Some(name) = &function.name {
        names.insert(name.clone());
    }
    collect_statement_local_names(&function.body, &mut names);
    names
}

fn collect_statement_local_names(body: &[Stmt], names: &mut HashSet<String>) {
    for stmt in body {
        match stmt {
            Stmt::VarDecl { declarations, .. } => {
                names.extend(
                    declarations
                        .iter()
                        .map(|declaration| declaration.name.clone()),
                );
            }
            Stmt::FunctionDecl { name, .. } => {
                names.insert(name.clone());
            }
            Stmt::Block { body, .. } => collect_statement_local_names(body, names),
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                collect_statement_local_names(std::slice::from_ref(consequent.as_ref()), names);
                if let Some(alternate) = alternate {
                    collect_statement_local_names(std::slice::from_ref(alternate.as_ref()), names);
                }
            }
            Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
                collect_statement_local_names(std::slice::from_ref(body.as_ref()), names);
            }
            Stmt::For { init, body, .. } => {
                if let Some(ForInit::VarDecl { declarations, .. }) = init {
                    names.extend(
                        declarations
                            .iter()
                            .map(|declaration| declaration.name.clone()),
                    );
                }
                collect_statement_local_names(std::slice::from_ref(body.as_ref()), names);
            }
            Stmt::ForIn { left, body, .. } => {
                if let ForInLeft::VarDecl { name, .. } = left {
                    names.insert(name.clone());
                }
                collect_statement_local_names(std::slice::from_ref(body.as_ref()), names);
            }
            Stmt::Switch { cases, .. } => {
                for case in cases {
                    collect_statement_local_names(&case.consequent, names);
                }
            }
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                collect_statement_local_names(block, names);
                if let Some(handler) = handler {
                    if let Some(param) = &handler.param {
                        names.insert(param.clone());
                    }
                    collect_statement_local_names(&handler.body, names);
                }
                if let Some(finalizer) = finalizer {
                    collect_statement_local_names(finalizer, names);
                }
            }
            _ => {}
        }
    }
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
        Value::Object(object) => Ok(object.own_property_keys()),
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

    let argument_values = eval_arguments(arguments, env)?;
    call_function(callee, this_value, argument_values, env, false)
}

fn eval_new(
    callee: &Expr,
    arguments: &[Expr],
    env: &mut HashMap<String, Value>,
) -> Result<Value, RuntimeError> {
    let callee = eval_expr(callee, env)?;
    let Value::Function(function) = &callee else {
        return Err(RuntimeError {
            message: "value is not a constructor".to_owned(),
        });
    };
    if !function.constructable {
        return Err(RuntimeError {
            message: "value is not a constructor".to_owned(),
        });
    }
    let argument_values = eval_arguments(arguments, env)?;
    let prototype = constructor_prototype(&callee);
    let this_value = Value::Object(ObjectRef::with_prototype(HashMap::new(), prototype));
    let result = call_function(callee, this_value.clone(), argument_values, env, true)?;
    match result {
        Value::Array(_) | Value::Function(_) | Value::Object(_) => Ok(result),
        _ => Ok(this_value),
    }
}

fn eval_arguments(
    arguments: &[Expr],
    env: &mut HashMap<String, Value>,
) -> Result<Vec<Value>, RuntimeError> {
    let mut argument_values = Vec::with_capacity(arguments.len());
    for argument in arguments {
        argument_values.push(eval_expr(argument, env)?);
    }
    Ok(argument_values)
}

fn call_function(
    callee: Value,
    this_value: Value,
    argument_values: Vec<Value>,
    env: &mut HashMap<String, Value>,
    is_construct: bool,
) -> Result<Value, RuntimeError> {
    let Value::Function(function) = callee.clone() else {
        return Err(RuntimeError {
            message: "value is not callable".to_owned(),
        });
    };
    if let Some(native) = function.native {
        return call_native_function(
            &function,
            native,
            this_value,
            argument_values,
            is_construct,
            env,
        );
    }
    let caller_names: Vec<String> = env.keys().cloned().collect();
    let function_local_names = collect_function_local_names(&function);
    let mut local_env = env.clone();
    for (name, value) in &function.env {
        local_env
            .entry(name.clone())
            .or_insert_with(|| value.clone());
    }
    if let Some(global_this) = env.get(GLOBAL_THIS_BINDING).cloned() {
        local_env.insert(GLOBAL_THIS_BINDING.to_owned(), global_this);
    }
    if let Some(name) = &function.name {
        local_env.insert(name.clone(), callee);
    }
    local_env.insert("this".to_owned(), this_value);
    local_env.insert(
        "arguments".to_owned(),
        Value::Array(ArrayRef::new(argument_values.clone())),
    );
    for (index, param) in function.params.iter().enumerate() {
        let value = argument_values
            .get(index)
            .cloned()
            .unwrap_or(Value::Undefined);
        local_env.insert(param.clone(), value);
    }

    let completion = eval_statement_list(&function.body, &mut local_env)?;
    for name in caller_names {
        if name != GLOBAL_THIS_BINDING && !function_local_names.contains(&name) {
            if let Some(value) = local_env.get(&name) {
                env.insert(name, value.clone());
            }
        }
    }

    match completion {
        Completion::Normal(value) => Ok(value),
        Completion::Return(value) => Ok(value),
        Completion::Break | Completion::Continue => Err(RuntimeError {
            message: "break or continue outside loop".to_owned(),
        }),
        Completion::Throw(value) => Err(RuntimeError {
            message: format!("throw statement executed: {}", error_value(value)),
        }),
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
            Ok(Value::Array(ArrayRef::new(values)))
        }
        Expr::Object { properties, .. } => {
            let mut values = HashMap::new();
            for property in properties {
                let key = match &property.key {
                    ObjectPropertyKey::Literal(key) => key.clone(),
                    ObjectPropertyKey::Computed(expr) => to_property_key(eval_expr(expr, env)?)?,
                };
                values.insert(key, eval_expr(&property.value, env)?);
            }
            Ok(Value::Object(ObjectRef::with_prototype(
                values,
                object_prototype(env),
            )))
        }
        Expr::Function {
            name,
            params,
            body,
            constructable,
            ..
        } => Ok(Value::Function(Function::new_user_with_constructable(
            name.clone(),
            params.clone(),
            body.clone(),
            env.clone(),
            *constructable,
        ))),
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
            operations::eval_unary(*op, argument)
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
        Expr::New {
            callee, arguments, ..
        } => eval_new(callee, arguments, env),
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
            operations::eval_binary(left, *op, right, env)
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
        AssignmentOp::AddAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::Add, right, env)?
        }
        AssignmentOp::SubAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::Sub, right, env)?
        }
        AssignmentOp::MulAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::Mul, right, env)?
        }
        AssignmentOp::PowAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::Pow, right, env)?
        }
        AssignmentOp::DivAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::Div, right, env)?
        }
        AssignmentOp::RemAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::Rem, right, env)?
        }
        AssignmentOp::ShlAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::Shl, right, env)?
        }
        AssignmentOp::ShrAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::Shr, right, env)?
        }
        AssignmentOp::UShrAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::UShr, right, env)?
        }
        AssignmentOp::BitwiseAndAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::BitwiseAnd, right, env)?
        }
        AssignmentOp::BitwiseXorAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::BitwiseXor, right, env)?
        }
        AssignmentOp::BitwiseOrAssign => {
            operations::eval_binary(read_target(target, env)?, BinaryOp::BitwiseOr, right, env)?
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
        (Value::Array(_), MemberProperty::Named(name)) => {
            Ok(inherited_array_prototype_property(env, name).unwrap_or(Value::Undefined))
        }
        (Value::Function(function), MemberProperty::Named(name)) if name == "length" => {
            Ok(Value::Number(function.params.len() as f64))
        }
        (Value::Function(function), property) => {
            let key = property_key(property, env)?;
            Ok(function
                .properties
                .borrow()
                .get(&key)
                .map(|property| property.value.clone())
                .or_else(|| inherited_object_prototype_property(env, &key))
                .unwrap_or(Value::Undefined))
        }
        (Value::String(value), MemberProperty::Named(name)) if name == "length" => {
            Ok(Value::Number(value.chars().count() as f64))
        }
        (Value::String(value), property) => {
            let key = property_key(property, env)?;
            Ok(string::string_property(&value, &key)
                .or_else(|| inherited_string_prototype_property(env, &key))
                .unwrap_or(Value::Undefined))
        }
        (Value::Boolean(_), MemberProperty::Named(name)) => Ok(
            boolean::inherited_boolean_prototype_property(env, name).unwrap_or(Value::Undefined),
        ),
        (Value::Number(_), MemberProperty::Named(name)) => {
            Ok(number::inherited_number_prototype_property(env, name).unwrap_or(Value::Undefined))
        }
        (Value::Array(elements), MemberProperty::Computed(index)) => {
            let index = eval_expr(index, env)?;
            let index = to_array_index(index)?;
            Ok(elements.get(index).unwrap_or(Value::Undefined))
        }
        (Value::Object(object), property) => {
            let key = property_key(property, env)?;
            Ok(object.get(&key).unwrap_or(Value::Undefined))
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
    let key = property_key(property, env)?;
    match object {
        Value::Object(object) => {
            object.set(key, value);
            Ok(())
        }
        Value::Function(function) => {
            function
                .properties
                .borrow_mut()
                .insert(key, Property::enumerable(value));
            Ok(())
        }
        Value::Array(elements) => {
            if key == "length" {
                elements.set_len(to_length(value)?);
            } else {
                let index = key.parse::<usize>().map_err(|_| RuntimeError {
                    message: "array property assignment requires an array index".to_owned(),
                })?;
                elements.set(index, value);
            }
            Ok(())
        }
        _ => Err(RuntimeError {
            message: "member assignment target is not an object".to_owned(),
        }),
    }
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
            object.delete_own_property(&key);
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

#[cfg(test)]
mod tests;
