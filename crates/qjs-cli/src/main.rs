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
    let mut args = env::args().skip(1);
    let Some(first) = args.next() else {
        return Err("usage: qjs (-e <source> | <file>)".to_owned());
    };

    let source = if first == "-e" {
        args.next()
            .ok_or_else(|| "missing source after -e".to_owned())?
    } else {
        fs::read_to_string(&first).map_err(|error| format!("failed to read `{first}`: {error}"))?
    };

    let value = eval(&source).map_err(|error| error.message)?;
    println!("{value:?}");
    Ok(())
}
