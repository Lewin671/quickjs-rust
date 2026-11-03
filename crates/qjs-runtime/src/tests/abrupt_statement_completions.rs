//! Target selection and observable statement-list completion on abrupt exits.
use crate::{Value, eval};

#[test]
fn unlabeled_break_skips_label_only_contexts() {
    assert_eq!(
        eval(include_str!(
            "../../../../tests/fixtures/compare-qjs/unlabeled-break-through-labels.js"
        )),
        Ok(Value::String("w1,end|f0|a0,a1,a2|switch".to_owned().into()))
    );
    assert!(qjs_parser::parse_script("L:{break;}").is_err());
}

#[test]
fn catch_abrupt_exits_publish_the_accumulated_statement_value() {
    assert_eq!(
        eval(include_str!(
            "../../../../tests/fixtures/compare-qjs/catch-abrupt-completion-values.js"
        )),
        Ok(Value::String("6|6|6|6|6|6".to_owned().into()))
    );
}

fn text(s: &str) -> Value {
    Value::String(s.to_owned().into())
}

/// A try statement completes with UpdateEmpty(C, undefined) (ES2025
/// 14.15.3), so a break leaving its catch through an `if` or `with` carries
/// `undefined`, never a value the catch produced before the jump.
#[test]
fn catch_jumps_through_updateempty_statements_carry_undefined() {
    assert_eq!(
        eval(include_str!(
            "../../../../tests/fixtures/compare-qjs/catch-break-updateempty-completion.js"
        )),
        Ok(text(
            "undefined|undefined|undefined|undefined|undefined|undefined|2|8|undefined"
        ))
    );
    // An empty catch block's break is UpdateEmpty(break, undefined) too, so
    // the try block's earlier value is gone (node agrees; quickjs-ng keeps 3).
    assert_eq!(
        eval("var w = 0; while (w++ < 2) { 5; try { 3; throw 0 } catch { break; } }"),
        Ok(Value::Undefined)
    );
    assert_eq!(
        eval("var w = 0; while (w++ < 2) { 5; try { 3; throw 0 } catch { } }"),
        Ok(Value::Undefined)
    );
}

/// A finally block that completes normally yields the completion it
/// interrupted: its own statements' values never leak into the enclosing
/// statement list, while a break leaving the finally carries its value.
#[test]
fn finally_normal_completion_never_replaces_the_interrupted_one() {
    assert_eq!(
        eval(include_str!(
            "../../../../tests/fixtures/compare-qjs/finally-normal-completion-isolation.js"
        )),
        Ok(text(
            "undefined|undefined|undefined|undefined|2|undefined|3|undefined|2|3"
        ))
    );
}

/// A labelled block shares its enclosing statement list's completion: a
/// break to it completes it with the break's (possibly empty) value.
#[test]
fn labelled_block_breaks_complete_with_the_carried_value() {
    assert_eq!(
        eval(include_str!(
            "../../../../tests/fixtures/compare-qjs/labelled-block-break-completion.js"
        )),
        Ok(text("undefined|s|undefined|undefined|1|1|2|1|3|undefined"))
    );
}

/// Declarations, empty blocks and empty labelled statements complete empty
/// and keep the previous value; breakable and conditional statements turn
/// an empty completion into `undefined`.
#[test]
fn empty_completions_keep_the_previous_statement_value() {
    assert_eq!(
        eval(include_str!(
            "../../../../tests/fixtures/compare-qjs/empty-statement-completion.js"
        )),
        Ok(text(
            "1|1|undefined|1|1|1|1|1|1|1|undefined|undefined|undefined|undefined|undefined|undefined|undefined|undefined"
        ))
    );
}
