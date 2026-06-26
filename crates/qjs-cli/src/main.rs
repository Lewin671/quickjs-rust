//! Command-line entry point for the rewrite.

use std::{
    env, fs,
    io::{self, BufRead, IsTerminal, Write},
    path::{Path, PathBuf},
    process::ExitCode,
};

use qjs_runtime::{
    EvalError, EvalErrorKind, ModuleResolveError, ModuleResolver, ResolvedModule, Value,
    eval_classified_with_resolver, eval_module_with_prelude,
};

fn usage(command: &str) -> String {
    format!(
        "usage: {command} [--raw] [--error-format=test262] [--interactive] | {command} [--raw] [--error-format=test262] [--module [--prelude <file>]] (-e <source> | <file> [script-arg...])"
    )
}

fn help(command: &str) -> String {
    format!(
        "\
quickjs-rust command-line host

Usage:
  {command} [--raw] [--error-format=test262] [--interactive]
  {command} [--raw] [--error-format=test262] (-e <source> | <file> [script-arg...])
  {command} [--raw] [--error-format=test262] --module [--prelude <file>] <module.mjs>

Options:
  -e <source>                 Evaluate source text as a script
  -i, --interactive, --repl   Start an interactive shell
  --module                    Evaluate the input file as an ECMAScript module
  --prelude <file>            Evaluate a script prelude before a module
  --raw                       Print JavaScript string values without Rust debug formatting
  --error-format=test262      Print harness-friendly error metadata
  --agent                     Enable the Test262 $262.agent harness in agents builds
  --agent-cannot-block        Make AgentCanSuspend() false in agents builds
  -h, --help                  Show this help text
  -V, --version               Show the CLI version
"
    )
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {}", error.message);
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug)]
struct CliError {
    message: String,
}

fn run() -> Result<(), CliError> {
    let command = command_name();
    let mut args = env::args().skip(1).collect::<Vec<_>>().into_iter();
    let mut raw_output = false;
    let mut test262_error_format = false;
    let mut module_mode = false;
    let mut interactive_mode = false;
    let mut prelude_path: Option<String> = None;
    // Test262 `$262.agent` harness: `--agent` runs the script as the main agent
    // (its `$262.agent.*` primitives become available); `--agent-cannot-block`
    // additionally makes `AgentCanSuspend()` false so `Atomics.wait` throws
    // (the `CanBlockIsFalse` flag). Both are parsed unconditionally (so the
    // harness flags are never "unknown arguments") and only change behavior in
    // the `agents`-feature build.
    let mut agent_mode = false;
    let mut agent_cannot_block = false;
    loop {
        match args.as_slice().first().map(String::as_str) {
            Some("--raw") => {
                raw_output = true;
                args.next();
            }
            Some("-h" | "--help") => {
                print!("{}", help(&command));
                return Ok(());
            }
            Some("-V" | "--version") => {
                println!("{command} {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            Some("--error-format=test262") => {
                test262_error_format = true;
                args.next();
            }
            Some("-i" | "--interactive" | "--repl") => {
                interactive_mode = true;
                args.next();
            }
            Some("--agent") => {
                agent_mode = true;
                args.next();
            }
            Some("--agent-cannot-block") => {
                agent_mode = true;
                agent_cannot_block = true;
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

    if interactive_mode {
        if module_mode {
            return Err(CliError {
                message: "--interactive cannot be combined with --module".to_owned(),
            });
        }
        if prelude_path.is_some() {
            return Err(CliError {
                message: "--interactive cannot be combined with --prelude".to_owned(),
            });
        }
        if !args.as_slice().is_empty() {
            return Err(CliError {
                message: "--interactive does not accept a file or -e source".to_owned(),
            });
        }
        return run_repl(
            raw_output,
            test262_error_format,
            agent_mode,
            agent_cannot_block,
        );
    }

    let Some(first) = args.next() else {
        if module_mode {
            return Err(CliError {
                message: usage(&command),
            });
        }
        return run_repl(
            raw_output,
            test262_error_format,
            agent_mode,
            agent_cannot_block,
        );
    };

    if module_mode {
        return run_module(&first, prelude_path.as_deref(), test262_error_format);
    }

    let (source, script_args, referrer) = if first == "-e" {
        let source = args.next().ok_or_else(|| CliError {
            message: "missing source after -e".to_owned(),
        })?;
        // A `-e` script has no file: root its dynamic-import referrer at a
        // synthetic file in the current directory so relative specifiers resolve
        // against the cwd.
        let referrer = env::current_dir()
            .map(|dir| dir.join("<eval>").to_string_lossy().into_owned())
            .unwrap_or_else(|_| "<eval>".to_owned());
        (source, vec!["-e".to_owned()], referrer)
    } else {
        let source = fs::read_to_string(&first).map_err(|error| CliError {
            message: format!("failed to read `{first}`: {error}"),
        })?;
        // Resolve dynamic imports against the script file's directory.
        let referrer = fs::canonicalize(&first)
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|_| first.clone());
        let script_args = std::iter::once(first).chain(args).collect();
        (source, script_args, referrer)
    };

    let source = with_script_args(&source, &script_args);
    // A script may use dynamic `import()`; install a filesystem-backed host so
    // those imports resolve relative to the script (or the cwd for `-e`).
    let result = eval_script(&source, &referrer, agent_mode, agent_cannot_block);
    let value = if test262_error_format {
        result.map_err(format_test262_error)?
    } else {
        result.map_err(|error| CliError {
            message: error.message,
        })?
    };
    println!("{}", format_value(&value, raw_output));
    Ok(())
}

fn command_name() -> String {
    env::args()
        .next()
        .and_then(|arg| {
            Path::new(&arg)
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| "qjs".to_owned())
}

/// Evaluates script-goal `source`. When `agent_mode` is set (Test262
/// `$262.agent` harness), the `agents`-feature build runs it as the main agent
/// so `$262.agent.*` works; `agent_cannot_block` makes `AgentCanSuspend()` false
/// so `Atomics.wait` throws. Outside the agents build both flags are inert.
fn eval_script(
    source: &str,
    referrer: &str,
    agent_mode: bool,
    agent_cannot_block: bool,
) -> Result<Value, EvalError> {
    #[cfg(feature = "agents")]
    if agent_mode {
        return qjs_runtime::eval_classified_with_resolver_in_agent(
            source,
            referrer,
            Box::new(FsResolver),
            !agent_cannot_block,
        );
    }
    let _ = (agent_mode, agent_cannot_block);
    eval_classified_with_resolver(source, referrer, Box::new(FsResolver))
}

fn run_repl(
    raw_output: bool,
    test262_error_format: bool,
    agent_mode: bool,
    agent_cannot_block: bool,
) -> Result<(), CliError> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let prompts = stdin.is_terminal() && stdout.is_terminal();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    run_repl_with_io(
        &mut input,
        &mut output,
        prompts,
        raw_output,
        test262_error_format,
        agent_mode,
        agent_cannot_block,
    )
}

fn run_repl_with_io<R: BufRead, W: Write>(
    input: &mut R,
    output: &mut W,
    prompts: bool,
    raw_output: bool,
    test262_error_format: bool,
    agent_mode: bool,
    agent_cannot_block: bool,
) -> Result<(), CliError> {
    let referrer = env::current_dir()
        .map(|dir| dir.join("<repl>").to_string_lossy().into_owned())
        .unwrap_or_else(|_| "<repl>".to_owned());
    let script_args = ["<repl>".to_owned()];
    let mut history: Vec<String> = Vec::new();
    let mut line = String::new();

    loop {
        if prompts {
            write!(output, "qjs> ").map_err(io_error)?;
            output.flush().map_err(io_error)?;
        }

        line.clear();
        let bytes = input.read_line(&mut line).map_err(io_error)?;
        if bytes == 0 {
            break;
        }

        let line_source = line.trim_end_matches(['\n', '\r']);
        let command = line_source.trim();
        if command.is_empty() {
            continue;
        }
        if matches!(command, ".exit" | ".quit") {
            break;
        }

        let mut source = String::new();
        for entry in &history {
            source.push_str(entry);
            source.push('\n');
        }
        source.push_str(line_source);
        let source = with_script_args(&source, &script_args);
        let result = eval_script(&source, &referrer, agent_mode, agent_cannot_block);
        match result {
            Ok(value) => {
                history.push(line_source.to_owned());
                writeln!(output, "{}", format_value(&value, raw_output)).map_err(io_error)?;
            }
            Err(error) if test262_error_format => {
                let error = format_test262_error(error);
                writeln!(output, "error: {}", error.message).map_err(io_error)?;
            }
            Err(error) => {
                writeln!(output, "error: {}", error.message).map_err(io_error)?;
            }
        }
    }

    Ok(())
}

fn io_error(error: io::Error) -> CliError {
    CliError {
        message: error.to_string(),
    }
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
    let result =
        eval_module_with_prelude(prelude.as_deref(), &source, &root_key, Box::new(FsResolver));
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
        let bytes = fs::read(&canonical).map_err(|error| ModuleResolveError {
            message: format!("Cannot load module '{specifier}': {error}"),
        })?;
        let source = String::from_utf8_lossy(&bytes).into_owned();
        Ok(ResolvedModule { key, source, bytes })
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
    let (hashbang, source) = split_hashbang_prefix(source);
    // Inject the declaration *after* the directive prologue, not before it, so a
    // leading `"use strict"` keeps its directive status. The prologue may be
    // preceded and interleaved by comments (every Test262 file opens with a
    // license/metadata comment block), which a literal prefix match misses; that
    // misplacement demoted the directive and silently dropped strict mode.
    let prologue_end = directive_prologue_end(source);
    let (prologue, rest) = source.split_at(prologue_end);
    let separator = if prologue.is_empty() || prologue.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    format!("{hashbang}{prologue}{separator}{declaration}{rest}")
}

/// Returns the byte offset at the end of `source`'s directive prologue: the
/// leading run of comments/whitespace and string-literal directive statements.
/// Injected top-level declarations placed here cannot demote a `"use strict"`
/// directive.
fn directive_prologue_end(source: &str) -> usize {
    let bytes = source.as_bytes();
    let mut index = 0;
    let mut prologue_end = 0;
    loop {
        index = skip_trivia(source, index);
        let Some(string_end) = string_literal_end(bytes, index) else {
            return prologue_end;
        };
        // The string is a directive only if it forms a complete statement:
        // terminated by `;`, end of input, or `}`, or — by ASI — followed across
        // a line break by a token that starts a new statement. A string that
        // continues an expression (`"x".length`, `"x" + y`) is not a directive,
        // and injecting after it would corrupt the program.
        let after = skip_trivia(source, string_end);
        let crossed_newline = source[string_end..after]
            .bytes()
            .any(|byte| byte == b'\n' || byte == b'\r');
        let consumed = match bytes.get(after).copied() {
            Some(b';') => after + 1,
            None | Some(b'}') => after,
            Some(byte) if crossed_newline && !is_expression_continuation(byte) => after,
            _ => return prologue_end,
        };
        prologue_end = consumed;
        index = consumed;
    }
}

/// Whether `byte` can begin a token that continues the expression a preceding
/// string literal started (so the string is not a complete directive statement).
fn is_expression_continuation(byte: u8) -> bool {
    matches!(
        byte,
        b'.' | b'('
            | b'['
            | b'+'
            | b'-'
            | b'*'
            | b'/'
            | b'%'
            | b'<'
            | b'>'
            | b'='
            | b'!'
            | b'&'
            | b'|'
            | b'^'
            | b'~'
            | b','
            | b'?'
            | b':'
            | b'`'
    )
}

/// Skips ASCII/JS whitespace, line terminators, and `//`/`/* */` comments.
fn skip_trivia(source: &str, mut index: usize) -> usize {
    let bytes = source.as_bytes();
    loop {
        match bytes.get(index) {
            Some(b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c) => index += 1,
            Some(b'/') if bytes.get(index + 1) == Some(&b'/') => {
                index += 2;
                while let Some(&ch) = bytes.get(index) {
                    if ch == b'\n' || ch == b'\r' {
                        break;
                    }
                    index += 1;
                }
            }
            Some(b'/') if bytes.get(index + 1) == Some(&b'*') => {
                index += 2;
                while index < bytes.len() {
                    if bytes[index] == b'*' && bytes.get(index + 1) == Some(&b'/') {
                        index += 2;
                        break;
                    }
                    index += 1;
                }
            }
            _ => return index,
        }
    }
}

/// If `bytes[index]` begins a `'`/`"` string literal, returns the byte offset
/// just past its closing quote; otherwise `None`.
fn string_literal_end(bytes: &[u8], index: usize) -> Option<usize> {
    let quote = match bytes.get(index) {
        Some(&q @ (b'"' | b'\'')) => q,
        _ => return None,
    };
    let mut cursor = index + 1;
    while let Some(&ch) = bytes.get(cursor) {
        match ch {
            b'\\' => cursor += 2,
            b'\n' | b'\r' => return None,
            c if c == quote => return Some(cursor + 1),
            _ => cursor += 1,
        }
    }
    None
}

fn split_hashbang_prefix(source: &str) -> (String, &str) {
    if !source.starts_with("#!") {
        return (String::new(), source);
    }
    for (index, ch) in source.char_indices() {
        if matches!(ch, '\n' | '\r' | '\u{2028}' | '\u{2029}') {
            let end = if ch == '\r' && source[index + ch.len_utf8()..].starts_with('\n') {
                index + 2
            } else {
                index + ch.len_utf8()
            };
            return (source[..end].to_owned(), &source[end..]);
        }
    }
    (format!("{source}\n"), "")
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

fn format_value(value: &Value, raw_output: bool) -> String {
    if !raw_output {
        return format!("{value:?}");
    }
    match value {
        Value::String(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Boolean(value) => value.to_string(),
        Value::Null => "null".to_owned(),
        Value::Undefined => "undefined".to_owned(),
        _ => format!("{value:?}"),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use qjs_runtime::{EvalError, EvalErrorKind};

    use super::{format_test262_error, run_repl_with_io, with_script_args};

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
    fn keeps_use_strict_first_when_preceded_by_a_comment() {
        // Every Test262 file opens with a license/metadata comment block; the
        // injected declaration must land after the directive, not before it.
        let source = "/*---\nmeta\n---*/\n\"use strict\";\neval = 42;";
        let wrapped = with_script_args(source, &["case.js".to_owned()]);

        let directive = wrapped.find("\"use strict\";").expect("directive kept");
        let injected = wrapped
            .find("var scriptArgs")
            .expect("declaration injected");
        assert!(
            directive < injected,
            "directive must precede the declaration"
        );
    }

    #[test]
    fn does_not_treat_a_leading_string_expression_as_a_directive() {
        // A string that continues an expression is not a directive; injecting
        // after it would corrupt the program.
        let source = "\"abc\".length;";
        let wrapped = with_script_args(source, &["case.js".to_owned()]);

        assert!(wrapped.starts_with("var scriptArgs = [\"case.js\"];\n\"abc\".length;"));
    }

    #[test]
    fn inserts_script_args_after_hashbang() {
        let source = "#!/usr/bin/env qjs\nanswer;";
        let wrapped = with_script_args(source, &["case.js".to_owned()]);

        assert!(wrapped.starts_with("#!/usr/bin/env qjs\nvar scriptArgs = [\"case.js\"];\n"));
    }

    #[test]
    fn inserts_script_args_after_hashbang_and_use_strict_directive() {
        let source = "#!/usr/bin/env qjs\r\n\"use strict\";\nthis === undefined;";
        let wrapped = with_script_args(source, &["case.js".to_owned()]);

        assert!(wrapped.starts_with(
            "#!/usr/bin/env qjs\r\n\"use strict\";\nvar scriptArgs = [\"case.js\"];\n"
        ));
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

    #[test]
    fn repl_evaluates_lines_until_exit_command() {
        let mut input = Cursor::new("1 + 2\n'hello'\n.exit\n");
        let mut output = Vec::new();

        run_repl_with_io(&mut input, &mut output, false, true, false, false, false)
            .expect("repl should evaluate input");

        assert_eq!(String::from_utf8(output).unwrap(), "3\nhello\n");
    }

    #[test]
    fn repl_reports_errors_and_continues() {
        let mut input = Cursor::new("missingName\n1 + 1\n.quit\n");
        let mut output = Vec::new();

        run_repl_with_io(&mut input, &mut output, false, true, false, false, false)
            .expect("repl should keep reading after evaluation errors");

        let output = String::from_utf8(output).unwrap();
        assert!(output.contains("error:"));
        assert!(output.ends_with("2\n"));
    }

    #[test]
    fn repl_keeps_successful_input_history() {
        let mut input = Cursor::new("let answer = 41\nanswer + 1\n.quit\n");
        let mut output = Vec::new();

        run_repl_with_io(&mut input, &mut output, false, true, false, false, false)
            .expect("repl should evaluate later input with earlier declarations");

        assert_eq!(String::from_utf8(output).unwrap(), "undefined\n42\n");
    }
}
