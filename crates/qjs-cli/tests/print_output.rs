//! `print` writes host UTF-8, whatever internal form the printed string has.

use std::process::Command;

fn print_bytes(source: &str) -> Vec<u8> {
    let output = Command::new(env!("CARGO_BIN_EXE_qjs"))
        .args(["-e", source])
        .output()
        .expect("qjs runs");
    assert!(output.status.success(), "{output:?}");
    let line_end = output
        .stdout
        .iter()
        .position(|&byte| byte == b'\n')
        .expect("print ends its line");
    output.stdout[..line_end].to_vec()
}

#[test]
fn print_recombines_surrogate_pairs_built_from_separate_halves() {
    // Concatenated halves and Unicode-mode empty-match replacement both
    // produce a pair whose halves are stored separately.
    assert_eq!(
        print_bytes(r#"print("-\uD83D" + "\uDE00")"#),
        "-😀".as_bytes()
    );
    assert_eq!(
        print_bytes(r#"print("\u{1F600}\u{1F600}".replace(/x*/gu, "-"))"#),
        "-😀-😀-".as_bytes()
    );
    assert_eq!(
        print_bytes(r#"print("a", "\u{1F600}b".replace(/b/u, "c"))"#),
        "a 😀c".as_bytes()
    );
}

#[test]
fn print_replaces_lone_surrogates() {
    assert_eq!(
        print_bytes(r#"print("a\uDE00b", "\uD83D")"#),
        "a\u{FFFD}b \u{FFFD}".as_bytes()
    );
}
