//! Command-line entry point for the rewrite.

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

use qjs_runtime::{
    EvalError, EvalErrorKind, ModuleResolveError, ModuleResolver, ResolvedModule, Value, eval,
    eval_classified, eval_module_with_prelude,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {}", error.message);
            ExitCode::FAILURE
        }
    }
}

struct CliError {
    message: String,
}

fn run() -> Result<(), CliError> {
    let mut args = env::args().skip(1).collect::<Vec<_>>().into_iter();
    let mut raw_output = false;
    let mut test262_error_format = false;
    let mut module_mode = false;
    let mut prelude_path: Option<String> = None;
    loop {
        match args.as_slice().first().map(String::as_str) {
            Some("--raw") => {
                raw_output = true;
                args.next();
            }
            Some("--error-format=test262") => {
                test262_error_format = true;
                args.next();
            }
            Some("--module") => {
                module_mode = true;
                args.next();
            }
            Some("--prelude") => {
                args.next();
                prelude_path = Some(args.next().ok_or_else(|| CliError {
                    message: "missing path after --prelude".to_owned(),
                })?);
            }
            _ => break,
        }
    }

    let Some(first) = args.next() else {
        return Err(CliError {
            message:
                "usage: qjs [--raw] [--error-format=test262] [--module [--prelude <file>]] (-e <source> | <file> [script-arg...])"
                    .to_owned(),
        });
    };

    if module_mode {
        return run_module(&first, prelude_path.as_deref(), test262_error_format);
    }

    let (source, script_args) = if first == "-e" {
        let source = args.next().ok_or_else(|| CliError {
            message: "missing source after -e".to_owned(),
        })?;
        (source, vec!["-e".to_owned()])
    } else {
        let source = fs::read_to_string(&first).map_err(|error| CliError {
            message: format!("failed to read `{first}`: {error}"),
        })?;
        let script_args = std::iter::once(first).chain(args).collect();
        (source, script_args)
    };

    let source = with_script_args(&source, &script_args);
    let value = if test262_error_format {
        eval_classified(&source).map_err(format_test262_error)?
    } else {
        eval(&source).map_err(|error| CliError {
            message: error.message,
        })?
    };
    if raw_output {
        print_raw(&value);
    } else {
        println!("{value:?}");
    }
    Ok(())
}

/// Evaluates `file` under the Module goal. Relative specifiers resolve against
/// the importing file's directory (canonicalized keys). An optional `prelude`
/// file is evaluated as a script in the module graph's realm first, so Test262
/// harness includes (which are script code) are visible to the module.
fn run_module(
    file: &str,
    prelude_path: Option<&str>,
    test262_error_format: bool,
) -> Result<(), CliError> {
    let source = fs::read_to_string(file).map_err(|error| CliError {
        message: format!("failed to read `{file}`: {error}"),
    })?;
    let prelude = match prelude_path {
        Some(path) => Some(fs::read_to_string(path).map_err(|error| CliError {
            message: format!("failed to read prelude `{path}`: {error}"),
        })?),
        None => None,
    };
    // Canonicalize the root specifier so relative imports resolve against a
    // stable absolute directory and the graph deduplicates by real path.
    let root_key = fs::canonicalize(file)
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|_| file.to_owned());
    let mut resolver = FsResolver;
    let result = eval_module_with_prelude(prelude.as_deref(), &source, &root_key, &mut resolver);
    match result {
        Ok(_) => Ok(()),
        Err(error) if test262_error_format => Err(format_test262_error(error)),
        Err(error) => Err(CliError {
            message: error.message,
        }),
    }
}

/// A filesystem [`ModuleResolver`]: resolves a (relative) specifier against the
/// importing module's directory and canonicalizes the result so the graph keys
/// modules by their real path. Lives in the CLI because the engine stays
/// agnostic of any host file layout.
struct FsResolver;

impl ModuleResolver for FsResolver {
    fn resolve(
        &mut self,
        specifier: &str,
        referrer: &str,
    ) -> Result<ResolvedModule, ModuleResolveError> {
        let base_dir = Path::new(referrer)
            .parent()
            .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
        let candidate = base_dir.join(specifier);
        let canonical = fs::canonicalize(&candidate).map_err(|error| ModuleResolveError {
            message: format!("Cannot resolve module '{specifier}': {error}"),
        })?;
        let key = canonical.to_string_lossy().into_owned();
        let source = fs::read_to_string(&canonical).map_err(|error| ModuleResolveError {
            message: format!("Cannot load module '{specifier}': {error}"),
        })?;
        Ok(ResolvedModule { key, source })
    }
}

fn format_test262_error(error: EvalError) -> CliError {
    let error_type = error_type(error.kind, &error.message);
    CliError {
        message: format!(
            "kind={} type={} message={}",
            error.kind.as_str(),
            error_type,
            error.message
        ),
    }
}

fn error_type(kind: EvalErrorKind, message: &str) -> &'static str {
    if matches!(kind, EvalErrorKind::Parse | EvalErrorKind::Early) {
        return "SyntaxError";
    }
    for name in [
        "AggregateError",
        "RangeError",
        "ReferenceError",
        "SyntaxError",
        "Test262Error",
        "TypeError",
        "URIError",
        "EvalError",
        "Error",
    ] {
        let typed_prefix = format!("{name}:");
        if message.starts_with(name)
            || message.contains(&typed_prefix)
            || (name != "Error" && message.contains(name))
        {
            return name;
        }
    }
    "Error"
}

fn with_script_args(source: &str, script_args: &[String]) -> String {
    let args = script_args
        .iter()
        .map(|arg| format!("\"{}\"", escape_js_string(arg)))
        .collect::<Vec<_>>()
        .join(", ");
    let declaration = format!("var scriptArgs = [{args}];\n");
    if let Some(rest) = source.strip_prefix("\"use strict\";\n") {
        return format!("\"use strict\";\n{declaration}{rest}");
    }
    if let Some(rest) = source.strip_prefix("\"use strict\";") {
        return format!("\"use strict\";\n{declaration}{rest}");
    }
    if let Some(rest) = source.strip_prefix("'use strict';\n") {
        return format!("'use strict';\n{declaration}{rest}");
    }
    if let Some(rest) = source.strip_prefix("'use strict';") {
        return format!("'use strict';\n{declaration}{rest}");
    }
    format!("{declaration}{source}")
}

fn escape_js_string(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

fn print_raw(value: &Value) {
    match value {
        Value::String(value) => println!("{value}"),
        Value::Number(value) => println!("{value}"),
        Value::Boolean(value) => println!("{value}"),
        Value::Null => println!("null"),
        Value::Undefined => println!("undefined"),
        _ => println!("{value:?}"),
    }
}

#[cfg(test)]
mod tests {
    use qjs_runtime::{EvalError, EvalErrorKind};

    use super::{format_test262_error, with_script_args};

    #[test]
    fn inserts_script_args_after_use_strict_directive() {
        let source = "\"use strict\";\nthis === undefined;";
        let wrapped = with_script_args(source, &["case.js".to_owned()]);

        assert!(wrapped.starts_with("\"use strict\";\nvar scriptArgs = [\"case.js\"];\n"));
    }

    #[test]
    fn inserts_script_args_after_same_line_use_strict_directive() {
        let source = "'use strict';this === undefined;";
        let wrapped = with_script_args(source, &["case.js".to_owned()]);

        assert!(wrapped.starts_with("'use strict';\nvar scriptArgs = [\"case.js\"];\n"));
    }

    #[test]
    fn formats_test262_error_stage_and_type() {
        let parse = format_test262_error(EvalError {
            kind: EvalErrorKind::Parse,
            message: "expected identifier".to_owned(),
        });
        assert_eq!(
            parse.message,
            "kind=parse type=SyntaxError message=expected identifier"
        );

        let runtime = format_test262_error(EvalError {
            kind: EvalErrorKind::Runtime,
            message: "throw statement executed: TypeError: incompatible receiver".to_owned(),
        });
        assert_eq!(
            runtime.message,
            "kind=runtime type=TypeError message=throw statement executed: TypeError: incompatible receiver"
        );

        let test262 = format_test262_error(EvalError {
            kind: EvalErrorKind::Runtime,
            message: "throw statement executed: Test262Error".to_owned(),
        });
        assert_eq!(
            test262.message,
            "kind=runtime type=Test262Error message=throw statement executed: Test262Error"
        );
    }
}
