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
