//! Command-line entry point for the rewrite.

use std::{env, fs, process::ExitCode};

use qjs_runtime::eval;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1).collect::<Vec<_>>().into_iter();
    let raw_output = matches!(args.as_slice().first().map(String::as_str), Some("--raw"));
    if raw_output {
        args.next();
    }

    let Some(first) = args.next() else {
        return Err("usage: qjs [--raw] (-e <source> | <file> [script-arg...])".to_owned());
    };

    let (source, script_args) = if first == "-e" {
        let source = args
            .next()
            .ok_or_else(|| "missing source after -e".to_owned())?;
        (source, vec!["-e".to_owned()])
    } else {
        let source = fs::read_to_string(&first)
            .map_err(|error| format!("failed to read `{first}`: {error}"))?;
        let script_args = std::iter::once(first).chain(args).collect();
        (source, script_args)
    };

    let source = with_script_args(&source, &script_args);
    let value = eval(&source).map_err(|error| error.message)?;
    if raw_output {
        print_raw(&value);
    } else {
        println!("{value:?}");
    }
    Ok(())
}

fn with_script_args(source: &str, script_args: &[String]) -> String {
    let args = script_args
        .iter()
        .map(|arg| format!("\"{}\"", escape_js_string(arg)))
        .collect::<Vec<_>>()
        .join(", ");
    format!("var scriptArgs = [{args}];\n{source}")
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

fn print_raw(value: &qjs_runtime::Value) {
    match value {
        qjs_runtime::Value::String(value) => println!("{value}"),
        qjs_runtime::Value::Number(value) => println!("{value}"),
        qjs_runtime::Value::Boolean(value) => println!("{value}"),
        qjs_runtime::Value::Null => println!("null"),
        qjs_runtime::Value::Undefined => println!("undefined"),
        _ => println!("{value:?}"),
    }
}
