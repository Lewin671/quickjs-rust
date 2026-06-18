use std::{cell::RefCell, rc::Rc};

use qjs_ast::Stmt;
use qjs_parser::parse_dynamic_function_script;

use crate::CallEnv;
use crate::function::CompiledUserFunction;
use crate::{
    Function, GLOBAL_THIS_BINDING, NativeFunction, Prototype, RuntimeError, Value,
    array::array_like_values_with_env, object::boxed_primitive, property_value,
    to_js_string_with_env, to_length_with_env,
};

const DYNAMIC_FUNCTION_REALM_GLOBAL: &str = "__quickjsRustDynamicFunctionRealm";
const GENERATOR_FUNCTION_REALM_PROTOTYPE: &str = "__quickjsRustRealmGeneratorFunctionPrototype";

pub(crate) fn native_function(
    constructor: &Function,
    _this_value: Value,
    argument_values: &[Value],
    is_construct: bool,
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (params, body) = function_source_parts(argument_values, env)?;
    let source = format!("function anonymous({params}\n) {{\n{body}\n}}");
    let script = parse_dynamic_function_script(&source).map_err(|error| RuntimeError {
        thrown: None,
        message: format!(
            "SyntaxError: invalid Function constructor source: {}",
            error.message
        ),
    })?;

    let Some(Stmt::FunctionDecl {
        name, params, body, ..
    }) = script.body.into_iter().next()
    else {
        return Err(RuntimeError {
            thrown: None,
            message: "Function constructor did not produce a function declaration".to_owned(),
        });
    };

    let created = Function::new_user(
        Some(name),
        params,
        body,
        dynamic_function_scope_snapshot(env),
    )?;
    // `new Function(...)` derives the [[Prototype]] from new.target, but a plain
    // `Function(...)` call must ignore the ambient new.target (e.g. when invoked
    // inside another constructor's body) and use %Function.prototype%.
    let prototype_slot = if is_construct {
        crate::native_construct_prototype_slot(constructor, env)?
    } else {
        crate::function_prototype(constructor).map(Prototype::Object)
    };
    created
        .set_internal_prototype_slot(prototype_slot)
        .map_err(|_| RuntimeError {
            thrown: None,
            message: "TypeError: dynamic function prototype could not be set".to_owned(),
        })?;
    Ok(Value::Function(created))
}

pub(crate) fn native_generator_function(
    constructor: &Function,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let (params, body) = function_source_parts(argument_values, env)?;
    let source = format!("function* anonymous({params}\n) {{\n{body}\n}}");
    let script = parse_dynamic_function_script(&source).map_err(|error| RuntimeError {
        thrown: None,
        message: format!(
            "SyntaxError: invalid GeneratorFunction constructor source: {}",
            error.message
        ),
    })?;

    let Some(Stmt::FunctionDecl {
        name,
        params,
        body,
        is_generator: true,
        is_async: false,
        ..
    }) = script.body.into_iter().next()
    else {
        return Err(RuntimeError {
            thrown: None,
            message: "GeneratorFunction constructor did not produce a generator declaration"
                .to_owned(),
        });
    };

    let is_strict = crate::function::is_strict_function_body(&body);
    let local_names =
        crate::function::collect_function_local_names(Some(&name), &params, &body, true);
    let bytecode = crate::bytecode::compile_generator_function_body(&params, &body, is_strict)?;
    let env_snapshot = dynamic_function_scope_snapshot(env);
    let created = Function::new_user_compiled(CompiledUserFunction {
        name: Some(name),
        has_name_binding: true,
        params,
        env: env_snapshot.clone(),
        bytecode: Rc::new(bytecode),
        local_names,
        constructable: false,
        is_strict,
        lexical_this: false,
        lexical_arguments: false,
        is_generator: true,
        is_async: false,
        is_class_constructor: false,
        is_derived_constructor: false,
        is_field_initializer: false,
        home_object: None,
        super_constructor: None,
        captured_env: Rc::new(RefCell::new(env_snapshot)),
        with_stack: Vec::new(),
        capture_writeback: None,
    });
    crate::generator::wire_generator_function_intrinsics(&created, env);
    created
        .set_internal_prototype_slot(generator_construct_prototype_slot(constructor, env)?)
        .map_err(|_| RuntimeError {
            thrown: None,
            message: "TypeError: dynamic generator function prototype could not be set".to_owned(),
        })?;
    Ok(Value::Function(created))
}

pub(crate) fn native_async_function_constructor(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    parse_dynamic_function_constructor_source(
        "AsyncFunction",
        "async function",
        argument_values,
        env,
    )?;
    Err(RuntimeError {
        thrown: None,
        message: "TypeError: AsyncFunction constructor is not implemented".to_owned(),
    })
}

pub(crate) fn native_async_generator_function_constructor(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    parse_dynamic_function_constructor_source(
        "AsyncGeneratorFunction",
        "async function*",
        argument_values,
        env,
    )?;
    Err(RuntimeError {
        thrown: None,
        message: "TypeError: AsyncGeneratorFunction constructor is not implemented".to_owned(),
    })
}

fn parse_dynamic_function_constructor_source(
    constructor_name: &str,
    function_prefix: &str,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<(), RuntimeError> {
    let (params, body) = function_source_parts(argument_values, env)?;
    let source = format!("{function_prefix} anonymous({params}\n) {{\n{body}\n}}");
    parse_dynamic_function_script(&source)
        .map(|_| ())
        .map_err(|error| RuntimeError {
            thrown: None,
            message: format!(
                "SyntaxError: invalid {constructor_name} constructor source: {}",
                error.message
            ),
        })
}

fn generator_construct_prototype_slot(
    constructor: &Function,
    env: &mut CallEnv,
) -> Result<Option<Prototype>, RuntimeError> {
    if let Some(Value::Function(new_target)) = env.get(crate::NEW_TARGET_BINDING)
        && let Some(Value::Object(realm_prototype)) = new_target
            .own_property(GENERATOR_FUNCTION_REALM_PROTOTYPE)
            .map(|property| property.value)
    {
        let prototype = property_value(Value::Function(new_target), "prototype", env)?;
        if !matches!(
            prototype,
            Value::Object(_) | Value::Function(_) | Value::Array(_) | Value::Proxy(_)
        ) {
            return Ok(Some(Prototype::Object(realm_prototype)));
        }
    }
    crate::native_construct_prototype_slot(constructor, env)
}

fn dynamic_function_scope_snapshot(env: &CallEnv) -> std::collections::HashMap<String, Value> {
    let mut snapshot = env.to_flat_map();
    if let Some(Value::Object(global)) = env.get(DYNAMIC_FUNCTION_REALM_GLOBAL) {
        for name in global.own_property_names() {
            if let Some(property) = global.own_property(&name) {
                snapshot.insert(name, property.value);
            }
        }
    }
    snapshot
}

fn function_source_parts(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<(String, String), RuntimeError> {
    let Some((body, params)) = argument_values.split_last() else {
        return Ok((String::new(), String::new()));
    };

    let params = params
        .iter()
        .cloned()
        .map(|value| to_js_string_with_env(value, env))
        .collect::<Result<Vec<_>, _>>()?
        .join(",");
    let body = to_js_string_with_env(body.clone(), env)?;
    Ok((params, body))
}

pub(crate) fn native_function_prototype_apply(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !matches!(this_value, Value::Function(_) | Value::Proxy(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "Function.prototype.apply target is not callable".to_owned(),
        });
    }

    let call_this = argument_values.first().cloned().unwrap_or(Value::Undefined);
    if let Some(result) = apply_dense_native_fast_path(&this_value, argument_values, env) {
        return result;
    }
    let apply_arguments = apply_argument_list(
        argument_values.get(1).cloned().unwrap_or(Value::Undefined),
        env,
    )?;

    crate::call_function(this_value, call_this, apply_arguments, env, false)
}

/// The `fn.apply(this, denseArray)` fast path for self-contained native
/// targets (currently `String.fromCodePoint`): reads the argument array
/// straight out of dense element storage and computes the result without
/// building a forwarding call environment. Exposed so the VM call site can
/// take it before materializing (and deep-cloning) the caller's frame
/// locals, which is what otherwise makes a repeated
/// `String.fromCodePoint.apply` in a `buildString`-style loop quadratic in
/// the accumulated string size.
pub(crate) fn apply_dense_native_fast_path(
    target: &Value,
    argument_values: &[Value],
    env: &CallEnv,
) -> Option<Result<Value, RuntimeError>> {
    let Value::Function(function) = target else {
        return None;
    };
    if function.native_kind() != Some(NativeFunction::StringFromCodePoint) {
        return None;
    }
    let Some(Value::Array(array)) = argument_values.get(1) else {
        return None;
    };
    array.with_dense_argument_elements(env, |elements| {
        crate::string::string_from_code_point_numbers(elements)
            .map(|result| result.map(Value::String))
    })?
}

fn apply_argument_list(arg_array: Value, env: &mut CallEnv) -> Result<Vec<Value>, RuntimeError> {
    match arg_array {
        Value::Null | Value::Undefined => Ok(Vec::new()),
        Value::Array(array) => {
            // A dense array with the default prototype and no exotic indexed
            // properties reads straight out of element storage, skipping the
            // per-index string-key allocation and prototype walk of the generic
            // property lookup. This is the hot path for `fn.apply(this, bigArray)`.
            if let Some(values) = array.dense_argument_values(env) {
                return Ok(values);
            }
            let receiver = Value::Array(array.clone());
            (0..array.len())
                .map(|index| property_value(receiver.clone(), &index.to_string(), env))
                .collect()
        }
        Value::Object(object) if object.to_string_tag().as_deref() == Some("Symbol") => {
            Err(apply_argument_list_type_error())
        }
        Value::Object(_) | Value::Proxy(_) => {
            array_like_values_with_env(arg_array, "Function.prototype.apply argument list", env)
        }
        Value::Function(_) => {
            let length =
                to_length_with_env(property_value(arg_array.clone(), "length", env)?, env)?;
            (0..length)
                .map(|index| property_value(arg_array.clone(), &index.to_string(), env))
                .collect()
        }
        Value::String(_)
        | Value::Number(_)
        | Value::BigInt(_)
        | Value::Boolean(_)
        | Value::Map(_)
        | Value::Set(_) => Err(apply_argument_list_type_error()),
    }
}

fn apply_argument_list_type_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "TypeError: Function.prototype.apply argument list must be an object".to_owned(),
    }
}

pub(crate) fn native_function_prototype_call(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !matches!(this_value, Value::Function(_) | Value::Proxy(_)) {
        return Err(RuntimeError {
            thrown: None,
            message: "Function.prototype.call target is not callable".to_owned(),
        });
    }

    let call_this = argument_values.first().cloned().unwrap_or(Value::Undefined);
    crate::call_function(
        this_value,
        call_this,
        argument_values.iter().skip(1).cloned().collect(),
        env,
        false,
    )
}

pub(crate) fn native_function_prototype_bind(
    this_value: Value,
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let Value::Function(target) = this_value.clone() else {
        return Err(RuntimeError {
            thrown: None,
            message: "Function.prototype.bind target is not callable".to_owned(),
        });
    };

    let bound_this = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let bound_arguments = argument_values.iter().skip(1).cloned().collect::<Vec<_>>();
    let length = target.params.length().saturating_sub(bound_arguments.len());
    let bound = Function::new_bound(this_value, bound_this, bound_arguments, length);
    Ok(Value::Function(bound))
}

pub(crate) fn native_function_prototype_has_instance(
    this_value: Value,
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    if !matches!(this_value, Value::Function(_)) {
        return Ok(Value::Boolean(false));
    }
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    crate::operations::ordinary_has_instance(value, this_value, env).map(Value::Boolean)
}

pub(crate) fn native_function_prototype_to_string(
    this_value: Value,
) -> Result<Value, RuntimeError> {
    let name = match &this_value {
        Value::Function(function) => function.name.clone().unwrap_or_default(),
        // A callable Proxy is an acceptable receiver: unwrap to the underlying
        // function for the name and emit a NativeFunction-shaped string.
        Value::Proxy(proxy) if crate::proxy::proxy_is_callable(proxy) => {
            callable_proxy_target_name(proxy)
        }
        _ => {
            return Err(RuntimeError {
                thrown: None,
                message: "TypeError: Function.prototype.toString requires a callable receiver"
                    .to_owned(),
            });
        }
    };
    Ok(Value::String(format!(
        "function {name}() {{ [native code] }}"
    )))
}

/// Resolves the name of the function wrapped (possibly through nested proxies)
/// by a callable Proxy, defaulting to the empty string.
fn callable_proxy_target_name(proxy: &crate::proxy::ProxyRef) -> String {
    let mut target = proxy.target();
    loop {
        match target {
            Value::Function(function) => return function.name.clone().unwrap_or_default(),
            Value::Proxy(inner) => target = inner.target(),
            _ => return String::new(),
        }
    }
}

pub(crate) fn native_throw_type_error() -> Result<Value, RuntimeError> {
    Err(RuntimeError {
        thrown: None,
        message: "TypeError: restricted function property access".to_owned(),
    })
}

pub(crate) fn function_call_this(this_arg: Option<Value>, env: &CallEnv, is_strict: bool) -> Value {
    let this_value = this_arg.unwrap_or(Value::Undefined);
    match this_value {
        Value::Null | Value::Undefined if !is_strict => {
            env.get(GLOBAL_THIS_BINDING).unwrap_or(Value::Undefined)
        }
        Value::String(_) | Value::Number(_) | Value::Boolean(_) | Value::BigInt(_)
            if !is_strict =>
        {
            boxed_primitive(this_value, env).expect("primitive value should box")
        }
        Value::Object(ref object) if !is_strict && crate::symbol::is_symbol_primitive(object) => {
            boxed_primitive(this_value, env).expect("primitive value should box")
        }
        value => value,
    }
}
