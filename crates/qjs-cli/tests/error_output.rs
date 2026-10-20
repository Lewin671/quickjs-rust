//! Uncaught-error messages reach stderr as host UTF-8, whatever internal form
//! the engine's message text has.

use std::process::Command;

fn stderr_bytes(args: &[&str]) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_qjs"))
        .args(args)
        .output()
        .expect("qjs runs");
    assert!(!output.status.success(), "{output:?}");
    output.stderr
}

#[test]
fn uncaught_error_messages_recombine_surrogate_pairs_built_from_halves() {
    let stderr = stderr_bytes(&["-e", r#"throw new Error("x" + "\uD83D" + "\uDE00")"#]);
    let text = std::str::from_utf8(&stderr).expect("stderr is UTF-8");
    assert!(text.ends_with("Error: x😀\n"), "{text:?}");

    let stderr = stderr_bytes(&["-e", r#"throw "y" + "\uD83D" + "\uDE00""#]);
    let text = std::str::from_utf8(&stderr).expect("stderr is UTF-8");
    assert!(text.ends_with(": y😀\n"), "{text:?}");
}

#[test]
fn uncaught_error_messages_replace_lone_surrogates() {
    let stderr = stderr_bytes(&["-e", r#"throw new Error("a\uDE00b")"#]);
    let text = std::str::from_utf8(&stderr).expect("stderr is UTF-8");
    assert!(text.ends_with("Error: a\u{FFFD}b\n"), "{text:?}");
}

#[test]
fn harness_formatted_errors_are_host_utf8() {
    let stderr = stderr_bytes(&[
        "--error-format=test262",
        "-e",
        r#"throw new TypeError("z" + "\uD83D" + "\uDE00")"#,
    ]);
    let text = std::str::from_utf8(&stderr).expect("stderr is UTF-8");
    assert!(text.contains("type=TypeError"), "{text:?}");
    assert!(text.ends_with("z😀\n"), "{text:?}");
}
