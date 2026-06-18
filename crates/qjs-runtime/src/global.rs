use qjs_parser::{EvalParseContext, parse_direct_eval_script, parse_script};
use std::collections::HashSet;

use crate::CallEnv;
use crate::{
    Function, GLOBAL_THIS_BINDING, NativeFunction, ObjectRef, Property, RuntimeError, Value,
    bytecode::{compile_direct_eval_script, eval_bytecode_with_env},
    string::{string_code_units, string_from_code_unit},
    to_js_string_with_env, to_number_with_env,
};

pub(super) fn install_globals(env: &mut CallEnv, global_this: &Value) {
    env.insert_realm("NaN".to_owned(), Value::Number(f64::NAN));
    env.insert_realm("Infinity".to_owned(), Value::Number(f64::INFINITY));
    env.insert_realm("undefined".to_owned(), Value::Undefined);
    env.insert_realm("globalThis".to_owned(), global_this.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_property(
            "NaN".to_owned(),
            Property::data(Value::Number(f64::NAN), false, false, false),
        );
        global_object.define_property(
            "Infinity".to_owned(),
            Property::data(Value::Number(f64::INFINITY), false, false, false),
        );
        global_object.define_property(
            "undefined".to_owned(),
            Property::data(Value::Undefined, false, false, false),
        );
        global_object.define_property(
            "globalThis".to_owned(),
            Property::data(global_this.clone(), false, true, true),
        );
        global_object.define_property(
            "NaN".to_owned(),
            Property::data(Value::Number(f64::NAN), false, false, false),
        );
        global_object.define_property(
            "Infinity".to_owned(),
            Property::data(Value::Number(f64::INFINITY), false, false, false),
        );
        global_object.define_property(
            "undefined".to_owned(),
            Property::data(Value::Undefined, false, false, false),
        );
    }

    define_global_function(
        env,
        global_this,
        "isFinite",
        1,
        NativeFunction::GlobalIsFinite,
    );
    define_global_function(env, global_this, "isNaN", 1, NativeFunction::GlobalIsNaN);
    define_global_function(env, global_this, "decodeURI", 1, NativeFunction::DecodeUri);
    define_global_function(
        env,
        global_this,
        "decodeURIComponent",
        1,
        NativeFunction::DecodeUriComponent,
    );
    define_global_function(env, global_this, "encodeURI", 1, NativeFunction::EncodeUri);
    define_global_function(
        env,
        global_this,
        "encodeURIComponent",
        1,
        NativeFunction::EncodeUriComponent,
    );
    define_global_function(env, global_this, "eval", 1, NativeFunction::Eval);
    define_global_function(env, global_this, "print", 1, NativeFunction::Print);
    define_global_function(
        env,
        global_this,
        "__quickjsRustAssertSameValue",
        3,
        NativeFunction::Test262AssertSameValue,
    );
    define_global_function(env, global_this, "escape", 1, NativeFunction::Escape);
    define_global_function(env, global_this, "unescape", 1, NativeFunction::Unescape);
    define_is_html_dda(env, global_this);
    define_global_function(
        env,
        global_this,
        "__quickjsRustDetachArrayBuffer",
        1,
        NativeFunction::DetachArrayBuffer,
    );
    define_global_function(
        env,
        global_this,
        "__quickjsRustEvalScript",
        1,
        NativeFunction::EvalScript,
    );
}

fn define_global_function(
    env: &mut CallEnv,
    global_this: &Value,
    key: &str,
    length: usize,
    native: NativeFunction,
) {
    let value = Value::Function(Function::new_native(Some(key), length, native, false));
    env.insert_realm(key.to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable(key.to_owned(), value);
    }
}

fn define_is_html_dda(env: &mut CallEnv, global_this: &Value) {
    let key = "__quickjsRustIsHTMLDDA";
    let value = Value::Function(crate::html_dda::new_is_html_dda_function());
    env.insert_realm(key.to_owned(), value.clone());
    if let Value::Object(global_object) = global_this {
        global_object.define_non_enumerable(key.to_owned(), value);
    }
}

/// Host `print`: stringifies each argument, joins them with spaces, writes the
/// line to stdout, and returns `undefined`. This is a host shim (QuickJS-NG's
/// `qjs` exposes the same global) used, among other things, by the Test262
/// async `$DONE` channel; the runtime stays unaware of Test262 conventions.
pub(super) fn native_global_print(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let mut line = String::new();
    for (index, value) in argument_values.iter().enumerate() {
        if index > 0 {
            line.push(' ');
        }
        line.push_str(&to_js_string_with_env(value.clone(), env)?);
    }
    println!("{line}");
    Ok(Value::Undefined)
}

pub(crate) fn native_test262_assert_same_value(
    argument_values: &[Value],
) -> Result<Value, RuntimeError> {
    let actual = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let expected = argument_values.get(1).cloned().unwrap_or(Value::Undefined);
    if actual.same_value(&expected) {
        return Ok(Value::Undefined);
    }
    let message = match argument_values.get(2) {
        Some(Value::String(message)) if !message.is_empty() => {
            format!("{message} Expected SameValue to be true")
        }
        _ => "Expected SameValue to be true".to_owned(),
    };
    Err(RuntimeError {
        thrown: None,
        message,
    })
}

pub(super) fn native_global_is_finite(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(to_number_with_env(value, env)?.is_finite()))
}

pub(super) fn native_global_is_nan(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    Ok(Value::Boolean(to_number_with_env(value, env)?.is_nan()))
}

pub(super) fn native_global_encode_uri(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    encode_uri(&source, UriEncodeKind::Uri).map(Value::String)
}

pub(super) fn native_global_encode_uri_component(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    encode_uri(&source, UriEncodeKind::Component).map(Value::String)
}

pub(super) fn native_global_decode_uri(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    decode_uri_string(&source).map(Value::String)
}

pub(super) fn native_global_decode_uri_component(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    decode_uri_component_string(&source).map(Value::String)
}

pub(crate) fn decode_uri_string(source: &str) -> Result<String, RuntimeError> {
    decode_uri(source, UriDecodeKind::Uri)
}

pub(crate) fn decode_uri_component_string(source: &str) -> Result<String, RuntimeError> {
    decode_uri(source, UriDecodeKind::Component)
}

pub(super) fn native_global_eval(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let Value::String(source) = value else {
        return Ok(value);
    };
    let direct_eval = matches!(
        env.get(crate::DIRECT_EVAL_BINDING),
        Some(Value::Boolean(true))
    );
    if let Some(value) = try_eval_regexp_literal_source(&source, env)? {
        return Ok(value);
    }
    let script = if direct_eval {
        parse_direct_eval_script(&source, direct_eval_parse_context(env))
    } else {
        parse_script(&source)
    }
    .map_err(|error| RuntimeError {
        thrown: None,
        message: format!("SyntaxError: {}", error.message),
    })?;
    let mut eval_env = if direct_eval {
        env.clone()
    } else {
        CallEnv::new(env.realm_rc())
    };
    let direct_function_eval = direct_eval && eval_env.get_local("this").is_some();
    // Direct eval inside strict code is itself strict even without its own
    // "use strict" prologue; seed the compiler so Annex B block-function
    // hoisting is correctly suppressed. Indirect eval is sloppy unless its own
    // body opts in.
    let caller_strict = direct_eval
        && matches!(
            env.get(crate::DIRECT_EVAL_STRICT_BINDING),
            Some(Value::Boolean(true))
        );
    let bytecode = compile_direct_eval_script(&script, caller_strict)?;
    let eval_strict = bytecode.is_strict();
    if direct_function_eval
        && matches!(
            eval_env.get(crate::DIRECT_EVAL_ARGUMENTS_BINDING),
            Some(Value::Boolean(true))
        )
        && bytecode
            .hoisted_local_names()
            .any(|name| name == "arguments")
    {
        // EvalDeclarationInstantiation: a direct eval may not hoist a `var` or
        // `function` declaration named `arguments` when the surrounding
        // function environment already binds `arguments` -- i.e. inside a
        // non-arrow function (which always has the arguments object) or inside
        // an arrow whose parameter list is named `arguments`. An arrow with no
        // such binding (or one that only binds `arguments` in its body) may
        // declare it freely.
        return Err(RuntimeError {
            thrown: None,
            message: "SyntaxError: cannot declare 'arguments' in function eval".to_owned(),
        });
    }
    if !direct_function_eval {
        validate_eval_global_lexical_bindings(&bytecode, &eval_env)?;
    }
    let caller_locals = eval_env.locals().keys().cloned().collect::<HashSet<_>>();
    let hoisted_names = bytecode
        .hoisted_local_names()
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    let hoisted_function_names = bytecode
        .hoisted_function_names()
        .map(str::to_owned)
        .collect::<HashSet<_>>();
    if !direct_function_eval && !eval_strict {
        validate_sloppy_global_eval_declarations(
            &bytecode,
            &eval_env,
            &caller_locals,
            &hoisted_function_names,
        )?;
    }
    let mut strict_direct_writeback_env = (direct_eval && eval_strict).then(|| env.clone());
    initialize_direct_eval_bindings(
        &bytecode,
        &mut eval_env,
        direct_function_eval,
        &caller_locals,
        eval_strict,
    );
    let result = eval_bytecode_with_env(&bytecode, eval_env.clone());
    for name in bytecode
        .hoisted_local_names()
        .chain(bytecode.global_names().iter().map(String::as_str))
    {
        if let Some(value) = result.binding(name) {
            if let Some(writeback_env) = strict_direct_writeback_env.as_mut() {
                if hoisted_names.contains(name) {
                    continue;
                }
                if caller_locals.contains(name) {
                    // Strict direct eval runs declarations in its own eval
                    // variable environment, but ordinary assignments to
                    // caller-scope bindings still write through to the caller.
                    writeback_env.insert(name.to_owned(), value.clone());
                }
                continue;
            }
            if caller_locals.contains(name) {
                // A caller frame binding (an outer `let`/`var` the eval'd code
                // assigned): write it back through the frame so the caller's
                // slot sees the update.
                eval_env.insert(name.to_owned(), value.clone());
            } else if direct_function_eval {
                eval_env.insert(name.to_owned(), value.clone());
            } else if hoisted_function_names.contains(name) {
                create_eval_global_function_binding(&mut eval_env, name, value.clone());
            } else if hoisted_names.contains(name) {
                create_eval_global_var_binding(&mut eval_env, name, value.clone());
            } else {
                define_eval_global_binding(&mut eval_env, name, value.clone());
            }
        }
    }
    // Indirect eval evaluates its lexical declarations (let/const/class) in a
    // fresh declarative environment whose parent is the global environment;
    // those bindings are discarded when the eval completes and never become
    // global lexical bindings. Only var/function declarations (handled above
    // via define_eval_global_binding) reach the global var environment.
    if direct_eval {
        *env = strict_direct_writeback_env.unwrap_or(eval_env);
    }
    result.value
}

/// Host `$262.evalScript`: evaluates `source` as a global script in the current
/// realm. Unlike indirect `eval`, a script's top-level lexical declarations
/// (`let`/`const`/`class`) become persistent global lexical bindings, so a
/// later declaration of the same name observes them (and var/function
/// declarations reach the global var environment). Used by the Test262 harness.
pub(super) fn native_eval_script(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let Value::String(source) = value else {
        return Ok(value);
    };
    let script = parse_script(&source).map_err(|error| RuntimeError {
        thrown: None,
        message: format!("SyntaxError: {}", error.message),
    })?;
    let bytecode = compile_direct_eval_script(&script, false)?;
    let mut eval_env = CallEnv::new(env.realm_rc());
    validate_eval_global_lexical_bindings(&bytecode, &eval_env)?;
    initialize_direct_eval_bindings(&bytecode, &mut eval_env, false, &HashSet::new(), false);
    let result = eval_bytecode_with_env(&bytecode, eval_env.clone());
    for name in bytecode
        .hoisted_local_names()
        .chain(bytecode.global_names().iter().map(String::as_str))
    {
        if let Some(value) = result.binding(name) {
            define_eval_global_binding(&mut eval_env, name, value.clone());
        }
    }
    // Top-level lexical declarations persist as global lexical bindings.
    let hoisted = bytecode.hoisted_local_names().collect::<HashSet<_>>();
    for name in bytecode.local_names() {
        if hoisted.contains(name) {
            continue;
        }
        if let Some(value) = result.binding(name) {
            eval_env.insert_realm(name.to_owned(), value.clone());
        }
    }
    result.value
}

pub(crate) fn try_eval_regexp_literal_source(
    source: &str,
    env: &CallEnv,
) -> Result<Option<Value>, RuntimeError> {
    let source = source.trim();
    if !source.starts_with('/') || source.starts_with("//") || source.starts_with("/*") {
        return Ok(None);
    }

    let mut in_class = false;
    let mut escaped = false;
    let mut close = None;
    for (index, ch) in source.char_indices().skip(1) {
        if escaped {
            if is_line_terminator(ch) {
                return Ok(None);
            }
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '[' if !in_class => in_class = true,
            ']' if in_class => in_class = false,
            '/' if !in_class => {
                close = Some(index);
                break;
            }
            ch if is_line_terminator(ch) => return Ok(None),
            _ => {}
        }
    }
    let Some(close) = close else {
        return Ok(None);
    };

    let mut flags_end = source.len();
    let mut semicolon = None;
    for (index, ch) in source[close + 1..].char_indices() {
        let absolute = close + 1 + index;
        if ch == ';' {
            flags_end = absolute;
            semicolon = Some(absolute);
            break;
        }
        if ch.is_whitespace() {
            flags_end = absolute;
            break;
        }
        if !ch.is_alphabetic() {
            return Ok(None);
        }
    }

    let rest_start = semicolon.map_or(flags_end, |index| index + 1);
    if !source[rest_start..].trim().is_empty() {
        return Ok(None);
    }

    let pattern = &source[1..close];
    let flags = &source[close + 1..flags_end];
    crate::regexp::regexp_literal_value(pattern, flags, env).map(Some)
}

fn is_line_terminator(ch: char) -> bool {
    matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

fn direct_eval_parse_context(env: &CallEnv) -> EvalParseContext {
    EvalParseContext {
        strict: matches!(
            env.get(crate::DIRECT_EVAL_STRICT_BINDING),
            Some(Value::Boolean(true))
        ),
        in_function: env.get_local("this").is_some(),
        in_method: env.get(crate::HOME_OBJECT_BINDING).is_some(),
        in_derived_constructor: env.get(crate::SUPER_CONSTRUCTOR_BINDING).is_some(),
        in_field_initializer: matches!(
            env.get(crate::FIELD_INITIALIZER_EVAL_BINDING),
            Some(Value::Boolean(true))
        ),
        private_names: env
            .private_environment()
            .map_or_else(Vec::new, |environment| environment.visible_names()),
    }
}

fn validate_eval_global_lexical_bindings(
    bytecode: &crate::bytecode::Bytecode,
    env: &CallEnv,
) -> Result<(), RuntimeError> {
    let global_this = env.get(GLOBAL_THIS_BINDING).and_then(|value| match value {
        Value::Object(object) => Some(object),
        _ => None,
    });
    if let Some(global_this) = &global_this {
        for name in bytecode.global_lexical_names() {
            if global_this
                .own_property(name)
                .is_some_and(|property| !property.configurable)
            {
                return Err(RuntimeError {
                    thrown: None,
                    message: format!(
                        "SyntaxError: global lexical declaration `{name}` conflicts with an existing var binding"
                    ),
                });
            }
        }
    }
    Ok(())
}

fn validate_sloppy_global_eval_declarations(
    bytecode: &crate::bytecode::Bytecode,
    env: &CallEnv,
    caller_locals: &HashSet<String>,
    function_names: &HashSet<String>,
) -> Result<(), RuntimeError> {
    let Some(global_this) = env.get(GLOBAL_THIS_BINDING).and_then(|value| match value {
        Value::Object(object) => Some(object),
        _ => None,
    }) else {
        return Ok(());
    };
    for name in bytecode.hoisted_local_names() {
        if (caller_locals.contains(name) || env.realm_contains(name))
            && !global_this.has_own_property(name)
        {
            return Err(RuntimeError {
                thrown: None,
                message: format!(
                    "SyntaxError: global var declaration `{name}` conflicts with a global lexical binding"
                ),
            });
        }
    }
    for name in function_names {
        if !can_declare_global_function(&global_this, name) {
            return Err(RuntimeError {
                thrown: None,
                message: format!("TypeError: cannot declare global function `{name}`"),
            });
        }
    }
    for name in bytecode.hoisted_local_names() {
        if function_names.contains(name) {
            continue;
        }
        if !can_declare_global_var(&global_this, name) {
            return Err(RuntimeError {
                thrown: None,
                message: format!("TypeError: cannot declare global var `{name}`"),
            });
        }
    }
    Ok(())
}

fn can_declare_global_var(global_this: &ObjectRef, name: &str) -> bool {
    global_this.has_own_property(name) || global_this.is_extensible()
}

fn can_declare_global_function(global_this: &ObjectRef, name: &str) -> bool {
    let Some(existing) = global_this.own_property(name) else {
        return global_this.is_extensible();
    };
    existing.configurable || (!existing.accessor && existing.writable && existing.enumerable)
}

fn initialize_direct_eval_bindings(
    bytecode: &crate::bytecode::Bytecode,
    env: &mut CallEnv,
    direct_function_eval: bool,
    caller_locals: &HashSet<String>,
    eval_strict: bool,
) {
    if !env.locals().contains_key("this")
        && let Some(value) = env.get("this")
    {
        env.insert("this".to_owned(), value);
    }
    for name in bytecode.hoisted_local_names() {
        if !eval_strict && caller_locals.contains(name) {
            continue;
        }
        if eval_strict {
            env.insert(name.to_owned(), Value::Undefined);
            continue;
        }
        if direct_function_eval {
            if !env.locals().contains_key(name) {
                env.insert(name.to_owned(), Value::Undefined);
            }
            continue;
        }
        let global_this = env.get(GLOBAL_THIS_BINDING).and_then(|value| match value {
            Value::Object(object) => Some(object),
            _ => None,
        });
        if let Some(property) = global_this
            .as_ref()
            .and_then(|object| object.own_property(name))
        {
            env.insert(name.to_owned(), property.value.clone());
            env.insert_realm(name.to_owned(), property.value);
        } else {
            env.insert(name.to_owned(), Value::Undefined);
            define_eval_global_binding(env, name, Value::Undefined);
        }
    }
}

fn create_eval_global_var_binding(env: &mut CallEnv, name: &str, value: Value) {
    let global_this = env.get(GLOBAL_THIS_BINDING).and_then(|value| match value {
        Value::Object(object) => Some(object),
        _ => None,
    });
    if let Some(global_this) = global_this {
        if global_this.has_own_property(name) {
            global_this.set(name.to_owned(), value.clone());
            let value = global_this
                .own_property(name)
                .map(|property| property.value)
                .unwrap_or(value);
            env.insert_realm(name.to_owned(), value);
            return;
        }
        global_this.define_property(
            name.to_owned(),
            Property::data(value.clone(), true, true, true),
        );
    }
    env.insert_realm(name.to_owned(), value);
}

fn create_eval_global_function_binding(env: &mut CallEnv, name: &str, value: Value) {
    let global_this = env.get(GLOBAL_THIS_BINDING).and_then(|value| match value {
        Value::Object(object) => Some(object),
        _ => None,
    });
    if let Some(global_this) = global_this {
        let property = match global_this.own_property(name) {
            Some(existing) if !existing.configurable => {
                let mut property = existing;
                property.value = value.clone();
                property
            }
            _ => Property::data(value.clone(), true, true, true),
        };
        global_this.define_property(name.to_owned(), property);
    }
    env.insert_realm(name.to_owned(), value);
}

fn define_eval_global_binding(env: &mut CallEnv, name: &str, value: Value) {
    let global_this = env.get(GLOBAL_THIS_BINDING).and_then(|value| match value {
        Value::Object(object) => Some(object),
        _ => None,
    });
    if let Some(global_this) = global_this {
        if global_this.has_own_property(name) {
            global_this.set(name.to_owned(), value.clone());
        } else {
            global_this.define_property(
                name.to_owned(),
                Property::data(value.clone(), true, true, true),
            );
        }
    }
    env.insert_realm(name.to_owned(), value);
}

pub(super) fn native_global_escape(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    let mut escaped = String::new();
    for code_unit in string_code_units(&source) {
        if is_escape_unescaped(code_unit) {
            escaped.push_str(&string_from_code_unit(code_unit));
        } else if code_unit <= 0xFF {
            escaped.push_str(&format!("%{code_unit:02X}"));
        } else {
            escaped.push_str(&format!("%u{code_unit:04X}"));
        }
    }
    Ok(Value::String(escaped))
}

fn is_escape_unescaped(code_unit: u16) -> bool {
    matches!(code_unit, 0x41..=0x5A | 0x61..=0x7A | 0x30..=0x39)
        || matches!(code_unit, 0x40 | 0x2A | 0x5F | 0x2B | 0x2D | 0x2E | 0x2F)
}

pub(super) fn native_global_unescape(
    argument_values: &[Value],
    env: &mut CallEnv,
) -> Result<Value, RuntimeError> {
    let value = argument_values.first().cloned().unwrap_or(Value::Undefined);
    let source = to_js_string_with_env(value, env)?;
    let mut output = String::new();
    let code_units = string_code_units(&source);
    let mut index = 0;
    while index < code_units.len() {
        if code_units[index] == b'%' as u16 {
            if let Some(code_unit) = parse_hex_escape(&code_units, index) {
                output.push_str(&string_from_code_unit(code_unit));
                index += if code_units.get(index + 1) == Some(&(b'u' as u16)) {
                    6
                } else {
                    3
                };
                continue;
            }
        }
        output.push_str(&string_from_code_unit(code_units[index]));
        index += 1;
    }
    Ok(Value::String(output))
}

fn parse_hex_escape(code_units: &[u16], index: usize) -> Option<u16> {
    if code_units.get(index + 1) == Some(&(b'u' as u16)) {
        return parse_hex_digits(code_units.get(index + 2..index + 6)?);
    }
    parse_hex_digits(code_units.get(index + 1..index + 3)?)
}

fn parse_hex_digits(digits: &[u16]) -> Option<u16> {
    let mut value = 0u16;
    for digit in digits {
        value = value.checked_mul(16)? + u16::try_from(hex_digit(*digit)?).ok()?;
    }
    Some(value)
}

fn hex_digit(code_unit: u16) -> Option<u32> {
    match code_unit {
        0x30..=0x39 => Some(u32::from(code_unit - 0x30)),
        0x61..=0x66 => Some(u32::from(code_unit - 0x61 + 10)),
        0x41..=0x46 => Some(u32::from(code_unit - 0x41 + 10)),
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum UriEncodeKind {
    Uri,
    Component,
}

#[derive(Clone, Copy)]
enum UriDecodeKind {
    Uri,
    Component,
}

fn encode_uri(source: &str, kind: UriEncodeKind) -> Result<String, RuntimeError> {
    let code_units = string_code_units(source);
    let mut output = String::new();
    let mut index = 0;
    while index < code_units.len() {
        let code_unit = code_units[index];
        let code_point = if is_high_surrogate(code_unit) {
            let Some(&low) = code_units.get(index + 1) else {
                return malformed_uri();
            };
            if !is_low_surrogate(low) {
                return malformed_uri();
            }
            index += 1;
            0x10000 + ((u32::from(code_unit) - 0xD800) << 10) + u32::from(low) - 0xDC00
        } else if is_low_surrogate(code_unit) {
            return malformed_uri();
        } else {
            u32::from(code_unit)
        };

        let character = char::from_u32(code_point).ok_or_else(uri_error)?;
        if is_uri_unescaped(character, kind) {
            output.push(character);
        } else {
            let mut buffer = [0; 4];
            for byte in character.encode_utf8(&mut buffer).as_bytes() {
                output.push('%');
                output.push(hex_upper(byte >> 4));
                output.push(hex_upper(byte & 0x0F));
            }
        }
        index += 1;
    }
    Ok(output)
}

fn decode_uri(source: &str, kind: UriDecodeKind) -> Result<String, RuntimeError> {
    if !source.contains('%') {
        return Ok(source.to_owned());
    }
    if source.is_ascii() {
        return decode_ascii_uri(source, kind);
    }

    let mut output = String::new();
    let chars: Vec<char> = source.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        if chars[index] != '%' {
            output.push(chars[index]);
            index += 1;
            continue;
        }

        let escape_start = index;
        let first_byte = percent_byte(&chars, index)?;
        index += 3;

        let expected_len = utf8_sequence_len(first_byte)?;
        let mut bytes = vec![first_byte];
        for _ in 1..expected_len {
            if index >= chars.len() || chars[index] != '%' {
                return malformed_uri();
            }
            bytes.push(percent_byte(&chars, index)?);
            index += 3;
        }

        let decoded = std::str::from_utf8(&bytes).map_err(|_| uri_error())?;
        if matches!(kind, UriDecodeKind::Uri) && decoded.chars().all(is_uri_reserved) {
            output.extend(chars[escape_start..index].iter());
        } else {
            output.push_str(decoded);
        }
    }
    Ok(output)
}

fn decode_ascii_uri(source: &str, kind: UriDecodeKind) -> Result<String, RuntimeError> {
    let bytes = source.as_bytes();
    let mut output = String::with_capacity(source.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            output.push(char::from(bytes[index]));
            index += 1;
            continue;
        }

        let escape_start = index;
        let first_byte = ascii_percent_byte(bytes, index)?;
        index += 3;

        let expected_len = utf8_sequence_len(first_byte)?;
        let mut decoded_bytes = [0u8; 4];
        decoded_bytes[0] = first_byte;
        for slot in decoded_bytes.iter_mut().take(expected_len).skip(1) {
            if index >= bytes.len() || bytes[index] != b'%' {
                return malformed_uri();
            }
            *slot = ascii_percent_byte(bytes, index)?;
            index += 3;
        }

        let decoded =
            std::str::from_utf8(&decoded_bytes[..expected_len]).map_err(|_| uri_error())?;
        if matches!(kind, UriDecodeKind::Uri) && decoded.chars().all(is_uri_reserved) {
            output.push_str(&source[escape_start..index]);
        } else {
            output.push_str(decoded);
        }
    }
    Ok(output)
}

fn ascii_percent_byte(bytes: &[u8], index: usize) -> Result<u8, RuntimeError> {
    let Some(high) = bytes.get(index + 1).and_then(|byte| ascii_hex_digit(*byte)) else {
        return malformed_uri();
    };
    let Some(low) = bytes.get(index + 2).and_then(|byte| ascii_hex_digit(*byte)) else {
        return malformed_uri();
    };
    Ok((high << 4) | low)
}

fn ascii_hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

fn is_uri_unescaped(character: char, kind: UriEncodeKind) -> bool {
    character.is_ascii_alphanumeric()
        || matches!(
            character,
            '-' | '_' | '.' | '!' | '~' | '*' | '\'' | '(' | ')'
        )
        || (matches!(kind, UriEncodeKind::Uri) && is_uri_reserved(character))
}

fn is_uri_reserved(character: char) -> bool {
    matches!(
        character,
        ';' | '/' | '?' | ':' | '@' | '&' | '=' | '+' | '$' | ',' | '#'
    )
}

fn is_high_surrogate(code_unit: u16) -> bool {
    (0xD800..=0xDBFF).contains(&code_unit)
}

fn is_low_surrogate(code_unit: u16) -> bool {
    (0xDC00..=0xDFFF).contains(&code_unit)
}

fn percent_byte(chars: &[char], index: usize) -> Result<u8, RuntimeError> {
    let Some(high) = chars.get(index + 1).and_then(|ch| ch.to_digit(16)) else {
        return malformed_uri();
    };
    let Some(low) = chars.get(index + 2).and_then(|ch| ch.to_digit(16)) else {
        return malformed_uri();
    };
    Ok(((high << 4) | low) as u8)
}

fn utf8_sequence_len(first_byte: u8) -> Result<usize, RuntimeError> {
    match first_byte {
        0x00..=0x7F => Ok(1),
        0xC2..=0xDF => Ok(2),
        0xE0..=0xEF => Ok(3),
        0xF0..=0xF4 => Ok(4),
        _ => malformed_uri(),
    }
}

fn hex_upper(nibble: u8) -> char {
    match nibble {
        0..=9 => char::from(b'0' + nibble),
        10..=15 => char::from(b'A' + nibble - 10),
        _ => unreachable!("nibble must be in 0..16"),
    }
}

fn malformed_uri<T>() -> Result<T, RuntimeError> {
    Err(uri_error())
}

fn uri_error() -> RuntimeError {
    RuntimeError {
        thrown: None,
        message: "URIError: malformed URI sequence".to_owned(),
    }
}
